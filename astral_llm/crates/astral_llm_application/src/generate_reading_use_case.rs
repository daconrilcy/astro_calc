use std::time::Duration;

use astral_llm_domain::{
    contract_versions::GenerationRunContractVersions,
    generation_response::{
        GenerateReadingResponse, GenerationFailedResponse, SafetyRejectedResponse,
        StructuredReadingResponse,
    },
    output_contract::GenerationMode,
    EngineDefaults, GenerateReadingRequest, GenerationError, GenerationErrorCode, PrivacyPolicy,
    ProviderKind, SafetyMode, ServiceLimits,
};
use astral_llm_infra::SharedCanonicalCatalog;

use astral_llm_providers::{GenerationMetadata, ProviderGenerationRequest};

use crate::astro_basis_validator::AstroBasisValidator;
use crate::astro_payload_normalizer::AstroPayloadNormalizer;
use crate::chapter_orchestrator::{new_run_id, ChapterOrchestrator};
use crate::domain_resolver::DomainResolver;
use crate::engine_defaults::{drop_unsupported_reasoning, resolve_engine_params, ResolvedEngineParams};
use crate::execution_audit::ExecutionAudit;
use crate::product_policy_validator::ProductPolicyValidator;
use crate::prompt_compiler::{PromptCompilationInput, PromptCompiler};
use crate::provider_router::ProviderRouter;
use crate::provider_schema_compiler::ProviderSchemaCompiler;
use crate::request_validator::RequestValidator;
use crate::response_validator::ResponseValidator;
use crate::safety_guard::SafetyGuard;
use crate::reading_quality_validator::ReadingQualityValidator;
use crate::safety_resolver::SafetyResolver;

pub struct GenerateReadingUseCase {
    pub router: ProviderRouter,
    compiler: PromptCompiler,
    validator: ResponseValidator,
    engine_defaults: EngineDefaults,
    limits: ServiceLimits,
    catalog: SharedCanonicalCatalog,
    privacy_policy: PrivacyPolicy,
}

pub struct UseCaseOutput {
    pub response: GenerateReadingResponse,
    pub audit: ExecutionAudit,
}

impl GenerateReadingUseCase {
    pub fn new(
        router: ProviderRouter,
        compiler: PromptCompiler,
        validator: ResponseValidator,
        engine_defaults: EngineDefaults,
        limits: ServiceLimits,
        catalog: SharedCanonicalCatalog,
        privacy_policy: PrivacyPolicy,
    ) -> Self {
        Self {
            router,
            compiler,
            validator,
            engine_defaults,
            limits,
            catalog,
            privacy_policy,
        }
    }

    pub async fn execute(&self, request: GenerateReadingRequest) -> GenerateReadingResponse {
        self.execute_with_audit(request, new_run_id())
            .await
            .response
    }

    pub async fn execute_with_audit(
        &self,
        request: GenerateReadingRequest,
        run_id: String,
    ) -> UseCaseOutput {
        let mut audit = ExecutionAudit::new(&run_id);
        audit.idempotency_key = request.idempotency_key.clone();

        let response = match self.execute_inner(&request, &run_id, &mut audit).await {
            Ok(reading) => GenerateReadingResponse::Success(StructuredReadingResponse {
                run_id: run_id.clone(),
                reading,
            }),
            Err(GenerationError::Detailed { detail, .. })
                if matches!(
                    detail.code,
                    GenerationErrorCode::SafetyRejected
                        | GenerationErrorCode::PostSafetyValidationFailed
                ) =>
            {
                build_safety_rejected(&run_id, &detail)
            }
            Err(err) => GenerateReadingResponse::Failed(GenerationFailedResponse {
                run_id: run_id.clone(),
                error: err.detail().clone(),
            }),
        };

        UseCaseOutput { response, audit }
    }

