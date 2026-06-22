use std::time::Duration;

use astral_llm_domain::{
    contract_versions::GenerationRunContractVersions,
    generation_response::{GenerateReadingResponse, SafetyRejectedResponse},
    model_usage_tier::ModelRouteContext,
    output_contract::GenerationMode,
    EngineDefaults, GenerateReadingRequest, GenerationError, GenerationErrorCode, PrivacyPolicy,
    ProviderKind, PublicTokenUsage, SafetyMode, ServiceLimits, TokenUsage,
};
use astral_llm_infra::{hash_json, SharedCanonicalCatalog};

use astral_llm_providers::{GenerationMetadata, ProviderGenerationRequest};
use chrono::Utc;

use crate::astro_basis_validator::AstroBasisValidator;
use crate::astro_payload_normalizer::AstroPayloadNormalizer;
use crate::chapter_orchestrator::{new_run_id, ChapterOrchestrator};
use crate::domain_resolver::DomainResolver;
use crate::engine_defaults::{
    drop_unsupported_reasoning, drop_unsupported_temperature, resolve_engine_params,
    resolve_service_engine_defaults, resolve_subtask_engine, ResolvedEngineParams,
};
use crate::execution_audit::ExecutionAudit;
use crate::interpretation_profile_resolver::InterpretationProfileResolver;
use crate::prompt_compiler::{PromptCompilationInput, PromptCompiler};
use crate::prompt_trace;
use crate::provider_router::ProviderRouter;
use crate::provider_schema_compiler::ProviderSchemaCompiler;
use crate::reading_persistence::{
    priced_usage_records, PersistedGenerationRunRecord, PersistedRunStatus, PersistedSafetyStatus,
    SharedReadingPersistence,
};
use crate::reading_quality_validator::ReadingQualityValidator;
use crate::reasoning_generation::{
    apply_reasoning_output_reserve, effective_temperature, resolve_reasoning_effort,
};
use crate::request_validator::RequestValidator;
use crate::response_validator::ResponseValidator;
use crate::safety_guard::SafetyGuard;
use crate::safety_resolver::SafetyResolver;
use crate::simplified_reading::{sun_sign_blocked, SIMPLIFIED_PROFILE};
use crate::simplified_reading_guard::{
    ambiguous_core_identity_violations, blocked_sign_affirmation_violations,
    profile_excluded_affirmation_violations, validate_allowed_astro_basis_ids,
};

pub struct GenerateReadingUseCase {
    pub router: ProviderRouter,
    compiler: PromptCompiler,
    validator: ResponseValidator,
    engine_defaults: EngineDefaults,
    limits: ServiceLimits,
    pub(super) catalog: SharedCanonicalCatalog,
    privacy_policy: PrivacyPolicy,
    legacy_product_code_shim_available: bool,
    persistence: Option<SharedReadingPersistence>,
}

pub struct UseCaseOutput {
    pub response: GenerateReadingResponse,
    pub audit: ExecutionAudit,
}

pub(super) struct SinglePassGenerationResult {
    pub reading: astral_llm_domain::NatalReadingResponse,
    pub used_provider: String,
    pub used_model: String,
    pub token_usage: Option<TokenUsage>,
    pub latency_ms: u64,
}

