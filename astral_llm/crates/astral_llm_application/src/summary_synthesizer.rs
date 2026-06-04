//! Synthese finale personnalisee apres generation chapitre par chapitre.

use std::time::Duration;

use astral_llm_domain::{
    generation_response::{ReadingChapter, ReadingSummary, SummaryProviderResponse},
    GenerateReadingRequest, GenerationError, GenerationErrorCode, SafetyMode, SafetyPolicy,
};
use astral_llm_infra::SharedCanonicalCatalog;
use astral_llm_providers::{GenerationMetadata, PromptMessage, PromptRole, ProviderGenerationRequest};

use crate::engine_defaults::ResolvedEngineParams;
use crate::provider_router::ProviderRouter;
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
    "chapter_orchestrated",
    "single_pass",
    "pipeline technique",
    "placeholder",
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
    ) -> Result<SummarySynthesisResult, GenerationError> {
        let messages = build_summary_messages(request, chapters, &self.catalog);
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
        let model_cap = self
            .router
            .capability_registry()
            .require(&engine.provider, &engine.model)?;
        let schema = canonical_schema
            .as_ref()
            .map(|s| ProviderSchemaCompiler::compile(s, model_cap))
            .transpose()?;

        let provider_request = ProviderGenerationRequest {
            model: engine.model.clone(),
            messages,
            structured_schema: schema,
            reasoning_effort: engine.reasoning_effort,
            temperature: engine.temperature,
            max_output_tokens: Some(600),
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

        validate_summary_content(&summary)?;

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

pub fn validate_summary_content(summary: &SummaryProviderResponse) -> Result<(), GenerationError> {
    let corpus = format!("{} {}", summary.title, summary.short_text).to_lowercase();

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
        if corpus.contains(pattern) {
            return Err(GenerationError::with_details(
                GenerationErrorCode::ReadingQualityFailed,
                "summary contains technical placeholder wording",
                serde_json::json!({ "pattern": pattern }),
            ));
        }
    }

    Ok(())
}

fn build_summary_messages(
    request: &GenerateReadingRequest,
    chapters: &[ReadingChapter],
    catalog: &SharedCanonicalCatalog,
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
         never mention pipelines, generation modes, or technical process."
    );

    let user = format!(
        "Product: {}\nAudience: {:?}\n\nChapters:\n{}\n\n\
         Write title (one evocative line) and short_text (2-3 sentences, symbolic framing, \
         no medical/legal/financial advice).",
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
            title: "Lecture natal_premium — synthese".into(),
            short_text: "Synthese produite par generation chapitre par chapitre.".into(),
        };
        assert!(validate_summary_content(&summary).is_err());
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
        assert!(validate_summary_content(&summary).is_ok());
    }
}
