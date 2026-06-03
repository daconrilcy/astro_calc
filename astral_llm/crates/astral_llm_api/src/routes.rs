use std::time::Instant;

use astral_llm_application::resolve_engine_params;
use astral_llm_domain::{GenerateReadingRequest, GenerateReadingResponse, GenerationErrorCode};
use astral_llm_infra::{
    error_code, hash_json, GenerationRunRecord, RunStatus, SafetyStatus,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/readings/generate", post(generate_reading))
        .route("/v1/readings/validate", post(validate_reading))
        .route("/v1/providers", get(list_providers))
        .route("/v1/schemas/{schema_version}", get(get_schema))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok", "service": "astral_llm_api" }))
}

async fn generate_reading(
    State(state): State<AppState>,
    Json(request): Json<GenerateReadingRequest>,
) -> Response {
    let started = Instant::now();
    let engine = resolve_engine_params(
        &request.engine,
        &state.config.engine_defaults(),
        state.config.limits.default_request_timeout_ms,
    );
    let input_hash = hash_json(
        &serde_json::to_value(&request).unwrap_or(serde_json::json!({})),
    );

    let response = state.use_case.execute(request.clone()).await;

    if let Some(persistence) = state.persistence.as_ref() {
        let record = build_run_record(&request, &engine, &response, &input_hash, started.elapsed());
        if let Err(err) = persistence.insert_run(&record).await {
            tracing::error!(error = %err, "failed to persist generation run");
        }
    }

    match &response {
        GenerateReadingResponse::Success(_) => (StatusCode::OK, Json(response)).into_response(),
        GenerateReadingResponse::SafetyRejected(_) => {
            (StatusCode::UNPROCESSABLE_ENTITY, Json(response)).into_response()
        }
        GenerateReadingResponse::Failed(failed) => {
            let status = map_error_status(&failed.error.code);
            (StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), Json(response)).into_response()
        }
    }
}

async fn validate_reading(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let schema_version = body
        .get("schema_version")
        .or_else(|| body.get("output_schema_version"))
        .and_then(|v| v.as_str())
        .unwrap_or("natal_reading_v1");

    match state.schema_registry.validate(schema_version, &body) {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({ "valid": true, "schema_version": schema_version })),
        )
            .into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "valid": false,
                "schema_version": schema_version,
                "error": err.detail()
            })),
        )
            .into_response(),
    }
}

async fn list_providers(State(state): State<AppState>) -> impl IntoResponse {
    let providers = state
        .use_case
        .router
        .provider_capabilities()
        .into_iter()
        .map(|(kind, caps)| {
            json!({
                "provider": kind.as_str(),
                "capabilities": caps
            })
        })
        .collect::<Vec<_>>();

    Json(json!({ "providers": providers }))
}

async fn get_schema(
    State(state): State<AppState>,
    Path(schema_version): Path<String>,
) -> Response {
    match state.schema_registry.get(&schema_version) {
        Some(schema) => (StatusCode::OK, Json(schema.clone())).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "schema not found", "schema_version": schema_version })),
        )
            .into_response(),
    }
}

fn map_error_status(code: &GenerationErrorCode) -> StatusCode {
    match code {
        GenerationErrorCode::InvalidInput => StatusCode::BAD_REQUEST,
        GenerationErrorCode::UnsupportedProvider | GenerationErrorCode::UnsupportedCapability => {
            StatusCode::BAD_REQUEST
        }
        GenerationErrorCode::ProviderTimeout => StatusCode::GATEWAY_TIMEOUT,
        GenerationErrorCode::ProviderRateLimited => StatusCode::TOO_MANY_REQUESTS,
        GenerationErrorCode::ProviderUnavailable | GenerationErrorCode::FallbackFailed => {
            StatusCode::BAD_GATEWAY
        }
        _ => StatusCode::UNPROCESSABLE_ENTITY,
    }
}

fn build_run_record(
    request: &GenerateReadingRequest,
    engine: &astral_llm_application::ResolvedEngineParams,
    response: &GenerateReadingResponse,
    input_hash: &str,
    elapsed: std::time::Duration,
) -> GenerationRunRecord {
    let (status, safety_status, output_hash, error_code_value, provider_used, model_used, prompt_family, prompt_version, token_in, token_out) =
        match response {
            GenerateReadingResponse::Success(success) => (
                RunStatus::Success,
                SafetyStatus::Passed,
                Some(hash_json(
                    &serde_json::to_value(&success.reading).unwrap_or(serde_json::json!({})),
                )),
                None,
                Some(success.reading.quality.used_provider.clone()),
                Some(success.reading.quality.used_model.clone()),
                success.reading.quality.prompt_family.clone(),
                success.reading.quality.prompt_version.clone(),
                None,
                None,
            ),
            GenerateReadingResponse::SafetyRejected(_) => (
                RunStatus::SafetyRejected,
                SafetyStatus::Rejected,
                None,
                Some("SAFETY_REJECTED".into()),
                None,
                None,
                String::new(),
                String::new(),
                None,
                None,
            ),
            GenerateReadingResponse::Failed(failed) => (
                RunStatus::Failed,
                SafetyStatus::NotChecked,
                None,
                Some(error_code(&failed.error)),
                None,
                None,
                String::new(),
                String::new(),
                None,
                None,
            ),
        };

    GenerationRunRecord {
        id: Uuid::new_v4(),
        request_id: request.request_id.clone(),
        product_code: request.product_context.product_code.clone(),
        astro_contract_version: request.astro_result.contract_version.clone(),
        output_schema_version: request.response_contract.output_schema_version.clone(),
        prompt_family,
        prompt_version,
        provider_requested: engine.provider.as_str().into(),
        provider_used,
        model_requested: engine.model.clone(),
        model_used,
        status,
        safety_status,
        input_hash: input_hash.to_string(),
        output_hash,
        token_input: token_in,
        token_output: token_out,
        latency_ms: Some(elapsed.as_millis() as i32),
        error_code: error_code_value,
        created_at: Utc::now(),
    }
}
