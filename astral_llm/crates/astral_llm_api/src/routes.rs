use std::time::Instant;

use astral_llm_application::{
    build_reading_request, resolve_engine_params, validate_simplified_calculation_request,
    GenerationTraceContext,
};
use astral_llm_domain::{
    generation_request::AudienceLevel, GenerateReadingRequest, GenerateReadingResponse,
    GenerationRunContractVersions,
};
use astral_llm_infra::{
    error_code, hash_json, redact_request_for_storage, GenerationRunRecord, IdempotencyClaim,
    RunStatus, SafetyStatus,
};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::api_contracts::{
    contracts_index, load_published_schema, openapi_bytes, readiness_details, service_not_ready,
};
use crate::api_error::{error_response, from_generation_error, map_generation_error_status, too_many_requests};
use crate::rate_limit::{rate_limit_key_id_from_headers, try_acquire_premium_addon, RateLimitReason};
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_live))
        .route("/health/live", get(health_live))
        .route("/health/ready", get(health_ready))
        .route("/v1/contracts", get(list_contracts))
        .route("/openapi.yaml", get(openapi_spec))
        .route("/v1/readings/generate", post(generate_reading))
        .route("/v1/readings/natal/simplified", post(generate_simplified_natal_reading))
        .route("/v1/readings/validate", post(validate_reading))
        .route("/v1/runs/{run_id}", get(get_run_audit))
        .route("/v1/providers", get(list_providers))
        .route("/v1/schemas/{schema_version}", get(get_schema))
        .with_state(state)
}

async fn health_live() -> impl IntoResponse {
    Json(json!({ "status": "ok", "service": "astral_llm_api" }))
}

async fn health_ready(State(state): State<AppState>) -> Response {
    let pool = state.persistence.as_ref().map(|p| p.pool());
    let (ready, details) = readiness_details(
        &state.config,
        pool,
        state.interpretation_profile_count,
    )
    .await;

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
        Err(message) => error_response(
            StatusCode::NOT_FOUND,
            "INTERNAL_ERROR",
            message,
            None,
        ),
    }
}

