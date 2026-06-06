use astral_llm_application::IntegrationJobValidator;
use astral_llm_domain::{
    integration::{IntegrationService, JobStatus},
    GenerationErrorCode,
};
use astral_llm_infra::{
    canonical_json_hash::{canonical_json_hash, job_logical_payload},
    JobRecord, NewJobRecord,
};
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::api_error::error_response;
use crate::rate_limit::rate_limit_key_id_from_headers;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ServicesQuery {
    #[serde(default)]
    include: Option<String>,
}

pub async fn list_services(
    State(state): State<AppState>,
    Query(query): Query<ServicesQuery>,
) -> impl IntoResponse {
    let include_planned = query.include.as_deref() == Some("planned");
    let services = state
        .use_case
        .catalog()
        .list_integration_services(include_planned)
        .into_iter()
        .map(service_catalog_item)
        .collect::<Vec<_>>();
    Json(json!({ "services": services }))
}

pub async fn get_service_contract(
    State(state): State<AppState>,
    axum::extract::Path(service_code): axum::extract::Path<String>,
) -> Response {
    let Some(service) = state.use_case.catalog().integration_service(&service_code) else {
        return error_response(
            StatusCode::NOT_FOUND,
            "SERVICE_NOT_FOUND",
            format!("unknown service_code: {service_code}"),
            None,
        );
    };

    Json(json!({
        "service_code": service.service_code,
        "profile_code": service.profile_code,
        "contracts": service_contracts(service),
        "schema_links": service_schema_links(service),
        "example_request": service.example_request_json,
        "mapping_notes": service_mapping_notes(service),
        "validation_notes": service_validation_notes(service),
    }))
    .into_response()
}

