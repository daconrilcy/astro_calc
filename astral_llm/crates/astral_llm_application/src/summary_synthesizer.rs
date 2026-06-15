//! Synthese finale personnalisee apres generation chapitre par chapitre.

use std::time::Duration;

use astral_llm_domain::{
    generation_response::{ReadingChapter, ReadingSummary, SummaryProviderResponse},
    model_usage_tier::ModelRouteContext,
    GenerateReadingRequest, GenerationError, GenerationErrorCode, SafetyMode, SafetyPolicy,
    TokenUsage, TokenUsageType,
};
use astral_llm_infra::SharedCanonicalCatalog;
use astral_llm_providers::{
    GenerationMetadata, PromptMessage, PromptRole, ProviderGenerationRequest,
};

use crate::engine_defaults::ResolvedEngineParams;
use crate::product_policy_validator::ProductPolicyValidator;
use crate::prompt_trace;
use crate::provider_router::ProviderRouter;
use crate::provider_schema_compiler::ProviderSchemaCompiler;
use crate::reasoning_generation::{
    apply_reasoning_output_reserve, effective_temperature, resolve_reasoning_effort,
    SUBTASK_BASE_OUTPUT_TOKENS,
};
use crate::response_validator::ResponseValidator;
use crate::safety_guard::SafetyGuard;
use crate::summary_forbidden_patterns::find_forbidden_summary_patterns;
use crate::summary_ux_rules::{count_words, validate_summary_ux, SummaryUxRules};
use crate::writing_language::WritingLanguageDirective;

use astral_llm_domain::chapter_orchestration::READING_SUMMARY_STEP_CODE;

const BANNED_TECHNICAL_SUMMARY_PATTERNS: &[&str] = &[
    "synthese produite par",
    "synthèse produite par",
    "generation chapitre par chapitre",
    "génération chapitre par chapitre",
    "lecture natal_premium",
    "lecture natal_basic",
    "lecture natal_prompter",
    "chapter_orchestrated",
    "single_pass",
    "pipeline technique",
    "placeholder",
];

const SUMMARY_UX_REPAIR_SUFFIX: &str = "Rewrite only title and short_text. \
title: max 12 words. short_text: max 2 sentences and max 75 words. \
Do not use: oracle, oracles, tirage, tirages, cartes tirées, consultation divinatoire, \
liane de constance, Tendance invite. Keep it clear, elegant and suitable for a UI card.";

const ASTRO_SUMMARY_MARKERS: &[&str] = &[
    "thème",
    "theme",
    "lecture",
    "configuration",
    "carte natale",
    "symbolique",
];

pub struct SummarySynthesisResult {
    pub summary: ReadingSummary,
    pub token_usage: Option<TokenUsage>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

pub struct SummarySynthesizer<'a> {
    router: &'a ProviderRouter,
    validator: &'a ResponseValidator,
    catalog: &'a SharedCanonicalCatalog,
}

