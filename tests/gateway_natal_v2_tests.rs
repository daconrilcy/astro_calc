use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use astral_contracts::{NatalVariant, ProductTier};
use astral_gateway::{
    contracts::NatalReadingRequestV2, natal::NatalGatewayPolicy, GenerateNatalReadingUseCase,
};
use astral_llm_domain::{
    generation_request::AudienceLevel,
    generation_response::{ConfidenceLevel, GenerateReadingResponse},
    output_contract::GenerationMode,
    GenerateReadingRequest, NatalReadingResponse, ReadingChapter, ReadingSummary,
};
use async_trait::async_trait;
use serde_json::{json, Value};

struct FakeCalculator;

#[async_trait]
impl astral_gateway::ports::CalculatorPort for FakeCalculator {
    async fn calculate_simplified_natal(
        &self,
        _request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        Ok(json!({
            "request_id": "req-1",
            "reading_hint": { "reading_completeness": "compact" },
            "simplified_payload": {
                "payload": {
                    "sun_sign": "gemini"
                }
            },
            "llm_payload": {
                "allowed_fact_codes": [],
                "allowed_astro_basis_fact_ids": [],
                "blocked_interpretation_fact_codes": [],
                "excluded_feature_codes": [],
                "profile_excluded_feature_codes": [],
                "allowed_limitation_mentions": []
            }
        }))
    }

    async fn calculate_full_natal(
        &self,
        _request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        Ok(json!({
            "response_contract_version": "astro_engine_response_v1",
            "calculation_result": { "status": "completed" },
            "audit_payload": {
                "contract_version": "natal_structured_v13",
                "payload": { "positions": [], "signals": [] }
            }
        }))
    }
}

struct FakeLlm;

struct PanicLlm;

struct CountingLlm {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl astral_gateway::ports::LlmPort for FakeLlm {
    async fn generate_reading(
        &self,
        request: &GenerateReadingRequest,
    ) -> Result<GenerateReadingResponse, astral_gateway::error::GatewayError> {
        Ok(GenerateReadingResponse::Success {
            run_id: "run-test".into(),
            reading: NatalReadingResponse {
                schema_version: "natal_reading_v1".into(),
                language: request.product_context.user_language.clone(),
                reading_type: request.product_context.product_code.clone(),
                summary: ReadingSummary {
                    title: "Test".into(),
                    short_text: "Test".into(),
                },
                chapters: vec![ReadingChapter {
                    code: "identity".into(),
                    title: "Identity".into(),
                    body: "Body".into(),
                    astro_basis: vec![],
                    confidence: ConfidenceLevel::Medium,
                    safety_flags: vec![],
                }],
                legal: astral_llm_domain::LegalBlock {
                    disclaimer: "Disclaimer".into(),
                },
                quality: astral_llm_domain::QualityMetadata {
                    used_provider: "fake".into(),
                    used_model: "fake".into(),
                    generation_mode: GenerationMode::SinglePass,
                    prompt_family: "test".into(),
                    prompt_version: "v1".into(),
                    astro_contract_version: request.astro_result.contract_version.clone(),
                    fallback_used: false,
                },
            },
            token_usage: None,
        })
    }
}

#[async_trait]
impl astral_gateway::ports::LlmPort for PanicLlm {
    async fn generate_reading(
        &self,
        _request: &GenerateReadingRequest,
    ) -> Result<GenerateReadingResponse, astral_gateway::error::GatewayError> {
        panic!("inspect path must not call the LLM port")
    }
}

#[async_trait]
impl astral_gateway::ports::LlmPort for CountingLlm {
    async fn generate_reading(
        &self,
        request: &GenerateReadingRequest,
    ) -> Result<GenerateReadingResponse, astral_gateway::error::GatewayError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        FakeLlm.generate_reading(request).await
    }
}

fn request(time: Option<&str>) -> NatalReadingRequestV2 {
    NatalReadingRequestV2 {
        context: astral_contracts::RequestContextCommon {
            request_id: Some("req-1".into()),
            idempotency_key: Some("idem-1".into()),
            target_language_code: "fr".into(),
            audience_level: "general".into(),
        },
        birth: astral_contracts::BirthInputCommon {
            date: "1990-06-15".into(),
            time: time.map(str::to_string),
            timezone: Some("Europe/Paris".into()),
            location: Some(astral_contracts::LocationCommon {
                latitude: 48.8566,
                longitude: 2.3522,
                label: Some("Paris".into()),
            }),
        },
    }
}