pub async fn submit_job(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let Some(jobs) = state.job_persistence.as_ref() else {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "SERVICE_UNAVAILABLE",
            "job persistence requires ASTRAL_LLM_ENABLE_PERSISTENCE=true",
            None,
        );
    };
    if let Err(err) = jobs.purge_expired_terminal_jobs().await {
        tracing::warn!(error = %err, "expired integration job purge failed before submit");
    }

    let service_code = body
        .get("service_code")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if service_code.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_INPUT",
            "service_code is required",
            None,
        );
    }

    let Some(service) = state.use_case.catalog().integration_service(&service_code) else {
        return error_response(
            StatusCode::NOT_FOUND,
            "SERVICE_NOT_FOUND",
            format!("unknown service_code: {service_code}"),
            None,
        );
    };

    if !service.availability.is_submittable() {
        return error_response(
            StatusCode::NOT_FOUND,
            "SERVICE_NOT_FOUND",
            format!("service not available for submission: {service_code}"),
            Some(json!({ "availability": service.availability.as_str() })),
        );
    }
    if !service_has_v1_orchestrator(service) {
        return error_response(
            StatusCode::NOT_IMPLEMENTED,
            "SERVICE_NOT_IMPLEMENTED",
            format!("service orchestration is not implemented: {service_code}"),
            Some(json!({ "service_code": service.service_code })),
        );
    }

    let idempotency_key = header_idempotency_key(&headers);
    if state.config.requires_strict_persistence() && idempotency_key.is_none() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "IDEMPOTENCY_KEY_REQUIRED",
            "Idempotency-Key header is required",
            None,
        );
    }
    let idempotency_key = idempotency_key.unwrap_or_else(|| Uuid::new_v4().to_string());
    let logical = job_logical_payload(&body);
    let idempotency_payload_hash = canonical_json_hash(&logical);
    let request_payload_hash = canonical_json_hash(&body);
    let tenant_id = tenant_id_from_headers(&headers);
    let api_key_id = integration_api_key_id(&headers, &state);

    match jobs
        .get_idempotency_identity(&tenant_id, &idempotency_key)
        .await
    {
        Ok(Some(existing)) if existing.api_key_id != api_key_id => {
            return error_response(
                StatusCode::CONFLICT,
                "IDEMPOTENCY_CONFLICT",
                "Idempotency-Key already used by a different API key",
                Some(json!({ "run_id": existing.run_id })),
            );
        }
        Ok(Some(existing)) if existing.service_code != service_code => {
            return error_response(
                StatusCode::CONFLICT,
                "IDEMPOTENCY_CONFLICT",
                "Idempotency-Key already used for a different service or payload",
                Some(json!({ "existing_service_code": existing.service_code })),
            );
        }
        Ok(Some(existing)) if existing.idempotency_payload_hash != idempotency_payload_hash => {
            return error_response(
                StatusCode::CONFLICT,
                "IDEMPOTENCY_CONFLICT",
                "Idempotency-Key already used with a different payload",
                Some(json!({ "run_id": existing.run_id })),
            );
        }
        Ok(_) => {}
        Err(err) => {
            tracing::error!(error = %err, "idempotency preflight failed");
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "SERVICE_UNAVAILABLE",
                "failed to check idempotency key",
                None,
            );
        }
    }

    let fallback_validator = IntegrationJobValidator::new();
    let validator = state
        .integration_job_validator
        .as_ref()
        .map(|v| v.as_ref())
        .unwrap_or(&fallback_validator);

    let validated = match validator.validate_job(&body, service) {
        Ok(v) => v,
        Err(err) => {
            let detail = err.detail();
            let status = if detail.code == GenerationErrorCode::SchemaValidationFailed {
                StatusCode::UNPROCESSABLE_ENTITY
            } else {
                StatusCode::BAD_REQUEST
            };
            return error_response(
                status,
                if status == StatusCode::UNPROCESSABLE_ENTITY {
                    "PAYLOAD_VALIDATION_FAILED"
                } else {
                    detail.code.as_str()
                },
                detail.message.clone(),
                detail.details.clone(),
            );
        }
    };

    let run_id = Uuid::new_v4();
    let expires_at = calculate_job_expires_at(Utc::now(), state.config.idempotency_ttl_hours);
    let new_job = NewJobRecord {
        run_id,
        service_code: validated.service_code.clone(),
        tenant_id: tenant_id.clone(),
        api_key_id: api_key_id.clone(),
        user_id: None,
        idempotency_key: idempotency_key.clone(),
        idempotency_payload_hash,
        request_payload_hash,
        request_json: body,
        expires_at: Some(expires_at),
        max_attempts: 3,
    };

    match jobs.claim_idempotent_insert(&new_job).await {
        Ok(astral_llm_infra::IdempotentJobClaim::Inserted { run_id }) => {
            job_accepted_response(run_id, &validated.service_code, JobStatus::Queued, None)
        }
        Ok(astral_llm_infra::IdempotentJobClaim::Replay(record)) => job_replay_response(&record),
        Ok(astral_llm_infra::IdempotentJobClaim::InProgress { run_id, status }) => {
            job_accepted_response(run_id, &validated.service_code, status, Some(2_000))
        }
        Ok(astral_llm_infra::IdempotentJobClaim::Conflict {
            existing_service_code,
        }) => error_response(
            StatusCode::CONFLICT,
            "IDEMPOTENCY_CONFLICT",
            "Idempotency-Key already used for a different service or payload",
            Some(json!({ "existing_service_code": existing_service_code })),
        ),
        Ok(astral_llm_infra::IdempotentJobClaim::ApiKeyMismatch { .. }) => error_response(
            StatusCode::CONFLICT,
            "IDEMPOTENCY_CONFLICT",
            "Idempotency-Key already used by a different API key",
            None,
        ),
        Ok(astral_llm_infra::IdempotentJobClaim::PayloadMismatch { .. }) => error_response(
            StatusCode::CONFLICT,
            "IDEMPOTENCY_CONFLICT",
            "Idempotency-Key already used with a different payload",
            None,
        ),
        Err(err) => {
            tracing::error!(error = %err, "job insert failed");
            error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "SERVICE_UNAVAILABLE",
                "failed to persist job",
                None,
            )
        }
    }
}

