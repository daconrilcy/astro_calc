use astral_calculator::config::{ephemeris_path_from_env, runtime_options_from_env};
use astral_calculator::db::connect_from_env;
use astral_calculator::engine::AstroEngineRequest;
use astral_calculator::ephemeris::SwissEphemerisEngine;
use astral_calculator::horoscope::HoroscopeCalculationRequest;
use astral_calculator::runtime::ChartCalculationRuntimeService;
use astral_calculator::simplified::AstroSimplifiedNatalRequest;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use tokio::net::TcpListener;
use tracing::error;

use crate::app::build_app;
use crate::config::AppConfig;
use crate::error::{
    self, internal_error, json_rejection, map_runtime_error, service_not_ready, validation_failed,
};
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
        .route(
            "/v1/calculations/natal/simplified",
            post(calculate_natal_simplified),
        )
        .route(
            "/v1/calculations/horoscope/daily-natal",
            post(calculate_horoscope_daily_natal),
        )
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

async fn calculate_natal_simplified(
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
        .validate("astro_simplified_natal_request_v1", &payload)
    {
        return validation_failed(
            "Request does not match astro_simplified_natal_request_v1.",
            Some(json!({ "errors": errors })),
        );
    }

    let request: AstroSimplifiedNatalRequest = match serde_json::from_value(payload) {
        Ok(value) => value,
        Err(err) => {
            return validation_failed(
                "Request does not match astro_simplified_natal_request_v1.",
                Some(json!({ "errors": [err.to_string()] })),
            )
        }
    };

    match state
        .service
        .calculate_simplified_natal_engine(request, &ephemeris_path_from_env())
        .await
    {
        Ok(response) => {
            let value = match serde_json::to_value(&response) {
                Ok(v) => v,
                Err(err) => return internal_error(format!("serialization failed: {err}")),
            };
            if let Err(errors) = state
                .schema_registry
                .validate("astro_simplified_natal_response_v1", &value)
            {
                error!(?errors, "simplified response failed schema validation");
                return internal_error(
                    "Engine produced a response that does not match astro_simplified_natal_response_v1.",
                );
            }
            Json(response).into_response()
        }
        Err(err) => map_runtime_error(err),
    }
}

async fn calculate_horoscope_daily_natal(
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
        .validate("horoscope_calculation_request_v1", &payload)
    {
        return validation_failed(
            "Request does not match horoscope_calculation_request_v1.",
            Some(json!({ "errors": errors })),
        );
    }

    let request: HoroscopeCalculationRequest = match serde_json::from_value(payload) {
        Ok(value) => value,
        Err(err) => {
            return validation_failed(
                "Request does not match horoscope_calculation_request_v1.",
                Some(json!({ "errors": [err.to_string()] })),
            )
        }
    };

    if let Err(response) =
        ensure_horoscope_natal_chart_ready(&state.pool, &request.chart_calculation_id).await
    {
        return response;
    }

    match state.service.calculate_horoscope_daily_natal(request).await {
        Ok(response) => {
            let value = match serde_json::to_value(&response) {
                Ok(v) => v,
                Err(err) => return internal_error(format!("serialization failed: {err}")),
            };
            if let Err(errors) = state
                .schema_registry
                .validate("horoscope_calculation_response_v1", &value)
            {
                error!(?errors, "horoscope response failed schema validation");
                return internal_error(
                    "Engine produced a response that does not match horoscope_calculation_response_v1.",
                );
            }
            Json(response).into_response()
        }
        Err(err) => map_runtime_error(err),
    }
}

async fn ensure_horoscope_natal_chart_ready(
    pool: &sqlx::PgPool,
    raw_id: &str,
) -> Result<(), Response> {
    let chart_id: i32 = raw_id.parse().map_err(|_| {
        error::error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "HOROSCOPE_NATAL_CHART_NOT_FOUND",
            "chart_calculation_id must reference an existing natal chart calculation.",
            Some(json!({ "chart_calculation_id": raw_id })),
        )
    })?;

    let row = sqlx::query("SELECT chart_type, status FROM astral_chart_calculations WHERE id = $1")
        .bind(chart_id)
        .fetch_optional(pool)
        .await
        .map_err(|err| {
            error!(error = %err, "horoscope natal chart lookup failed");
            internal_error("Failed to load natal chart calculation.")
        })?;

    let Some(row) = row else {
        return Err(error::error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "HOROSCOPE_NATAL_CHART_NOT_FOUND",
            "Natal chart calculation was not found.",
            Some(json!({ "chart_calculation_id": raw_id })),
        ));
    };
    let chart_type: String = row.get("chart_type");
    let status: String = row.get("status");
    if chart_type != "natal" || status != "completed" {
        return Err(error::error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "HOROSCOPE_NATAL_CHART_OBSOLETE",
            "Natal chart calculation is not completed or is not compatible with horoscope V1.",
            Some(json!({
                "chart_calculation_id": raw_id,
                "chart_type": chart_type,
                "status": status
            })),
        ));
    }
    Ok(())
}

async fn openapi_spec(State(state): State<AppState>) -> Result<Response, Response> {
    let bytes =
        openapi_bytes(&state.config.openapi_path, &state.config.schemas_dir).map_err(|err| {
            error::error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                err,
                None,
            )
        })?;
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
    let schema_registry = SchemaRegistry::from_dir(&config.schemas_dir)
        .map_err(|err| format!("schema bootstrap: {err}"))?;

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
