use astral_calculator::config::{ephemeris_path_from_env, runtime_options_from_env};
use astral_calculator::db::connect_from_env;
use astral_calculator::engine::AstroEngineRequest;
use astral_calculator::ephemeris::SwissEphemerisEngine;
use astral_calculator::runtime::ChartCalculationRuntimeService;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tracing::error;

use crate::app::build_app;
use crate::config::AppConfig;
use crate::error::{self, internal_error, json_rejection, map_runtime_error, service_not_ready, validation_failed};
use crate::reference_status::{
    check_reference_status, database_ready, ensure_ready, is_ready, readiness_report,
    reference_check_details,
};
use crate::schema_registry::{openapi_bytes, SchemaRegistry};
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_live))
        .route("/health/live", get(health_live))
        .route("/health/ready", get(health_ready))
        .route("/v1/contracts", get(list_contracts))
        .route("/v1/schemas/{version}", get(get_schema))
        .route("/v1/reference/status", get(reference_status))
        .route("/v1/calculations/validate", post(validate_calculation))
        .route("/v1/calculations/natal", post(calculate_natal))
        .route("/openapi.yaml", get(openapi_spec))
        .with_state(state)
}

async fn health_live() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "astral_calculator_api"
    }))
}

async fn health_ready(State(state): State<AppState>) -> Response {
    let db_ok = database_ready(&state.pool).await;
    let status = check_reference_status(&state.pool).await;
    if !db_ok || !is_ready(&status) {
        let message = if !db_ok {
            "PostgreSQL is not reachable."
        } else {
            "Calculator is not ready."
        };
        return service_not_ready(message, readiness_report(db_ok, &status));
    }

    Json(json!({
        "status": "ready",
        "service": "astral_calculator_api"
    }))
    .into_response()
}

async fn list_contracts(State(state): State<AppState>) -> impl IntoResponse {
    Json(json!({
        "service": "astral_calculator_api",
        "contracts": state.schema_registry.contract_links(),
        "openapi": "/openapi.yaml"
    }))
}

async fn get_schema(
    State(state): State<AppState>,
    Path(version): Path<String>,
) -> Result<Json<Value>, Response> {
    state
        .schema_registry
        .get(&version)
        .cloned()
        .map(Json)
        .ok_or_else(|| {
            error::error_response(
                StatusCode::NOT_FOUND,
                "UNSUPPORTED_CONTRACT_VERSION",
                format!("Unknown schema version: {version}"),
                None,
            )
        })
}

async fn reference_status(State(state): State<AppState>) -> impl IntoResponse {
    let db_ok = database_ready(&state.pool).await;
    let status = check_reference_status(&state.pool).await;
    Json(json!({
        "status": status.status,
        "database": db_ok,
        "checks": reference_check_details(&status),
    }))
}

#[derive(Debug, Deserialize)]
struct ValidateBody {
    schema_version: String,
    payload: Value,
}

async fn validate_calculation(
    State(state): State<AppState>,
    body: Result<Json<ValidateBody>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let Json(body) = match body {
        Ok(value) => value,
        Err(rejection) => return json_rejection(rejection),
    };

    match state
        .schema_registry
        .validate(&body.schema_version, &body.payload)
    {
        Ok(()) => Json(json!({ "valid": true })).into_response(),
        Err(errors) => validation_failed(
            format!("Payload does not match {}.", body.schema_version),
            Some(json!({ "errors": errors })),
        ),
    }
}

async fn calculate_natal(
    State(state): State<AppState>,
    body: Result<Json<Value>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if let Err(details) = ensure_ready(&state.pool).await {
        return service_not_ready("Calculator is not ready.", details);
    }

    let Json(payload) = match body {
        Ok(value) => value,
        Err(rejection) => return json_rejection(rejection),
    };

    if let Err(errors) = state
        .schema_registry
        .validate("astro_engine_request_v1", &payload)
    {
        return validation_failed(
            "Request does not match astro_engine_request_v1.",
            Some(json!({ "errors": errors })),
        );
    }

    let request: AstroEngineRequest = match serde_json::from_value(payload) {
        Ok(value) => value,
        Err(err) => {
            return validation_failed(
                "Request does not match astro_engine_request_v1.",
                Some(json!({ "errors": [err.to_string()] })),
            )
        }
    };

    match state.service.calculate_natal_engine(request).await {
        Ok(response) => {
            let value = match serde_json::to_value(&response) {
                Ok(v) => v,
                Err(err) => return internal_error(format!("serialization failed: {err}")),
            };
            if let Err(errors) = state
                .schema_registry
                .validate("astro_engine_response_v1", &value)
            {
                error!(?errors, "engine response failed schema validation");
                return internal_error(
                    "Engine produced a response that does not match astro_engine_response_v1.",
                );
            }
            Json(response).into_response()
        }
        Err(err) => map_runtime_error(err),
    }
}

async fn openapi_spec(State(state): State<AppState>) -> Result<Response, Response> {
    let bytes = openapi_bytes(&state.config.openapi_path, &state.config.schemas_dir).map_err(
        |err| {
            error::error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                err,
                None,
            )
        },
    )?;
    Ok((
        StatusCode::OK,
        [("content-type", "application/yaml")],
        bytes,
    )
        .into_response())
}

pub async fn serve(config: AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    config.validate()?;

    let pool = connect_from_env().await?;
    let ephemeris = SwissEphemerisEngine::new(ephemeris_path_from_env());
    let service =
        ChartCalculationRuntimeService::new(pool.clone(), ephemeris, runtime_options_from_env());
    let schema_registry =
        SchemaRegistry::from_dir(&config.schemas_dir).map_err(|err| format!("schema bootstrap: {err}"))?;

    let state = AppState {
        config: config.clone(),
        pool,
        service: std::sync::Arc::new(service),
        schema_registry: std::sync::Arc::new(schema_registry),
    };

    let app = build_app(state);
    let listener = TcpListener::bind(config.bind_addr).await?;
    tracing::info!(addr = %config.bind_addr, "astral_calculator_api listening");
    axum::serve(listener, app).await?;
    Ok(())
}
