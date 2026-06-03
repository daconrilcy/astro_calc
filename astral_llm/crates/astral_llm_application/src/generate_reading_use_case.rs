use std::time::Duration;

use astral_llm_domain::{
    generation_response::{
        GenerateReadingResponse, GenerationFailedResponse, SafetyRejectedResponse,
        StructuredReadingResponse,
    },
    output_contract::GenerationMode,
    EngineDefaults, GenerateReadingRequest, GenerationError, GenerationErrorCode, SafetyMode,
    ServiceLimits,
};
use astral_llm_infra::SharedCanonicalCatalog;

use astral_llm_providers::{GenerationMetadata, ProviderGenerationRequest};

use crate::chapter_orchestrator::{new_run_id, ChapterOrchestrator};
use crate::domain_selector::select_domains;
use crate::engine_defaults::{resolve_engine_params, ResolvedEngineParams};
use crate::prompt_compiler::{PromptCompilationInput, PromptCompiler};
use crate::provider_router::ProviderRouter;
use crate::request_validator::RequestValidator;
use crate::response_validator::ResponseValidator;
use crate::safety_guard::SafetyGuard;
use crate::safety_resolver::SafetyResolver;

pub struct GenerateReadingUseCase {
    pub router: ProviderRouter,
    compiler: PromptCompiler,
    validator: ResponseValidator,
    engine_defaults: EngineDefaults,
    limits: ServiceLimits,
    catalog: SharedCanonicalCatalog,
}

impl GenerateReadingUseCase {
    pub fn new(
        router: ProviderRouter,
        compiler: PromptCompiler,
        validator: ResponseValidator,
        engine_defaults: EngineDefaults,
        limits: ServiceLimits,
        catalog: SharedCanonicalCatalog,
    ) -> Self {
        Self {
            router,
            compiler,
            validator,
            engine_defaults,
            limits,
            catalog,
        }
    }

    pub async fn execute(&self, request: GenerateReadingRequest) -> GenerateReadingResponse {
        let run_id = new_run_id();
        match self.execute_inner(&request, &run_id).await {
            Ok(reading) => GenerateReadingResponse::Success(StructuredReadingResponse {
                run_id,
                reading,
            }),
            Err(GenerationError::Detailed { detail, .. })
                if matches!(
                    detail.code,
                    GenerationErrorCode::SafetyRejected
                        | GenerationErrorCode::PostSafetyValidationFailed
                ) =>
            {
                GenerateReadingResponse::SafetyRejected(SafetyRejectedResponse {
                    run_id,
                    violations: detail
                        .details
                        .as_ref()
                        .and_then(|v| v.get("violations"))
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_else(|| vec![detail.message.clone()]),
                })
            }
            Err(err) => GenerateReadingResponse::Failed(GenerationFailedResponse {
                run_id,
                error: err.detail().clone(),
            }),
        }
    }

    async fn execute_inner(
        &self,
        request: &GenerateReadingRequest,
        run_id: &str,
    ) -> Result<astral_llm_domain::NatalReadingResponse, GenerationError> {
        RequestValidator::validate(request, &self.limits, &self.catalog)?;

        let engine = resolve_engine_params(
            &request.engine,
            &self.engine_defaults,
            self.limits.default_request_timeout_ms,
        );
        RequestValidator::validate_engine_resolved(&engine.provider, &engine.model)?;

        let product_default =
            SafetyResolver::product_default_for(&request.product_context.product_code);
        let safety_policy =
            SafetyResolver::resolve(&product_default, request.safety_policy.as_ref());

        SafetyGuard::validate_request(request, &safety_policy, &self.catalog).map_err(|violations| {
            GenerationError::with_details(
                GenerationErrorCode::SafetyRejected,
                "request failed safety validation",
                serde_json::json!({ "violations": violations }),
            )
        })?;

        if self
            .validator
            .schema_registry()
            .get(&request.response_contract.output_schema_version)
            .is_none()
        {
            return Err(GenerationError::new(
                GenerationErrorCode::InvalidInput,
                format!(
                    "unsupported output_schema_version: {}",
                    request.response_contract.output_schema_version
                ),
            ));
        }

        let reading = match request.response_contract.generation_mode {
            GenerationMode::ChapterOrchestrated => {
                let orchestrator = ChapterOrchestrator::new(
                    &self.router,
                    &self.compiler,
                    &self.validator,
                    &self.catalog,
                    &self.limits,
                );
                orchestrator
                    .generate(request, &engine, &safety_policy, run_id)
                    .await?
            }
            GenerationMode::SinglePass => {
                self.generate_single_pass(request, &engine, &safety_policy, run_id)
                    .await?
            }
        };

        SafetyGuard::validate_response(
            &reading,
            &safety_policy,
            &request.astrologer_profile.forbidden_wording,
            &self.catalog,
        )
        .map_err(|violations| {
            GenerationError::with_details(
                GenerationErrorCode::PostSafetyValidationFailed,
                "generated content failed safety validation",
                serde_json::json!({ "violations": violations }),
            )
        })?;

        Ok(reading)
    }

    async fn generate_single_pass(
        &self,
        request: &GenerateReadingRequest,
        engine: &ResolvedEngineParams,
        safety_policy: &astral_llm_domain::SafetyPolicy,
        run_id: &str,
    ) -> Result<astral_llm_domain::NatalReadingResponse, GenerationError> {
        let domains = select_domains(request, &self.catalog, &self.limits);
        let bundle = self
            .compiler
            .compile(PromptCompilationInput {
                request,
                safety_policy,
                selected_domains: &domains,
                chapter_code: None,
                catalog: &self.catalog,
            })
            .map_err(|e| GenerationError::new(GenerationErrorCode::InvalidInput, e))?;

        let messages = self.compiler.to_provider_messages(&bundle);
        let schema = self
            .validator
            .schema_registry()
            .provider_schema(&request.response_contract.output_schema_version)
            .cloned();

        let provider_request = ProviderGenerationRequest {
            model: engine.model.clone(),
            messages,
            structured_schema: schema,
            reasoning_effort: engine.reasoning_effort,
            temperature: engine.temperature,
            max_output_tokens: engine.max_output_tokens,
            safety_mode: if engine.provider == astral_llm_domain::ProviderKind::Mistral {
                SafetyMode::PlatformAndNative
            } else {
                SafetyMode::PlatformRulesOnly
            },
            timeout: Duration::from_millis(engine.timeout_ms.unwrap_or(120_000)),
            metadata: GenerationMetadata {
                run_id: run_id.to_string(),
                request_id: request.request_id.clone(),
                product_code: request.product_context.product_code.clone(),
                chapter_code: None,
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

        let json = route.response.parsed_json.ok_or_else(|| {
            GenerationError::new(
                GenerationErrorCode::InvalidJsonOutput,
                "provider returned no JSON",
            )
        })?;

        let mut reading = self.validator.validate_and_parse(
            &request.response_contract.output_schema_version,
            &json,
            &request.response_contract.chapters,
        )?;

        reading.quality.used_provider = route.used_provider.as_str().to_string();
        reading.quality.used_model = route.response.model_used;
        reading.quality.prompt_family = bundle.prompt_family;
        reading.quality.prompt_version = bundle.prompt_version;
        reading.quality.astro_contract_version = request.astro_result.contract_version.clone();
        reading.quality.fallback_used = route.fallback_used;
        reading.language = request.product_context.user_language.clone();
        reading.reading_type = request.product_context.product_code.clone();

        Ok(reading)
    }
}