    async fn execute_inner(
        &self,
        request: &GenerateReadingRequest,
        run_id: &str,
        audit: &mut ExecutionAudit,
    ) -> Result<astral_llm_domain::NatalReadingResponse, GenerationError> {
        RequestValidator::validate(request, &self.limits, &self.catalog)?;

        let mut engine = resolve_engine_params(
            &request.engine,
            &self.engine_defaults,
            self.limits.default_request_timeout_ms,
        );
        drop_unsupported_reasoning(&mut engine, self.router.capability_registry());
        RequestValidator::validate_engine_resolved(&engine.provider, &engine.model)?;

        let product_policy =
            ProductPolicyValidator::validate(request, &self.catalog, &engine.provider, &engine.model)?;

        self.router.capability_registry().validate_request_capabilities(
            &engine.provider,
            &engine.model,
            engine.reasoning_effort,
            true,
        )?;

        if !self.privacy_policy.allow_external_provider
            && engine.provider != ProviderKind::Fake
        {
            return Err(GenerationError::new(
                GenerationErrorCode::PolicyViolation,
                "external LLM providers are disabled by privacy policy",
            ));
        }

        let astro_facts =
            AstroPayloadNormalizer::normalize(&request.astro_result, &self.privacy_policy)?;

        let product_default =
            SafetyResolver::product_default_for(&request.product_context.product_code);
        let safety_policy =
            SafetyResolver::resolve(&product_default, request.safety_policy.as_ref());

        SafetyGuard::validate_request(request, &safety_policy, &self.catalog).map_err(|violations| {
            GenerationError::with_details(
                GenerationErrorCode::SafetyRejected,
                "request failed safety validation",
                serde_json::json!({ "violations": violations, "category": "request_safety" }),
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
                let result = orchestrator
                    .generate(
                        request,
                        &engine,
                        &safety_policy,
                        &astro_facts,
                        product_policy,
                        run_id,
                        audit,
                    )
                    .await?;
                audit.selected_domains = result.plan.selected_domains.clone();
                result.reading
            }
            GenerationMode::SinglePass => {
                let domains = DomainResolver::resolve(
                    request,
                    &self.catalog,
                    &self.limits,
                    product_policy,
                );
                audit.selected_domains = domains.clone();
                self.generate_single_pass(
                    request,
                    &engine,
                    &safety_policy,
                    &astro_facts,
                    &domains,
                    run_id,
                )
                .await?
            }
        };

        AstroBasisValidator::validate_chapters(&reading.chapters, &astro_facts)?;

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

        ReadingQualityValidator::validate_for_product(request, &reading)?;
        Ok(reading)
    }

    async fn generate_single_pass(
        &self,
        request: &GenerateReadingRequest,
        engine: &ResolvedEngineParams,
        safety_policy: &astral_llm_domain::SafetyPolicy,
        astro_facts: &astral_llm_domain::NormalizedAstroFacts,
        domains: &[String],
        run_id: &str,
    ) -> Result<astral_llm_domain::NatalReadingResponse, GenerationError> {
        let bundle = self
            .compiler
            .compile(PromptCompilationInput {
                request,
                safety_policy,
                astro_facts,
                selected_domains: domains,
                chapter_code: None,
                catalog: &self.catalog,
            })
            .map_err(|e| GenerationError::new(GenerationErrorCode::InvalidInput, e))?;

        let messages = self.compiler.to_provider_messages(&bundle);
        let canonical_schema = self
            .validator
            .schema_registry()
            .provider_schema(&request.response_contract.output_schema_version)
            .cloned();

        let model_cap = self
            .router
            .capability_registry()
            .require(&engine.provider, &engine.model)?;

        let schema = if let Some(ref canonical) = canonical_schema {
            Some(ProviderSchemaCompiler::compile(canonical, model_cap)?)
        } else {
            None
        };

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
                &engine.model,
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

        let prompt_family = bundle.prompt_family.clone();
        let prompt_version = bundle.prompt_version.clone();

        reading.quality.used_provider = route.used_provider.as_str().to_string();
        reading.quality.used_model = route.response.model_used;
        reading.quality.prompt_family = prompt_family.clone();
        reading.quality.prompt_version = prompt_version.clone();
        reading.quality.astro_contract_version = request.astro_result.contract_version.clone();
        reading.quality.fallback_used = route.fallback_used;
        reading.language = request.product_context.user_language.clone();
        reading.reading_type = request.product_context.product_code.clone();

        let _versions = GenerationRunContractVersions::new(
            &request.astro_result.contract_version,
            &request.response_contract.output_schema_version,
            &prompt_family,
            &prompt_version,
        );

        Ok(reading)
    }
}

fn build_safety_rejected(
    run_id: &str,
    detail: &astral_llm_domain::GenerationErrorDetail,
) -> GenerateReadingResponse {
    let violations: Vec<String> = detail
        .details
        .as_ref()
        .and_then(|v| v.get("violations"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_else(|| vec![detail.message.clone()]);
    let category = detail
        .details
        .as_ref()
        .and_then(|v| v.get("category"))
        .and_then(|v| v.as_str())
        .unwrap_or("safety_policy")
        .to_string();
    GenerateReadingResponse::SafetyRejected(SafetyRejectedResponse::new(
        run_id,
        category,
        detail.message.clone(),
        detail
            .details
            .as_ref()
            .and_then(|v| v.get("rule_id"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        violations,
    ))
}