#[derive(Debug, Clone)]
struct RunAuditContext {
    request_id: Option<String>,
    idempotency_key: Option<String>,
    product_code: String,
    user_language: String,
    astro_contract_version: String,
    output_schema_version: String,
    provider_requested: String,
    model_requested: String,
    generation_mode: String,
    safety_policy_version: String,
    input_hash: String,
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
        legacy_product_code_shim_available: bool,
        persistence: Option<SharedReadingPersistence>,
    ) -> Self {
        Self {
            router,
            compiler,
            validator,
            engine_defaults,
            limits,
            catalog,
            privacy_policy,
            legacy_product_code_shim_available,
            persistence,
        }
    }

    /// Normalise la requete (shim legacy + `generation_mode` depuis le profil) avant idempotence / rate limit.
    pub fn catalog(&self) -> &SharedCanonicalCatalog {
        &self.catalog
    }

    pub fn engine_defaults(&self) -> &EngineDefaults {
        &self.engine_defaults
    }

    pub fn persistence(&self) -> Option<&SharedReadingPersistence> {
        self.persistence.as_ref()
    }

    pub fn prepare_request(
        &self,
        request: &mut GenerateReadingRequest,
    ) -> Result<(), GenerationError> {
        InterpretationProfileResolver::normalize_request(
            request,
            &self.catalog,
            self.legacy_product_code_shim_available,
        )
    }

    pub fn requires_premium_rate_limit(&self, request: &GenerateReadingRequest) -> bool {
        InterpretationProfileResolver::requires_premium_rate_limit(request, &self.catalog)
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
        let started_at = std::time::Instant::now();
        let run_context = RunAuditContext {
            request_id: request.request_id.clone(),
            idempotency_key: request.idempotency_key.clone(),
            product_code: request.product_context.product_code.clone(),
            user_language: request.product_context.user_language.clone(),
            astro_contract_version: request.astro_result.contract_version.clone(),
            output_schema_version: request.response_contract.output_schema_version.clone(),
            provider_requested: request
                .engine
                .provider
                .clone()
                .unwrap_or_else(|| self.engine_defaults.provider.clone())
                .as_str()
                .to_string(),
            model_requested: request
                .engine
                .model
                .clone()
                .unwrap_or_else(|| self.engine_defaults.model.clone()),
            generation_mode: request
                .response_contract
                .generation_mode
                .as_str()
                .to_string(),
            safety_policy_version: "product_default".into(),
            input_hash: hash_json(&serde_json::to_value(&request).unwrap_or_default()),
        };

        self.persist_run_started(&run_context, &run_id, &audit)
            .await;

        let response = match self.execute_inner(request, &run_id, &mut audit).await {
            Ok(reading) => {
                let token_usage = self.audit_public_usage(
                    reading.quality.used_provider.as_str(),
                    reading.quality.used_model.as_str(),
                    audit.aggregate_detailed_usage(),
                );
                GenerateReadingResponse::Success {
                    run_id: run_id.clone(),
                    reading,
                    token_usage,
                }
            }
            Err(GenerationError::Detailed { detail, .. })
                if matches!(
                    detail.code,
                    GenerationErrorCode::SafetyRejected
                        | GenerationErrorCode::PostSafetyValidationFailed
                ) =>
            {
                let token_usage = audit
                    .steps
                    .last()
                    .map(|step| (step.provider.as_str(), step.model.as_str()))
                    .and_then(|(provider, model)| {
                        self.audit_public_usage(provider, model, audit.aggregate_detailed_usage())
                    });
                build_safety_rejected(&run_id, &detail, token_usage)
            }
            Err(err) => {
                let token_usage = audit
                    .steps
                    .last()
                    .map(|step| (step.provider.as_str(), step.model.as_str()))
                    .and_then(|(provider, model)| {
                        self.audit_public_usage(provider, model, audit.aggregate_detailed_usage())
                    });
                GenerateReadingResponse::Failed {
                    run_id: run_id.clone(),
                    error: err.detail().clone(),
                    token_usage,
                }
            }
        };

        self.persist_run_finished(&run_context, &response, started_at.elapsed(), &audit)
            .await;

        UseCaseOutput { response, audit }
    }

    async fn execute_inner(
        &self,
        mut request: GenerateReadingRequest,
        run_id: &str,
        audit: &mut ExecutionAudit,
    ) -> Result<astral_llm_domain::NatalReadingResponse, GenerationError> {
        // Idempotent : l'API peut deja avoir appele prepare_request().
        InterpretationProfileResolver::normalize_request(
            &mut request,
            &self.catalog,
            self.legacy_product_code_shim_available,
        )?;
        RequestValidator::validate(&request, &self.limits, &self.catalog)?;

        let service_defaults =
            resolve_service_engine_defaults(&self.engine_defaults, &self.catalog, &request);
        let mut engine = resolve_engine_params(
            &request.engine,
            &service_defaults,
            self.limits.default_request_timeout_ms,
        );
        let registry = self.router.capability_registry();
        drop_unsupported_reasoning(&mut engine, registry);
        drop_unsupported_temperature(&mut engine, registry);
        self.router
            .capability_registry()
            .validate_engine_for_context(
                ModelRouteContext::PrimaryReading,
                &engine.provider,
                &engine.model,
                engine.allow_oracle_benchmark,
            )?;

        let validated = InterpretationProfileResolver::validate_product(
            &request,
            &self.catalog,
            &engine.provider,
            &engine.model,
        )?;
        let product_policy = &validated.policy;
        let interpretation = validated.interpretation.as_ref();

        self.router
            .capability_registry()
            .validate_request_capabilities(
                ModelRouteContext::PrimaryReading,
                &engine.provider,
                &engine.model,
                engine.reasoning_effort,
                true,
            )?;

        if matches!(
            request.response_contract.generation_mode,
            GenerationMode::ChapterOrchestrated
        ) {
            let summary_engine =
                resolve_subtask_engine(&engine, &request.engine, Some(&validated.policy));
            self.router
                .capability_registry()
                .validate_request_capabilities(
                    ModelRouteContext::Subtask,
                    &summary_engine.provider,
                    &summary_engine.model,
                    engine.reasoning_effort,
                    true,
                )?;
        }

        if !self.privacy_policy.allow_external_provider && engine.provider != ProviderKind::Fake {
            return Err(GenerationError::new(
                GenerationErrorCode::PolicyViolation,
                "external LLM providers are disabled by privacy policy",
            ));
        }

        let astro_facts = AstroPayloadNormalizer::normalize(
            &request.astro_result,
            &self.privacy_policy,
            &self.catalog,
            &request.product_context.user_language,
        )?;

        let product_default = SafetyResolver::product_default_for(
            &request.product_context.product_code,
            interpretation,
        );
        let safety_policy =
            SafetyResolver::resolve(&product_default, request.safety_policy.as_ref());

        SafetyGuard::validate_request(&request, &safety_policy, &self.catalog).map_err(
            |violations| {
                GenerationError::with_details(
                    GenerationErrorCode::SafetyRejected,
                    "request failed safety validation",
                    serde_json::json!({ "violations": violations, "category": "request_safety" }),
                )
            },
        )?;

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
                        &request,
                        &engine,
                        &safety_policy,
                        &astro_facts,
                        product_policy,
                        interpretation,
                        run_id,
                        audit,
                    )
                    .await?;
                audit.selected_domains = result.plan.selected_domains.clone();
                result.reading
            }
            GenerationMode::SinglePass => {
                let domains = DomainResolver::resolve(
                    &request,
                    &self.catalog,
                    &self.limits,
                    product_policy,
                    interpretation,
                );
                audit.selected_domains = domains.clone();
                self.generate_single_pass_hardened(
                    &request,
                    &engine,
                    &safety_policy,
                    &astro_facts,
                    &domains,
                    product_policy,
                    interpretation,
                    run_id,
                    audit,
                )
                .await?
            }
        };

        if !matches!(
            request.response_contract.generation_mode,
            GenerationMode::ChapterOrchestrated
        ) {
            AstroBasisValidator::validate_chapters(
                &reading.chapters,
                &astro_facts,
                &self.catalog,
                product_policy,
            )?;
        }

        if request
            .product_context
            .interpretation_profile_code
            .as_deref()
            == Some(SIMPLIFIED_PROFILE)
            && matches!(
                request.response_contract.generation_mode,
                GenerationMode::ChapterOrchestrated
            )
        {
            self.validate_simplified_reading(&request, &reading)?;
        }

        if !matches!(
            request.response_contract.generation_mode,
            GenerationMode::SinglePass
        ) {
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
        }

        ReadingQualityValidator::validate_for_product(&request, &reading, interpretation)?;
        Ok(reading)
    }

    pub(super) fn validate_simplified_reading(
        &self,
        request: &GenerateReadingRequest,
        reading: &astral_llm_domain::NatalReadingResponse,
    ) -> Result<(), GenerationError> {
        let controls = request
            .astro_result
            .data
            .get("llm_controls")
            .ok_or_else(|| {
                GenerationError::new(
                    GenerationErrorCode::InvalidInput,
                    "simplified reading missing llm_controls in astro payload",
                )
            })?;

        let allowed_ids = controls
            .get("allowed_astro_basis_fact_ids")
            .and_then(|v| v.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        validate_allowed_astro_basis_ids(&reading.chapters, &allowed_ids)?;

        let blocked = controls
            .get("blocked_interpretation_fact_codes")
            .and_then(|v| v.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let profile_excluded = controls
            .get("profile_excluded_feature_codes")
            .and_then(|v| v.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mut violations = blocked_sign_affirmation_violations(
            reading,
            &blocked,
            &self.catalog,
            &request.product_context.user_language,
        );
        violations.extend(profile_excluded_affirmation_violations(
            reading,
            &profile_excluded,
        ));
        violations.extend(ambiguous_core_identity_violations(
            reading,
            sun_sign_blocked(controls),
            &request.product_context.user_language,
        ));

        if violations.is_empty() {
            Ok(())
        } else {
            Err(GenerationError::with_details(
                GenerationErrorCode::PostSafetyValidationFailed,
                "generated content failed simplified reading guard",
                serde_json::json!({ "violations": violations }),
            ))
        }
    }

    pub(super) async fn generate_single_pass(
        &self,
        request: &GenerateReadingRequest,
        engine: &ResolvedEngineParams,
        safety_policy: &astral_llm_domain::SafetyPolicy,
        astro_facts: &astral_llm_domain::NormalizedAstroFacts,
        domains: &[String],
        product_policy: &astral_llm_domain::ProductGenerationPolicy,
        interpretation: Option<
            &crate::interpretation_profile_resolver::ResolvedInterpretationContext,
        >,
        run_id: &str,
        repair_instruction: Option<&str>,
    ) -> Result<SinglePassGenerationResult, GenerationError> {
        let chapter_code = request
            .response_contract
            .chapters
            .first()
            .map(|c| c.code.as_str());

        let bundle = self
            .compiler
            .compile(PromptCompilationInput {
                request,
                safety_policy,
                astro_facts,
                selected_domains: domains,
                chapter_code,
                chapter_evidence_pack: None,
                catalog: &self.catalog,
                interpretation,
                repair_instruction,
            })
            .map_err(|e| GenerationError::new(GenerationErrorCode::InvalidInput, e))?;

        prompt_trace::log_prompt_bundle(run_id, None, &bundle, &self.compiler, Some("primary"));
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

        let base_tokens = engine
            .max_output_tokens
            .or(request.response_contract.global_max_tokens)
            .unwrap_or(product_policy.max_output_tokens);

        let provider_request = ProviderGenerationRequest {
            model: engine.model.clone(),
            messages,
            structured_schema: schema,
            reasoning_effort: resolve_reasoning_effort(
                model_cap,
                product_policy,
                engine.reasoning_effort,
                astral_llm_domain::ModelRouteContext::PrimaryReading,
            ),
            temperature: effective_temperature(model_cap, engine.temperature),
            max_output_tokens: Some(apply_reasoning_output_reserve(model_cap, base_tokens)),
            safety_mode: if engine.provider == astral_llm_domain::ProviderKind::Mistral {
                SafetyMode::PlatformAndNative
            } else {
                SafetyMode::PlatformRulesOnly
            },
            timeout: Duration::from_millis(engine.timeout_ms.unwrap_or(900_000)),
            metadata: GenerationMetadata {
                run_id: run_id.to_string(),
                request_id: request.request_id.clone(),
                product_code: request.product_context.product_code.clone(),
                chapter_code: chapter_code.map(str::to_string),
                prompt_trace_step: Some("single_pass_generate".into()),
                prompt_trace_attempt: Some(if repair_instruction.is_some() {
                    "repair".into()
                } else {
                    "primary".into()
                }),
                prompt_family: Some(bundle.prompt_family.clone()),
                prompt_version: Some(bundle.prompt_version.clone()),
            },
        };

        let route_started_at = std::time::Instant::now();
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
        let route_latency_ms =
            u64::try_from(route_started_at.elapsed().as_millis()).unwrap_or(u64::MAX);

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

        for chapter in &mut reading.chapters {
            crate::evidence_fact_parse::normalize_chapter_astro_basis_fact_ids(
                chapter,
                astro_facts,
            );
        }

        let prompt_family = bundle.prompt_family.clone();
        let prompt_version = bundle.prompt_version.clone();

        let used_provider = route.used_provider.as_str().to_string();
        let used_model = route.response.model_used.clone();
        reading.quality.used_provider = used_provider.clone();
        reading.quality.used_model = used_model.clone();
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

        Ok(SinglePassGenerationResult {
            reading,
            used_provider,
            used_model,
            token_usage: route.response.usage,
            latency_ms: route_latency_ms,
        })
    }

    async fn persist_run_started(
        &self,
        context: &RunAuditContext,
        run_id: &str,
        audit: &ExecutionAudit,
    ) {
        let Some(persistence) = self.persistence.as_ref() else {
            return;
        };
        let Ok(run_uuid) = uuid::Uuid::parse_str(run_id) else {
            return;
        };
        let record = PersistedGenerationRunRecord {
            id: run_uuid,
            request_id: context.request_id.clone(),
            idempotency_key: context.idempotency_key.clone(),
            product_code: context.product_code.clone(),
            user_language: context.user_language.clone(),
            astro_contract_version: context.astro_contract_version.clone(),
            output_schema_version: context.output_schema_version.clone(),
            prompt_family: "-".into(),
            prompt_version: "-".into(),
            safety_policy_version: context.safety_policy_version.clone(),
            provider_requested: context.provider_requested.clone(),
            provider_used: None,
            model_requested: context.model_requested.clone(),
            model_used: None,
            generation_mode: context.generation_mode.clone(),
            fallback_used: false,
            selected_domains: if audit.selected_domains.is_empty() {
                None
            } else {
                Some(serde_json::json!(audit.selected_domains))
            },
            status: PersistedRunStatus::Pending,
            safety_status: PersistedSafetyStatus::NotChecked,
            input_hash: context.input_hash.clone(),
            output_hash: None,
            token_input: None,
            token_output: None,
            latency_ms: None,
            error_code: None,
            created_at: Utc::now(),
        };
        if let Err(err) = persistence.upsert_run(&record).await {
            tracing::warn!(run_id, error = %err, "failed to persist pending generation run");
        }
    }

    async fn persist_run_finished(
        &self,
        context: &RunAuditContext,
        response: &GenerateReadingResponse,
        latency: std::time::Duration,
        audit: &ExecutionAudit,
    ) {
        let Some(persistence) = self.persistence.as_ref() else {
            return;
        };
        let Ok(run_uuid) = uuid::Uuid::parse_str(&audit.run_id) else {
            return;
        };
        let (token_input, token_output) = audit.aggregate_token_usage();
        let (status, safety_status, provider_used, model_used, output_hash, error) = match response
        {
            GenerateReadingResponse::Success { reading, .. } => (
                PersistedRunStatus::Success,
                PersistedSafetyStatus::Passed,
                Some(reading.quality.used_provider.clone()),
                Some(reading.quality.used_model.clone()),
                serde_json::to_value(response).ok().map(|v| hash_json(&v)),
                None,
            ),
            GenerateReadingResponse::SafetyRejected { error, .. } => (
                PersistedRunStatus::SafetyRejected,
                PersistedSafetyStatus::Rejected,
                None,
                None,
                serde_json::to_value(response).ok().map(|v| hash_json(&v)),
                Some(error.code.clone()),
            ),
            GenerateReadingResponse::Failed { error, .. } => (
                PersistedRunStatus::Failed,
                PersistedSafetyStatus::NotChecked,
                None,
                None,
                serde_json::to_value(response).ok().map(|v| hash_json(&v)),
                Some(error.code.as_str().to_string()),
            ),
        };
        let final_record = PersistedGenerationRunRecord {
            id: run_uuid,
            request_id: context.request_id.clone(),
            idempotency_key: context.idempotency_key.clone(),
            product_code: context.product_code.clone(),
            user_language: context.user_language.clone(),
            astro_contract_version: context.astro_contract_version.clone(),
            output_schema_version: context.output_schema_version.clone(),
            prompt_family: match response {
                GenerateReadingResponse::Success { reading, .. } => {
                    reading.quality.prompt_family.clone()
                }
                _ => "-".into(),
            },
            prompt_version: match response {
                GenerateReadingResponse::Success { reading, .. } => {
                    reading.quality.prompt_version.clone()
                }
                _ => "-".into(),
            },
            safety_policy_version: context.safety_policy_version.clone(),
            provider_requested: context.provider_requested.clone(),
            provider_used,
            model_requested: context.model_requested.clone(),
            model_used,
            generation_mode: context.generation_mode.clone(),
            fallback_used: matches!(response, GenerateReadingResponse::Success { reading, .. } if reading.quality.fallback_used),
            selected_domains: if audit.selected_domains.is_empty() {
                None
            } else {
                Some(serde_json::json!(audit.selected_domains))
            },
            status,
            safety_status,
            input_hash: context.input_hash.clone(),
            output_hash,
            token_input,
            token_output,
            latency_ms: Some(i32::try_from(latency.as_millis()).unwrap_or(i32::MAX)),
            error_code: error,
            created_at: Utc::now(),
        };
        if let Err(err) = persistence.upsert_run(&final_record).await {
            tracing::warn!(run_id = %audit.run_id, error = %err, "failed to persist final generation run");
        }
        let step_ids = match persistence.insert_steps(run_uuid, &audit.steps).await {
            Ok(step_ids) => step_ids,
            Err(err) => {
                tracing::warn!(run_id = %audit.run_id, error = %err, "failed to persist generation steps");
                return;
            }
        };
        if let Some(run_usage) = audit.aggregate_detailed_usage().and_then(|usage| {
            let provider = final_record
                .provider_used
                .as_deref()
                .or(Some(final_record.provider_requested.as_str()))?;
            let model = final_record
                .model_used
                .as_deref()
                .or(Some(final_record.model_requested.as_str()))?;
            self.priced_usage(provider, model, usage)
        }) {
            let usage_records = priced_usage_records(&run_usage);
            if let Err(err) = persistence
                .replace_run_token_usages(run_uuid, &usage_records)
                .await
            {
                tracing::warn!(run_id = %audit.run_id, error = %err, "failed to persist run token usage");
            }
        }
        for (step_id, step) in step_ids.into_iter().zip(&audit.steps) {
            let Some(step_usage) = step.token_usage.clone() else {
                continue;
            };
            let Some(step_usage) = self.priced_usage(&step.provider, &step.model, step_usage)
            else {
                continue;
            };
            let usage_records = priced_usage_records(&step_usage);
            if let Err(err) = persistence
                .replace_step_token_usages(step_id, &usage_records)
                .await
            {
                tracing::warn!(run_id = %audit.run_id, step_id = %step_id, error = %err, "failed to persist step token usage");
            }
        }
    }
}

fn build_safety_rejected(
    run_id: &str,
    detail: &astral_llm_domain::GenerationErrorDetail,
    token_usage: Option<PublicTokenUsage>,
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
    GenerateReadingResponse::SafetyRejected {
        run_id: run_id.to_string(),
        error: SafetyRejectedResponse::new(
            run_id,
            category,
            detail.message.clone(),
            detail
                .details
                .as_ref()
                .and_then(|v| v.get("rule_id"))
                .and_then(|v| v.as_str())
                .map(str::to_string),
            violations.clone(),
        )
        .error,
        violations,
        token_usage,
    }
}

impl GenerateReadingUseCase {
    fn priced_usage(&self, provider: &str, model: &str, usage: TokenUsage) -> Option<TokenUsage> {
        let provider_kind = match provider.trim().to_lowercase().as_str() {
            "openai" => ProviderKind::OpenAi,
            "anthropic" => ProviderKind::Anthropic,
            "mistral" => ProviderKind::Mistral,
            "fake" => ProviderKind::Fake,
            other => ProviderKind::Custom(other.to_string()),
        };
        let capability = self
            .router
            .capability_registry()
            .require(&provider_kind, model)
            .ok()?;
        Some(usage.priced(&capability.token_pricing()))
    }

    fn audit_public_usage(
        &self,
        provider: &str,
        model: &str,
        usage: Option<TokenUsage>,
    ) -> Option<PublicTokenUsage> {
        let usage = self.priced_usage(provider, model, usage?)?;
        let provider_kind = match provider.trim().to_lowercase().as_str() {
            "openai" => ProviderKind::OpenAi,
            "anthropic" => ProviderKind::Anthropic,
            "mistral" => ProviderKind::Mistral,
            "fake" => ProviderKind::Fake,
            other => ProviderKind::Custom(other.to_string()),
        };
        let capability = self
            .router
            .capability_registry()
            .require(&provider_kind, model)
            .ok()?;
        Some(usage.with_pricing(
            &capability.token_pricing(),
            capability.pricing_source.clone(),
            provider.to_string(),
            model.to_string(),
        ))
    }
}
