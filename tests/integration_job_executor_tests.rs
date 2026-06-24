use astral_llm_application::{
    build_provider_map, IntegrationJobExecutor, ModelCapabilityRegistry, PromptCompiler,
    ProviderCircuitBreaker, ProviderRouter, ResponseValidator, SchemaRegistry,
};
use astral_llm_application::{core::calculator::CalculatorPort, IntegrationJobValidator};
use astral_llm_application::{supports_integration_service, unified_result_envelope};
use astral_llm_domain::integration::{CalculationMode, IntegrationService, ServiceAvailability};
use astral_llm_domain::{
    astrologer_profile::{JargonLevel, ToneProfile, WordingStyle},
    engine_params::EngineParams,
    generation_request::{AudienceLevel, GenerateReadingRequest, ProductContext},
    output_contract::{GenerationMode, OutputFormat, ResponseContract},
    provider::{ProviderCapabilities, ProviderKind},
    AstroCalculationPayload, AstrologerProfile, EngineDefaults, FallbackPolicy, GenerationError,
    PrivacyPolicy, ServiceLimits,
};
use astral_llm_infra::{
    bootstrap_astro_basis_roles, bootstrap_domains, bootstrap_interpretation_profiles,
    bootstrap_product_policies, CanonicalCatalog,
};
use astral_llm_providers::{
    FakeProvider, LlmProvider, LlmProviderError, ProviderGenerationRequest,
    ProviderGenerationResponse,
};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

fn service(
    service_code: &str,
    orchestration_mode: &str,
    calculation_mode: CalculationMode,
    payload_contract: &str,
) -> IntegrationService {
    IntegrationService {
        service_code: service_code.into(),
        profile_code: "natal_basic".into(),
        product_code: "natal_prompter".into(),
        label_fr: "Service".into(),
        description_fr: "Service".into(),
        orchestration_mode: orchestration_mode.into(),
        orchestration_mode_typed: None,
        calculation_mode,
        service_request_contract: "integration_job_request_v1".into(),
        payload_contract: payload_contract.into(),
        service_response_contract: "integration_job_response_v1".into(),
        public_request_contract: None,
        calculator_request_contract: None,
        llm_request_contract: None,
        public_response_contract: None,
        calculation_output_contract: None,
        reading_output_contract: "generate_reading_response_v1".into(),
        sync_endpoint: None,
        async_endpoint: "/v1/jobs".into(),
        supports_async: true,
        supports_sync_legacy: false,
        supports_mercure: false,
        availability: ServiceAvailability::Active,
        example_request_json: None,
        sort_order: 1,
    }
}

fn test_catalog() -> Arc<CanonicalCatalog> {
    Arc::new(CanonicalCatalog {
        astrological_domains: bootstrap_domains(),
        astro_basis_roles: bootstrap_astro_basis_roles(),
        product_generation_policies: bootstrap_product_policies(),
        interpretation_profiles: bootstrap_interpretation_profiles(),
        ..Default::default()
    })
}

struct FixtureCalculator {
    natal_response: Option<serde_json::Value>,
}

impl FixtureCalculator {
    fn unused() -> Self {
        Self {
            natal_response: None,
        }
    }

    fn full_natal() -> Self {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..");
        let raw =
            std::fs::read_to_string(root.join(
                "contracts/integration/examples/natal_calculation_response_v1.paris_1990.json",
            ))
            .expect("natal calculation response fixture");
        Self {
            natal_response: Some(serde_json::from_str(&raw).expect("valid natal fixture json")),
        }
    }
}

impl CalculatorPort for FixtureCalculator {
    async fn calculate_simplified_natal(
        &self,
        _request: &serde_json::Value,
    ) -> Result<serde_json::Value, GenerationError> {
        unreachable!("from-payload jobs do not call the calculator")
    }

    async fn calculate_natal(
        &self,
        _request: &serde_json::Value,
    ) -> Result<serde_json::Value, GenerationError> {
        Ok(self
            .natal_response
            .clone()
            .expect("fixture calculator missing natal response"))
    }

    async fn calculate_horoscope_daily_natal(
        &self,
        _request: &serde_json::Value,
    ) -> Result<serde_json::Value, GenerationError> {
        unreachable!("from-payload jobs do not call the calculator")
    }

    async fn calculate_horoscope_period_natal(
        &self,
        _request: &serde_json::Value,
    ) -> Result<serde_json::Value, GenerationError> {
        unreachable!("from-payload jobs do not call the calculator")
    }
}

struct CaptureProvider {
    kind: ProviderKind,
    requests: Mutex<Vec<ProviderGenerationRequest>>,
}

