use std::time::Instant;

use astral_llm_application::{resolve_engine_params, GenerationTraceContext};
use astral_llm_domain::{
    GenerateReadingRequest, GenerateReadingResponse, GenerationErrorCode,
    GenerationRunContractVersions, GenerationMode,
};
use astral_llm_infra::{
    error_code, hash_json, redact_request_for_storage, GenerationRunRecord,     IdempotencyClaim, RunStatus, SafetyStatus,
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

use crate::rate_limit::{rate_limit_key_id_from_headers, try_acquire_premium_addon, RateLimitReason};
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/readings/generate", post(generate_reading))
        .route("/v1/readings/validate", post(validate_reading))
        .route("/v1/runs/{run_id}", get(get_run_audit))
        .route("/v1/providers", get(list_providers))
        .route("/v1/schemas/{schema_version}", get(get_schema))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok", "service": "astral_llm_api" }))
}

async fn generate_reading(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut request): Json<GenerateReadingRequest>,
) -> Response {
    if request.idempotency_key.is_none() {
        request.idempotency_key = header_idempotency_key(&headers);
    }

    let idempotency_key = request.idempotency_key.clone();
    let product_code = request.product_context.product_code.clone();

    if state.config.requires_strict_persistence() && idempotency_key.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "IDEMPOTENCY_KEY_REQUIRED",
                "message": "Idempotency-Key is required for public production exposure"
            })),
        )
            .into_response();
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
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": "IDEMPOTENCY_PAYLOAD_MISMATCH",
                        "message": "idempotency key reused with a different request payload"
                    })),
                )
                    .into_response();
            }
            Ok(IdempotencyClaim::Acquired { .. }) => {}
            Err(err) => {
                tracing::error!(error = %err, "idempotency claim failed");
                if state.config.requires_strict_persistence() {
                    return (
                        StatusCode::SERVICE_UNAVAILABLE,
                        Json(json!({
                            "error": "IDEMPOTENCY_UNAVAILABLE",
                            "message": "idempotency store unavailable; refusing duplicate-risk request"
                        })),
                    )
                        .into_response();
                }
            }
        }
    }

    let is_premium =
        matches!(request.response_contract.generation_mode, GenerationMode::ChapterOrchestrated);
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
            let status = map_error_status(&failed.error.code);
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
    (
        StatusCode::TOO_MANY_REQUESTS,
        Json(json!({ "error": "too_many_requests", "message": message })),
    )
        .into_response()
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
        "models": models,
        "circuit_breakers": circuits
    }))
}

async fn get_schema(
    State(state): State<AppState>,
    axum::extract::Path(schema_version): axum::extract::Path<String>,
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

async fn get_run_audit(
    State(state): State<AppState>,
    axum::extract::Path(run_id): axum::extract::Path<String>,
) -> Response {
    let Some(persistence) = state.persistence.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": "PERSISTENCE_DISABLED",
                "message": "run audit requires ASTRAL_LLM_ENABLE_PERSISTENCE=true"
            })),
        )
            .into_response();
    };

    let run_uuid = match Uuid::parse_str(&run_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "INVALID_RUN_ID",
                    "message": "run_id must be a UUID"
                })),
            )
                .into_response();
        }
    };

    match persistence.get_run_audit(run_uuid).await {
        Ok(Some(audit)) => (StatusCode::OK, Json(audit)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "RUN_NOT_FOUND",
                "message": "no generation run for this run_id",
                "run_id": run_id
            })),
        )
            .into_response(),
        Err(err) => {
            tracing::error!(error = %err, run_id = %run_id, "failed to load run audit");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "AUDIT_LOOKUP_FAILED",
                    "message": "failed to load run audit"
                })),
            )
                .into_response()
        }
    }
}

fn map_error_status(code: &GenerationErrorCode) -> StatusCode {
    match code {
        GenerationErrorCode::InvalidInput => StatusCode::BAD_REQUEST,
        GenerationErrorCode::UnsupportedProvider
        | GenerationErrorCode::UnsupportedCapability
        | GenerationErrorCode::ProductPolicyViolation
        | GenerationErrorCode::PolicyViolation => StatusCode::BAD_REQUEST,
        GenerationErrorCode::ProviderTimeout => StatusCode::GATEWAY_TIMEOUT,
        GenerationErrorCode::ProviderRateLimited => StatusCode::TOO_MANY_REQUESTS,
        GenerationErrorCode::ProviderUnavailable | GenerationErrorCode::FallbackFailed => {
            StatusCode::BAD_GATEWAY
        }
        GenerationErrorCode::ReadingQualityFailed => StatusCode::UNPROCESSABLE_ENTITY,
        _ => StatusCode::UNPROCESSABLE_ENTITY,
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
