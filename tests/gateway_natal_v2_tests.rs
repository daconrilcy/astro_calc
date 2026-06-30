use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use astral_contracts::{NatalVariant, ProductTier};
use astral_gateway::{
    contracts::NatalReadingRequestV2, natal::NatalGatewayPolicy, GenerateNatalReadingUseCase,
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
            "calculation_result": {
                "status": "completed",
                "ephemeris_version": "Swiss Ephe v2.10",
                "precision": "+ 0°00'01"
            },
            "audit_payload": {
                "contract_version": "natal_structured_v14",
                "payload": {
                    "positions": [],
                    "signals": [],
                    "chart_emphasis": {
                        "dominant_houses": [
                            { "house_number": 2, "theme_code": "resources", "score": 0.8 },
                            { "house_number": 1, "theme_code": "identity", "score": 0.6 }
                        ]
                    }
                }
            },
            "llm_payload": {
                "chart": {
                    "calculation": {
                        "zodiac": "Tropical",
                        "coordinates": "Geocentric",
                        "house_system": "Placidus"
                    }
                }
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
        request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        Ok(fake_reading_response(request))
    }

    async fn prepare_natal_explanations(
        &self,
        _request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        Ok(json!({
            "explanations": {
                "status": "complete",
                "items": [{
                    "fact_id": "placement:sun:taurus:house:10",
                    "kind_code": "placement",
                    "title": "Soleil en Taureau",
                    "explanation": "Soleil en Taureau donne un repere neutre sur l'identite stable et concrete.",
                    "expression_primary": "Maison 10",
                    "source": "cache"
                }],
                "missing_fact_ids": [],
                "errors": []
            },
            "neutral_explanations": {
                "_type": "neutral_natal_explanations",
                "items": [{
                    "fact_id": "placement:sun:taurus:house:10",
                    "title": "Soleil en Taureau",
                    "explanation": "Soleil en Taureau donne un repere neutre sur l'identite stable et concrete."
                }]
            }
        }))
    }
}

#[async_trait]
impl astral_gateway::ports::LlmPort for PanicLlm {
    async fn generate_reading(
        &self,
        _request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        panic!("inspect path must not call the LLM port")
    }
}

#[async_trait]
impl astral_gateway::ports::LlmPort for CountingLlm {
    async fn generate_reading(
        &self,
        request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        FakeLlm.generate_reading(request).await
    }
}

fn fake_reading_response(request: &Value) -> Value {
    json!({
        "status": "success",
        "run_id": "run-test",
        "reading": {
            "schema_version": "natal_reading_v1",
            "language": request.pointer("/product_context/user_language").and_then(Value::as_str).unwrap_or("fr"),
            "reading_type": request.pointer("/product_context/product_code").and_then(Value::as_str).unwrap_or("natal_prompter"),
            "summary": {
                "title": "Test",
                "short_text": "Test"
            },
            "chapters": [{
                "code": "identity",
                "title": "Identity",
                "body": "Body",
                "astro_basis": [],
                "confidence": "medium",
                "safety_flags": []
            }],
            "legal": {
                "disclaimer": "Disclaimer"
            },
            "quality": {
                "used_provider": "fake",
                "used_model": "fake",
                "generation_mode": "single_pass",
                "prompt_family": "test",
                "prompt_version": "v1",
                "astro_contract_version": request.pointer("/astro_result/contract_version").and_then(Value::as_str).unwrap_or("test"),
                "fallback_used": false
            }
        },
        "token_usage": null
    })
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
    assert_eq!(
        response
            .debug
            .as_ref()
            .and_then(|debug| debug.get("run_id"))
            .and_then(|value| value.as_str()),
        Some("run-test")
    );
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
    assert_eq!(free.default_audience_level(), "beginner");
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

    assert_eq!(
        response
            .reading
            .pointer("/reading/language")
            .and_then(Value::as_str),
        Some("fr")
    );
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
    assert_eq!(
        response
            .llm_request
            .pointer("/astro_result/data/calculation_result/ephemeris_version")
            .and_then(Value::as_str),
        Some("Swiss Ephe v2.10")
    );
    assert_eq!(
        response
            .llm_request
            .pointer("/astro_result/data/llm_payload/chart/calculation/zodiac")
            .and_then(Value::as_str),
        Some("Tropical")
    );
    assert_eq!(
        response
            .llm_request
            .pointer("/astro_result/data/chart_emphasis/dominant_houses/0/house_number")
            .and_then(Value::as_i64),
        Some(2)
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
    assert_eq!(
        response.reading.get("status").and_then(Value::as_str),
        Some("success")
    );
}

#[tokio::test]
async fn natal_gateway_exposes_explanations_and_injects_prompt_block() {
    let use_case = GenerateNatalReadingUseCase::new(Arc::new(FakeCalculator), Arc::new(FakeLlm));

    let response = use_case
        .execute(
            NatalGatewayPolicy {
                variant: NatalVariant::Full,
                tier: ProductTier::Basic,
            },
            request(Some("14:30:00")),
        )
        .await
        .expect("reading response");

    assert_eq!(
        response
            .explanations
            .as_ref()
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str),
        Some("complete")
    );
    assert!(response.reading.get("status").is_some());
    assert_eq!(
        response
            .debug
            .as_ref()
            .and_then(
                |debug| debug.pointer("/llm_request/astro_result/data/neutral_explanations/_type")
            )
            .and_then(Value::as_str),
        Some("neutral_natal_explanations")
    );
}