pub async fn get_job_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(run_id): axum::extract::Path<String>,
) -> Response {
    let Some(jobs) = state.job_persistence.as_ref() else {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "SERVICE_UNAVAILABLE",
            "job persistence requires ASTRAL_LLM_ENABLE_PERSISTENCE=true",
            None,
        );
    };
    if let Err(err) = jobs.purge_expired_terminal_jobs().await {
        tracing::warn!(error = %err, "expired integration job purge failed before status lookup");
    }

    let run_uuid = match Uuid::parse_str(&run_id) {
        Ok(id) => id,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_INPUT",
                "run_id must be a UUID",
                None,
            );
        }
    };

    let tenant_id = tenant_id_from_headers(&headers);
    let api_key_id = integration_api_key_id(&headers, &state);

    let record = match jobs.get_job_by_run_id(&tenant_id, run_uuid).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return error_response(
                StatusCode::NOT_FOUND,
                "JOB_NOT_FOUND",
                "no job for this run_id",
                Some(json!({ "run_id": run_id })),
            );
        }
        Err(err) => {
            tracing::error!(error = %err, "job lookup failed");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "JOB_LOOKUP_FAILED",
                "failed to load job",
                None,
            );
        }
    };

    if record.api_key_id != api_key_id {
        return error_response(
            StatusCode::NOT_FOUND,
            "JOB_NOT_FOUND",
            "no job for this run_id",
            Some(json!({ "run_id": run_id })),
        );
    }

    if let Some(expires) = record.expires_at {
        if expires < Utc::now() && record.status.is_terminal() {
            return error_response(
                StatusCode::NOT_FOUND,
                "JOB_NOT_FOUND",
                "job expired and was purged",
                Some(json!({ "run_id": run_id })),
            );
        }
    }

    Json(job_status_body(&record)).into_response()
}

fn job_accepted_response(
    run_id: Uuid,
    service_code: &str,
    status: JobStatus,
    poll_after_ms: Option<u64>,
) -> Response {
    let mut body = json!({
        "run_id": run_id.to_string(),
        "status": status.as_str(),
        "service_code": service_code,
        "poll_url": format!("/v1/jobs/{run_id}"),
    });
    if let Some(ms) = poll_after_ms {
        body["poll_after_ms"] = json!(ms);
    }
    (StatusCode::ACCEPTED, Json(body)).into_response()
}

fn job_replay_response(record: &JobRecord) -> Response {
    let mut body = json!({
        "run_id": record.run_id.to_string(),
        "status": record.status.as_str(),
        "service_code": record.service_code,
        "poll_url": format!("/v1/jobs/{}", record.run_id),
    });
    if record.status == JobStatus::Completed {
        if let Some(result) = &record.result_json {
            body["result"] = result.clone();
        }
        return (StatusCode::OK, Json(body)).into_response();
    }
    (StatusCode::ACCEPTED, Json(body)).into_response()
}

fn job_status_body(record: &JobRecord) -> serde_json::Value {
    let mut body = json!({
        "run_id": record.run_id.to_string(),
        "service_code": record.service_code,
        "status": record.status.as_str(),
        "submitted_at": record.submitted_at.to_rfc3339(),
        "started_at": record.started_at.map(|t| t.to_rfc3339()),
        "completed_at": record.completed_at.map(|t| t.to_rfc3339()),
        "poll_after_ms": poll_after_ms_for_status(record.status),
    });
    if record.status == JobStatus::Completed {
        if let Some(result) = &record.result_json {
            body["result"] = result.clone();
        }
    }
    if let Some(error) = &record.error_json {
        body["error"] = error.clone();
    }
    body
}

fn poll_after_ms_for_status(status: JobStatus) -> u64 {
    match status {
        JobStatus::Queued => 2_000,
        JobStatus::Running => 3_000,
        _ => 0,
    }
}

pub fn calculate_job_expires_at(now: DateTime<Utc>, ttl_hours: i64) -> DateTime<Utc> {
    now + Duration::hours(ttl_hours.max(1))
}

pub fn service_has_v1_orchestrator(service: &IntegrationService) -> bool {
    match service.calculation_mode {
        astral_llm_domain::CalculationMode::SimplifiedNatal => {
            service.service_code == "natal_simplified"
        }
        astral_llm_domain::CalculationMode::FullNatal => service.service_code.starts_with("natal_"),
        astral_llm_domain::CalculationMode::None => {
            service.is_from_payload()
                || service.service_code == "horoscope_basic_daily_natal_3_slots"
        }
    }
}

