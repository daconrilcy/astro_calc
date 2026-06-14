use std::sync::Arc;

use astral_contracts::{NatalVariant, ProductTier};
use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use tokio::net::TcpListener;
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};

use crate::{
    clients::HttpLlmClient,
    config::AppConfig,
    contracts::NatalReadingRequestV2,
    horoscope::{GenerateHoroscopeDailyReadingUseCase, GenerateHoroscopePeriodReadingUseCase},
    natal::NatalGatewayPolicy,
    state::AppState,
};
use astral_llm_application::{HoroscopePeriodPublicRequest, HoroscopePublicRequest};

pub fn router(state: AppState) -> Router {
    router_with_timeout(state, std::time::Duration::from_secs(60))
}

pub fn router_with_timeout(state: AppState, request_timeout: std::time::Duration) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/health/live", get(health))
        .route("/health/ready", get(health_ready))
        .route("/v2/natal/simplified/free", post(natal_simplified_free))
        .route("/v2/natal/simplified/basic", post(natal_simplified_basic))
        .route("/v2/natal/simplified/premium", post(natal_simplified_premium))
        .route("/v2/natal/full/free", post(natal_full_free))
        .route("/v2/natal/full/basic", post(natal_full_basic))
        .route("/v2/natal/full/premium", post(natal_full_premium))
        .route("/v2/horoscope/daily/free", post(horoscope_daily_free))
        .route("/v2/horoscope/daily/basic", post(horoscope_daily_basic))
        .route("/v2/horoscope/daily/premium", post(horoscope_daily_premium))
        .route("/v2/horoscope/period/free", post(horoscope_period_free))
        .route("/v2/horoscope/period/basic", post(horoscope_period_basic))
        .route("/v2/horoscope/period/premium", post(horoscope_period_premium))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            request_timeout,
        ))
}

async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok", "service": "astral_gateway" }))
}

async fn health_ready() -> impl IntoResponse {
    Json(json!({ "status": "ready", "service": "astral_gateway" }))
}

async fn natal_simplified_free(
    State(state): State<AppState>,
    Json(request): Json<NatalReadingRequestV2>,
) -> Result<Json<serde_json::Value>, crate::error::GatewayError> {
    natal_handler(state, NatalVariant::Simplified, ProductTier::Free, request).await
}

async fn natal_simplified_basic(
    State(state): State<AppState>,
    Json(request): Json<NatalReadingRequestV2>,
) -> Result<Json<serde_json::Value>, crate::error::GatewayError> {
    natal_handler(state, NatalVariant::Simplified, ProductTier::Basic, request).await
}

async fn natal_simplified_premium(
    State(state): State<AppState>,
    Json(request): Json<NatalReadingRequestV2>,
) -> Result<Json<serde_json::Value>, crate::error::GatewayError> {
    natal_handler(state, NatalVariant::Simplified, ProductTier::Premium, request).await
}

async fn natal_full_free(
    State(state): State<AppState>,
    Json(request): Json<NatalReadingRequestV2>,
) -> Result<Json<serde_json::Value>, crate::error::GatewayError> {
    natal_handler(state, NatalVariant::Full, ProductTier::Free, request).await
}

async fn natal_full_basic(
    State(state): State<AppState>,
    Json(request): Json<NatalReadingRequestV2>,
) -> Result<Json<serde_json::Value>, crate::error::GatewayError> {
    natal_handler(state, NatalVariant::Full, ProductTier::Basic, request).await
}

async fn natal_full_premium(
    State(state): State<AppState>,
    Json(request): Json<NatalReadingRequestV2>,
) -> Result<Json<serde_json::Value>, crate::error::GatewayError> {
    natal_handler(state, NatalVariant::Full, ProductTier::Premium, request).await
}

async fn horoscope_daily_free(
    State(state): State<AppState>,
    Json(request): Json<HoroscopePublicRequest>,
) -> Result<Json<serde_json::Value>, crate::error::GatewayError> {
    horoscope_daily_handler(
        state,
        astral_contracts::HOROSCOPE_FREE_DAILY_SERVICE_CODE,
        request,
    )
    .await
}

