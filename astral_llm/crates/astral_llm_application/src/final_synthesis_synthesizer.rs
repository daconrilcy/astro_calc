//! Chapitre de synthese integrative finale (profils premium_plus).

use std::time::Duration;

use astral_llm_domain::{
    chapter_orchestration::ReadingPlanChapter,
    generation_response::{ChapterProviderResponse, ReadingChapter},
    interpretation_profile::SYNTHESIS_CHAPTER_CODE,
    interpretive_evidence::ChapterEvidencePack,
    model_usage_tier::ModelRouteContext,
    output_contract::ChapterContract,
    GenerateReadingRequest, GenerationError, GenerationErrorCode, NormalizedAstroFacts,
    ProductGenerationPolicy, SafetyMode, SafetyPolicy, TokenUsage, TokenUsageType,
};
use astral_llm_infra::SharedCanonicalCatalog;
use astral_llm_providers::{GenerationMetadata, ProviderGenerationRequest};

use crate::astro_basis_role_normalizer::AstroBasisRoleNormalizer;
use crate::astro_basis_validator::AstroBasisValidator;
use crate::astro_label_humanizer::AstroLabelHumanizer;
use crate::astro_payload_normalizer::AstroPayloadNormalizer;
use crate::chapter_evidence_basis_enricher::ChapterEvidenceBasisEnricher;
use crate::chapter_evidence_coherence::ChapterEvidenceCoherence;
use crate::chapter_quality_repair::{ChapterRepairKind, TooShortRepairMode};
use crate::engine_defaults::ResolvedEngineParams;
use crate::interpretation_profile_resolver::ResolvedInterpretationContext;
use crate::product_policy_validator::ProductPolicyValidator;
use crate::prompt_trace;
use crate::provider_router::ProviderRouter;
use crate::provider_schema_compiler::pin_chapter_code;
use crate::provider_schema_compiler::ProviderSchemaCompiler;
use crate::reading_catalog::AstroBasisRoleCatalogView;
use crate::reasoning_generation::{effective_temperature, resolve_reasoning_effort};
use crate::response_validator::ResponseValidator;
use crate::safety_guard::SafetyGuard;
use crate::token_budget::TokenBudget;
use crate::writing_language::WritingLanguageDirective;
use astral_llm_providers::PromptMessage;

pub struct FinalSynthesisResult {
    pub chapter: ReadingChapter,
    pub token_usage: Option<TokenUsage>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

pub struct FinalSynthesisSynthesizer<'a> {
    router: &'a ProviderRouter,
    validator: &'a ResponseValidator,
    catalog: &'a SharedCanonicalCatalog,
}

impl<'a> FinalSynthesisSynthesizer<'a> {
    pub fn new(
        router: &'a ProviderRouter,
        validator: &'a ResponseValidator,
        catalog: &'a SharedCanonicalCatalog,
    ) -> Self {
        Self {
            router,
            validator,
            catalog,
        }
    }

