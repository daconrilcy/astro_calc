use crate::generate_reading_use_case::GenerateReadingUseCase;
use crate::text_reprocessing_service_adapter::{
    reprocess_horoscope_daily, reprocess_horoscope_period,
};
use astral_llm_domain::{
    model_usage_tier::ModelRouteContext, EngineDefaults, GenerationError, GenerationErrorCode,
    ProviderKind, ReasoningEffort, SafetyMode,
};
use astral_llm_infra::{hash_json, GenerationRunRecord, RunStatus, SafetyStatus};
use astral_llm_providers::{
    GenerationMetadata, PromptMessage, PromptRole, ProviderGenerationRequest,
};
use chrono::{Datelike, NaiveDate};
use chrono_tz::Tz;
use jsonschema::JSONSchema;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use std::time::Duration as StdDuration;
pub(crate) mod daily;
pub(crate) mod errors;
pub(crate) mod orchestrators;
pub(crate) mod period;
pub(crate) mod reference_data;
pub(crate) mod schema;
pub(crate) mod service_codes;
pub(crate) mod text;
pub(crate) mod types;
pub(crate) mod writer_engine;
pub(crate) use daily::*;
pub use daily::{
    aggregate_themes, build_calculation_request, build_calculation_request_for_service,
    build_interpretation_request, daily_response_provider_schema, daily_writer_messages,
    daily_writer_response, score_calculation, validate_public_request, validate_response_evidence,
};
pub(crate) use errors::*;
pub use orchestrators::{
    HoroscopeBasicDailyNatalOrchestrator, HoroscopeDailyNatalOrchestrator,
    HoroscopeFreeDailyOrchestrator, HoroscopePeriodNatalOrchestrator,
    HoroscopePremiumDailyLocalOrchestrator,
};
pub(crate) use period::*;
pub use period::{
    build_period_calculation_request, build_period_calculation_request_for_service,
    build_period_interpretation_request, build_period_writer_request, fake_period_writer_response,
    period_editorial_audit, period_quality_audit, period_response_provider_schema,
    period_style_editor_max_output_tokens, period_writer_max_output_tokens, period_writer_messages,
    period_writer_reasoning_effort, period_writer_response_with_quality_loop,
    postprocess_period_provider_response, repair_period_response_shape,
    reprocess_horoscope_period_payload, validate_period_interpretation_request_schema,
    validate_period_provider_public_payload, validate_period_public_request,
    validate_period_public_word_count, validate_period_response_contract,
    validate_period_response_contract_gates, validate_period_response_evidence,
    validate_period_response_quality_gates, validate_period_response_schema,
    validate_period_writer_request_schema, validate_scan_plan, validate_semantic_brief_is_atomic,
};
pub use reference_data::public_watch_point_for_theme;
pub(crate) use reference_data::*;
pub(crate) use schema::*;
pub use schema::{validate_horoscope_response_schema, validate_interpretation_request_schema};
pub(crate) use service_codes::*;
pub use service_codes::{
    HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE, HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE,
    HOROSCOPE_FREE_DAILY_SERVICE_CODE, HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
    HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
    HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE, HOROSCOPE_SERVICE_CODE,
};
pub(crate) use text::*;
pub(crate) use types::*;
pub use types::{
    AstrologerPersona, HoroscopeLocation, HoroscopePeriodPublicRequest, HoroscopePublicRequest,
    ScoredSignal, SlotInterpretationPlan, SlotProfile, TargetLanguageCode,
};
pub(crate) use writer_engine::*;

