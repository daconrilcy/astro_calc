use std::{fs, path::PathBuf, sync::Arc};

use astral_gateway::{
    router,
    state::AppState,
    NatalReadingRequestV2,
    ports::{CalculatorPort, LlmPort},
};
use astral_llm_application::{HoroscopePeriodPublicRequest, HoroscopePublicRequest};
use astral_llm_domain::{
    generation_response::{ConfidenceLevel, GenerateReadingResponse, StructuredReadingResponse},
    output_contract::GenerationMode,
    GenerateReadingRequest, NatalReadingResponse, ReadingChapter, ReadingSummary,
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
                "contract_version": "natal_structured_v13",
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
        request: &GenerateReadingRequest,
    ) -> Result<GenerateReadingResponse, astral_gateway::error::GatewayError> {
        Ok(GenerateReadingResponse::Success(StructuredReadingResponse {
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
        }))
    }

    async fn render_horoscope_daily(
        &self,
        _request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        Ok(self.daily.clone())
    }

    async fn render_horoscope_period(
        &self,
        _request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        Ok(self.period.clone())
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
async fn v2_horoscope_route_is_available() {
    let response = app()
        .oneshot(
            Request::post("/v2/horoscope/daily/free")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&HoroscopePublicRequest {
                        date: "2026-06-14".into(),
                        timezone: "Europe/Paris".into(),
                        target_language: "fr".into(),
                        chart_calculation_id: "chart-1".into(),
                        location: None,
                        audience_level: "general".into(),
                        detail_level: None,
                    })
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
                    serde_json::to_vec(&HoroscopePeriodPublicRequest {
                        anchor_date: "2026-06-14".into(),
                        timezone: "Europe/Paris".into(),
                        target_language: "fr".into(),
                        target_language_code: None,
                        chart_calculation_id: "chart-1".into(),
                        audience_level: "general".into(),
                        astrologer_persona: None,
                        language_compat_warning: None,
                    })
                    .expect("request json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
}
