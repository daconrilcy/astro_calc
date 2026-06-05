//! Synthese finale personnalisee apres generation chapitre par chapitre.

use std::time::Duration;

use astral_llm_domain::{
    generation_response::{ReadingChapter, ReadingSummary, SummaryProviderResponse},
    model_usage_tier::ModelRouteContext,
    GenerateReadingRequest, GenerationError, GenerationErrorCode, SafetyMode, SafetyPolicy,
};
use astral_llm_infra::SharedCanonicalCatalog;
use astral_llm_providers::{GenerationMetadata, PromptMessage, PromptRole, ProviderGenerationRequest};

use crate::engine_defaults::ResolvedEngineParams;
use crate::product_policy_validator::ProductPolicyValidator;
use crate::provider_router::ProviderRouter;
use crate::reasoning_generation::{
    apply_reasoning_output_reserve, effective_temperature, resolve_reasoning_effort,
    SUBTASK_BASE_OUTPUT_TOKENS,
};
use crate::prompt_trace;
use crate::provider_schema_compiler::ProviderSchemaCompiler;
use crate::response_validator::ResponseValidator;
use crate::safety_guard::SafetyGuard;
use crate::writing_language::WritingLanguageDirective;

use astral_llm_domain::chapter_orchestration::READING_SUMMARY_STEP_CODE;