async fn horoscope_daily_basic(
    State(state): State<AppState>,
    Json(request): Json<HoroscopePublicRequest>,
) -> Result<Json<serde_json::Value>, crate::error::GatewayError> {
    horoscope_daily_handler(
        state,
        astral_contracts::HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE,
        request,
    )
    .await
}

async fn horoscope_daily_premium(
    State(state): State<AppState>,
    Json(request): Json<HoroscopePublicRequest>,
) -> Result<Json<serde_json::Value>, crate::error::GatewayError> {
    horoscope_daily_handler(
        state,
        astral_contracts::HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
        request,
    )
    .await
}

async fn horoscope_period_free(
    State(state): State<AppState>,
    Json(request): Json<HoroscopePeriodPublicRequest>,
) -> Result<Json<serde_json::Value>, crate::error::GatewayError> {
    horoscope_period_handler(
        state,
        astral_contracts::HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        request,
    )
    .await
}

async fn horoscope_period_basic(
    State(state): State<AppState>,
    Json(request): Json<HoroscopePeriodPublicRequest>,
) -> Result<Json<serde_json::Value>, crate::error::GatewayError> {
    horoscope_period_handler(
        state,
        astral_contracts::HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        request,
    )
    .await
}

async fn horoscope_period_premium(
    State(state): State<AppState>,
    Json(request): Json<HoroscopePeriodPublicRequest>,
) -> Result<Json<serde_json::Value>, crate::error::GatewayError> {
    horoscope_period_handler(
        state,
        astral_contracts::HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        request,
    )
    .await
}

async fn natal_handler(
    state: AppState,
    variant: NatalVariant,
    tier: ProductTier,
    request: NatalReadingRequestV2,
) -> Result<Json<serde_json::Value>, crate::error::GatewayError> {
    let response = state
        .natal_use_case()
        .execute(NatalGatewayPolicy { variant, tier }, request)
        .await?;
    let payload = serde_json::to_value(response)
        .map_err(|err| crate::error::GatewayError::Internal(format!("serialization failed: {err}")))?;
    Ok(Json(payload))
}

async fn horoscope_daily_handler(
    state: AppState,
    service_code: &str,
    request: HoroscopePublicRequest,
) -> Result<Json<serde_json::Value>, crate::error::GatewayError> {
    let response = GenerateHoroscopeDailyReadingUseCase::new(
        state.calculator.clone(),
        state.llm.clone(),
    )
    .execute(service_code, request)
    .await?;
    let payload = serde_json::to_value(response)
        .map_err(|err| crate::error::GatewayError::Internal(format!("serialization failed: {err}")))?;
    Ok(Json(payload))
}

async fn horoscope_period_handler(
    state: AppState,
    service_code: &str,
    request: HoroscopePeriodPublicRequest,
) -> Result<Json<serde_json::Value>, crate::error::GatewayError> {
    let response = GenerateHoroscopePeriodReadingUseCase::new(
        state.calculator.clone(),
        state.llm.clone(),
    )
    .execute(service_code, request)
    .await?;
    let payload = serde_json::to_value(response)
        .map_err(|err| crate::error::GatewayError::Internal(format!("serialization failed: {err}")))?;
    Ok(Json(payload))
}

pub async fn serve(config: AppConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let calculator = astral_llm_infra::CalculatorClient::new(
        config.calculator_base_url,
        config.calculator_api_key,
        config.request_timeout_ms,
    )?;
    let llm = HttpLlmClient::new(
        config.llm_base_url,
        config.llm_api_key,
        config.request_timeout_ms,
    )?;
    let state = AppState {
        calculator: Arc::new(calculator),
        llm: Arc::new(llm),
    };
    let app = router_with_timeout(
        state,
        std::time::Duration::from_millis(config.request_timeout_ms.max(1_000)),
    );
    let listener = TcpListener::bind(&config.bind_addr).await?;
    tracing::info!(addr = %config.bind_addr, "astral_gateway listening");
    axum::serve(listener, app).await?;
    Ok(())
}