    pub async fn synthesize(
        &self,
        request: &GenerateReadingRequest,
        prior_chapters: &[ReadingChapter],
        chapter: &ReadingPlanChapter,
        contract: &ChapterContract,
        chapter_pack: Option<&ChapterEvidencePack>,
        astro_facts: &NormalizedAstroFacts,
        engine: &ResolvedEngineParams,
        safety_policy: &SafetyPolicy,
        product_policy: &ProductGenerationPolicy,
        interpretation: Option<&ResolvedInterpretationContext>,
        run_id: &str,
        repair: Option<ChapterRepairKind>,
    ) -> Result<FinalSynthesisResult, GenerationError> {
        let messages = build_synthesis_messages(
            request,
            prior_chapters,
            chapter,
            chapter_pack,
            astro_facts,
            self.catalog,
            interpretation,
            repair.as_ref(),
        );
        prompt_trace::log_provider_messages(
            run_id,
            Some(SYNTHESIS_CHAPTER_CODE),
            None,
            None,
            Some("final_synthesis"),
            &messages,
        );

        self.router
            .capability_registry()
            .validate_engine_for_context(
                ModelRouteContext::PrimaryReading,
                &engine.provider,
                &engine.model,
                engine.allow_oracle_benchmark,
            )?;
        let model_cap = self
            .router
            .capability_registry()
            .require(&engine.provider, &engine.model)?;

        ProductPolicyValidator::validate_against_policy(
            request,
            product_policy,
            &engine.provider,
            &engine.model,
        )?;

        let canonical_schema = self
            .validator
            .schema_registry()
            .provider_schema("chapter_provider_v1")
            .cloned();
        let mut provider_schema = canonical_schema.ok_or_else(|| {
            GenerationError::new(
                GenerationErrorCode::SchemaValidationFailed,
                "chapter_provider_v1 schema missing",
            )
        })?;
        pin_chapter_code(&mut provider_schema, SYNTHESIS_CHAPTER_CODE);
        let schema = ProviderSchemaCompiler::compile(&provider_schema, model_cap)?;

        let provider_request = ProviderGenerationRequest {
            model: engine.model.clone(),
            messages,
            structured_schema: Some(schema),
            reasoning_effort: resolve_reasoning_effort(
                model_cap,
                product_policy,
                engine.reasoning_effort,
                ModelRouteContext::PrimaryReading,
            ),
            temperature: effective_temperature(model_cap, engine.temperature),
            max_output_tokens: Some(TokenBudget::chapter_max_tokens(
                contract,
                request
                    .engine
                    .max_output_tokens
                    .or(request.response_contract.global_max_tokens),
                model_cap,
            )),
            safety_mode: resolve_safety_mode(&engine.provider),
            timeout: Duration::from_millis(engine.timeout_ms.unwrap_or(900_000)),
            metadata: GenerationMetadata {
                run_id: run_id.to_string(),
                request_id: request.request_id.clone(),
                product_code: request.product_context.product_code.clone(),
                chapter_code: Some(SYNTHESIS_CHAPTER_CODE.into()),
                prompt_trace_step: Some("final_synthesis_generate".into()),
                prompt_trace_attempt: Some(
                    if repair.is_some() {
                        "repair"
                    } else {
                        "final_synthesis"
                    }
                    .into(),
                ),
                prompt_family: None,
                prompt_version: None,
            },
        };

        let route = self
            .router
            .generate(
                provider_request,
                engine.provider.clone(),
                &engine.model,
                engine.allow_fallback,
                true,
                ModelRouteContext::PrimaryReading,
            )
            .await?;

        let json = route.response.parsed_json.ok_or_else(|| {
            GenerationError::new(
                GenerationErrorCode::InvalidJsonOutput,
                "provider returned no JSON for final synthesis chapter",
            )
        })?;

        self.validator.validate_chapter(&json)?;

        let mut chapter_reading: ChapterProviderResponse =
            serde_json::from_value(json).map_err(|e| {
                GenerationError::new(
                    GenerationErrorCode::InvalidJsonOutput,
                    format!("final synthesis deserialization failed: {e}"),
                )
            })?;

        if chapter_reading.code != SYNTHESIS_CHAPTER_CODE {
            tracing::warn!(
                expected = SYNTHESIS_CHAPTER_CODE,
                received = %chapter_reading.code,
                "final synthesis chapter code normalized after provider drift"
            );
            chapter_reading.code = SYNTHESIS_CHAPTER_CODE.into();
        }

        let mut reading_chapter = ReadingChapter {
            code: chapter_reading.code,
            title: chapter_reading.title,
            body: chapter_reading.body,
            astro_basis: chapter_reading.astro_basis,
            confidence: chapter_reading.confidence,
            safety_flags: vec![],
        };

        AstroBasisRoleNormalizer::normalize_chapter(&mut reading_chapter, chapter_pack);
        if let Some(pack) = chapter_pack {
            ChapterEvidenceBasisEnricher::enrich_missing_pack_slots(&mut reading_chapter, pack);
            AstroBasisRoleNormalizer::normalize_chapter(&mut reading_chapter, chapter_pack);
        }

        AstroLabelHumanizer::new(self.catalog).enrich_chapter_astro_basis(
            &mut reading_chapter.astro_basis,
            astro_facts,
            &request.product_context.user_language,
        );
        crate::evidence_fact_parse::normalize_chapter_astro_basis_fact_ids_with_catalog(
            &mut reading_chapter,
            astro_facts,
            self.catalog.as_ref(),
            &request.product_context.user_language,
        );

        AstroBasisValidator::validate_chapter_with_pack(
            &reading_chapter,
            astro_facts,
            chapter_pack,
            AstroBasisRoleCatalogView::new(self.catalog),
            product_policy,
        )?;

        reading_chapter.body = crate::safety_guard::ensure_symbolic_framing_text(
            &reading_chapter.body,
            &request.product_context.user_language,
            self.catalog,
        );

        if let Some(pack) = chapter_pack {
            ChapterEvidenceCoherence::validate_premium(
                &reading_chapter,
                pack,
                self.catalog.as_ref(),
                &request.product_context.user_language,
            )?;
        }

        SafetyGuard::validate_chapter_text(
            &reading_chapter.body,
            safety_policy,
            &request.astrologer_profile.forbidden_wording,
            self.catalog,
        )
        .map_err(|violations| {
            GenerationError::with_details(
                GenerationErrorCode::PostSafetyValidationFailed,
                "final synthesis chapter failed safety validation",
                serde_json::json!({ "violations": violations }),
            )
        })?;

        let input_tokens = route
            .response
            .usage
            .as_ref()
            .and_then(|u| u.tokens_for(TokenUsageType::Input));
        let output_tokens = route
            .response
            .usage
            .as_ref()
            .and_then(|u| u.tokens_for(TokenUsageType::Output));

        Ok(FinalSynthesisResult {
            chapter: reading_chapter,
            token_usage: route.response.usage.clone(),
            input_tokens,
            output_tokens,
        })
    }
}