async fn generate_reading(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut request): Json<GenerateReadingRequest>,
) -> Response {
    if request.idempotency_key.is_none() {
        request.idempotency_key = header_idempotency_key(&headers);
    }

    if let Err(err) = state.use_case.prepare_request(&mut request) {
        return from_generation_error(err);
    }

    let idempotency_key = request.idempotency_key.clone();
    let product_code = request.product_context.product_code.clone();

    if state.config.requires_strict_persistence() && idempotency_key.is_none() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "IDEMPOTENCY_KEY_REQUIRED",
            "Idempotency-Key is required for public production exposure",
            None,
        );
    }

    let redacted = redact_request_for_storage(&request);
    let input_hash = hash_json(&redacted);

    let run_uuid = Uuid::new_v4();
    let run_id = run_uuid.to_string();

    if let (Some(key), Some(persistence)) = (&idempotency_key, state.persistence.as_ref()) {
        match persistence
            .claim_idempotency(
                key,
                &product_code,
                run_uuid,
                &input_hash,
                state.config.idempotency_ttl_hours,
            )
            .await
        {
            Ok(IdempotencyClaim::Replay(response)) => {
                return (StatusCode::OK, Json(response)).into_response();
            }
            Ok(IdempotencyClaim::InProgress { run_id: existing }) => {
                return (
                    StatusCode::CONFLICT,
                    Json(json!({
                        "status": "pending",
                        "run_id": existing.to_string(),
                        "message": "generation already in progress for this idempotency key"
                    })),
                )
                    .into_response();
            }
            Ok(IdempotencyClaim::PayloadMismatch) => {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "IDEMPOTENCY_PAYLOAD_MISMATCH",
                    "idempotency key reused with a different request payload",
                    None,
                );
            }
            Ok(IdempotencyClaim::Acquired { .. }) => {}
            Err(err) => {
                tracing::error!(error = %err, "idempotency claim failed");
                if state.config.requires_strict_persistence() {
                    return error_response(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "IDEMPOTENCY_UNAVAILABLE",
                        "idempotency store unavailable; refusing duplicate-risk request",
                        None,
                    );
                }
            }
        }
    }

    let is_premium = state.use_case.requires_premium_rate_limit(&request);
    let _premium_permit = if is_premium {
        let key_id = rate_limit_key_id_from_headers(&headers, &state);
        match try_acquire_premium_addon(&state, &key_id) {
            Ok(permit) => Some(permit),
            Err(reason) => return premium_rate_limit_response(reason).into_response(),
        }
    } else {
        None
    };

    let started = Instant::now();
    let engine = resolve_engine_params(
        &request.engine,
        &state.config.engine_defaults(),
        state.config.limits.default_request_timeout_ms,
    );
    let trace = GenerationTraceContext::from_request(&run_id, &request);
    trace.started(
        &engine,
        request.response_contract.generation_mode.as_str(),
    );

    let output = state
        .use_case
        .execute_with_audit(request.clone(), run_id.clone())
        .await;
    let response = output.response;
    let audit = output.audit;
    trace.finished(&response, started.elapsed().as_millis() as u64, &audit);

    if let (Some(key), Some(persistence)) = (&idempotency_key, state.persistence.as_ref()) {
        let terminal_status = idempotency_terminal_status(&response);
        if let Err(err) = persistence
            .finalize_idempotency(key, &product_code, terminal_status, Some(&response))
            .await
        {
            tracing::error!(error = %err, "failed to finalize idempotency record");
        }
    }

    if let Some(persistence) = state.persistence.as_ref() {
        let record = build_run_record(
            &request,
            &engine,
            &response,
            &input_hash,
            started.elapsed(),
            &audit,
            run_uuid,
        );
        if let Err(err) = persistence.insert_run(&record).await {
            tracing::error!(error = %err, "failed to persist generation run");
        } else {
            if !audit.steps.is_empty() {
                if let Err(err) = persistence.insert_steps(record.id, &audit.steps).await {
                    tracing::error!(error = %err, "failed to persist generation steps");
                }
            }
            if state.config.store_sanitized_payloads {
                let sanitized_response = redact_response_for_storage(&response);
                let prompt_hash = hash_json(&serde_json::json!({
                    "prompt_family": record.prompt_family,
                    "prompt_version": record.prompt_version,
                }));
                let astro_hash = input_hash.clone();
                if let Err(err) = persistence
                    .insert_payloads(
                        record.id,
                        &redacted,
                        &sanitized_response,
                        &prompt_hash,
                        &astro_hash,
                    )
                    .await
                {
                    tracing::error!(error = %err, "failed to persist sanitized payloads");
                }
            }
        }
    }

    map_response(response)
}

#[derive(Debug, serde::Deserialize)]
struct SimplifiedNatalReadingBody {
    #[serde(flatten)]
    calculation: serde_json::Value,
    #[serde(default = "default_user_language")]
    user_language: String,
    #[serde(default = "default_audience_level")]
    audience_level: AudienceLevel,
}

fn default_audience_level() -> AudienceLevel {
    AudienceLevel::Beginner
}

fn default_user_language() -> String {
    "fr".to_string()
}

async fn generate_simplified_natal_reading(
    State(state): State<AppState>,
    Json(body): Json<SimplifiedNatalReadingBody>,
) -> Response {
    let Some(client) = state.calculator_client.as_ref() else {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "CALCULATOR_UNAVAILABLE",
            "Calculator client is not configured (ASTRAL_CALCULATOR_HOST/PORT).",
            None,
        );
    };

    let calculation = match validate_simplified_calculation_request(&body.calculation) {
        Ok(()) => body.calculation,
        Err(err) => return from_generation_error(err),
    };

    let calculation = match client.calculate_simplified_natal(&calculation).await {
        Ok(value) => value,
        Err(err) => return from_generation_error(err),
    };

    let mut reading_request = match build_reading_request(
        &calculation,
        &body.user_language,
        body.audience_level,
    ) {
        Ok(value) => value,
        Err(err) => return from_generation_error(err),
    };

    if let Err(err) = state.use_case.prepare_request(&mut reading_request) {
        return from_generation_error(err);
    }

    let run_id = Uuid::new_v4().to_string();
    let output = state
        .use_case
        .execute_with_audit(reading_request, run_id.clone())
        .await;

    let reading_completeness = calculation
        .pointer("/reading_hint/reading_completeness")
        .and_then(|v| v.as_str())
        .unwrap_or("partial");

    let status = simplified_reading_http_status(&output.response);

    (
        status,
        Json(json!({
            "reading_completeness": reading_completeness,
            "calculation": calculation,
            "reading": output.response,
            "run_id": run_id,
        })),
    )
        .into_response()
}

