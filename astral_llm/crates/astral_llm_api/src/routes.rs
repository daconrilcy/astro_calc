use astral_llm_application::{daily_writer_response, period_writer_response_with_quality_loop};
use astral_llm_domain::GenerateReadingRequest;
use astral_llm_domain::GenerationRunContractVersions;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use uuid::Uuid;

use crate::api_contracts::{
    contracts_index, load_published_schema, openapi_bytes, readiness_details, service_not_ready,
};
use crate::api_error::{error_response, from_generation_error};
use crate::integration_routes::{get_job_status, get_service_contract, list_services, submit_job};
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_live))
        .route("/health/live", get(health_live))
        .route("/health/ready", get(health_ready))
        .route("/v1/contracts", get(list_contracts))
        .route("/openapi.yaml", get(openapi_spec))
        .route("/v1/readings/validate", post(validate_reading))
        .route(
            "/v1/internal/readings/render",
            post(render_reading_internal),
        )
        .route(
            "/v1/internal/horoscope/daily/render",
            post(render_horoscope_daily_internal),
        )
        .route(
            "/v1/internal/horoscope/period/render",
            post(render_horoscope_period_internal),
        )
        .route("/v1/runs/{run_id}", get(get_run_audit))
        .route("/v1/providers", get(list_providers))
        .route("/v1/schemas/{schema_version}", get(get_schema))
        .route("/v1/services", get(list_services))
        .route(
            "/v1/services/{service_code}/contract",
            get(get_service_contract),
        )
        .route("/v1/jobs", post(submit_job))
        .route("/v1/jobs/{run_id}", get(get_job_status))
        .with_state(state)
}

async fn health_live() -> impl IntoResponse {
    Json(json!({ "status": "ok", "service": "astral_llm_api" }))
}

async fn health_ready(State(state): State<AppState>) -> Response {
    let pool = state.persistence.as_ref().map(|p| p.pool());
    let (ready, details) =
        readiness_details(&state.config, pool, state.interpretation_profile_count).await;

    if ready {
        Json(json!({
            "status": "ready",
            "service": "astral_llm_api"
        }))
        .into_response()
    } else {
        let (status, body) = service_not_ready("LLM gateway is not ready.", details);
        (status, body).into_response()
    }
}

async fn list_contracts() -> impl IntoResponse {
    Json(contracts_index())
}

async fn openapi_spec() -> Response {
    match openapi_bytes() {
        Ok(bytes) => (
            StatusCode::OK,
            [("content-type", "application/yaml")],
            bytes,
        )
            .into_response(),
        Err(message) => error_response(StatusCode::NOT_FOUND, "INTERNAL_ERROR", message, None),
    }
}

async fn render_horoscope_daily_internal(
    State(state): State<AppState>,
    Json(request): Json<serde_json::Value>,
) -> Response {
    match daily_writer_response(&state.use_case, &request, None).await {
        Ok(response) => Json(response).into_response(),
        Err(err) => from_generation_error(err),
    }
}

async fn render_reading_internal(
    State(state): State<AppState>,
    Json(request): Json<GenerateReadingRequest>,
) -> Response {
    Json(state.use_case.execute(request).await).into_response()
}

async fn render_horoscope_period_internal(
    State(state): State<AppState>,
    Json(request): Json<serde_json::Value>,
) -> Response {
    match period_writer_response_with_quality_loop(&state.use_case, &request, None).await {
        Ok(response) => Json(response).into_response(),
        Err(err) => from_generation_error(err),
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
    let models = state
        .use_case
        .router
        .list_model_capabilities()
        .into_iter()
        .map(|cap| {
            let provider_caps = cap.to_provider_capabilities();
            json!({
                "provider": cap.provider.as_str(),
                "model": cap.model,
                "capabilities": cap,
                "provider_capabilities": provider_caps,
            })
        })
        .collect::<Vec<_>>();

    let circuits: Vec<_> = state
        .use_case
        .router
        .circuit_states()
        .into_iter()
        .map(|(provider, state)| {
            json!({
                "provider": provider,
                "circuit": match state {
                    astral_llm_application::CircuitBreakerState::Closed => "closed",
                    astral_llm_application::CircuitBreakerState::Open => "open",
                    astral_llm_application::CircuitBreakerState::HalfOpen => "half_open",
                }
            })
        })
        .collect();

    Json(json!({
        "provider_capability_version": GenerationRunContractVersions::PROVIDER_CAPABILITY_VERSION,
        "default_provider": state.config.default_provider.as_str(),
        "default_model": state.config.default_model,
        "fake_enabled": state.config.enable_fake_provider,
        "models": models,
        "circuit_breakers": circuits
    }))
}

async fn get_schema(
    State(state): State<AppState>,
    axum::extract::Path(schema_version): axum::extract::Path<String>,
) -> Response {
    if let Some(schema) = state.schema_registry.get(&schema_version) {
        return (StatusCode::OK, Json(schema.clone())).into_response();
    }

    if let Some(schema) = load_published_schema(&schema_version) {
        return (StatusCode::OK, Json(schema)).into_response();
    }

    error_response(
        StatusCode::NOT_FOUND,
        "UNSUPPORTED_CONTRACT_VERSION",
        format!("Unknown schema version: {schema_version}"),
        None,
    )
}

async fn get_run_audit(
    State(state): State<AppState>,
    axum::extract::Path(run_id): axum::extract::Path<String>,
) -> Response {
    let Some(persistence) = state.persistence.as_ref() else {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "PERSISTENCE_DISABLED",
            "run audit requires ASTRAL_LLM_ENABLE_PERSISTENCE=true",
            None,
        );
    };

    let run_uuid = match Uuid::parse_str(&run_id) {
        Ok(id) => id,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_RUN_ID",
                "run_id must be a UUID",
                None,
            );
        }
    };

    match persistence.get_run_audit(run_uuid).await {
        Ok(Some(audit)) => (StatusCode::OK, Json(audit)).into_response(),
        Ok(None) => error_response(
            StatusCode::NOT_FOUND,
            "RUN_NOT_FOUND",
            "no generation run for this run_id",
            Some(json!({ "run_id": run_id })),
        ),
        Err(err) => {
            tracing::error!(error = %err, run_id = %run_id, "failed to load run audit");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "AUDIT_LOOKUP_FAILED",
                "failed to load run audit",
                None,
            )
        }
    }
}