fn service_catalog_item(service: &IntegrationService) -> serde_json::Value {
    json!({
        "service_code": service.service_code,
        "label_fr": service.label_fr,
        "description_fr": service.description_fr,
        "availability": service.availability.as_str(),
        "interpretation_profile_code": service.profile_code,
        "generation_mode": profile_generation_mode(service),
        "orchestration_mode": service.orchestration_mode,
        "calculation_mode": service.calculation_mode.as_str(),
        "quality_tier": quality_tier(service),
        "async_recommended": service.supports_async,
        "supports_async": service.supports_async,
        "supports_sync_legacy": service.supports_sync_legacy,
        "supports_mercure": service.supports_mercure,
        "contracts": service_contracts(service),
        "endpoints": {
            "submit_async": service.async_endpoint,
            "contract_detail": format!("/v1/services/{}/contract", service.service_code),
            "submit_sync_legacy": service.sync_endpoint,
        }
    })
}

fn service_contracts(service: &IntegrationService) -> serde_json::Value {
    json!({
        "service_request": service.service_request_contract,
        "payload": service.payload_contract,
        "service_response": service.service_response_contract,
        "calculation_output": service.calculation_output_contract,
        "reading_output": service.reading_output_contract,
    })
}

fn service_schema_links(service: &IntegrationService) -> serde_json::Value {
    json!({
        "integration_job_request_v1": format!("/v1/schemas/{}", service.service_request_contract),
        "payload": schema_link_for_contract(&service.payload_contract),
        "integration_job_status_v1": format!("/v1/schemas/{}", service.service_response_contract),
        "reading_output": format!("/v1/schemas/{}", service.reading_output_contract),
    })
}

fn schema_link_for_contract(contract: &str) -> String {
    match contract {
        "astro_simplified_natal_request_v1" | "astro_engine_request_v1" => {
            format!("/v1/schemas/{contract}")
        }
        _ => format!("/v1/schemas/{contract}"),
    }
}

fn service_mapping_notes(service: &IntegrationService) -> Vec<&'static str> {
    match service.calculation_mode {
        astral_llm_domain::CalculationMode::SimplifiedNatal => vec![
            "payload = astro_simplified_natal_request_v1",
            "orchestration: calcul simplifié puis lecture natal_simplified",
        ],
        astral_llm_domain::CalculationMode::FullNatal => vec![
            "payload = astro_engine_request_v1",
            "orchestration: calcul moteur puis mapping engine → generate_reading_request",
        ],
        astral_llm_domain::CalculationMode::None => {
            if service.service_code == "horoscope_basic_daily_natal_3_slots" {
                vec![
                    "payload = horoscope_basic_daily_natal_request_v1",
                    "orchestration: calculator horoscope facts -> deterministic scoring -> fake horoscope response",
                ]
            } else {
                vec![
                    "payload = generate_reading_request_v1",
                    "interpretation_profile_code must match service profile_code",
                ]
            }
        }
    }
}

fn service_validation_notes(service: &IntegrationService) -> Vec<String> {
    let mut notes = vec![
        "Envelope validated against integration_job_request_v1".to_string(),
        format!("Payload validated against {}", service.payload_contract),
    ];
    if service.is_from_payload() {
        notes.push(format!(
            "payload.product_context.interpretation_profile_code must equal '{}'",
            service.profile_code
        ));
    } else if service.service_code == "horoscope_basic_daily_natal_3_slots" {
        notes.push("chart_calculation_id is required; inline birth_data is out of V1 scope".into());
    }
    notes
}

fn profile_generation_mode(service: &IntegrationService) -> Option<&str> {
    match service.profile_code.as_str() {
        "natal_simplified" => Some("single_pass"),
        "natal_light" | "natal_basic" => Some("single_pass"),
        "natal_premium" | "natal_premium_plus" => Some("chapter_orchestrated"),
        _ => None,
    }
}

fn quality_tier(service: &IntegrationService) -> Option<&'static str> {
    if service.service_code.contains("simplified") {
        Some("simplified")
    } else if service.service_code.contains("premium_plus") {
        Some("premium_plus")
    } else if service.service_code.contains("premium") {
        Some("premium")
    } else if service.service_code.contains("basic") {
        Some("basic")
    } else if service.service_code.contains("light") {
        Some("light")
    } else {
        None
    }
}

fn header_idempotency_key(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn tenant_id_from_headers(headers: &HeaderMap) -> String {
    headers
        .get("X-Tenant-Id")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("default")
        .to_string()
}

pub fn integration_api_key_id(headers: &HeaderMap, state: &AppState) -> String {
    let id = rate_limit_key_id_from_headers(headers, state);
    if id == "key:anonymous" && !state.config.requires_auth() {
        "key:dev-local".into()
    } else {
        id
    }
}