fn build_synthesis_messages(
    request: &GenerateReadingRequest,
    prior_chapters: &[ReadingChapter],
    chapter: &ReadingPlanChapter,
    pack: Option<&ChapterEvidencePack>,
    astro_facts: &NormalizedAstroFacts,
    catalog: &SharedCanonicalCatalog,
    interpretation: Option<&ResolvedInterpretationContext>,
    repair: Option<&ChapterRepairKind>,
) -> Vec<PromptMessage> {
    let language_block =
        WritingLanguageDirective::prompt_block(catalog, &request.product_context.user_language);
    let public_abbreviation_rule =
        WritingLanguageDirective::public_abbreviation_rule(&request.product_context.user_language);
    let chapter_digest: Vec<serde_json::Value> = prior_chapters
        .iter()
        .map(|c| {
            serde_json::json!({
                "code": c.code,
                "title": c.title,
                "body_excerpt": truncate_words(&c.body, 120),
            })
        })
        .collect();

    let evidence_block = pack.map(|p| {
        AstroPayloadNormalizer::to_chapter_evidence_pack_block(
            p,
            catalog,
            &request.product_context.user_language,
            astro_facts,
        )
    });

    let task_fragment = interpretation
        .and_then(|ctx| ctx.profile.document.task_fragment.clone())
        .unwrap_or_default();

    let body_structure = interpretation.and_then(|ctx| ctx.profile.body_structure());
    let paragraph_count = body_structure.map(|bs| bs.paragraph_count).unwrap_or(6);
    let (para_min_w, para_max_w) = body_structure
        .map(|bs| (bs.paragraph_min_words, bs.paragraph_max_words))
        .unwrap_or((80, 120));
    let (min_w, target_w, max_w) = interpretation
        .map(|ctx| ctx.profile.synthesis_word_targets())
        .unwrap_or((
            chapter.min_words as u16,
            chapter.target_words as u16,
            chapter.max_words as u16,
        ));
    let system = format!(
        "{language_block}\n\n\
         {public_abbreviation_rule}\n\n\
         Write the final integrative synthesis chapter of a natal reading (code: synthesis). \
         Output JSON matching chapter_provider_v1 (code, title, body, astro_basis, confidence). \
         This is NOT a new astrological domain chapter: weave together themes from prior chapters. \
         Cover: guiding line of the chart, main tensions, dominant resources, symbolic non-prescriptive counsel, closing phrase. \
         Use exactly {paragraph_count} editorial paragraphs ({para_min_w}-{para_max_w} words each; \
         total body {min_w}-{max_w} words, target ~{target_w}). \
         Frame as symbolic and interpretive throughout the body: use non-deterministic language \
         (French: symbolique, suggère, peut, invite, tendance, met en lumière; \
         English: symbolic, suggests, may, invites). \
         Avoid categorical predictions or prescriptive advice. {task_fragment}"
    );

    let repair_block = repair
        .map(|r| synthesis_repair_directive(chapter, r))
        .unwrap_or_default();

    let user = format!(
        "Product: {}\nAudience: {:?}\n\nPrior chapters:\n{}\n\n\
         Global evidence pack (cite fact_ids in astro_basis when used):\n{}\n\
         {repair_block}\n\
         Write title, body and astro_basis for chapter code \"synthesis\".",
        request.product_context.product_code,
        request.product_context.audience_level,
        serde_json::to_string_pretty(&chapter_digest).unwrap_or_default(),
        evidence_block
            .map(|v| serde_json::to_string_pretty(&v).unwrap_or_default())
            .unwrap_or_else(|| "{}".into()),
    );

    vec![
        PromptMessage {
            role: astral_llm_providers::PromptRole::System,
            content: system,
        },
        PromptMessage {
            role: astral_llm_providers::PromptRole::User,
            content: user,
        },
    ]
}

