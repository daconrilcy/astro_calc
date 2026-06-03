use std::time::Duration;

use astral_llm_domain::{
    generation_response::{
        ChapterProviderResponse, LegalBlock, NatalReadingResponse, QualityMetadata,
        ReadingChapter, ReadingSummary,
    },
    output_contract::{ChapterContract, GenerationMode},
    GenerateReadingRequest, GenerationError, GenerationErrorCode, SafetyPolicy, SafetyMode,
};
use astral_llm_infra::SharedCanonicalCatalog;
use astral_llm_providers::{GenerationMetadata, ProviderGenerationRequest};
use uuid::Uuid;

use crate::domain_selector::select_domains;
use crate::engine_defaults::ResolvedEngineParams;
use crate::prompt_compiler::{PromptCompilationInput, PromptCompiler};
use crate::provider_router::ProviderRouter;
use crate::response_validator::ResponseValidator;
use crate::token_budget::TokenBudget;
use astral_llm_domain::ServiceLimits;

pub struct ChapterOrchestrator<'a> {
    router: &'a ProviderRouter,
    compiler: &'a PromptCompiler,
    validator: &'a ResponseValidator,
    catalog: &'a SharedCanonicalCatalog,
    limits: &'a ServiceLimits,
}

impl<'a> ChapterOrchestrator<'a> {
    pub fn new(
        router: &'a ProviderRouter,
        compiler: &'a PromptCompiler,
        validator: &'a ResponseValidator,
        catalog: &'a SharedCanonicalCatalog,
        limits: &'a ServiceLimits,
    ) -> Self {
        Self {
            router,
            compiler,
            validator,
            catalog,
            limits,
        }
    }

    pub async fn generate(
        &self,
        request: &GenerateReadingRequest,
        engine: &ResolvedEngineParams,
        safety_policy: &SafetyPolicy,
        run_id: &str,
    ) -> Result<NatalReadingResponse, GenerationError> {
        let domains = select_domains(request, self.catalog, self.limits);
        let chapters = resolve_chapters(request, &domains);
        let mut generated = Vec::new();
        let mut last_bundle = None;
        let mut used_provider = engine.provider.as_str().to_string();
        let mut used_model = engine.model.clone();
        let mut fallback_used = false;

        for chapter in &chapters {
            let bundle = self
                .compiler
                .compile(PromptCompilationInput {
                    request,
                    safety_policy,
                    selected_domains: &domains,
                    chapter_code: Some(&chapter.code),
                    catalog: self.catalog,
                })
                .map_err(|e| GenerationError::new(GenerationErrorCode::InvalidInput, e))?;
            last_bundle = Some(bundle.clone());

            let messages = self.compiler.to_provider_messages(&bundle);
            let schema = self.validator.schema_registry().provider_schema("chapter_provider_v1").cloned();

            let safety_mode = resolve_safety_mode(engine, &used_provider);

            let provider_request = ProviderGenerationRequest {
                model: engine.model.clone(),
                messages,
                structured_schema: schema,
                reasoning_effort: engine.reasoning_effort,
                temperature: engine.temperature,
                max_output_tokens: Some(TokenBudget::chapter_max_tokens(
                    chapter,
                    request.response_contract.global_max_tokens,
                )),
                safety_mode,
                timeout: Duration::from_millis(engine.timeout_ms.unwrap_or(120_000)),
                metadata: GenerationMetadata {
                    run_id: run_id.to_string(),
                    request_id: request.request_id.clone(),
                    product_code: request.product_context.product_code.clone(),
                    chapter_code: Some(chapter.code.clone()),
                },
            };

            let route = self
                .router
                .generate(
                    provider_request,
                    engine.provider.clone(),
                    engine.allow_fallback,
                    true,
                )
                .await?;

            used_provider = route.used_provider.as_str().to_string();
            used_model = route.response.model_used.clone();
            fallback_used |= route.fallback_used;

            let json = route.response.parsed_json.ok_or_else(|| {
                GenerationError::new(
                    GenerationErrorCode::InvalidJsonOutput,
                    "provider returned no JSON for chapter",
                )
            })?;

            self.validator.validate_chapter(&json)?;

            let chapter_reading: ChapterProviderResponse =
                serde_json::from_value(json).map_err(|e| {
                    GenerationError::new(
                        GenerationErrorCode::InvalidJsonOutput,
                        format!("chapter deserialization failed: {e}"),
                    )
                })?;

            if chapter_reading.code != chapter.code {
                return Err(GenerationError::with_details(
                    GenerationErrorCode::SchemaValidationFailed,
                    "chapter code mismatch",
                    serde_json::json!({
                        "expected": chapter.code,
                        "received": chapter_reading.code
                    }),
                ));
            }

            TokenBudget::validate_chapter_lengths(
                &[(chapter.code.clone(), chapter_reading.body.clone())],
                std::slice::from_ref(chapter),
            )?;

            generated.push(ReadingChapter {
                code: chapter_reading.code,
                title: chapter_reading.title,
                body: chapter_reading.body,
                astro_basis: chapter_reading.astro_basis,
                confidence: chapter_reading.confidence,
                safety_flags: vec![],
            });
        }

        let bundle = last_bundle.expect("at least one chapter");
        Ok(NatalReadingResponse {
            schema_version: request.response_contract.output_schema_version.clone(),
            language: request.product_context.user_language.clone(),
            reading_type: request.product_context.product_code.clone(),
            summary: ReadingSummary {
                title: format!("Lecture {} — synthese", request.product_context.product_code),
                short_text: "Synthese produite par generation chapitre par chapitre.".into(),
            },
            chapters: generated,
            legal: LegalBlock {
                disclaimer: default_disclaimer(
                    request.response_contract.include_legal_disclaimer,
                    &request.product_context.user_language,
                ),
            },
            quality: QualityMetadata {
                used_provider,
                used_model,
                generation_mode: GenerationMode::ChapterOrchestrated,
                prompt_family: bundle.prompt_family,
                prompt_version: bundle.prompt_version,
                astro_contract_version: request.astro_result.contract_version.clone(),
                fallback_used,
            },
        })
    }
}

fn resolve_safety_mode(_engine: &ResolvedEngineParams, used_provider: &str) -> SafetyMode {
    if used_provider == "mistral" {
        SafetyMode::PlatformAndNative
    } else {
        SafetyMode::PlatformRulesOnly
    }
}

fn resolve_chapters(request: &GenerateReadingRequest, domains: &[String]) -> Vec<ChapterContract> {
    if !request.response_contract.chapters.is_empty() {
        return request.response_contract.chapters.clone();
    }

    domains
        .iter()
        .map(|code| ChapterContract {
            code: code.clone(),
            title: humanize_domain(code),
            min_words: Some(80),
            max_words: Some(250),
            target_tokens: Some(400),
            required_fields: vec!["body".into()],
        })
        .collect()
}

fn humanize_domain(code: &str) -> String {
    code.replace('_', " ")
}

fn default_disclaimer(include: bool, _language: &str) -> String {
    if include {
        "Cette lecture est une interpretation symbolique et ne remplace aucun avis medical, \
         psychologique, juridique ou financier."
            .into()
    } else {
        String::new()
    }
}

pub fn new_run_id() -> String {
    Uuid::new_v4().to_string()
}