pub(crate) async fn persist_horoscope_run_started(
    use_case: &GenerateReadingUseCase,
    run_id: &str,
    service_code: &str,
    output_schema_version: &str,
    prompt_family: &str,
    prompt_version: &str,
    provider_requested: &ProviderKind,
    model_requested: &str,
    request: &Value,
) {
    let Some(persistence) = use_case.persistence() else {
        return;
    };
    let Ok(run_uuid) = uuid::Uuid::parse_str(run_id) else {
        return;
    };
    let record = GenerationRunRecord {
        id: run_uuid,
        request_id: None,
        idempotency_key: None,
        product_code: service_code.to_string(),
        user_language: request
            .pointer("/target_language_code")
            .and_then(Value::as_str)
            .or_else(|| request.pointer("/target_language").and_then(Value::as_str))
            .unwrap_or("fr")
            .to_string(),
        astro_contract_version: request
            .pointer("/contract_version")
            .and_then(Value::as_str)
            .unwrap_or("-")
            .to_string(),
        output_schema_version: output_schema_version.to_string(),
        prompt_family: prompt_family.to_string(),
        prompt_version: prompt_version.to_string(),
        safety_policy_version: "runtime".into(),
        provider_requested: provider_requested.as_str().to_string(),
        provider_used: None,
        model_requested: model_requested.to_string(),
        model_used: None,
        generation_mode: "single_pass".into(),
        fallback_used: false,
        selected_domains: None,
        status: RunStatus::Pending,
        safety_status: SafetyStatus::NotChecked,
        input_hash: hash_json(request),
        output_hash: None,
        token_input: None,
        token_output: None,
        latency_ms: None,
        error_code: None,
        created_at: chrono::Utc::now(),
    };
    if let Err(err) = persistence.upsert_run(&record).await {
        tracing::warn!(run_id, error = %err, "failed to persist pending horoscope run");
    }
}

pub(crate) async fn persist_horoscope_run_finished(
    use_case: &GenerateReadingUseCase,
    run_id: &str,
    service_code: &str,
    output_schema_version: &str,
    prompt_family: &str,
    prompt_version: &str,
    provider_requested: &ProviderKind,
    model_requested: &str,
    request: &Value,
    result: &Result<Value, GenerationError>,
    started_at: std::time::Instant,
) {
    let Some(persistence) = use_case.persistence() else {
        return;
    };
    let Ok(run_uuid) = uuid::Uuid::parse_str(run_id) else {
        return;
    };
    let (status, safety_status, provider_used, model_used, fallback_used, output_hash, error_code) =
        match result {
            Ok(response) => (
                RunStatus::Success,
                SafetyStatus::Passed,
                response
                    .pointer("/quality/provider")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                response
                    .pointer("/quality/model")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                response
                    .pointer("/quality/fallback_used")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                Some(hash_json(response)),
                None,
            ),
            Err(err) => (
                RunStatus::Failed,
                SafetyStatus::NotChecked,
                None,
                None,
                false,
                None,
                Some(err.detail().code.as_str().to_string()),
            ),
        };
    let record = GenerationRunRecord {
        id: run_uuid,
        request_id: None,
        idempotency_key: None,
        product_code: service_code.to_string(),
        user_language: request
            .pointer("/target_language_code")
            .and_then(Value::as_str)
            .or_else(|| request.pointer("/target_language").and_then(Value::as_str))
            .unwrap_or("fr")
            .to_string(),
        astro_contract_version: request
            .pointer("/contract_version")
            .and_then(Value::as_str)
            .unwrap_or("-")
            .to_string(),
        output_schema_version: output_schema_version.to_string(),
        prompt_family: prompt_family.to_string(),
        prompt_version: prompt_version.to_string(),
        safety_policy_version: "runtime".into(),
        provider_requested: provider_requested.as_str().to_string(),
        provider_used,
        model_requested: model_requested.to_string(),
        model_used,
        generation_mode: "single_pass".into(),
        fallback_used,
        selected_domains: None,
        status,
        safety_status,
        input_hash: hash_json(request),
        output_hash,
        token_input: None,
        token_output: None,
        latency_ms: Some(i32::try_from(started_at.elapsed().as_millis()).unwrap_or(i32::MAX)),
        error_code,
        created_at: chrono::Utc::now(),
    };
    if let Err(err) = persistence.upsert_run(&record).await {
        tracing::warn!(run_id, error = %err, "failed to persist final horoscope run");
    }
}