const BANNED_SUMMARY_PATTERNS: &[&str] = &[
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
    "tirage",
    "cartes tirées",
    "cartes tirees",
    "oracle",
    "consultation divinatoire",
    "tendance invite",
    "le tirage",
    "liane de constance",
];

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
        let messages =
            build_summary_messages(request, chapters, &self.catalog, repair_instruction);
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
        self.router.capability_registry().validate_engine_for_context(
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
            timeout: Duration::from_millis(engine.timeout_ms.unwrap_or(120_000)),
            metadata: GenerationMetadata {
                run_id: run_id.to_string(),
                request_id: request.request_id.clone(),
                product_code: request.product_context.product_code.clone(),
                chapter_code: Some(READING_SUMMARY_STEP_CODE.into()),
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

        let input_tokens = route.response.usage.as_ref().map(|u| u.input_tokens);
        let output_tokens = route.response.usage.as_ref().map(|u| u.output_tokens);

        Ok(SummarySynthesisResult {
            summary: ReadingSummary {
                title: summary.title,
                short_text: summary.short_text,
            },
            input_tokens,
            output_tokens,
        })
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

    if word_count(&summary.short_text) < 20 {
        return Err(GenerationError::new(
            GenerationErrorCode::ReadingQualityFailed,
            "summary text too short for premium synthesis",
        ));
    }

    for pattern in BANNED_SUMMARY_PATTERNS {
        if corpus_matches_banned_pattern(&corpus, pattern) {
            return Err(GenerationError::with_details(
                GenerationErrorCode::ReadingQualityFailed,
                "summary contains technical placeholder wording",
                serde_json::json!({ "pattern": pattern }),
            ));
        }
    }

    if short_trimmed.to_lowercase().starts_with("tendance") {
        return Err(GenerationError::new(
            GenerationErrorCode::ReadingQualityFailed,
            "summary must not start with 'Tendance'",
        ));
    }

    if interpretation_profile_code == Some("natal_premium_plus")
        && !ASTRO_SUMMARY_MARKERS
            .iter()
            .any(|marker| corpus.contains(marker))
    {
        return Err(GenerationError::new(
            GenerationErrorCode::ReadingQualityFailed,
            "premium plus summary lacks astrological framing marker",
        ));
    }

    Ok(())
}

/// Mot entier pour patterns courts (`tirage`, `oracle`) afin d'eviter les faux positifs (`retirage`).
fn corpus_matches_banned_pattern(corpus: &str, pattern: &str) -> bool {
    if pattern.contains(' ') {
        return corpus.contains(pattern);
    }
    corpus
        .split(|c: char| !c.is_alphanumeric())
        .any(|token| !token.is_empty() && token == pattern)
}

pub fn is_summary_banned_pattern_error(err: &GenerationError) -> bool {
    if err.detail().code != GenerationErrorCode::ReadingQualityFailed {
        return false;
    }
    err.detail()
        .details
        .as_ref()
        .and_then(|d| d.get("pattern"))
        .is_some()
}

pub fn deterministic_safe_summary_fallback() -> ReadingSummary {
    ReadingSummary {
        title: "Présence intérieure, construction et ouverture".into(),
        short_text: "Cette lecture symbolique met en lumière une carte natale structurée par la \
            profondeur, la constance et le besoin d'une croissance habitée. Les chapitres \
            décrivent une tension féconde entre sécurité, expression personnelle, lien aux autres \
            et transformation intérieure."
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
         Write a concise, personalized natal reading summary. \
         Output JSON with title and short_text only. \
         The summary must synthesize dominant themes from the chapter excerpts — \
         never mention pipelines, generation modes, or technical process. \
         Frame the reading as symbolic and interpretive. Prefer phrasing such as \
         « Cette lecture symbolique suggère… », « L'ensemble invite… », « Le thème évoque… ». \
         Use natal vocabulary (thème, carte natale, configuration) — never divinatory wording (tirage, oracle). \
         Avoid forced poetic clichés (e.g. liane de constance); prefer a concrete, evocative title."
    );

    let repair_block = repair_instruction
        .map(|r| format!("\n\nREPAIR: {r}"))
        .unwrap_or_default();

    let user = format!(
        "Product: {}\nAudience: {:?}\n\nChapters:\n{}\n\n\
         Write title (one evocative line) and short_text (2-3 sentences, symbolic framing, \
         no medical/legal/financial advice).{repair_block}",
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

fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

fn resolve_safety_mode(provider: &astral_llm_domain::ProviderKind) -> SafetyMode {
    if matches!(provider, astral_llm_domain::ProviderKind::Mistral) {
        SafetyMode::PlatformAndNative
    } else {
        SafetyMode::PlatformRulesOnly
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_technical_placeholder_summary() {
        let summary = SummaryProviderResponse {
            title: "Lecture natal_prompter — synthese".into(),
            short_text: "Synthese produite par generation chapitre par chapitre.".into(),
        };
        assert!(validate_summary_content(&summary, None).is_err());
    }

    #[test]
    fn accepts_personalized_summary() {
        let summary = SummaryProviderResponse {
            title: "Une dynamique d'affirmation et de profondeur".into(),
            short_text: "Votre theme met en avant une dynamique d'affirmation personnelle, \
                une grande richesse emotionnelle et un chemin relationnel structurant. \
                Cette configuration symbolique invite a accueillir les transitions interieures \
                comme des espaces de croissance authentique.".into(),
        };
        assert!(validate_summary_content(&summary, None).is_ok());
    }

    #[test]
    fn does_not_ban_tirage_as_substring_of_retirage() {
        let summary = SummaryProviderResponse {
            title: "Une dynamique de retirage et de recentrage".into(),
            short_text: "Cette lecture symbolique evoque un theme de recentrage interieur \
                avec une profondeur emotionnelle et une clarte relationnelle dans la vie \
                quotidienne et les choix professionnels.".into(),
        };
        assert!(validate_summary_content(&summary, Some("natal_premium_plus")).is_ok());
    }

    #[test]
    fn bans_divinatory_tirage_as_whole_word() {
        let summary = SummaryProviderResponse {
            title: "Une lecture symbolique du theme".into(),
            short_text: "Ce tirage evoque une presence attentive et une quete de sens \
                dans les relations et la vie professionnelle avec une profondeur \
                emotionnelle et une stabilite interieure.".into(),
        };
        assert!(validate_summary_content(&summary, Some("natal_premium_plus")).is_err());
    }

    #[test]
    fn rejects_liane_de_constance_in_title() {
        let summary = SummaryProviderResponse {
            title: "Identité et profondeur: présence magnétique, liane de constance et de sens"
                .into(),
            short_text: "Cette lecture symbolique met en lumière une carte natale riche en \
                tensions fécondes entre sécurité intérieure, expression personnelle et besoin de \
                croissance relationnelle authentique dans la vie quotidienne.".into(),
        };
        let err = validate_summary_content(&summary, Some("natal_premium_plus")).unwrap_err();
        assert!(is_summary_banned_pattern_error(&err));
    }

    #[test]
    fn deterministic_fallback_passes_validation() {
        let summary = deterministic_safe_summary_fallback();
        let provider = SummaryProviderResponse {
            title: summary.title,
            short_text: summary.short_text,
        };
        assert!(validate_summary_content(&provider, Some("natal_premium_plus")).is_ok());
    }

    #[test]
    fn premium_plus_summary_requires_astro_marker() {
        let summary = SummaryProviderResponse {
            title: "Une dynamique personnelle".into(),
            short_text: "Vous avancez avec assurance et une grande sensibilite aux relations \
                humaines, en cultivant des liens authentiques et une ecoute attentive des autres \
                dans votre vie quotidienne et professionnelle.".into(),
        };
        assert!(
            validate_summary_content(&summary, Some("natal_premium_plus")).is_err(),
            "summary without natal marker must fail for premium_plus"
        );
        let with_marker = SummaryProviderResponse {
            title: "Une lecture symbolique".into(),
            short_text: summary.short_text.replace(
                "Vous avancez",
                "Cette lecture symbolique suggere que vous avancez",
            ),
        };
        assert!(validate_summary_content(&with_marker, Some("natal_premium_plus")).is_ok());
    }
}