fn synthesis_repair_directive(chapter: &ReadingPlanChapter, repair: &ChapterRepairKind) -> String {
    match repair {
        ChapterRepairKind::TooShort {
            words,
            min_words,
            target_words,
            mode,
        } => {
            let mode_hint = match mode {
                TooShortRepairMode::ExpandSameChapter => {
                    "EXPAND MODE: keep title and astro_basis; lengthen existing paragraphs."
                }
                TooShortRepairMode::RewriteChapter => {
                    "REWRITE MODE: rewrite the full body while keeping all fact_ids valid."
                }
            };
            format!(
                "\nREPAIR: synthesis chapter is only {words} words; expand to at least {min_words} \
                 (target ~{target_words}). {mode_hint} \
                 Integrate prior chapter themes without repeating openings."
            )
        }
        ChapterRepairKind::Repetition { score, max_allowed } => format!(
            "\nREPAIR: synthesis repetition score {score} exceeds {max_allowed}; vary vocabulary \
             and sentence openings while keeping fact_ids valid."
        ),
        ChapterRepairKind::SymbolicFraming => {
            "\nREPAIR: synthesis body lacks symbolic/interpretive framing. Rewrite with explicit \
             non-deterministic language (French: symbolique, suggère, peut, invite, tendance, \
             met en lumière). Keep fact_ids valid; avoid prescriptive advice."
                .into()
        }
        _ => format!(
            "\nREPAIR: rewrite chapter '{}' addressing the quality issue noted above.",
            chapter.code
        ),
    }
}

fn truncate_words(text: &str, max_words: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().take(max_words).collect();
    let mut out = words.join(" ");
    if text.split_whitespace().count() > max_words {
        out.push_str("…");
    }
    out
}

fn resolve_safety_mode(provider: &astral_llm_domain::ProviderKind) -> SafetyMode {
    if matches!(provider, astral_llm_domain::ProviderKind::Mistral) {
        SafetyMode::PlatformAndNative
    } else {
        SafetyMode::PlatformRulesOnly
    }
}
