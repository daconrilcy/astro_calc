use std::{fs, path::PathBuf, sync::Arc};

use astral_gateway::{
    ports::{CalculatorPort, LlmPort},
    router,
    routes::request_timeout_with_margin,
    state::AppState,
    NatalReadingRequestV2,
};
use async_trait::async_trait;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;

struct FakeCalculator {
    daily: Value,
    period: Value,
}

#[async_trait]
impl CalculatorPort for FakeCalculator {
    async fn calculate_simplified_natal(
        &self,
        _request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        Ok(json!({
            "request_id": "req-1",
            "reading_hint": { "reading_completeness": "compact" },
            "simplified_payload": { "payload": { "sun_sign": "gemini" } },
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
                "contract_version": "natal_structured_v14",
                "payload": { "positions": [], "signals": [] }
            }
        }))
    }

    async fn calculate_horoscope_daily_natal(
        &self,
        _request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        Ok(self.daily.clone())
    }

    async fn calculate_horoscope_period_natal(
        &self,
        _request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        Ok(self.period.clone())
    }
}

struct FakeLlm {
    daily: Value,
    period: Value,
}

#[async_trait]
impl LlmPort for FakeLlm {
    async fn generate_reading(
        &self,
        request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        Ok(json!({
            "status": "success",
            "run_id": "run-test",
            "reading": {
                "schema_version": "natal_reading_v1",
                "language": request.pointer("/product_context/user_language").and_then(Value::as_str).unwrap_or("fr"),
                "reading_type": request.pointer("/product_context/product_code").and_then(Value::as_str).unwrap_or("natal_prompter"),
                "summary": { "title": "Test", "short_text": "Test" },
                "chapters": [{
                    "code": "identity",
                    "title": "Identity",
                    "body": "Body",
                    "astro_basis": [],
                    "confidence": "medium",
                    "safety_flags": []
                }],
                "legal": { "disclaimer": "Disclaimer" },
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
        }))
    }

    async fn build_horoscope_daily_calculation_request(
        &self,
        request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        Ok(json!({
            "contract_version": "horoscope_calculation_request",
            "service_code": request.get("service_code").cloned().unwrap_or(Value::Null),
            "period": {
                "date": request.pointer("/public_request/date").cloned().unwrap_or(Value::Null),
                "timezone": request.pointer("/public_request/timezone").cloned().unwrap_or(Value::Null)
            },
            "chart_calculation_id": request.pointer("/public_request/chart_calculation_id").cloned().unwrap_or(Value::Null),
            "slots": []
        }))
    }

    async fn build_horoscope_period_calculation_request(
        &self,
        request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        Ok(json!({
            "contract_version": "horoscope_period_calculation_request",
            "service_code": request.get("service_code").cloned().unwrap_or(Value::Null),
            "chart_calculation_id": request.pointer("/public_request/chart_calculation_id").cloned().unwrap_or(Value::Null),
            "period_resolution": {},
            "scan_plan": { "snapshots": [] }
        }))
    }

    async fn render_horoscope_daily_gateway(
        &self,
        request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        Ok(json!({ "llm_request": request, "reading": self.daily }))
    }

    async fn render_horoscope_period_gateway(
        &self,
        request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        Ok(json!({
            "llm_request": request,
            "reading": self.period,
            "period_editorial_audit": { "warnings": [] }
        }))
    }
}

fn app() -> axum::Router {
    router(AppState {
        calculator: Arc::new(FakeCalculator {
            daily: read_golden("horoscope_calculation_response_basic_daily_paris_1990.json"),
            period: read_golden(
                "horoscope_period_calculation_response_free_next_7_days_paris_1990.json",
            ),
        }),
        llm: Arc::new(FakeLlm {
            daily: read_golden("horoscope_response_basic_daily_fake.json"),
            period: read_golden("horoscope_period_response_free_next_7_days_fake.json"),
        }),
    })
}

fn natal_request(time: Option<&str>) -> NatalReadingRequestV2 {
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

fn read_golden(name: &str) -> Value {
    let path = repo_root().join("tests").join("golden").join(name);
    serde_json::from_str(&fs::read_to_string(path).expect("read golden")).expect("json")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root")
}

#[tokio::test]
async fn v2_natal_route_maps_to_expected_product_code() {
    let response = app()
        .oneshot(
            Request::post("/v2/natal/simplified/free")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&natal_request(None)).expect("request json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body"),
    )
    .expect("json");
    assert_eq!(body["metadata"]["product_code"], "natal_simplified_free");
}

#[tokio::test]
async fn v2_natal_inspect_route_returns_pre_llm_payload() {
    let response = app()
        .oneshot(
            Request::post("/v2/natal/full/basic/inspect")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&natal_request(Some("14:30:00"))).expect("request json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body"),
    )
    .expect("json");
    assert_eq!(
        body["metadata"]["contract_version"],
        "natal_inspection_response_v2"
    );
    assert!(body.get("llm_request").is_some());
    assert!(body.get("reading").is_none());
}

#[tokio::test]
async fn v2_horoscope_route_is_available() {
    let response = app()
        .oneshot(
            Request::post("/v2/horoscope/daily/free")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "date": "2026-06-14",
                        "timezone": "Europe/Paris",
                        "target_language": "fr",
                        "chart_calculation_id": "chart-1",
                        "audience_level": "general"
                    }))
                    .expect("request json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn legacy_sync_routes_are_not_exposed_anymore() {
    for path in ["/v1/readings/generate", "/v1/readings/natal/simplified"] {
        let response = app()
            .oneshot(
                Request::post(path)
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
    }
}

#[tokio::test]
async fn full_natal_route_rejects_missing_birth_time() {
    let response = app()
        .oneshot(
            Request::post("/v2/natal/full/basic")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&natal_request(None)).expect("request json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn period_horoscope_route_is_available() {
    let response = app()
        .oneshot(
            Request::post("/v2/horoscope/period/free")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "anchor_date": "2026-06-14",
                        "timezone": "Europe/Paris",
                        "target_language": "fr",
                        "chart_calculation_id": "chart-1",
                        "audience_level": "general"
                    }))
                    .expect("request json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn gateway_request_timeout_margin_adds_five_seconds() {
    assert_eq!(request_timeout_with_margin(900_000).as_millis(), 905_000);
    assert_eq!(request_timeout_with_margin(0).as_millis(), 6_000);
}