impl CaptureProvider {
    fn new(kind: ProviderKind) -> Self {
        Self {
            kind,
            requests: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl LlmProvider for CaptureProvider {
    fn kind(&self) -> ProviderKind {
        self.kind.clone()
    }

    fn capabilities(&self) -> ProviderCapabilities {
        FakeProvider.capabilities()
    }

    async fn generate(
        &self,
        request: ProviderGenerationRequest,
    ) -> Result<ProviderGenerationResponse, LlmProviderError> {
        self.requests.lock().await.push(request.clone());
        FakeProvider.generate(request).await
    }
}

fn use_case_with_capture(
    provider: Arc<CaptureProvider>,
) -> astral_llm_application::GenerateReadingUseCase {
    let router = ProviderRouter::new(
        build_provider_map(vec![provider]),
        FallbackPolicy::disabled(),
        Arc::new(ModelCapabilityRegistry::bootstrap_dev_fallback()),
        PrivacyPolicy::default(),
        Arc::new(ProviderCircuitBreaker::new(5, 60)),
        None,
    );
    let prompts = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../prompts");
    astral_llm_application::GenerateReadingUseCase::new(
        router,
        PromptCompiler::new(prompts),
        ResponseValidator::new(Arc::new(SchemaRegistry::new())),
        EngineDefaults {
            provider: ProviderKind::Fake,
            model: "fake-model".into(),
        },
        ServiceLimits::default(),
        test_catalog(),
        PrivacyPolicy::default(),
        true,
        None,
    )
}

fn from_payload_request() -> GenerateReadingRequest {
    GenerateReadingRequest {
        request_id: Some("job-test".into()),
        idempotency_key: None,
        product_context: ProductContext {
            product_code: "natal_prompter".into(),
            interpretation_profile_code: Some("natal_basic".into()),
            user_language: "fr".into(),
            audience_level: AudienceLevel::Beginner,
        },
        astro_result: AstroCalculationPayload {
            contract_version: "natal_structured_v14".into(),
            chart_type: "natal".into(),
            data: serde_json::json!({
                "planets": {
                    "sun": { "house": 10, "sign": "taurus" },
                    "moon": { "house": 6, "sign": "capricorn" }
                },
                "angles": {
                    "mc": { "sign": "aries" }
                }
            }),
        },
        astrologer_profile: AstrologerProfile {
            profile_id: None,
            name: None,
            tone: ToneProfile::Warm,
            jargon_level: JargonLevel::Beginner,
            wording_style: WordingStyle::Clear,
            preferred_domains: vec!["identity".into(), "career".into()],
            forbidden_wording: vec![],
            custom_instructions: None,
        },
        engine: EngineParams {
            provider: None,
            model: None,
            allow_fallback: true,
            ..Default::default()
        },
        response_contract: ResponseContract {
            output_schema_version: "natal_reading_v1".into(),
            generation_mode: GenerationMode::SinglePass,
            format: OutputFormat::StructuredJson,
            chapters: vec![],
            global_max_tokens: None,
            include_astro_sources: true,
            include_legal_disclaimer: true,
        },
        safety_policy: None,
    }
}

#[test]
fn supports_horoscope_services_from_shared_registry() {
    let service = service(
        astral_contracts::HOROSCOPE_FREE_DAILY_SERVICE_CODE,
        "calculator_then_llm",
        CalculationMode::None,
        "horoscope_daily_natal_request",
    );

    assert!(supports_integration_service(&service));
}

#[test]
fn supports_simplified_and_full_natal_for_current_orchestrations() {
    let simplified = service(
        "natal_simplified",
        "calculator_then_llm",
        CalculationMode::SimplifiedNatal,
        "astro_simplified_natal_request_v1",
    );
    let full = service(
        "natal_basic",
        "legacy_unified",
        CalculationMode::FullNatal,
        "astro_engine_request_v1",
    );

    assert!(supports_integration_service(&simplified));
    assert!(supports_integration_service(&full));
}

#[test]
fn rejects_natal_services_when_contract_or_orchestration_is_wrong() {
    let wrong_contract = service(
        "natal_simplified",
        "calculator_then_llm",
        CalculationMode::SimplifiedNatal,
        "generate_reading_request_v1",
    );
    let wrong_mode = service(
        "natal_basic",
        "public_gateway",
        CalculationMode::FullNatal,
        "astro_engine_request_v1",
    );

    assert!(!supports_integration_service(&wrong_contract));
    assert!(!supports_integration_service(&wrong_mode));
}

#[test]
fn supports_interpretation_only_from_payload_and_rejects_others() {
    let valid = service(
        "natal_basic_from_payload",
        "llm_only",
        CalculationMode::None,
        "generate_reading_request_v1",
    );
    let wrong_contract = service(
        "natal_basic_from_payload",
        "interpretation_only",
        CalculationMode::None,
        "astro_engine_request_v1",
    );
    let none_mode = service(
        "ad_hoc",
        "calculator_then_llm",
        CalculationMode::None,
        "generate_reading_request_v1",
    );

    assert!(supports_integration_service(&valid));
    assert!(!supports_integration_service(&wrong_contract));
    assert!(!supports_integration_service(&none_mode));
}

#[tokio::test]
async fn from_payload_job_exposes_explanations_and_injects_neutral_prompt_block() {
    let provider = Arc::new(CaptureProvider::new(ProviderKind::OpenAi));
    let use_case = use_case_with_capture(provider.clone());
    let calculator = FixtureCalculator::unused();
    let executor = IntegrationJobExecutor::new(&calculator, &use_case);
    let mut service = service(
        "natal_basic_from_payload",
        "interpretation_only",
        CalculationMode::None,
        "generate_reading_request_v1",
    );
    service.availability = ServiceAvailability::Active;

    let body = serde_json::json!({
        "service_code": "natal_basic_from_payload",
        "payload": serde_json::to_value(from_payload_request()).unwrap(),
        "user_language": "fr",
        "audience_level": "beginner"
    });
    let job = IntegrationJobValidator::new()
        .validate_job(&body, &service)
        .expect("valid from-payload job");

    let result = executor
        .execute(&service, &job, Some("00000000-0000-0000-0000-000000000123"))
        .await
        .expect("job execution");

    let explanations = match result.outcome {
        astral_llm_application::UnifiedReadingOutcome::Reading {
            calculation,
            reading,
            reading_completeness,
            explanations,
        } => {
            let explanations = explanations.expect("job exposes explanations");
            let envelope = unified_result_envelope(
                calculation,
                &reading,
                reading_completeness,
                Some(explanations.clone()),
            );
            assert_eq!(envelope["explanations"], explanations);
            explanations
        }
        other => panic!("expected reading outcome, got {other:?}"),
    };
    assert_eq!(explanations["status"], "complete");
    assert!(
        explanations["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty()),
        "explanations should contain public items: {explanations:?}"
    );

    let captured = provider.requests.lock().await;
    let reading_prompts = captured
        .iter()
        .filter(|request| request.metadata.product_code == "natal_prompter")
        .collect::<Vec<_>>();
    assert!(
        !reading_prompts.is_empty(),
        "expected at least one natal reading provider call"
    );
    assert!(
        reading_prompts.iter().any(|request| {
            request
                .messages
                .iter()
                .any(|message| message.content.contains("\"neutral_explanations\""))
        }),
        "reading prompt should include neutral_explanations"
    );
}

#[tokio::test]
async fn full_natal_job_exposes_explanations_and_injects_neutral_prompt_block() {
    let provider = Arc::new(CaptureProvider::new(ProviderKind::OpenAi));
    let use_case = use_case_with_capture(provider.clone());
    let calculator = FixtureCalculator::full_natal();
    let executor = IntegrationJobExecutor::new(&calculator, &use_case);
    let service = service(
        "natal_basic",
        "legacy_unified",
        CalculationMode::FullNatal,
        "astro_engine_request_v1",
    );
    let job = astral_llm_application::ValidatedIntegrationJob {
        service_code: "natal_basic".into(),
        profile_code: "natal_basic".into(),
        envelope: serde_json::json!({
            "service_code": "natal_basic",
            "payload": {},
            "user_language": "fr",
            "audience_level": "beginner"
        }),
        payload: serde_json::json!({}),
        user_language: "fr".into(),
        audience_level: "beginner".into(),
    };

    let result = executor
        .execute(&service, &job, Some("00000000-0000-0000-0000-000000000456"))
        .await
        .expect("full natal job execution");

    let explanations = match result.outcome {
        astral_llm_application::UnifiedReadingOutcome::Reading {
            calculation,
            reading,
            reading_completeness,
            explanations,
        } => {
            let explanations = explanations.expect("full natal job exposes explanations");
            let envelope = unified_result_envelope(
                calculation,
                &reading,
                reading_completeness,
                Some(explanations.clone()),
            );
            assert_eq!(envelope["explanations"], explanations);
            explanations
        }
        other => panic!("expected reading outcome, got {other:?}"),
    };
    assert_eq!(explanations["status"], "complete");
    assert!(
        explanations["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty()),
        "full natal explanations should contain public items: {explanations:?}"
    );

    let captured = provider.requests.lock().await;
    let reading_prompts = captured
        .iter()
        .filter(|request| request.metadata.product_code == "natal_prompter")
        .collect::<Vec<_>>();
    assert!(
        !reading_prompts.is_empty(),
        "expected at least one full natal reading provider call"
    );
    assert!(
        reading_prompts.iter().any(|request| {
            request
                .messages
                .iter()
                .any(|message| message.content.contains("\"neutral_explanations\""))
        }),
        "full natal reading prompt should include neutral_explanations"
    );
}