impl<'a> SummarySynthesizer<'a> {
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
        chapters: &[ReadingChapter],
        engine: &ResolvedEngineParams,
        safety_policy: &SafetyPolicy,
        run_id: &str,
        repair_instruction: Option<&str>,
    ) -> Result<SummarySynthesisResult, GenerationError> {
        let messages = build_summary_messages(request, chapters, &self.catalog, repair_instruction);
        prompt_trace::log_provider_messages(
            run_id,
            Some(READING_SUMMARY_STEP_CODE),
            None,
            None,
            Some("summary"),
            &messages,
        );
        let canonical_schema = self
            .validator
            .schema_registry()
            .provider_schema("summary_provider_v1")
            .cloned();
        self.router
            .capability_registry()
            .validate_engine_for_context(
                ModelRouteContext::Subtask,
                &engine.provider,
                &engine.model,
                engine.allow_oracle_benchmark,
            )?;
        let model_cap = self
            .router
            .capability_registry()
            .require(&engine.provider, &engine.model)?;
        let schema = canonical_schema
            .as_ref()
            .map(|s| ProviderSchemaCompiler::compile(s, model_cap))
            .transpose()?;

        let product_policy = self
            .catalog
            .product_policy(&request.product_context.product_code)
            .ok_or_else(|| {
                GenerationError::new(
                    GenerationErrorCode::ProductPolicyViolation,
                    "no generation policy for product",
                )
            })?;
        ProductPolicyValidator::validate_against_policy(
            request,
            product_policy,
            &engine.provider,
            &engine.model,
        )?;

        let provider_request = ProviderGenerationRequest {
            model: engine.model.clone(),
            messages,
            structured_schema: schema,
            reasoning_effort: resolve_reasoning_effort(
                model_cap,
                product_policy,
                engine.reasoning_effort,
                ModelRouteContext::Subtask,
            ),
            temperature: effective_temperature(model_cap, engine.temperature),
            max_output_tokens: Some(apply_reasoning_output_reserve(
                model_cap,
                SUBTASK_BASE_OUTPUT_TOKENS,
            )),
            safety_mode: resolve_safety_mode(&engine.provider),
            timeout: Duration::from_millis(engine.timeout_ms.unwrap_or(900_000)),
            metadata: GenerationMetadata {
                run_id: run_id.to_string(),
                request_id: request.request_id.clone(),
                product_code: request.product_context.product_code.clone(),
                chapter_code: Some(READING_SUMMARY_STEP_CODE.into()),
                prompt_trace_step: Some("summary_generate".into()),
                prompt_trace_attempt: Some(
                    repair_instruction
                        .map(|_| "repair")
                        .unwrap_or("summary")
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
                ModelRouteContext::Subtask,
            )
            .await?;

        let json = route.response.parsed_json.ok_or_else(|| {
            GenerationError::new(
                GenerationErrorCode::InvalidJsonOutput,
                "provider returned no JSON for summary synthesis",
            )
        })?;

        self.validator.validate_summary(&json)?;

        let summary: SummaryProviderResponse = serde_json::from_value(json).map_err(|e| {
            GenerationError::new(
                GenerationErrorCode::InvalidJsonOutput,
                format!("summary deserialization failed: {e}"),
            )
        })?;

        validate_summary_content(
            &summary,
            request
                .product_context
                .interpretation_profile_code
                .as_deref(),
        )?;

        SafetyGuard::validate_chapter_text(
            &format!("{} {}", summary.title, summary.short_text),
            safety_policy,
            &request.astrologer_profile.forbidden_wording,
            self.catalog,
        )
        .map_err(|violations| {
            GenerationError::with_details(
                GenerationErrorCode::PostSafetyValidationFailed,
                "summary failed safety validation",
                serde_json::json!({ "violations": violations }),
            )
        })?;

        let input_tokens = route.response.usage.as_ref().and_then(|u| u.input_tokens());
        let output_tokens = route
            .response
            .usage
            .as_ref()
            .and_then(|u| u.output_tokens());

        Ok(SummarySynthesisResult {
            summary: ReadingSummary {
                title: summary.title,
                short_text: summary.short_text,
            },
            token_usage: route.response.usage.clone(),
            input_tokens,
            output_tokens,
        })
    }
}

trait LegacyTokenUsageAccess {
    fn input_tokens(&self) -> Option<u32>;
    fn output_tokens(&self) -> Option<u32>;
}

impl LegacyTokenUsageAccess for TokenUsage {
    fn input_tokens(&self) -> Option<u32> {
        self.tokens_for(TokenUsageType::Input)
    }

    fn output_tokens(&self) -> Option<u32> {
        self.tokens_for(TokenUsageType::Output)
    }
}

pub fn validate_summary_content(
    summary: &SummaryProviderResponse,
    interpretation_profile_code: Option<&str>,
) -> Result<(), GenerationError> {
    let corpus = format!("{} {}", summary.title, summary.short_text).to_lowercase();
    let short_trimmed = summary.short_text.trim();

    if summary.title.trim().len() < 8 {
        return Err(GenerationError::new(
            GenerationErrorCode::ReadingQualityFailed,
            "summary title too short",
        ));
    }

    if count_words(&summary.short_text) < 20 {
        return Err(GenerationError::new(
            GenerationErrorCode::ReadingQualityFailed,
            "summary text too short for premium synthesis",
        ));
    }

    let joined = format!("{}\n{}", summary.title, summary.short_text);
    let forbidden_matches = find_forbidden_summary_patterns(&joined);
    if !forbidden_matches.is_empty() {
        return Err(GenerationError::with_details(
            GenerationErrorCode::ReadingQualityFailed,
            "summary contains forbidden divinatory wording",
            serde_json::json!({
                "summary_retryable": true,
                "reason": "forbidden_pattern",
                "matches": forbidden_matches,
            }),
        ));
    }

    for pattern in BANNED_TECHNICAL_SUMMARY_PATTERNS {
        if corpus.contains(pattern) {
            return Err(GenerationError::with_details(
                GenerationErrorCode::ReadingQualityFailed,
                "summary contains technical placeholder wording",
                serde_json::json!({
                    "summary_retryable": true,
                    "reason": "technical_placeholder",
                    "pattern": pattern,
                }),
            ));
        }
    }

    validate_summary_ux(
        summary.title.trim(),
        summary.short_text.trim(),
        &SummaryUxRules::default(),
    )?;

    if short_trimmed.to_lowercase().starts_with("tendance") {
        return Err(GenerationError::with_details(
            GenerationErrorCode::ReadingQualityFailed,
            "summary must not start with 'Tendance'",
            serde_json::json!({
                "summary_retryable": true,
                "reason": "forbidden_pattern",
                "matches": ["tendance"],
            }),
        ));
    }

    if interpretation_profile_code == Some("natal_premium_plus")
        && !ASTRO_SUMMARY_MARKERS
            .iter()
            .any(|marker| corpus.contains(marker))
    {
        return Err(GenerationError::with_details(
            GenerationErrorCode::ReadingQualityFailed,
            "premium plus summary lacks astrological framing marker",
            serde_json::json!({
                "summary_retryable": true,
                "reason": "missing_astro_marker",
            }),
        ));
    }

    Ok(())
}

pub fn is_summary_banned_pattern_error(err: &GenerationError) -> bool {
    if err.detail().code != GenerationErrorCode::ReadingQualityFailed {
        return false;
    }
    let details = err.detail().details.as_ref();
    details
        .and_then(|d| d.get("summary_retryable"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        || details.and_then(|d| d.get("pattern")).is_some()
}

pub fn deterministic_safe_summary_fallback() -> ReadingSummary {
    ReadingSummary {
        title: "Présence intérieure, constance et ouverture".into(),
        short_text: "Cette lecture symbolique met en lumière une carte natale structurée par la \
            profondeur, la fiabilité et le besoin d'une croissance habitée. Les chapitres \
            développent ensuite les nuances du thème, entre identité, relations, ressources et \
            chemin d'évolution."
            .into(),
    }
}

fn build_summary_messages(
    request: &GenerateReadingRequest,
    chapters: &[ReadingChapter],
    catalog: &SharedCanonicalCatalog,
    repair_instruction: Option<&str>,
) -> Vec<PromptMessage> {
    let language_block =
        WritingLanguageDirective::prompt_block(catalog, &request.product_context.user_language);
    let chapter_digest: Vec<serde_json::Value> = chapters
        .iter()
        .map(|c| {
            serde_json::json!({
                "code": c.code,
                "title": c.title,
                "body_excerpt": truncate_words(&c.body, 80),
            })
        })
        .collect();

    let system = format!(
        "{language_block}\n\n\
         Write a concise UX summary for a natal reading.\n\
         Rules:\n\
         - Output JSON with title and short_text only.\n\
         - title: maximum 12 words.\n\
         - short_text: maximum 2 sentences.\n\
         - short_text: 55–75 words.\n\
         - Do not use divinatory vocabulary: oracle, oracles, tirage, tirages, cartes tirées.\n\
         - Do not compress too many ideas into a single long sentence.\n\
         - Let the final synthesis chapter carry the depth; the summary is only a clear entry point for the UI.\n\
         - Synthesize dominant themes from the chapter excerpts — never mention pipelines, generation modes, or technical process.\n\
         - Frame the reading as symbolic and interpretive. Prefer phrasing such as \
         « Cette lecture symbolique suggère… », « L'ensemble invite… », « Le thème évoque… ».\n\
         - Use natal vocabulary (thème, carte natale, configuration).\n\
         - Avoid forced poetic clichés (e.g. liane de constance); prefer a concrete, evocative title."
    );

    let repair_block = repair_instruction
        .map(|r| {
            format!(
                "\n\nREPAIR SUMMARY:\nThe previous summary was too long or used forbidden wording.\n\
                 {r}\n{SUMMARY_UX_REPAIR_SUFFIX}"
            )
        })
        .unwrap_or_default();

    let user = format!(
        "Product: {}\nAudience: {:?}\n\nChapters:\n{}\n\n\
         Write title (one evocative line, max 12 words) and short_text (max 2 sentences, 55–75 words, \
         symbolic framing, no medical/legal/financial advice).{repair_block}",
        request.product_context.product_code,
        request.product_context.audience_level,
        serde_json::to_string_pretty(&chapter_digest).unwrap_or_default(),
    );

    vec![
        PromptMessage {
            role: PromptRole::System,
            content: system,
        },
        PromptMessage {
            role: PromptRole::User,
            content: user,
        },
    ]
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