#[tokio::test]
async fn natal_gateway_supports_simplified_premium_v2() {
    let use_case = GenerateNatalReadingUseCase::new(Arc::new(FakeCalculator), Arc::new(FakeLlm));
    let response = use_case
        .execute(
            NatalGatewayPolicy {
                variant: NatalVariant::Simplified,
                tier: ProductTier::Premium,
            },
            request(None),
        )
        .await
        .expect("response");

    assert_eq!(response.metadata.product_code, "natal_simplified_premium");
    assert_eq!(response.metadata.variant, "simplified");
    assert_eq!(response.debug.as_ref().and_then(|debug| debug.get("run_id")).and_then(|value| value.as_str()), Some("run-test"));
    assert_eq!(
        response
            .debug
            .as_ref()
            .and_then(|debug| debug.get("llm_request"))
            .and_then(|value| value.get("product_context"))
            .and_then(|value| value.get("interpretation_profile_code"))
            .and_then(|value| value.as_str()),
        Some("natal_simplified")
    );
}

#[tokio::test]
async fn natal_gateway_requires_time_for_full_variant() {
    let use_case = GenerateNatalReadingUseCase::new(Arc::new(FakeCalculator), Arc::new(FakeLlm));
    let err = use_case
        .execute(
            NatalGatewayPolicy {
                variant: NatalVariant::Full,
                tier: ProductTier::Free,
            },
            request(None),
        )
        .await
        .expect_err("must fail");

    assert!(err.to_string().contains("birth.time is required"));
}

#[test]
fn natal_policy_maps_expected_profiles_and_projection_levels() {
    let free = NatalGatewayPolicy {
        variant: NatalVariant::Full,
        tier: ProductTier::Free,
    };
    let premium = NatalGatewayPolicy {
        variant: NatalVariant::Full,
        tier: ProductTier::Premium,
    };

    assert_eq!(free.projection_level(), "compact");
    assert_eq!(premium.projection_level(), "rich");
    assert!(matches!(
        free.default_audience_level(),
        AudienceLevel::Beginner
    ));
    assert_eq!(premium.interpretation_profile_code(), "natal_premium");
}

#[tokio::test]
async fn natal_gateway_respects_explicit_audience_level() {
    let use_case = GenerateNatalReadingUseCase::new(Arc::new(FakeCalculator), Arc::new(FakeLlm));
    let mut req = request(None);
    req.context.audience_level = "expert".into();

    let response = use_case
        .execute(
            NatalGatewayPolicy {
                variant: NatalVariant::Simplified,
                tier: ProductTier::Free,
            },
            req,
        )
        .await
        .expect("response");

    match response.reading {
        GenerateReadingResponse::Success { reading, .. } => {
            assert_eq!(reading.language, "fr");
        }
        other => panic!("unexpected reading response: {other:?}"),
    }
}

#[tokio::test]
async fn natal_gateway_inspect_builds_llm_request_without_calling_llm() {
    let use_case = GenerateNatalReadingUseCase::new(Arc::new(FakeCalculator), Arc::new(PanicLlm));
    let response = use_case
        .inspect(
            NatalGatewayPolicy {
                variant: NatalVariant::Full,
                tier: ProductTier::Basic,
            },
            request(Some("14:30:00")),
        )
        .await
        .expect("inspection response");

    assert_eq!(response.metadata.product_code, "natal_full_basic");
    assert_eq!(
        response
            .llm_request
            .get("product_context")
            .and_then(|value| value.get("interpretation_profile_code"))
            .and_then(|value| value.as_str()),
        Some("natal_basic")
    );
}

#[tokio::test]
async fn natal_gateway_execute_calls_llm_for_standard_flow() {
    let calls = Arc::new(AtomicUsize::new(0));
    let use_case = GenerateNatalReadingUseCase::new(
        Arc::new(FakeCalculator),
        Arc::new(CountingLlm {
            calls: calls.clone(),
        }),
    );

    let response = use_case
        .execute(
            NatalGatewayPolicy {
                variant: NatalVariant::Simplified,
                tier: ProductTier::Free,
            },
            request(None),
        )
        .await
        .expect("reading response");

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(matches!(response.reading, GenerateReadingResponse::Success { .. }));
}