fn simplified_reading_http_status(response: &GenerateReadingResponse) -> StatusCode {
    match response {
        GenerateReadingResponse::Success(_) => StatusCode::OK,
        GenerateReadingResponse::SafetyRejected(_) => StatusCode::UNPROCESSABLE_ENTITY,
        GenerateReadingResponse::Failed(failed) => {
            let status = map_generation_error_status(&failed.error.code);
            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

fn idempotency_terminal_status(response: &GenerateReadingResponse) -> &'static str {
    match response {
        GenerateReadingResponse::Success(_) => "completed",
        GenerateReadingResponse::SafetyRejected(_) => "safety_rejected",
        GenerateReadingResponse::Failed(_) => "failed",
    }
}

fn map_response(response: GenerateReadingResponse) -> Response {
    match &response {
        GenerateReadingResponse::Success(_) => (StatusCode::OK, Json(response)).into_response(),
        GenerateReadingResponse::SafetyRejected(_) => {
            (StatusCode::UNPROCESSABLE_ENTITY, Json(response)).into_response()
        }
        GenerateReadingResponse::Failed(failed) => {
            let status = map_generation_error_status(&failed.error.code);
            (
                StatusCode::from_u16(status.as_u16())
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(response),
            )
                .into_response()
        }
    }
}

fn premium_rate_limit_response(reason: RateLimitReason) -> Response {
    let message = match reason {
        RateLimitReason::PremiumConcurrent => "API key premium concurrent limit reached",
        _ => "API key rate limit reached",
    };
    too_many_requests(message)
}

fn redact_response_for_storage(response: &GenerateReadingResponse) -> serde_json::Value {
    let value = serde_json::to_value(response).unwrap_or_else(|_| serde_json::json!({}));
    astral_llm_infra::redact_value(&value)
}

fn header_idempotency_key(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
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

fn build_run_record(
    request: &GenerateReadingRequest,
    engine: &astral_llm_application::ResolvedEngineParams,
    response: &GenerateReadingResponse,
    input_hash: &str,
    elapsed: std::time::Duration,
    audit: &astral_llm_application::ExecutionAudit,
    run_uuid: Uuid,
) -> GenerationRunRecord {
    let (token_in, token_out) = audit.aggregate_token_usage();

    let (
        status,
        safety_status,
        output_hash,
        error_code_value,
        provider_used,
        model_used,
        prompt_family,
        prompt_version,
        token_in,
        token_out,
        fallback_used,
    ) = match response {
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
            token_in,
            token_out,
            success.reading.quality.fallback_used,
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
            false,
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
            false,
        ),
    };

    let selected_domains = if audit.selected_domains.is_empty() {
        None
    } else {
        Some(serde_json::json!(audit.selected_domains))
    };

    GenerationRunRecord {
        id: run_uuid,
        request_id: request.request_id.clone(),
        idempotency_key: request.idempotency_key.clone(),
        product_code: request.product_context.product_code.clone(),
        user_language: request.product_context.user_language.clone(),
        astro_contract_version: request.astro_result.contract_version.clone(),
        output_schema_version: request.response_contract.output_schema_version.clone(),
        prompt_family,
        prompt_version,
        safety_policy_version: GenerationRunContractVersions::SAFETY_POLICY_VERSION.to_string(),
        provider_requested: engine.provider.as_str().into(),
        provider_used,
        model_requested: engine.model.clone(),
        model_used,
        generation_mode: request.response_contract.generation_mode.as_str().to_string(),
        fallback_used,
        selected_domains,
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
