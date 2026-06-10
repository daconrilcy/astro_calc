use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use crate::french_typography::french_elision_violations;
use crate::text_reprocessing_service_adapter::{
    reprocess_horoscope_daily, reprocess_horoscope_period,
};

use astral_llm_domain::{
    model_usage_tier::ModelRouteContext, EngineDefaults, GenerationError, GenerationErrorCode,
    ProviderKind, SafetyMode,
};
use astral_llm_providers::{
    GenerationMetadata, PromptMessage, PromptRole, ProviderGenerationRequest,
};
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use chrono_tz::Tz;
use jsonschema::JSONSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration as StdDuration;

use crate::generate_reading_use_case::GenerateReadingUseCase;

pub const HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE: &str = "horoscope_basic_daily_natal_3_slots";
pub const HOROSCOPE_FREE_DAILY_SERVICE_CODE: &str = "horoscope_free_daily";
pub const HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE: &str =
    "horoscope_premium_daily_local_2h_slots";
pub const HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE: &str =
    "horoscope_basic_next_7_days_natal";
pub const HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE: &str = "horoscope_free_next_7_days_natal";
pub const HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE: &str =
    "horoscope_premium_next_7_days_natal";
pub const HOROSCOPE_SERVICE_CODE: &str = HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE;
const HOROSCOPE_PRODUCT_CODE: &str = "horoscope";

const FREE_PERIOD_NONE_WATCH_SUMMARY: &str = "Aucun point de vigilance dominant ne ressort cette semaine. Gardez simplement une marge d'observation si un échange ou une décision demande plus de temps que prévu.";

const SERVICES_JSON: &str = include_str!("../../../../../json_db/horoscope_services.json");
const TIME_SLOTS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_time_slot_profiles.json");
const OBJECT_WEIGHTS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_transiting_object_weights.json");
const TARGET_WEIGHTS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_natal_target_weights.json");
const ASPECT_WEIGHTS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_aspect_weights.json");
const ORB_BANDS_JSON: &str = include_str!("../../../../../json_db/horoscope_orb_weight_bands.json");
const TONE_LABELS_JSON: &str = include_str!("../../../../../json_db/horoscope_tone_labels.json");
const DETAIL_PROFILES_JSON: &str =
    include_str!("../../../../../json_db/horoscope_detail_profiles.json");
const NATAL_FOCUS_LABELS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_natal_focus_labels.json");
const PERIOD_STYLE_VARIANTS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_period_style_variants.json");
const THEME_MAPPINGS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_signal_theme_mappings.json");
const THEME_ADVICE_AXES_JSON: &str =
    include_str!("../../../../../json_db/horoscope_theme_advice_axes.json");
const DOMAIN_SCORE_MAPPINGS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_domain_score_mappings.json");
const SHORTLIST_JSON: &str =
    include_str!("../../../../../json_db/horoscope_shortlist_profiles.json");
const INTENSITY_JSON: &str = include_str!("../../../../../json_db/horoscope_intensity_bands.json");
const DURATION_CLASSES_JSON: &str =
    include_str!("../../../../../json_db/horoscope_duration_classes.json");
const SCORING_PARAMETERS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_scoring_parameters.json");
const SUPPORTED_OBJECTS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_supported_objects.json");
const SUPPORTED_ASPECTS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_supported_aspects.json");
const INTERPRETATION_REQUEST_SCHEMA_JSON: &str =
    include_str!("../../../../../contracts/llm/horoscope_interpretation_request_v1.schema.json");
const RESPONSE_SCHEMA_JSON: &str =
    include_str!("../../../../../contracts/llm/horoscope_response_v1.schema.json");
const PERIOD_PROFILES_JSON: &str =
    include_str!("../../../../../json_db/astral_time_period_profiles.json");
const SCAN_PROFILES_JSON: &str =
    include_str!("../../../../../json_db/horoscope_scan_profiles.json");
const PERIOD_INTERPRETATION_REQUEST_SCHEMA_JSON: &str = include_str!(
    "../../../../../contracts/llm/horoscope_period_interpretation_request_v1.schema.json"
);
const PERIOD_RESPONSE_SCHEMA_JSON: &str =
    include_str!("../../../../../contracts/llm/horoscope_period_response_v1.schema.json");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HoroscopePublicRequest {
    pub date: String,
    pub timezone: String,
    pub target_language: String,
    pub chart_calculation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<HoroscopeLocation>,
    #[serde(default = "default_audience")]
    pub audience_level: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail_level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HoroscopeLocation {
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SlotProfile {
    pub service_code: String,
    pub slot_code: String,
    pub start_local_time: String,
    pub end_local_time: String,
    pub reference_local_time: String,
    pub slot_label: Option<String>,
    pub is_public: Option<bool>,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoredSignal {
    pub evidence_key: String,
    pub fact_type: String,
    pub slot_id: String,
    pub source: String,
    pub transiting_object: String,
    pub natal_target: Option<String>,
    pub aspect: Option<String>,
    pub orb_deg: Option<f64>,
    pub natal_house: Option<i64>,
    pub theme_code: String,
    pub priority_score: f64,
    pub intensity: String,
    pub tone: String,
    pub duration_class: String,
    pub confidence_score: f64,
    pub human_label: String,
    pub score_breakdown: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SlotInterpretationPlan {
    pub slot_code: String,
    pub slot_label: String,
    pub specificity: String,
    pub theme_code: Option<String>,
    pub tone: Option<String>,
    pub intensity: Option<String>,
    pub main_signal_keys: Vec<String>,
    pub required_evidence_keys: Vec<String>,
    pub advice_axis: Option<String>,
    pub avoid_axis: Option<String>,
    pub watch_point: Option<String>,
    pub best_for: Vec<String>,
    pub fallback_reason: Option<String>,
}

pub struct HoroscopeBasicDailyNatalOrchestrator;
pub struct HoroscopeFreeDailyOrchestrator;
pub struct HoroscopePremiumDailyLocalOrchestrator;
pub struct HoroscopeDailyNatalOrchestrator;
pub struct HoroscopePeriodNatalOrchestrator;

impl HoroscopeBasicDailyNatalOrchestrator {
    pub async fn execute(
        calculator: &astral_llm_infra::CalculatorClient,
        use_case: &GenerateReadingUseCase,
        payload: &Value,
    ) -> Result<serde_json::Value, GenerationError> {
        HoroscopeDailyNatalOrchestrator::execute(
            HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE,
            calculator,
            use_case,
            payload,
        )
        .await
    }
}

impl HoroscopeFreeDailyOrchestrator {
    pub async fn execute(
        calculator: &astral_llm_infra::CalculatorClient,
        use_case: &GenerateReadingUseCase,
        payload: &Value,
    ) -> Result<serde_json::Value, GenerationError> {
        HoroscopeDailyNatalOrchestrator::execute(
            HOROSCOPE_FREE_DAILY_SERVICE_CODE,
            calculator,
            use_case,
            payload,
        )
        .await
    }
}

impl HoroscopePremiumDailyLocalOrchestrator {
    pub async fn execute(
        calculator: &astral_llm_infra::CalculatorClient,
        use_case: &GenerateReadingUseCase,
        payload: &Value,
    ) -> Result<serde_json::Value, GenerationError> {
        HoroscopeDailyNatalOrchestrator::execute(
            HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
            calculator,
            use_case,
            payload,
        )
        .await
    }
}

impl HoroscopePeriodNatalOrchestrator {
    pub async fn execute(
        service_code: &str,
        calculator: &astral_llm_infra::CalculatorClient,
        use_case: &GenerateReadingUseCase,
        payload: &Value,
    ) -> Result<serde_json::Value, GenerationError> {
        validate_period_service_code(service_code)?;
        let public = validate_period_public_request(payload)?;
        let calculation_request =
            build_period_calculation_request_for_service(service_code, &public)?;
        let calculation = calculator
            .calculate_horoscope_period_natal(&calculation_request)
            .await
            .map_err(|err| {
                GenerationError::with_details(
                    GenerationErrorCode::ProviderUnavailable,
                    format!(
                        "HOROSCOPE_PERIOD_CALCULATION_FAILED: {}",
                        err.detail().message
                    ),
                    Value::Null,
                )
            })?;
        let interpretation = build_period_interpretation_request(&public, &calculation)?;
        let mut response = period_writer_response(use_case, &interpretation).await?;
        enforce_period_overview_personalization(&mut response);
        enforce_period_domain_personalization(&interpretation, &mut response);
        enforce_premium_period_advice_synthesis(&interpretation, &mut response);
        validate_period_response_schema(&response)?;
        validate_period_response_evidence(&interpretation, &response)?;

        Ok(json!({
            "calculation": calculation,
            "interpretation_request": interpretation,
            "reading": response
        }))
    }
}

impl HoroscopeDailyNatalOrchestrator {
    pub async fn execute(
        service_code: &str,
        calculator: &astral_llm_infra::CalculatorClient,
        use_case: &GenerateReadingUseCase,
        payload: &Value,
    ) -> Result<serde_json::Value, GenerationError> {
        let public = validate_public_request(payload)?;
        let calculation_request = build_calculation_request_for_service(service_code, &public)?;
        let calculation = calculator
            .calculate_horoscope_daily_natal(&calculation_request)
            .await
            .map_err(|err| {
                GenerationError::with_details(
                    GenerationErrorCode::ProviderUnavailable,
                    format!("HOROSCOPE_CALCULATOR_UNAVAILABLE: {}", err.detail().message),
                    Value::Null,
                )
            })?;

        let signals = score_calculation(&calculation)?;
        let interpretation = build_interpretation_request(&public, &calculation, &signals)?;
        let response = daily_writer_response(use_case, &interpretation).await?;
        validate_horoscope_response_schema(&response)?;
        validate_response_evidence(&interpretation, &response)?;

        Ok(json!({
            "calculation": calculation,
            "interpretation_request": interpretation,
            "reading": response
        }))
    }
}

pub fn validate_public_request(payload: &Value) -> Result<HoroscopePublicRequest, GenerationError> {
    let request: HoroscopePublicRequest =
        serde_json::from_value(payload.clone()).map_err(|err| {
            GenerationError::with_details(
                GenerationErrorCode::InvalidInput,
                format!("HOROSCOPE_PAYLOAD_INVALID: {err}"),
                Value::Null,
            )
        })?;
    if request.chart_calculation_id.trim().is_empty() {
        return Err(horoscope_error("HOROSCOPE_NATAL_CHART_REQUIRED"));
    }
    NaiveDate::parse_from_str(&request.date, "%Y-%m-%d").map_err(|_| {
        GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            "HOROSCOPE_PAYLOAD_INVALID: date must be YYYY-MM-DD",
            Value::Null,
        )
    })?;
    if request.timezone.parse::<Tz>().is_err() {
        return Err(GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            "HOROSCOPE_PAYLOAD_INVALID: timezone must be an IANA timezone",
            Value::Null,
        ));
    }
    Ok(request)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HoroscopePeriodPublicRequest {
    pub anchor_date: String,
    pub timezone: String,
    pub target_language: String,
    pub chart_calculation_id: String,
    #[serde(default = "default_audience")]
    pub audience_level: String,
}

pub fn validate_period_public_request(
    payload: &Value,
) -> Result<HoroscopePeriodPublicRequest, GenerationError> {
    let request: HoroscopePeriodPublicRequest =
        serde_json::from_value(payload.clone()).map_err(|err| {
            GenerationError::with_details(
                GenerationErrorCode::InvalidInput,
                format!("HOROSCOPE_PERIOD_PAYLOAD_INVALID: {err}"),
                Value::Null,
            )
        })?;
    if request.chart_calculation_id.trim().is_empty() {
        return Err(horoscope_error("HOROSCOPE_PERIOD_NATAL_CHART_REQUIRED"));
    }
    NaiveDate::parse_from_str(&request.anchor_date, "%Y-%m-%d").map_err(|_| {
        GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            "HOROSCOPE_PERIOD_ANCHOR_DATE_REQUIRED",
            Value::Null,
        )
    })?;
    if request.timezone.parse::<Tz>().is_err() {
        return Err(GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            "HOROSCOPE_PERIOD_TIMEZONE_REQUIRED",
            Value::Null,
        ));
    }
    Ok(request)
}

pub fn build_period_calculation_request(
    public: &HoroscopePeriodPublicRequest,
) -> Result<Value, GenerationError> {
    build_period_calculation_request_for_service(
        HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        public,
    )
}

pub fn build_period_calculation_request_for_service(
    service_code: &str,
    public: &HoroscopePeriodPublicRequest,
) -> Result<Value, GenerationError> {
    validate_period_service_code(service_code)?;
    let profile = period_service_profile(service_code)?;
    let period_resolution = resolve_period(public)?;
    let scan_plan = build_scan_plan(
        &period_resolution,
        profile
            .scan_profile_code
            .as_deref()
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"))?,
    )?;
    validate_scan_plan(&period_resolution, &scan_plan)?;
    Ok(json!({
        "contract_version": "horoscope_period_calculation_request_v1",
        "service_code": service_code,
        "chart_calculation_id": public.chart_calculation_id,
        "period_resolution": period_resolution,
        "scan_plan": scan_plan
    }))
}

fn resolve_period(public: &HoroscopePeriodPublicRequest) -> Result<Value, GenerationError> {
    let profiles = rows(PERIOD_PROFILES_JSON)?;
    let profile_defs = serde_json::from_value::<Vec<astral_time_window::PeriodProfileDefinition>>(
        Value::Array(profiles),
    )
    .map_err(|err| {
        GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            format!("HOROSCOPE_PERIOD_PROFILE_UNSUPPORTED: {err}"),
            Value::Null,
        )
    })?;
    let resolver = astral_time_window::PeriodWindowResolver::new(profile_defs);
    let request = astral_time_window::PeriodWindowRequest {
        period_profile_code: "next_7_days".into(),
        anchor_date: public.anchor_date.clone(),
        timezone: public.timezone.clone(),
        custom_start_date: None,
        custom_end_date: None,
    };
    let resolved = resolver.resolve(&request).map_err(|err| {
        GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            format!("HOROSCOPE_PERIOD_PROFILE_UNSUPPORTED: {err}"),
            Value::Null,
        )
    })?;
    let tz = public.timezone.parse::<Tz>().map_err(|_| {
        GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            "HOROSCOPE_PERIOD_TIMEZONE_REQUIRED",
            Value::Null,
        )
    })?;
    let start_utc = local_to_utc(tz, resolved.start_datetime_local)?;
    let end_utc = local_to_utc(tz, resolved.end_datetime_local)?;
    let start_date = resolved.start_datetime_local.date();
    let included_dates = (0..resolved.duration_days)
        .map(|offset| {
            (start_date + Duration::days(offset))
                .format("%Y-%m-%d")
                .to_string()
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "period_profile_code": "next_7_days",
        "anchor_date": public.anchor_date,
        "timezone": public.timezone,
        "start_datetime_local": resolved.start_datetime_local.format("%Y-%m-%dT%H:%M:%S").to_string(),
        "end_datetime_local": resolved.end_datetime_local.format("%Y-%m-%dT%H:%M:%S").to_string(),
        "start_datetime_utc": start_utc,
        "end_datetime_utc": end_utc,
        "end_exclusive": resolved.end_exclusive,
        "duration_days": resolved.duration_days,
        "included_dates": included_dates,
        "included_days": resolved.included_days
    }))
}

fn build_scan_plan(
    period_resolution: &Value,
    scan_profile_code: &str,
) -> Result<Value, GenerationError> {
    let scan_profile = scan_profile(scan_profile_code)?;
    let tz = period_resolution["timezone"]
        .as_str()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"))?
        .parse::<Tz>()
        .map_err(|_| horoscope_error("HOROSCOPE_PERIOD_TIMEZONE_REQUIRED"))?;
    let dates = period_resolution["included_dates"]
        .as_array()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"))?;
    let reference_times = scan_profile.reference_times()?;
    let mut snapshots = Vec::new();
    for value in dates {
        let date = value
            .as_str()
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"))?;
        let parsed = NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|_| horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"))?;
        for time in &reference_times {
            let local = parsed.and_time(*time);
            let utc = local_to_utc(tz, local)?;
            let time_label = time.format("%H:%M").to_string();
            let key_suffix = if scan_profile_code == "daily_noon_7_days" {
                "noon".to_string()
            } else {
                time_label.clone()
            };
            snapshots.push(json!({
                "snapshot_key": format!("{date}:{key_suffix}"),
                "date": date,
                "reference_time_local": time_label,
                "reference_datetime_local": local.format("%Y-%m-%dT%H:%M:%S").to_string(),
                "reference_datetime_utc": utc
            }));
        }
    }
    let duration_days = period_resolution["duration_days"]
        .as_u64()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"))?
        as usize;
    if snapshots.len() != duration_days * scan_profile.expected_snapshots_per_day {
        return Err(horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"));
    }
    Ok(json!({
        "scan_profile_code": scan_profile_code,
        "granularity": scan_profile.granularity,
        "snapshot_count": snapshots.len(),
        "snapshots": snapshots
    }))
}

pub fn validate_scan_plan(
    period_resolution: &Value,
    scan_plan: &Value,
) -> Result<(), GenerationError> {
    let start = period_resolution["start_datetime_utc"]
        .as_str()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"))?;
    let end = period_resolution["end_datetime_utc"]
        .as_str()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"))?;
    let start = chrono::DateTime::parse_from_rfc3339(start)
        .map_err(|_| horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"))?;
    let end = chrono::DateTime::parse_from_rfc3339(end)
        .map_err(|_| horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"))?;
    require_canonical_utc_offset(
        period_resolution["start_datetime_utc"]
            .as_str()
            .unwrap_or(""),
    )?;
    require_canonical_utc_offset(period_resolution["end_datetime_utc"].as_str().unwrap_or(""))?;
    let included = period_resolution["included_dates"]
        .as_array()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"))?;
    let snapshots = scan_plan["snapshots"]
        .as_array()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"))?;
    if scan_plan["snapshot_count"].as_u64() != Some(snapshots.len() as u64) {
        return Err(horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"));
    }
    let scan_profile_code = scan_plan["scan_profile_code"]
        .as_str()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"))?;
    let scan_profile = scan_profile(scan_profile_code)?;
    if snapshots.len() != included.len() * scan_profile.expected_snapshots_per_day {
        return Err(horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"));
    }
    let mut keys = HashSet::new();
    let mut dates = HashSet::new();
    for snapshot in snapshots {
        let key = snapshot["snapshot_key"]
            .as_str()
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"))?;
        if !keys.insert(key.to_string()) {
            return Err(horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"));
        }
        let date = snapshot["date"]
            .as_str()
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"))?;
        dates.insert(date.to_string());
        let utc = snapshot["reference_datetime_utc"]
            .as_str()
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"))?;
        require_canonical_utc_offset(utc)?;
        let utc = chrono::DateTime::parse_from_rfc3339(utc)
            .map_err(|_| horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"))?;
        if utc < start || utc >= end {
            return Err(horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"));
        }
    }
    for date in included.iter().filter_map(|value| value.as_str()) {
        if !dates.contains(date) {
            return Err(horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"));
        }
    }
    Ok(())
}

fn require_canonical_utc_offset(raw: &str) -> Result<(), GenerationError> {
    let parsed = chrono::DateTime::parse_from_rfc3339(raw)
        .map_err(|_| horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"))?;
    if parsed.with_timezone(&chrono::Utc).to_rfc3339() != raw {
        return Err(horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"));
    }
    Ok(())
}

fn local_to_utc(tz: Tz, local: NaiveDateTime) -> Result<String, GenerationError> {
    tz.from_local_datetime(&local)
        .single()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"))
        .map(|value| value.with_timezone(&chrono::Utc).to_rfc3339())
}

pub fn build_calculation_request(
    request: &HoroscopePublicRequest,
) -> Result<serde_json::Value, GenerationError> {
    build_calculation_request_for_service(HOROSCOPE_SERVICE_CODE, request)
}

pub fn build_calculation_request_for_service(
    service_code: &str,
    request: &HoroscopePublicRequest,
) -> Result<serde_json::Value, GenerationError> {
    validate_supported_service_code(service_code)?;
    validate_public_request_for_service(service_code, request)?;
    let refs = ReferenceData::load(service_code)?;
    let slots = slot_profiles(service_code)?;
    let mut out = json!({
        "contract_version": "horoscope_calculation_request_v1",
        "service_code": service_code,
        "chart_calculation_id": request.chart_calculation_id,
        "period": {
            "date": request.date,
            "timezone": request.timezone
        },
        "slots": slots.into_iter().map(|slot| json!({
            "slot_code": slot.slot_code,
            "start_local_time": slot.start_local_time,
            "end_local_time": slot.end_local_time,
            "reference_local_time": slot.reference_local_time
        })).collect::<Vec<_>>()
    });
    if service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
        out["location"] = serde_json::to_value(
            request
                .location
                .as_ref()
                .ok_or_else(|| horoscope_error("HOROSCOPE_LOCATION_REQUIRED"))?,
        )
        .expect("location serializes");
        out["slot_profile_code"] = json!("daily_2h_slots");
        out["house_system_code"] = json!(refs
            .service_profile
            .house_system_code
            .as_deref()
            .ok_or_else(|| horoscope_error("HOROSCOPE_SCORING_FAILED"))?);
        out["calculation_features"] = json!([
            "sky_snapshot",
            "moon_context",
            "natal_transits",
            "natal_house_activations",
            "local_chart",
            "local_angles",
            "local_houses",
            "local_house_placements"
        ]);
    }
    Ok(out)
}

fn validate_public_request_for_service(
    service_code: &str,
    request: &HoroscopePublicRequest,
) -> Result<(), GenerationError> {
    if service_code != HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
        return Ok(());
    }
    let location = request
        .location
        .as_ref()
        .ok_or_else(|| horoscope_error("HOROSCOPE_LOCATION_REQUIRED"))?;
    if !(-90.0..=90.0).contains(&location.latitude)
        || !(-180.0..=180.0).contains(&location.longitude)
    {
        return Err(GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            "HOROSCOPE_PAYLOAD_INVALID: location latitude/longitude out of range",
            Value::Null,
        ));
    }
    Ok(())
}

pub fn score_calculation(calculation: &Value) -> Result<Vec<ScoredSignal>, GenerationError> {
    let service_code = service_code_from_value(calculation)?;
    validate_premium_calculation_local_chart(service_code, calculation)?;
    let refs = ReferenceData::load(service_code)?;
    let mut out = Vec::new();
    let slots = calculation
        .get("slots")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_CALCULATION_FAILED"))?;
    for slot in slots {
        let slot_id = slot
            .get("slot_code")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        for fact in slot
            .get("transits_to_natal")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
        {
            out.push(score_fact(&refs, slot_id, fact)?);
        }
    }
    out.sort_by(|a, b| {
        b.priority_score
            .partial_cmp(&a.priority_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.evidence_key.cmp(&b.evidence_key))
    });
    Ok(out)
}

pub fn aggregate_themes(signals: &[ScoredSignal]) -> Vec<Value> {
    let mut totals: HashMap<String, f64> = HashMap::new();
    for signal in signals {
        *totals.entry(signal.theme_code.clone()).or_default() += signal.priority_score;
    }
    let mut themes = totals.into_iter().collect::<Vec<_>>();
    themes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    themes
        .into_iter()
        .map(|(theme_code, score)| json!({ "theme_code": theme_code, "score": round2(score) }))
        .collect()
}

pub fn build_interpretation_request(
    public: &HoroscopePublicRequest,
    calculation: &Value,
    signals: &[ScoredSignal],
) -> Result<Value, GenerationError> {
    let service_code = service_code_from_value(calculation)?;
    let refs = ReferenceData::load(service_code)?;
    let shortlist = refs.shortlist.clone();
    let slot_plans = build_slot_plans(&refs, calculation, signals)?;
    let selected_keys = slot_plans
        .iter()
        .flat_map(|slot| slot.required_evidence_keys.iter())
        .cloned()
        .collect::<HashSet<_>>();
    let mut selected_signals = signals
        .iter()
        .filter(|signal| signal.priority_score >= shortlist.min_priority_score)
        .filter(|signal| selected_keys.contains(&signal.evidence_key))
        .cloned()
        .collect::<Vec<_>>();
    selected_signals.sort_by(|a, b| {
        b.priority_score
            .partial_cmp(&a.priority_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.evidence_key.cmp(&b.evidence_key))
    });
    if selected_signals.is_empty() {
        return Err(horoscope_error("HOROSCOPE_NO_SIGNIFICANT_SIGNAL"));
    }
    let mut main_signals = selected_signals.clone();
    main_signals.truncate(shortlist.max_main_signals);
    let evidence = selected_signals
        .iter()
        .take(shortlist.max_evidence)
        .map(|signal| serde_json::to_value(signal).expect("signal serializes"))
        .collect::<Vec<_>>();
    build_interpretation_request_from_signals(
        public,
        calculation,
        &refs,
        slot_plans,
        main_signals,
        evidence,
    )
}

fn build_interpretation_request_from_signals(
    public: &HoroscopePublicRequest,
    calculation: &Value,
    refs: &ReferenceData,
    slot_plans: Vec<SlotInterpretationPlan>,
    main_signals: Vec<ScoredSignal>,
    evidence: Vec<Value>,
) -> Result<Value, GenerationError> {
    let service_code = service_code_from_value(calculation)?;
    let shortlist = refs.shortlist.clone();
    let dominant_themes = aggregate_themes(&main_signals)
        .into_iter()
        .take(shortlist.max_dominant_themes)
        .collect::<Vec<_>>();
    let overview_evidence = main_signals
        .iter()
        .take(3)
        .map(|signal| signal.evidence_key.clone())
        .collect::<Vec<_>>();
    let top = main_signals.first();
    let request = json!({
        "contract_version": "horoscope_interpretation_request_v1",
        "service_code": service_code,
        "period": premium_period(public, service_code, calculation),
        "target_language": public.target_language,
        "day_overview": {
            "dominant_theme": top.map(|signal| signal.theme_code.as_str()).unwrap_or("daily_focus"),
            "tone": top.map(|signal| signal.tone.as_str()).unwrap_or("mixed"),
            "intensity": top.map(|signal| signal.intensity.as_str()).unwrap_or("medium"),
            "summary_hint": "Introduire la tonalite generale sans recopier ce texte dans chaque slot.",
            "evidence_keys": overview_evidence
        },
        "slots": slot_plans,
        "main_signals": main_signals,
        "dominant_themes": dominant_themes,
        "evidence": evidence
    });
    let request = if service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
        let mut request = request;
        request["best_slots"] = json!(build_best_slots(&request));
        request["watch_slots"] = json!(build_watch_slots(&request));
        request["domain_sections"] = json!(build_domain_sections(&request));
        request
    } else {
        request
    };
    validate_interpretation_request_schema(&request)?;
    Ok(request)
}

pub fn build_period_interpretation_request(
    public: &HoroscopePeriodPublicRequest,
    calculation: &Value,
) -> Result<Value, GenerationError> {
    let service_code = calculation
        .get("service_code")
        .and_then(Value::as_str)
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_CALCULATION_FAILED"))?;
    validate_period_service_code(service_code)?;
    let service_profile = period_service_profile(service_code)?;
    let detail_profile_code = service_profile
        .detail_profile_code
        .as_deref()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_PROFILE_UNSUPPORTED"))?;
    let detail = period_detail_profile(detail_profile_code)?;
    let period_resolution = calculation
        .get("period_resolution")
        .cloned()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_CALCULATION_FAILED"))?;
    let scan_plan = calculation
        .get("scan_plan")
        .cloned()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_CALCULATION_FAILED"))?;
    validate_scan_plan(&period_resolution, &scan_plan)?;

    let snapshots = calculation
        .get("snapshots")
        .and_then(|value| value.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_CALCULATION_FAILED"))?;
    let included_dates = period_resolution
        .get("included_dates")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect::<Vec<_>>();
    if included_dates.len() != 7 {
        return Err(horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"));
    }
    let scan_profile_code = scan_plan["scan_profile_code"]
        .as_str()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"))?;
    let scan_profile = scan_profile(scan_profile_code)?;
    if snapshots.len() != included_dates.len() * scan_profile.expected_snapshots_per_day {
        return Err(horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"));
    }

    let evidence = period_evidence_from_snapshots(snapshots)?
        .into_iter()
        .take(detail.max_evidence)
        .collect::<Vec<_>>();
    if evidence.is_empty() {
        return Err(horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"));
    }
    let events = build_period_events(&evidence, &period_resolution)?;
    let daily_plans = if detail.include_daily_timeline {
        build_daily_plans(&included_dates, &events)?
    } else {
        Vec::new()
    };
    let mut key_days = build_period_key_day_markers(&events, detail.max_key_days);
    if is_free_period_service(service_code) && key_days.is_empty() {
        if let Some(first) = evidence.first() {
            key_days.push(json!({
                "date": first["date"],
                "title": "Jour à retenir",
                "reason": "Un repère utile ressort pour comprendre la tendance sans détailler chaque journée.",
                "evidence_keys": [first["evidence_key"].clone()],
                "fallback_reason": null
            }));
        }
    }
    if is_free_period_service(service_code) {
        for day in &mut key_days {
            day["title"] = json!("Jour à retenir");
        }
    }
    let key_dates = key_days
        .iter()
        .filter_map(|day| day.get("date").and_then(Value::as_str).map(str::to_string))
        .collect::<HashSet<_>>();
    let watch_days = if detail.include_watch_days {
        build_period_watch_day_markers(&events, detail.max_watch_days)
    } else {
        Vec::new()
    };
    let watch_dates = watch_days
        .iter()
        .filter_map(|day| day.get("date").and_then(Value::as_str).map(str::to_string))
        .collect::<HashSet<_>>();
    let best_days = if detail.include_best_days {
        build_period_best_day_markers(&events, &watch_dates, &key_dates, detail.max_best_days)
    } else {
        Vec::new()
    };
    let best_windows = if detail.include_best_windows {
        build_period_best_windows(&events, &scan_plan, detail.max_best_windows)
    } else {
        Vec::new()
    };
    let watch_windows = if detail.include_watch_windows {
        build_period_watch_windows(&events, &scan_plan, &best_windows, detail.max_watch_windows)
    } else {
        Vec::new()
    };
    let watch_summary_plan = build_period_watch_summary_plan(
        &watch_days,
        is_premium_period_service(service_code),
        &watch_windows,
    );
    let strategy = if detail.include_strategy_section {
        json!({
            "title": "Stratégie de semaine",
            "focus": "Lire d'abord le mouvement général, puis le détail de chaque journée, puis utiliser les fenêtres horaires comme repères pratiques sans ajouter de nouvelles dates dans les conseils.",
            "best_use": "Réserver les fenêtres favorables déjà listées aux échanges, décisions et actions concrètes.",
            "recovery": "Après les fenêtres de vigilance déjà listées, revenir au fil général avant de relancer un sujet.",
            "evidence_keys": evidence.iter().take(4).filter_map(|item| item["evidence_key"].as_str()).collect::<Vec<_>>()
        })
    } else {
        Value::Null
    };
    let mut request = json!({
        "contract_version": "horoscope_period_interpretation_request_v1",
        "service_code": service_code,
        "period_resolution": period_resolution,
        "scan_plan": scan_plan,
        "target_language": public.target_language,
        "detail_profile_code": detail_profile_code,
        "week_overview_plan": {
            "dominant_theme": events.first().and_then(|event| event["theme_code"].as_str()).unwrap_or("weekly_focus"),
            "tone": events.first().and_then(|event| event["tone"].as_str()).unwrap_or("constructive"),
            "trajectory_hint": "Construire une lecture coherente sur la periode, pas sept lectures quotidiennes independantes.",
            "evidence_keys": evidence.iter().take(4).filter_map(|item| item["evidence_key"].as_str()).collect::<Vec<_>>()
        },
        "period_events": events.clone(),
        "main_events": events.iter().take(detail.max_main_events).cloned().collect::<Vec<_>>(),
        "key_days": key_days,
        "best_days": best_days,
        "watch_days": watch_days,
        "watch_summary_plan": watch_summary_plan,
        "daily_plans": daily_plans,
        "domain_sections": if detail.include_domain_sections { build_period_domain_sections(&evidence, detail.max_domain_sections) } else { Vec::new() },
        "evidence": evidence
    });
    if detail.include_best_windows
        || detail.include_watch_windows
        || detail.include_strategy_section
    {
        request["best_windows"] = json!(best_windows);
        request["watch_windows"] = json!(watch_windows);
        request["strategy"] = strategy;
        request["premium_scores"] = json!(build_period_premium_scores(&request));
    }
    validate_period_interpretation_request_schema(&request)?;
    Ok(request)
}

fn period_evidence_from_snapshots(snapshots: &[Value]) -> Result<Vec<Value>, GenerationError> {
    let mut out = Vec::new();
    for snapshot in snapshots {
        let date = snapshot["date"]
            .as_str()
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_CALCULATION_FAILED"))?;
        for fact in snapshot
            .get("transits_to_natal")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
        {
            let key = fact["evidence_key"]
                .as_str()
                .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"))?;
            let fact_type = fact["fact_type"].as_str().unwrap_or("transit_active");
            let object = fact["transiting_object"].as_str().unwrap_or("moon");
            let aspect = fact.get("aspect").and_then(|value| value.as_str());
            let orb_deg = fact.get("orb_deg").and_then(Value::as_f64);
            if let Some(aspect_code) = aspect {
                if is_period_major_aspect(aspect_code)
                    && orb_deg.unwrap_or(f64::INFINITY) > period_max_major_aspect_orb_deg()
                {
                    return Err(horoscope_error("HOROSCOPE_PERIOD_CALCULATION_FAILED"));
                }
            }
            let theme = match object {
                "venus" => "relationship",
                "mars" => "energy",
                "mercury" => "communication",
                "jupiter" => "integration",
                "sun" => "clarity",
                _ => "organization",
            };
            let tone = period_internal_tone(theme, fact_type, aspect);
            let public_orb = if aspect.is_some() {
                fact.get("orb_deg").cloned().unwrap_or(Value::Null)
            } else {
                Value::Null
            };
            let natal_focus_code = period_natal_focus_code(fact);
            let natal_focus = period_natal_focus(&natal_focus_code);
            let human_label = format!(
                "{} met en avant le thème {} en touchant {}",
                period_object_public_label(object),
                period_theme_public_label(theme),
                natal_focus.label
            );
            out.push(json!({
                "evidence_key": key,
                "date": date,
                "snapshot_key": snapshot["snapshot_key"].as_str().unwrap_or(""),
                "fact_type": fact_type,
                "source": fact["source"].as_str().unwrap_or("calculator"),
                "transiting_object": object,
                "natal_target": fact.get("natal_target").cloned().unwrap_or(Value::Null),
                "aspect": fact.get("aspect").cloned().unwrap_or(Value::Null),
                "orb_deg": public_orb,
                "natal_house": fact.get("natal_house").cloned().unwrap_or(Value::Null),
                "theme_code": theme,
                "tone": tone,
                "natal_focus_code": natal_focus_code,
                "natal_focus_label": natal_focus.label,
                "natal_focus_hint": natal_focus.hint,
                "personalization_hint": format!(
                    "Personnaliser ce signal par {} plutôt que rester sur un conseil générique.",
                    natal_focus.label
                ),
                "human_label": human_label
            }));
        }
    }
    Ok(out)
}

fn build_period_events(
    evidence: &[Value],
    period_resolution: &Value,
) -> Result<Vec<Value>, GenerationError> {
    let included = period_resolution["included_dates"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str())
        .collect::<HashSet<_>>();
    let theme_counts = evidence
        .iter()
        .filter_map(|item| item.get("theme_code").and_then(Value::as_str))
        .fold(HashMap::<&str, usize>::new(), |mut counts, theme| {
            *counts.entry(theme).or_default() += 1;
            counts
        });
    let mut out = Vec::new();
    for item in evidence {
        let date = item["date"]
            .as_str()
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"))?;
        if !included.contains(date) {
            return Err(horoscope_error("HOROSCOPE_PERIOD_EVENT_OUTSIDE_WINDOW"));
        }
        let evidence_key = item["evidence_key"].as_str().unwrap_or("");
        let event_type = if item["fact_type"].as_str() == Some("moon_house_by_day") {
            "moon_house_by_day"
        } else if item["fact_type"].as_str() == Some("transit_to_natal")
            && item.get("orb_deg").and_then(|v| v.as_f64()).unwrap_or(9.0) <= 1.0
        {
            "transit_exact"
        } else if item["fact_type"].as_str() == Some("transit_context") {
            "transit_context"
        } else {
            "transit_active"
        };
        let theme_code = item["theme_code"].as_str().unwrap_or("organization");
        let score = period_event_score(
            item,
            event_type,
            *theme_counts.get(theme_code).unwrap_or(&1),
        );
        out.push(json!({
            "event_key": format!("event:{evidence_key}"),
            "event_type": event_type,
            "date": date,
            "snapshot_key": item.get("snapshot_key").cloned().unwrap_or(Value::Null),
            "theme_code": item["theme_code"],
            "tone": item["tone"],
            "aspect": item.get("aspect").cloned().unwrap_or(Value::Null),
            "score": score,
            "natal_focus_hint": item.get("natal_focus_hint").cloned().unwrap_or(Value::Null),
            "personalization_hint": item.get("personalization_hint").cloned().unwrap_or(Value::Null),
            "evidence_keys": [evidence_key]
        }));
    }
    if out.is_empty() {
        return Err(horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"));
    }
    out.sort_by(period_event_sort);
    Ok(out)
}

fn build_daily_plans(
    included_dates: &[String],
    events: &[Value],
) -> Result<Vec<Value>, GenerationError> {
    let mut out = Vec::new();
    let mut theme_counts = HashMap::<String, usize>::new();
    for date in included_dates {
        let event = select_daily_plan_event(date, events, &theme_counts)?;
        let theme = event["theme_code"].as_str().unwrap_or("organization");
        *theme_counts.entry(theme.to_string()).or_default() += 1;
        let theme_label = period_theme_public_label(theme);
        let tone = event["tone"].as_str().unwrap_or("focused");
        let evidence_keys = event["evidence_keys"].clone();
        let style = period_style_variant_for_theme(theme);
        let personalization_hint = event
            .get("personalization_hint")
            .and_then(Value::as_str)
            .unwrap_or_else(|| period_event_personalization_hint(event));
        let natal_focus_hint = event
            .get("natal_focus_hint")
            .and_then(Value::as_str)
            .unwrap_or(personalization_hint);
        out.push(json!({
            "date": date,
            "day_label": public_day_label(date),
            "theme_code": theme,
            "theme_label": theme_label,
            "tone": tone,
            "summary_hint": format!("Synthèse journalière centrée sur {theme_label} avec une nuance natale lisible."),
            "advice_hint": period_advice_hint(theme, natal_focus_hint),
            "style_variant_code": style.code,
            "avoid_terms": style.avoid_terms,
            "natal_focus_hint": natal_focus_hint,
            "personalization_hint": personalization_hint,
            "evidence_keys": evidence_keys
        }));
    }
    Ok(out)
}

fn select_daily_plan_event<'a>(
    date: &str,
    events: &'a [Value],
    theme_counts: &HashMap<String, usize>,
) -> Result<&'a Value, GenerationError> {
    let candidates = events
        .iter()
        .filter(|event| event["date"].as_str() == Some(date))
        .collect::<Vec<_>>();
    let candidates = if candidates.is_empty() {
        events.iter().collect::<Vec<_>>()
    } else {
        candidates
    };
    let best = candidates
        .iter()
        .copied()
        .min_by(|left, right| period_event_sort(left, right))
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"))?;
    let Some(best_theme) = best["theme_code"].as_str() else {
        return Ok(best);
    };
    if theme_counts.get(best_theme).copied().unwrap_or(0) < 5 {
        return Ok(best);
    }
    Ok(candidates
        .iter()
        .copied()
        .filter(|event| {
            let theme = event["theme_code"].as_str().unwrap_or("");
            theme != best_theme && theme_counts.get(theme).copied().unwrap_or(0) < 5
        })
        .min_by(|left, right| period_event_sort(left, right))
        .unwrap_or(best))
}

fn period_internal_tone(theme: &str, fact_type: &str, aspect: Option<&str>) -> &'static str {
    match aspect {
        Some("square") | Some("opposition") => "careful",
        Some("trine") | Some("sextile") => "supportive",
        Some("conjunction") => "active",
        _ => match (theme, fact_type) {
            ("relationship", _) => "supportive",
            ("energy", _) | ("communication", _) => "active",
            ("integration", _) => "mixed",
            ("clarity", _) | ("organization", _) | ("routine", _) => "focused",
            _ => "focused",
        },
    }
}

fn period_event_score(item: &Value, event_type: &str, theme_count: usize) -> f64 {
    let orb = item.get("orb_deg").and_then(Value::as_f64);
    let base = match event_type {
        "transit_exact" => 0.98 - orb.unwrap_or(1.0).min(1.0) * 0.08,
        "transit_active" => 0.90 - orb.unwrap_or(6.0).min(6.0) * 0.025,
        "moon_house_by_day" => {
            0.60 + item
                .get("natal_house")
                .and_then(Value::as_i64)
                .map_or(0.0, |_| 0.05)
        }
        "transit_context" => 0.45 + context_object_bonus(item["transiting_object"].as_str()),
        _ => 0.50,
    };
    let repetition_bonus = ((theme_count.saturating_sub(1)).min(3) as f64) * 0.03;
    round2((base + repetition_bonus).min(1.0))
}

fn context_object_bonus(object: Option<&str>) -> f64 {
    match object {
        Some("sun") | Some("jupiter") => 0.12,
        Some("venus") | Some("mars") | Some("mercury") => 0.08,
        Some("moon") => 0.05,
        _ => 0.0,
    }
}

fn period_event_sort(left: &Value, right: &Value) -> std::cmp::Ordering {
    let left_score = left.get("score").and_then(Value::as_f64).unwrap_or(0.0);
    let right_score = right.get("score").and_then(Value::as_f64).unwrap_or(0.0);
    right_score
        .partial_cmp(&left_score)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| {
            left.get("date")
                .and_then(Value::as_str)
                .unwrap_or("")
                .cmp(right.get("date").and_then(Value::as_str).unwrap_or(""))
        })
}

fn is_period_watch_event(event: &Value) -> bool {
    let tone = event.get("tone").and_then(Value::as_str);
    let aspect = event.get("aspect").and_then(Value::as_str);
    tone == Some("careful") || matches!(aspect, Some("square") | Some("opposition"))
}

fn build_period_key_day_markers(events: &[Value], limit: usize) -> Vec<Value> {
    let Some(top_score) = events.first().and_then(|event| event["score"].as_f64()) else {
        return Vec::new();
    };
    let min_score = top_score - 0.08;
    let theme_counts = period_theme_counts(events);
    let mut candidates = events
        .iter()
        .filter(|event| {
            let score = event["score"].as_f64().unwrap_or(0.0);
            score >= 0.60 && score >= min_score
        })
        .cloned()
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        let left_score = left["score"].as_f64().unwrap_or(0.0);
        let right_score = right["score"].as_f64().unwrap_or(0.0);
        right_score
            .partial_cmp(&left_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                let left_theme = left["theme_code"].as_str().unwrap_or("");
                let right_theme = right["theme_code"].as_str().unwrap_or("");
                theme_counts
                    .get(left_theme)
                    .unwrap_or(&usize::MAX)
                    .cmp(theme_counts.get(right_theme).unwrap_or(&usize::MAX))
            })
            .then_with(|| {
                left["date"]
                    .as_str()
                    .unwrap_or("")
                    .cmp(right["date"].as_str().unwrap_or(""))
            })
    });
    build_period_day_markers_from_events(
        &candidates,
        limit,
        "Jour clé",
        PeriodMarkerRole::Key,
        None,
        None,
    )
}

#[derive(Clone, Copy)]
enum PeriodMarkerRole {
    Key,
    Best,
    Watch,
}

fn build_period_day_markers_from_events(
    events: &[Value],
    limit: usize,
    title: &str,
    role: PeriodMarkerRole,
    exclude_dates: Option<&HashSet<String>>,
    fallback_reason: Option<&str>,
) -> Vec<Value> {
    let mut seen_dates = HashSet::new();
    events
        .iter()
        .filter(|event| {
            let date = event.get("date").and_then(Value::as_str).unwrap_or("");
            !exclude_dates.map_or(false, |dates| dates.contains(date))
                && seen_dates.insert(date.to_string())
        })
        .take(limit)
        .map(|event| {
            json!({
                "date": event["date"],
                "title": title,
                "reason": period_marker_reason(role, event),
                "evidence_keys": event["evidence_keys"],
                "fallback_reason": fallback_reason.map_or(Value::Null, |reason| json!(reason))
            })
        })
        .collect()
}

fn build_period_best_day_markers(
    events: &[Value],
    watch_dates: &HashSet<String>,
    key_dates: &HashSet<String>,
    limit: usize,
) -> Vec<Value> {
    let mut used_themes = HashSet::new();
    let mut used_dates = HashSet::new();
    let mut out = Vec::new();
    for event in events {
        let date = event["date"].as_str().unwrap_or("");
        let theme = event["theme_code"].as_str().unwrap_or("");
        if watch_dates.contains(date)
            || key_dates.contains(date)
            || is_period_watch_event(event)
            || !used_dates.insert(date.to_string())
            || !used_themes.insert(theme.to_string())
        {
            continue;
        }
        out.push(build_period_marker(
            event,
            period_best_day_title(theme),
            PeriodMarkerRole::Best,
            None,
        ));
        if out.len() == limit {
            break;
        }
    }
    out
}

fn build_period_watch_day_markers(events: &[Value], limit: usize) -> Vec<Value> {
    let tension_candidates = events
        .iter()
        .filter(|event| is_period_watch_event(event))
        .cloned()
        .collect::<Vec<_>>();
    build_period_day_markers_from_events(
        &tension_candidates,
        limit,
        "Jour de vigilance",
        PeriodMarkerRole::Watch,
        None,
        None,
    )
}

fn build_period_watch_summary_plan(
    watch_days: &[Value],
    premium: bool,
    watch_windows: &[Value],
) -> Value {
    if watch_days.is_empty() {
        if premium && !watch_windows.is_empty() {
            return json!({
                "status": "low",
                "text": "Aucune fenêtre de vigilance forte ne ressort, mais certains moments demandent de limiter la dispersion et de garder une marge.",
                "evidence_keys": watch_windows
                    .iter()
                    .flat_map(|window| window["evidence_keys"].as_array().into_iter().flatten())
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
            });
        }
        return json!({
            "status": "none",
            "text": FREE_PERIOD_NONE_WATCH_SUMMARY,
            "evidence_keys": []
        });
    }
    json!({
        "status": "active",
        "text": "Un point de vigilance ressort et mérite une attention mesurée.",
        "evidence_keys": watch_days
            .iter()
            .flat_map(|day| day["evidence_keys"].as_array().into_iter().flatten())
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
    })
}

fn build_period_best_windows(events: &[Value], scan_plan: &Value, limit: usize) -> Vec<Value> {
    let snapshot_keys = scan_plan_snapshot_keys_by_date(scan_plan);
    let mut out = Vec::new();
    for event in events.iter().filter(|event| !is_period_watch_event(event)) {
        let Some(window) = build_period_window(event, &snapshot_keys, false) else {
            continue;
        };
        out.push(window);
        if out.len() == limit {
            break;
        }
    }
    out
}

fn build_period_watch_windows(
    events: &[Value],
    scan_plan: &Value,
    best_windows: &[Value],
    limit: usize,
) -> Vec<Value> {
    let snapshot_keys = scan_plan_snapshot_keys_by_date(scan_plan);
    let best_keys = best_windows
        .iter()
        .flat_map(|window| {
            window["source_snapshot_keys"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
        })
        .collect::<HashSet<_>>();
    let mut out = Vec::new();
    let candidates = events
        .iter()
        .filter(|event| is_period_watch_event(event))
        .collect::<Vec<_>>();
    let candidates = if candidates.is_empty() {
        events
            .iter()
            .filter(|event| !is_period_watch_event(event))
            .collect::<Vec<_>>()
    } else {
        candidates
    };
    for event in candidates {
        let Some(window) = build_period_window(event, &snapshot_keys, true) else {
            continue;
        };
        let overlaps_best = window["source_snapshot_keys"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .any(|key| best_keys.contains(key));
        if overlaps_best {
            continue;
        }
        out.push(window);
        if out.len() == limit {
            break;
        }
    }
    out
}

fn build_period_window(
    event: &Value,
    snapshot_keys: &HashMap<String, Vec<(String, String)>>,
    watch: bool,
) -> Option<Value> {
    let date = event["date"].as_str()?;
    let snapshots = snapshot_keys.get(date)?;
    let event_snapshot = event
        .get("snapshot_key")
        .and_then(Value::as_str)
        .and_then(|key| {
            snapshots
                .iter()
                .position(|(_, snapshot_key)| snapshot_key == key)
        })
        .unwrap_or(0);
    let (start_label, snapshot_key) = snapshots.get(event_snapshot)?.clone();
    let end_label = snapshots
        .get(event_snapshot + 1)
        .map(|(time, _)| time.clone())
        .unwrap_or_else(|| "00:00".to_string());
    let theme = event["theme_code"].as_str().unwrap_or("organization");
    let tone = event["tone"].as_str().unwrap_or("focused");
    let evidence_keys = event["evidence_keys"].clone();
    if watch {
        Some(json!({
            "date": date,
            "time_range_label": format!("{start_label}–{end_label}"),
            "source_snapshot_keys": [snapshot_key],
            "title": period_watch_window_title(theme, &start_label),
            "theme": period_theme_public_label(theme),
            "tone": period_tone_public_label(tone),
            "watch_point": period_watch_window_point(theme),
            "evidence_keys": evidence_keys
        }))
    } else {
        Some(json!({
            "date": date,
            "time_range_label": format!("{start_label}–{end_label}"),
            "source_snapshot_keys": [snapshot_key],
            "title": period_best_window_title(theme, &start_label),
            "theme": period_theme_public_label(theme),
            "tone": period_tone_public_label(tone),
            "reason": period_best_window_reason(theme),
            "best_for": period_best_window_best_for(theme, &start_label),
            "evidence_keys": evidence_keys
        }))
    }
}

fn scan_plan_snapshot_keys_by_date(scan_plan: &Value) -> HashMap<String, Vec<(String, String)>> {
    let mut by_date: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for snapshot in scan_plan["snapshots"].as_array().into_iter().flatten() {
        let Some(date) = snapshot["date"].as_str() else {
            continue;
        };
        let time = snapshot["reference_time_local"]
            .as_str()
            .unwrap_or("12:00")
            .to_string();
        let key = snapshot["snapshot_key"].as_str().unwrap_or("").to_string();
        by_date
            .entry(date.to_string())
            .or_default()
            .push((time, key));
    }
    for items in by_date.values_mut() {
        items.sort_by(|left, right| left.0.cmp(&right.0));
    }
    by_date
}

fn period_best_window_title(theme: &str, start_label: &str) -> &'static str {
    match (theme, start_label) {
        ("relationship", "00:00") => "Apaiser une attente personnelle",
        ("relationship", "06:00") => "Ouvrir un échange utile",
        ("relationship", "12:00") => "Clarifier une attente relationnelle",
        ("relationship", _) => "Retrouver une fluidité relationnelle",
        ("energy", "00:00") => "Relancer l'élan sans brusquer",
        ("energy", "06:00") => "Passer à l'action courte",
        ("energy", "12:00") => "Canaliser l'énergie disponible",
        ("energy", _) => "Transformer l'élan en décision",
        ("communication", "00:00") => "Préparer une parole nette",
        ("communication", "06:00") => "Formuler le message essentiel",
        ("communication", "12:00") => "Mettre les mots au bon endroit",
        ("communication", _) => "Répondre avec plus de précision",
        ("clarity", "00:00") => "Reprendre l'initiative personnelle",
        ("clarity", "06:00") => "Clarifier le cap visible",
        ("clarity", "12:00") => "Choisir une suite simple",
        ("clarity", _) => "Retrouver une impulsion créative",
        ("integration", "00:00") => "Stabiliser une base intérieure",
        ("integration", "06:00") => "Consolider ce qui doit durer",
        ("integration", "12:00") => "Relier les décisions au cadre",
        ("integration", _) => "Préparer une suite plus stable",
        (_, "00:00") => "Reprendre l'initiative personnelle",
        (_, "06:00") => "Clarifier le cap visible",
        (_, "12:00") => "Stabiliser une décision utile",
        _ => "Retrouver une impulsion créative",
    }
}

fn period_watch_window_title(theme: &str, start_label: &str) -> &'static str {
    match (theme, start_label) {
        ("communication", _) => "Limiter les réponses trop rapides",
        ("energy", _) => "Canaliser la réaction",
        ("relationship", _) => "Préserver une marge relationnelle",
        ("clarity", _) => "Reporter une conclusion trop nette",
        ("integration", _) => "Ne pas surcharger le cadre",
        (_, "00:00") => "Éviter de tout relancer d'un coup",
        (_, "06:00") => "Garder une marge avant d'agir",
        (_, "12:00") => "Limiter la dispersion du milieu de journée",
        _ => "Ralentir avant de répondre",
    }
}

fn period_best_window_reason(theme: &str) -> &'static str {
    match theme {
        "relationship" => "Ce créneau se prête à un échange plus simple et mieux ajusté.",
        "energy" => "Ce créneau aide à transformer l'élan en action courte.",
        "communication" => "Ce créneau favorise une formulation plus nette.",
        "clarity" => "Ce créneau aide à trier et décider sans disperser l'attention.",
        "integration" => "Ce créneau aide à consolider ce qui a déjà été compris.",
        _ => "Ce créneau peut servir à poser une action simple et vérifiable.",
    }
}

fn period_watch_window_point(theme: &str) -> &'static str {
    match theme {
        "communication" => "Éviter de répondre trop vite si l'échange devient tendu.",
        "energy" => "Ralentir avant d'agir sous impulsion.",
        "relationship" => "Ne pas surinterpréter une réaction ou un silence.",
        "clarity" => "Reporter une conclusion si les informations restent incomplètes.",
        _ => "Garder une marge avant de transformer l'impression en décision définitive.",
    }
}

fn period_best_window_best_for(theme: &str, start_label: &str) -> Vec<&'static str> {
    match (theme, start_label) {
        ("relationship", "00:00") => vec![
            "apaiser une attente personnelle",
            "préparer un échange sensible",
            "retrouver une disponibilité affective",
        ],
        ("relationship", "06:00") => vec![
            "ouvrir un échange utile",
            "clarifier une attente",
            "réparer un malentendu simple",
        ],
        ("relationship", "12:00") => vec![
            "poser un accord concret",
            "nommer un besoin relationnel",
            "ajuster une attente partagée",
        ],
        ("relationship", _) => vec![
            "fluidifier une relation",
            "répondre avec nuance",
            "consolider un lien utile",
        ],
        ("energy", "00:00") => vec![
            "préparer l'élan du jour",
            "choisir une action courte",
            "éviter de démarrer trop vite",
        ],
        ("energy", "06:00") => vec![
            "lancer une action courte",
            "débloquer une décision pratique",
            "poser une limite concrète",
        ],
        ("energy", "12:00") => vec![
            "canaliser l'effort utile",
            "traiter un point actif",
            "agir sans disperser l'énergie",
        ],
        ("energy", _) => vec![
            "transformer l'élan en décision",
            "conclure une action simple",
            "récupérer après l'effort",
        ],
        ("communication", "00:00") => vec![
            "préparer une formulation",
            "ordonner les arguments",
            "clarifier l'intention du message",
        ],
        ("communication", "06:00") => vec![
            "envoyer un message clair",
            "préparer une réponse",
            "nommer une priorité",
        ],
        ("communication", "12:00") => vec![
            "ajuster une réponse",
            "tenir un échange précis",
            "réduire les explications inutiles",
        ],
        ("communication", _) => vec![
            "répondre avec précision",
            "clore une discussion utile",
            "poser un cadre verbal",
        ],
        ("clarity", "00:00") => vec![
            "reprendre l'initiative personnelle",
            "poser un repère simple",
            "préparer le rythme du jour",
        ],
        ("clarity", "06:00") => vec![
            "clarifier le cap visible",
            "organiser la prochaine étape",
            "rendre une priorité lisible",
        ],
        ("clarity", "12:00") => vec![
            "trier les options",
            "choisir une suite simple",
            "mettre à jour une priorité",
        ],
        ("clarity", _) => vec![
            "retrouver une impulsion créative",
            "assouplir une décision",
            "préserver un élan durable",
        ],
        ("integration", "00:00") => vec![
            "stabiliser une base intérieure",
            "préparer une consolidation",
            "faire le point avant d'élargir",
        ],
        ("integration", "06:00") => vec![
            "consolider une avancée",
            "revenir à l'essentiel",
            "stabiliser une décision",
        ],
        ("integration", "12:00") => vec![
            "relier une décision au cadre",
            "vérifier la tenue d'un engagement",
            "ordonner ce qui doit durer",
        ],
        ("integration", _) => vec![
            "préparer une suite stable",
            "assimiler une étape",
            "réduire ce qui surcharge",
        ],
        (_, "00:00") => vec![
            "reprendre l'initiative personnelle",
            "poser un repère simple",
            "préparer le rythme du jour",
        ],
        (_, "06:00") => vec![
            "clarifier le cap visible",
            "organiser la prochaine étape",
            "rendre une priorité lisible",
        ],
        (_, "12:00") => vec![
            "stabiliser une décision utile",
            "trier les options concrètes",
            "réduire la dispersion",
        ],
        _ => vec![
            "retrouver une impulsion créative",
            "assouplir une décision",
            "préserver un élan durable",
        ],
    }
}

fn build_period_premium_scores(request: &Value) -> Value {
    let events = request["period_events"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let event_score = events
        .first()
        .and_then(|event| event["score"].as_f64())
        .unwrap_or(0.0);
    let tension_score = events
        .iter()
        .filter(|event| is_period_watch_event(event))
        .filter_map(|event| event["score"].as_f64())
        .fold(0.0, f64::max);
    let support_score = events
        .iter()
        .filter(|event| !is_period_watch_event(event))
        .filter_map(|event| event["score"].as_f64())
        .fold(0.0, f64::max);
    json!({
        "event_score": round2(event_score),
        "day_score": round2(event_score * 0.92),
        "window_score": round2(support_score.max(tension_score) * 0.95),
        "domain_score": round2(period_domain_coverage_score(&events)),
        "tension_score": round2(tension_score),
        "support_score": round2(support_score),
        "clarity_score": round2(period_theme_score(&events, "clarity")),
        "relationship_score": round2(period_theme_score(&events, "relationship")),
        "energy_score": round2(period_theme_score(&events, "energy")),
        "decision_score": round2(period_theme_score(&events, "communication").max(period_theme_score(&events, "clarity"))),
        "integration_score": round2(period_theme_score(&events, "integration"))
    })
}

fn period_theme_score(events: &[Value], theme: &str) -> f64 {
    events
        .iter()
        .filter(|event| event["theme_code"].as_str() == Some(theme))
        .filter_map(|event| event["score"].as_f64())
        .fold(0.0, f64::max)
}

fn period_domain_coverage_score(events: &[Value]) -> f64 {
    if events.is_empty() {
        return 0.0;
    }
    let distinct_themes = events
        .iter()
        .filter_map(|event| event["theme_code"].as_str())
        .collect::<HashSet<_>>()
        .len() as f64;
    let evidence_coverage = (events.len() as f64 / 50.0).min(1.0);
    let theme_coverage = (distinct_themes / 6.0).min(1.0);
    round2((theme_coverage * 0.7) + (evidence_coverage * 0.3))
}

fn build_period_marker(
    event: &Value,
    title: &str,
    role: PeriodMarkerRole,
    fallback_reason: Option<&str>,
) -> Value {
    json!({
        "date": event["date"],
        "title": title,
        "reason": period_marker_reason(role, event),
        "evidence_keys": event["evidence_keys"],
        "fallback_reason": fallback_reason.map_or(Value::Null, |reason| json!(reason))
    })
}

fn period_marker_reason(role: PeriodMarkerRole, event: &Value) -> String {
    let date = event.get("date").and_then(Value::as_str).unwrap_or("");
    let theme = event
        .get("theme_code")
        .and_then(Value::as_str)
        .unwrap_or("principal");
    let theme_label = period_theme_public_label(theme);
    let focus = period_public_focus_text(event);
    match role {
        PeriodMarkerRole::Key => format!(
            "{} est un jour clé parce que {} y pèse davantage dans les choix concrets liés à {}. C'est une priorité à retenir pour repérer ce qui mérite une décision, un tri ou une mise au clair.",
            public_day_label(date),
            theme_label,
            focus
        ),
        PeriodMarkerRole::Best => format!(
            "{} se prête mieux à une action simple autour de {} : {} aide à choisir le bon message, confirmer un rendez-vous ou terminer une tâche sans tout rouvrir.",
            public_day_label(date),
            focus,
            theme_label
        ),
        PeriodMarkerRole::Watch => format!(
            "{} demande plus de mesure parce que {} peut rendre les réactions plus rapides autour de {}. Prenez le temps de vérifier les faits, le ton et la portée d'une décision avant de répondre.",
            public_day_label(date),
            theme_label,
            focus
        ),
    }
}

fn period_best_day_title(theme: &str) -> &'static str {
    match theme {
        "relationship" => "Meilleur jour relationnel",
        "clarity" => "Jour de clarté",
        "energy" | "communication" => "Meilleur jour d'action",
        "integration" => "Jour d'intégration",
        "organization" | "routine" => "Jour le plus structurant",
        _ => "Jour favorable",
    }
}

fn period_theme_counts(events: &[Value]) -> HashMap<&str, usize> {
    events
        .iter()
        .filter_map(|event| event["theme_code"].as_str())
        .fold(HashMap::new(), |mut counts, theme| {
            *counts.entry(theme).or_default() += 1;
            counts
        })
}

fn build_period_domain_sections(evidence: &[Value], max_sections: usize) -> Vec<Value> {
    let mut by_theme: HashMap<String, Vec<&Value>> = HashMap::new();
    for item in evidence {
        let theme = item["theme_code"].as_str().unwrap_or("organization");
        by_theme.entry(theme.to_string()).or_default().push(item);
    }
    let mut themes = by_theme
        .into_iter()
        .map(|(theme, items)| {
            let score = items.len() as f64
                + items
                    .iter()
                    .filter_map(|item| item.get("orb_deg").and_then(Value::as_f64))
                    .map(|orb| (6.0 - orb).max(0.0) / 10.0)
                    .sum::<f64>();
            (theme, items, score)
        })
        .collect::<Vec<_>>();
    themes.sort_by(|left, right| {
        right
            .2
            .partial_cmp(&left.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.0.cmp(&right.0))
    });
    themes
        .into_iter()
        .take(max_sections)
        .map(|(theme, items, _)| {
            let first = items.first().copied().unwrap_or(&Value::Null);
            let evidence_keys = items
                .iter()
                .filter_map(|item| item["evidence_key"].as_str())
                .take(3)
                .collect::<Vec<_>>();
            let label = period_theme_public_label(&theme);
            let natal_hint = first["natal_focus_hint"]
                .as_str()
                .unwrap_or("Relier ce domaine à un repère personnel important.");
            let personalization = first["personalization_hint"].as_str().unwrap_or(natal_hint);
            json!({
                "domain": label,
                "title": period_domain_title(&theme),
                "focus": period_domain_focus(&theme, personalization),
                "natal_focus_hint": natal_hint,
                "personalization_hint": personalization,
                "evidence_keys": evidence_keys
            })
        })
        .collect::<Vec<_>>()
}

async fn period_writer_response(
    use_case: &GenerateReadingUseCase,
    request: &Value,
) -> Result<Value, GenerationError> {
    let defaults = horoscope_writer_engine_defaults(use_case);
    if defaults.provider == ProviderKind::Fake {
        return fake_period_writer_response(request);
    }

    let schema = period_response_provider_schema(request)?;
    let provider_request = ProviderGenerationRequest {
        model: defaults.model.clone(),
        messages: period_writer_messages(request)?,
        structured_schema: Some(schema),
        reasoning_effort: None,
        temperature: Some(0.4),
        max_output_tokens: Some(period_writer_max_output_tokens(request)),
        safety_mode: SafetyMode::PlatformRulesOnly,
        timeout: StdDuration::from_secs(180),
        metadata: GenerationMetadata {
            run_id: uuid::Uuid::new_v4().to_string(),
            request_id: None,
            product_code: request["service_code"]
                .as_str()
                .unwrap_or(HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE)
                .to_string(),
            chapter_code: None,
        },
    };

    let routed = use_case
        .router
        .generate(
            provider_request,
            defaults.provider.clone(),
            &defaults.model,
            false,
            true,
            ModelRouteContext::PrimaryReading,
        )
        .await?;
    if routed.used_provider == ProviderKind::Fake {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_TECHNICAL_CODE_LEAK",
            json!({ "provider": "fake" }),
        ));
    }
    let mut response = routed
        .response
        .parsed_json
        .or_else(|| parse_period_provider_json(&routed.response.raw_text))
        .ok_or_else(|| {
            GenerationError::with_details(
                GenerationErrorCode::PostSafetyValidationFailed,
                format!(
                    "HOROSCOPE_PERIOD_RESPONSE_INVALID: provider_response_not_json raw_text_len={}",
                    routed.response.raw_text.len()
                ),
                json!({
                    "reason": "provider_response_not_json",
                    "raw_text_len": routed.response.raw_text.len()
                }),
            )
        })?;
    if !response
        .get("quality")
        .map_or(false, |value| value.is_object())
    {
        response["quality"] = json!({});
    }
    response["quality"]["provider"] = json!(routed.used_provider.as_str());
    response["quality"]["model"] = json!(routed.response.model_used);
    response["quality"]["fallback_used"] = json!(routed.fallback_used);
    repair_period_response_shape(request, &mut response);
    normalize_period_public_tones(request, &mut response);
    response = postprocess_period_provider_response(request, response);
    enforce_period_domain_personalization(request, &mut response);
    enforce_premium_period_advice_synthesis(request, &mut response);
    validate_period_provider_public_payload(&response)?;
    Ok(response)
}

#[doc(hidden)]
pub fn period_response_provider_schema(request: &Value) -> Result<Value, GenerationError> {
    let mut schema: Value = serde_json::from_str(PERIOD_RESPONSE_SCHEMA_JSON).map_err(|err| {
        GenerationError::with_details(
            GenerationErrorCode::SchemaValidationFailed,
            format!("HOROSCOPE_PERIOD_RESPONSE_INVALID: {err}"),
            Value::Null,
        )
    })?;
    schema.as_object_mut().map(|object| {
        object.remove("allOf");
    });
    let free = is_free_period_request(request);
    let premium = is_premium_period_request(request);
    if free {
        {
            let required = schema
                .get_mut("required")
                .and_then(Value::as_array_mut)
                .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_RESPONSE_INVALID"))?;
            *required = vec![
                json!("contract_version"),
                json!("service_code"),
                json!("period_resolution"),
                json!("summary"),
                json!("dominant_theme"),
                json!("key_days"),
                json!("advice"),
                json!("watch_summary"),
                json!("evidence_summary"),
                json!("quality"),
            ];
        }
        let properties = schema
            .get_mut("properties")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_RESPONSE_INVALID"))?;
        for field in [
            "week_overview",
            "best_days",
            "watch_days",
            "daily_timeline",
            "domain_sections",
            "best_windows",
            "watch_windows",
            "strategy",
        ] {
            properties.remove(field);
        }
        properties["advice"] = json!({ "type": "string" });
        properties["key_days"] = json!({
            "type": "array",
            "minItems": 1,
            "maxItems": 2,
            "items": { "$ref": "#/definitions/day_marker" }
        });
        properties["evidence_summary"] = json!({
            "type": "array",
            "minItems": 1,
            "maxItems": 3,
            "items": { "$ref": "#/definitions/evidence_summary_item" }
        });
        properties["watch_summary"] = json!({ "$ref": "#/definitions/free_watch_summary" });
        return Ok(schema);
    }
    if premium {
        let required = schema
            .get_mut("required")
            .and_then(Value::as_array_mut)
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_RESPONSE_INVALID"))?;
        for field in [
            "week_overview",
            "best_days",
            "watch_days",
            "daily_timeline",
            "domain_sections",
            "best_windows",
            "watch_windows",
            "strategy",
        ] {
            if !required.iter().any(|value| value.as_str() == Some(field)) {
                required.push(json!(field));
            }
        }
        required
            .retain(|value| !matches!(value.as_str(), Some("summary") | Some("dominant_theme")));
        let properties = schema
            .get_mut("properties")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_RESPONSE_INVALID"))?;
        properties.remove("summary");
        properties.remove("dominant_theme");
    } else {
        {
            let required = schema
                .get_mut("required")
                .and_then(Value::as_array_mut)
                .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_RESPONSE_INVALID"))?;
            for field in [
                "week_overview",
                "best_days",
                "watch_days",
                "daily_timeline",
                "domain_sections",
            ] {
                if !required.iter().any(|value| value.as_str() == Some(field)) {
                    required.push(json!(field));
                }
            }
            required.retain(|value| {
                !matches!(
                    value.as_str(),
                    Some("summary")
                        | Some("dominant_theme")
                        | Some("best_windows")
                        | Some("watch_windows")
                        | Some("strategy")
                )
            });
        }
        let properties = schema
            .get_mut("properties")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_RESPONSE_INVALID"))?;
        properties.remove("summary");
        properties.remove("dominant_theme");
        properties.remove("best_windows");
        properties.remove("watch_windows");
        properties.remove("strategy");
    }
    Ok(schema)
}

fn parse_period_provider_json(raw: &str) -> Option<Value> {
    serde_json::from_str::<Value>(raw)
        .ok()
        .or_else(|| {
            let trimmed = raw.trim();
            let unfenced = trimmed
                .strip_prefix("```json")
                .or_else(|| trimmed.strip_prefix("```"))
                .and_then(|value| value.strip_suffix("```"))
                .map(str::trim)
                .unwrap_or(trimmed);
            serde_json::from_str::<Value>(unfenced).ok()
        })
        .or_else(|| {
            extract_balanced_json_object(raw).and_then(|json| serde_json::from_str(&json).ok())
        })
}

fn extract_balanced_json_object(raw: &str) -> Option<String> {
    let start = raw.find('{')?;
    let mut depth = 0_i32;
    let mut in_string = false;
    let mut escaped = false;
    for (offset, ch) in raw[start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(raw[start..start + offset + ch.len_utf8()].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

fn period_writer_messages(request: &Value) -> Result<Vec<PromptMessage>, GenerationError> {
    let compact = serde_json::to_string(request).map_err(|err| {
        GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            format!("HOROSCOPE_PERIOD_RESPONSE_INVALID: {err}"),
            Value::Null,
        )
    })?;
    let limits = period_word_limits_for_request(request);
    if is_free_period_request(request) {
        return Ok(vec![
            PromptMessage {
                role: PromptRole::System,
                content: format!(
                    "Tu écris un horoscope Free des 7 prochains jours en français. Retourne uniquement un JSON conforme au schéma fourni. N'expose jamais daily_timeline, best_days, watch_days, windows, domain_sections ou strategy. N'invente aucune preuve et n'affiche aucun code interne. Le texte public doit compter entre {} et {} mots, sans dépasser {} mots.",
                    limits.target_min, limits.target_max, limits.hard_limit
                ),
            },
            PromptMessage {
                role: PromptRole::User,
                content: format!(
                    "Construis horoscope_period_response_v1 Free compact. Produis summary, dominant_theme, 1 à 2 key_days sous forme de jours à retenir, advice en 1 à 3 phrases, watch_summary court, evidence_summary limitée à 1 à 3 entrées. key_days sont des repères utiles, jamais des meilleurs jours ni des créneaux favorables. Si watch_summary.status vaut none, garde evidence_keys vide et explique brièvement qu'aucun signal dominant ne ressort tout en donnant une marge d'observation concrète. summary.text doit rester entre 90 et 180 mots et mentionner au maximum deux dates explicites. Requête JSON:\n{compact}"
                ),
            },
        ]);
    }
    if is_premium_period_request(request) {
        return Ok(vec![
            PromptMessage {
                role: PromptRole::System,
                content: format!(
                    "Tu écris une lecture Premium d'horoscope de période en français. Retourne uniquement un objet JSON conforme au schéma fourni. N'invente aucune preuve: chaque evidence_key publique et chaque source_snapshot_key doit provenir de la requête. N'affiche jamais les codes internes, les clés de preuve, les noms techniques de transits, les theme_code anglais, les codes tone anglais, ni les consignes internes. Ecris pour une personne, pas pour expliquer une grille: privilégie les situations concrètes, les gestes observables, les décisions, les échanges, le rythme, les limites et les marges de manœuvre. Bannir les formulations de structure comme signal, thème qui devient lisible, devient plus lisible, plus lisibles, relief principal, trajectoire de période, timeline, fil de semaine, appui concret dans la trajectoire, ou paraphrase des consignes. La lecture doit être lisible dans cet ordre: vue d'ensemble, repères de période très courts, détail des 7 journées, domaines, fenêtres horaires, stratégie. Les 7 entrées quotidiennes doivent couvrir exactement les 7 dates, avec une formulation distincte pour chaque journée. La lecture publique doit compter entre {} et {} mots, sans dépasser {} mots.",
                    limits.target_min, limits.target_max, limits.hard_limit
                ),
            },
            PromptMessage {
                role: PromptRole::User,
                content: format!(
                    "Construis horoscope_period_response_v1 Premium pour cette requête. La valeur Premium doit venir de best_windows, watch_windows, strategy, 3 à 5 domain_sections, d'une section evidence_summary utile, et d'une semaine plus pilotable que le Basic. evidence_summary est la section des clés d'appui: sélectionne des evidence_keys distinctes, présentes dans la requête, qui expliquent les interprétations principales sans répéter mécaniquement les mêmes clés. key_days, best_days et watch_days restent des listes courtes pour le JSON, mais leur explication doit être reprise naturellement dans l'entrée daily_timeline de la même date: si une journée est clé, favorable ou de vigilance, le texte de cette journée doit le faire comprendre et expliquer pourquoi, sans renvoyer à une autre section. Les listes key_days/best_days/watch_days ne doivent donc pas devenir des mini-interprétations séparées. best_windows/watch_windows sont des plages horaires et doivent garder les source_snapshot_keys fournis. Développe week_overview, strategy, advice, domain_sections, windows et les 7 entrées daily_timeline afin d'atteindre {} à {} mots publics. advice et strategy doivent synthétiser une méthode d'usage sans ajouter de nouvelles dates explicites: renvoie aux fenêtres déjà listées plutôt que refaire un calendrier. Utilise les libellés français déjà présents, pas les codes internes. Pour chaque entrée daily_timeline, garde le thème principal du daily_plan et mentionne au besoin les éléments secondaires du même jour dans le texte afin d'éviter une semaine monotone. Remplace les abstractions par des exemples de vie courante: message à envoyer, rendez-vous à cadrer, charge à alléger, décision à différer, tâche à terminer, conversation à pacifier. Respecte les avoid_terms des daily_plans et ne répète pas plus de deux fois les mêmes amorces comme clarifier, ajuster, intégrer, restez concret, gardez une marge ou choisissez une seule priorité. Requête JSON:\n{compact}",
                    limits.target_min, limits.target_max
                ),
            },
        ]);
    }
    Ok(vec![
        PromptMessage {
            role: PromptRole::System,
            content: format!(
                "Tu écris une lecture d'horoscope de période en français. Retourne uniquement un objet JSON conforme au schéma fourni. N'invente aucune preuve: chaque evidence_key publique doit provenir de la requête. N'affiche jamais les codes internes, les clés de preuve, les noms techniques de transits, les theme_code anglais, ni les codes tone anglais. La timeline doit couvrir exactement les 7 dates, avec des formulations variées et une trajectoire globale. La lecture publique doit compter entre {} et {} mots, sans dépasser {} mots.",
                limits.target_min, limits.target_max, limits.hard_limit
            ),
        },
        PromptMessage {
            role: PromptRole::User,
            content: format!(
                "Construis horoscope_period_response_v1 pour cette requête d'interprétation. Utilise les libellés français déjà présents, pas les codes internes. Développe week_overview, watch_summary, advice, domain_sections et les 7 entrées daily_timeline afin d'atteindre {} à {} mots publics. Utilise les indications internes de personnalisation natale pour écrire une nuance lisible dans au moins 4 jours, chaque domaine et la vue d'ensemble, sans recopier les noms de champs ni les consignes internes. Respecte les avoid_terms des daily_plans pour éviter les répétitions. Requête JSON:\n{compact}",
                limits.target_min, limits.target_max
            ),
        },
    ])
}

#[doc(hidden)]
pub fn fake_period_writer_response(request: &Value) -> Result<Value, GenerationError> {
    if is_free_period_request(request) {
        return fake_free_period_writer_response(request);
    }
    let daily_timeline = request["daily_plans"]
        .as_array()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_TIMELINE_MISSING"))?
        .iter()
        .enumerate()
        .map(|(index, day)| {
            let theme = day["theme_code"].as_str().unwrap_or("organisation");
            let theme_label = day["theme_label"]
                .as_str()
                .unwrap_or_else(|| period_theme_public_label(theme));
            let text = ensure_period_personalization_text(
                &period_public_day_text(day, index),
                &format!(
                    "Vos repères personnels aident ici à agir plus simplement autour de {}.",
                    period_public_focus_text(day)
                ),
            );
            json!({
                "date": day["date"],
                "day_label": day["day_label"],
                "theme": theme_label,
                "tone": period_tone_public_label(day["tone"].as_str().unwrap_or("focused")),
                "text": text,
                "advice": period_public_day_advice(day),
                "evidence_keys": day["evidence_keys"]
            })
        })
        .collect::<Vec<_>>();
    let domain_sections = request["domain_sections"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|section| {
            json!({
                "domain": section["domain"],
                "title": section["title"],
                "text": period_public_domain_text(section),
                "evidence_keys": section["evidence_keys"]
            })
        })
        .collect::<Vec<_>>();
    let service_code = request["service_code"]
        .as_str()
        .unwrap_or(HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE);
    let mut response = json!({
        "contract_version": "horoscope_period_response_v1",
        "service_code": service_code,
        "period_resolution": request["period_resolution"],
        "week_overview": {
            "title": "Vos 7 prochains jours",
            "text": "La période se lit comme une progression continue : d'abord clarifier les priorités dans les relations directes, puis ajuster les échanges et terminer sur une intégration plus posée.",
            "trajectory": "Une trajectoire globale relie les jours clés, les besoins émotionnels et les choix à consolider."
        },
        "key_days": request["key_days"],
        "best_days": request["best_days"],
        "watch_days": request["watch_days"],
        "watch_summary": request["watch_summary_plan"],
        "daily_timeline": daily_timeline,
        "domain_sections": domain_sections,
        "advice": {
            "main": "Avancez par étapes courtes et gardez une trace de ce qui évolue d'un jour à l'autre.",
            "best_use": "Planifier, prioriser et consolider les échanges importants.",
            "avoid": "Transformer un signal quotidien en certitude définitive."
        },
        "evidence_summary": request["evidence"].as_array().into_iter().flatten().take(5).map(|item| json!({
            "evidence_key": item["evidence_key"],
            "date": item["date"],
            "label": item["human_label"]
        })).collect::<Vec<_>>(),
        "quality": {
            "daily_timeline_count": 7,
            "evidence_guard_passed": true,
            "best_watch_overlap_passed": true,
            "provider": "fake",
            "model": "fake-model",
            "fallback_used": false,
            "period_contract": "basic_next_7_days"
        }
    });
    if is_premium_period_service(service_code) {
        response["best_windows"] = request["best_windows"].clone();
        response["watch_windows"] = request["watch_windows"].clone();
        response["strategy"] = json!({
            "title": request["strategy"]["title"].as_str().unwrap_or("Stratégie de semaine"),
            "text": "Utilisez les meilleurs créneaux pour agir court et les moments de vigilance pour ralentir avant de répondre. La stratégie consiste à alterner décision, clarification et récupération sans transformer la semaine en suite d'urgences.",
            "best_use": request["strategy"]["best_use"].as_str().unwrap_or("Réserver les créneaux soutenants aux échanges utiles."),
            "recovery": request["strategy"]["recovery"].as_str().unwrap_or("Préserver des temps de recul après les moments plus réactifs."),
            "evidence_keys": request["strategy"]["evidence_keys"]
        });
        response["quality"]["period_contract"] = json!("premium_next_7_days");
    }
    Ok(response)
}

fn fake_free_period_writer_response(request: &Value) -> Result<Value, GenerationError> {
    let evidence = request["evidence"]
        .as_array()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_FREE_EVIDENCE_MISSING"))?;
    let primary = evidence
        .first()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_FREE_EVIDENCE_MISSING"))?;
    let evidence_key = primary["evidence_key"].clone();
    let date = primary["date"]
        .as_str()
        .or_else(|| request["period_resolution"]["included_dates"][0].as_str())
        .unwrap_or("2026-06-07");
    let theme = period_theme_public_label(primary["theme_code"].as_str().unwrap_or("organization"));
    let key_days = request["key_days"]
        .as_array()
        .into_iter()
        .flatten()
        .take(2)
        .cloned()
        .collect::<Vec<_>>();
    let key_days = if key_days.is_empty() {
        vec![json!({
            "date": date,
            "title": "Jour à retenir",
            "reason": format!("Le thème {} ressort plus nettement et donne un repère utile sans en faire un verdict.", theme),
            "evidence_keys": [evidence_key.clone()],
            "fallback_reason": null
        })]
    } else {
        key_days
    };
    Ok(json!({
        "contract_version": "horoscope_period_response_v1",
        "service_code": HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        "period_resolution": request["period_resolution"],
        "summary": {
            "title": "Vos 7 prochains jours",
            "text": format!("Les prochains jours donnent surtout une tendance à comprendre plutôt qu'un planning à suivre. Autour du {date}, le climat met l'accent sur {theme} : une priorité simple, un échange à clarifier ou une routine à stabiliser peut devenir le fil conducteur. L'intérêt est de repérer ce qui demande de l'attention sans découper chaque journée ni chercher une fenêtre idéale. Gardez une marge pour ajuster votre rythme, observez les moments où les émotions accélèrent les décisions, puis revenez à une action concrète. Cette lecture reste volontairement compacte : elle sert de boussole générale pour choisir ce qui mérite d'être traité maintenant et ce qui peut attendre.")
        },
        "dominant_theme": {
            "theme": theme,
            "text": format!("Le thème dominant est {theme}. Il invite à privilégier une décision simple, reliée à vos repères personnels, plutôt qu'une dispersion sur plusieurs sujets.")
        },
        "key_days": key_days,
        "advice": "Choisissez une seule priorité observable et gardez assez de souplesse pour l'ajuster. Notez ce qui se répète avant de conclure.",
        "watch_summary": {
            "status": "low",
            "text": "Une vigilance légère suffit : ralentir si une réaction paraît plus forte que la situation.",
            "evidence_keys": [evidence_key]
        },
        "evidence_summary": evidence.iter().take(3).map(|item| json!({
            "evidence_key": item["evidence_key"],
            "date": item["date"],
            "label": item["human_label"]
        })).collect::<Vec<_>>(),
        "quality": {
            "daily_timeline_count": 0,
            "evidence_guard_passed": true,
            "best_watch_overlap_passed": true,
            "provider": "fake",
            "model": "fake-model",
            "fallback_used": false,
            "period_contract": "free_next_7_days"
        }
    }))
}

pub fn repair_period_response_shape(request: &Value, response: &mut Value) {
    let service_code = request["service_code"]
        .as_str()
        .unwrap_or(HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE);
    response["contract_version"] = json!("horoscope_period_response_v1");
    response["service_code"] = json!(service_code);
    response["period_resolution"] = request["period_resolution"].clone();
    if is_free_period_service(service_code) {
        repair_free_period_response_shape(request, response);
        return;
    }

    response["week_overview"] = sanitize_period_week_overview(response.get("week_overview"));
    response["advice"] = sanitize_period_advice(response.get("advice"));
    response["key_days"] = sanitize_period_markers(response.get("key_days"), &request["key_days"]);
    response["best_days"] =
        sanitize_period_markers(response.get("best_days"), &request["best_days"]);
    response["watch_days"] =
        sanitize_period_markers(response.get("watch_days"), &request["watch_days"]);
    response["watch_summary"] = sanitize_period_watch_summary(
        response.get("watch_summary"),
        &request["watch_summary_plan"],
    );
    response["daily_timeline"] =
        sanitize_period_daily_timeline(response.get("daily_timeline"), request);
    response["domain_sections"] =
        sanitize_period_domain_sections(response.get("domain_sections"), request);
    if is_premium_period_service(service_code) {
        response["best_windows"] =
            sanitize_period_windows(response.get("best_windows"), request, "best_windows");
        response["watch_windows"] =
            sanitize_period_windows(response.get("watch_windows"), request, "watch_windows");
        response["strategy"] = sanitize_period_strategy(response.get("strategy"), request);
    } else {
        response.as_object_mut().map(|map| {
            map.remove("best_windows");
            map.remove("watch_windows");
            map.remove("strategy");
        });
    }
    response.as_object_mut().map(|map| {
        map.remove("summary");
        map.remove("dominant_theme");
    });
    response["evidence_summary"] =
        sanitize_period_evidence_summary(response.get("evidence_summary"), request);
    ensure_period_response_minimum_words(request, response);
    normalize_period_week_overview_repetition(response);
    normalize_period_repetitive_public_phrases(response);
    dedupe_period_daily_timeline_texts(request, response);
    enforce_period_domain_personalization(request, response);

    let provider = response["quality"]["provider"]
        .as_str()
        .unwrap_or("openai")
        .to_string();
    let model = response["quality"]["model"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let fallback_used = response["quality"]["fallback_used"]
        .as_bool()
        .unwrap_or(false);
    response["quality"] = json!({
        "daily_timeline_count": response["daily_timeline"].as_array().map(|days| days.len()).unwrap_or(0) as i64,
        "evidence_guard_passed": true,
        "best_watch_overlap_passed": true,
        "provider": provider,
        "model": model,
        "fallback_used": fallback_used,
        "period_contract": "horoscope_period_response_v1"
    });
}

fn reprocess_horoscope_daily_payload(response: Value) -> Value {
    reprocess_horoscope_daily("fr", response, None).payload
}

#[doc(hidden)]
pub fn reprocess_horoscope_period_payload(response: Value) -> Value {
    reprocess_horoscope_period("fr", response, None).payload
}

#[doc(hidden)]
pub fn postprocess_period_provider_response(request: &Value, response: Value) -> Value {
    let mut response = reprocess_horoscope_period_payload(response);
    prune_period_response_variant_fields(request, &mut response);
    finalize_period_response_words_and_repetition(request, &mut response);
    prune_period_response_variant_fields(request, &mut response);
    response
}

fn finalize_period_response_words_and_repetition(request: &Value, response: &mut Value) {
    ensure_period_response_minimum_words(request, response);
    normalize_period_week_overview_repetition(response);
    normalize_period_repetitive_public_phrases(response);
    dedupe_period_daily_timeline_texts(request, response);
    enforce_period_overview_personalization(response);
    enforce_period_domain_personalization(request, response);
    enforce_premium_period_advice_synthesis(request, response);
    ensure_period_response_minimum_words(request, response);
    normalize_period_week_overview_repetition(response);
    normalize_period_repetitive_public_phrases(response);
    dedupe_period_daily_timeline_texts(request, response);
    enforce_period_overview_personalization(response);
    enforce_period_domain_personalization(request, response);
    enforce_premium_period_advice_synthesis(request, response);
    ensure_period_response_minimum_words(request, response);
    dedupe_period_daily_timeline_texts(request, response);
    enforce_period_overview_personalization(response);
    enforce_period_domain_personalization(request, response);
    enforce_premium_period_advice_synthesis(request, response);
}

#[doc(hidden)]
pub fn prune_period_response_variant_fields(request: &Value, response: &mut Value) {
    let service_code = request["service_code"]
        .as_str()
        .unwrap_or(HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE);
    if is_free_period_service(service_code) {
        response.as_object_mut().map(|map| {
            map.remove("week_overview");
            map.remove("best_days");
            map.remove("watch_days");
            map.remove("daily_timeline");
            map.remove("domain_sections");
            map.remove("best_windows");
            map.remove("watch_windows");
            map.remove("strategy");
        });
        return;
    }
    response["watch_summary"] = sanitize_period_watch_summary(
        response.get("watch_summary"),
        &request["watch_summary_plan"],
    );
    response.as_object_mut().map(|map| {
        map.remove("summary");
        map.remove("dominant_theme");
    });
    if !is_premium_period_service(service_code) {
        response.as_object_mut().map(|map| {
            map.remove("best_windows");
            map.remove("watch_windows");
            map.remove("strategy");
        });
    }
}

fn enforce_period_overview_personalization(response: &mut Value) {
    let text = response
        .pointer("/week_overview/text")
        .and_then(Value::as_str)
        .unwrap_or("");
    let trajectory = response
        .pointer("/week_overview/trajectory")
        .and_then(Value::as_str)
        .unwrap_or("");
    if period_text_has_personalization(&format!("{text} {trajectory}")) {
        return;
    }
    let addition = "Les priorités de la semaine prennent appui sur vos repères personnels pour relier les jours clés à une trajectoire plus lisible.";
    response["week_overview"]["text"] = json!(sanitize_period_public_string(&format!(
        "{} {}",
        text.trim(),
        addition
    )));
}

fn enforce_period_domain_personalization(request: &Value, response: &mut Value) {
    let fallback_sections = request["domain_sections"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let fallback_by_domain = fallback_sections
        .iter()
        .filter_map(|section| {
            Some((
                section.get("domain")?.as_str()?.to_string(),
                section.clone(),
            ))
        })
        .collect::<HashMap<_, _>>();
    let Some(sections) = response
        .get_mut("domain_sections")
        .and_then(Value::as_array_mut)
    else {
        return;
    };
    for (index, section) in sections.iter_mut().enumerate() {
        let text = section.get("text").and_then(Value::as_str).unwrap_or("");
        if period_text_has_personalization(text) {
            continue;
        }
        let fallback = section
            .get("domain")
            .and_then(Value::as_str)
            .and_then(|domain| fallback_by_domain.get(domain))
            .or_else(|| fallback_sections.get(index))
            .unwrap_or(section);
        let addition = period_public_domain_interpretive_sentence(fallback);
        section["text"] = json!(sanitize_period_public_string(&format!(
            "{} {}",
            text.trim(),
            addition
        )));
    }
}

fn enforce_premium_period_advice_synthesis(request: &Value, response: &mut Value) {
    if !is_premium_period_request(request) {
        return;
    }
    let advice_text = [
        response.pointer("/advice/main").and_then(Value::as_str),
        response.pointer("/advice/best_use").and_then(Value::as_str),
        response.pointer("/advice/avoid").and_then(Value::as_str),
        response.pointer("/strategy/text").and_then(Value::as_str),
        response
            .pointer("/strategy/best_use")
            .and_then(Value::as_str),
        response
            .pointer("/strategy/recovery")
            .and_then(Value::as_str),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ");
    if explicit_date_count(&advice_text) == 0 {
        return;
    }
    response["advice"] = json!({
        "main": "Utilisez la timeline pour comprendre le rythme quotidien, puis les fenêtres déjà listées pour choisir quand agir ou ralentir.",
        "best_use": "Réserver les fenêtres favorables déjà listées aux échanges utiles, aux décisions courtes et aux actions concrètes.",
        "avoid": "Transformer les repères datés en nouveau calendrier de consignes."
    });
    response["strategy"] = sanitize_period_strategy(None, request);
}

fn repair_free_period_response_shape(request: &Value, response: &mut Value) {
    response["summary"] = sanitize_free_period_summary(response.get("summary"));
    response["dominant_theme"] =
        sanitize_free_period_dominant_theme(response.get("dominant_theme"), request);
    response["key_days"] = sanitize_period_markers(response.get("key_days"), &request["key_days"]);
    if response["key_days"].as_array().map(Vec::len).unwrap_or(0) == 0 {
        let first = request["evidence"]
            .as_array()
            .and_then(|items| items.first());
        response["key_days"] = json!([{
            "date": first.and_then(|item| item["date"].as_str()).unwrap_or("2026-06-07"),
            "title": "Jour à retenir",
            "reason": "Un repère utile ressort pour organiser la semaine sans détailler chaque journée.",
            "evidence_keys": first.and_then(|item| item["evidence_key"].as_str()).map(|key| vec![key]).unwrap_or_default(),
            "fallback_reason": null
        }]);
    }
    if response["key_days"].as_array().map(Vec::len).unwrap_or(0) > 2 {
        response["key_days"] = json!(response["key_days"]
            .as_array()
            .unwrap()
            .iter()
            .take(2)
            .cloned()
            .collect::<Vec<_>>());
    }
    if let Some(days) = response["key_days"].as_array_mut() {
        for day in days {
            day["title"] = json!("Jour à retenir");
        }
    }
    response["advice"] = json!(sanitize_period_public_string(
        response
            .get("advice")
            .and_then(Value::as_str)
            .unwrap_or("Choisissez une priorité simple et gardez une marge d'ajustement.")
    ));
    response["watch_summary"] =
        sanitize_free_period_watch_summary(response.get("watch_summary"), request);
    response["evidence_summary"] =
        sanitize_period_evidence_summary(response.get("evidence_summary"), request);
    if response["evidence_summary"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0)
        > 3
    {
        response["evidence_summary"] = json!(response["evidence_summary"]
            .as_array()
            .unwrap()
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>());
    }
    response.as_object_mut().map(|map| {
        for field in [
            "week_overview",
            "best_days",
            "watch_days",
            "daily_timeline",
            "domain_sections",
            "best_windows",
            "watch_windows",
            "strategy",
        ] {
            map.remove(field);
        }
    });
    let provider = response["quality"]["provider"]
        .as_str()
        .unwrap_or("openai")
        .to_string();
    let model = response["quality"]["model"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let fallback_used = response["quality"]["fallback_used"]
        .as_bool()
        .unwrap_or(false);
    response["quality"] = json!({
        "daily_timeline_count": 0,
        "evidence_guard_passed": true,
        "best_watch_overlap_passed": true,
        "provider": provider,
        "model": model,
        "fallback_used": fallback_used,
        "period_contract": "free_next_7_days"
    });
}

fn sanitize_free_period_summary(value: Option<&Value>) -> Value {
    json!({
        "title": sanitize_period_public_string(value.and_then(|v| v.get("title")).and_then(Value::as_str).unwrap_or("Vos 7 prochains jours")),
        "text": sanitize_period_public_string(value.and_then(|v| v.get("text")).and_then(Value::as_str).unwrap_or("Les prochains jours donnent une tendance à comprendre plutôt qu'un planning détaillé. Repérez le thème qui revient, choisissez une priorité simple et laissez de la place pour ajuster votre rythme sans chercher à tout décider maintenant."))
    })
}

fn sanitize_free_period_dominant_theme(value: Option<&Value>, request: &Value) -> Value {
    let fallback_theme = request["week_overview_plan"]["dominant_theme"]
        .as_str()
        .map(period_theme_public_label)
        .unwrap_or("organisation");
    json!({
        "theme": sanitize_period_public_string(value.and_then(|v| v.get("theme")).and_then(Value::as_str).unwrap_or(fallback_theme)),
        "text": sanitize_period_public_string(value.and_then(|v| v.get("text")).and_then(Value::as_str).unwrap_or("Ce thème donne le relief principal de la semaine et aide à choisir une action concrète sans ouvrir trop de sujets."))
    })
}

fn sanitize_free_period_watch_summary(value: Option<&Value>, request: &Value) -> Value {
    let allowed_keys = request["evidence"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item["evidence_key"].as_str())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let allowed = allowed_keys
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let generated_status = value
        .and_then(|item| item.get("status"))
        .and_then(Value::as_str)
        .filter(|status| matches!(*status, "none" | "low" | "present"));
    let status = generated_status.unwrap_or("none");
    let fallback_text = if status == "none" {
        FREE_PERIOD_NONE_WATCH_SUMMARY
    } else {
        "Une vigilance légère suffit : ralentir si une réaction paraît plus forte que la situation."
    };
    let text = sanitize_period_public_string(
        value
            .and_then(|item| item.get("text"))
            .and_then(Value::as_str)
            .filter(|text| !text.trim().is_empty())
            .unwrap_or(fallback_text),
    );
    let mut evidence_keys = value
        .and_then(|item| item.get("evidence_keys"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|key| allowed.contains(*key))
        .map(|key| json!(key))
        .collect::<Vec<_>>();
    if status != "none" && evidence_keys.is_empty() {
        if let Some(first) = allowed_keys.first() {
            evidence_keys.push(json!(first));
        }
    }
    if status == "none" {
        evidence_keys.clear();
    }
    json!({
        "status": status,
        "text": text,
        "evidence_keys": evidence_keys
    })
}

fn is_premium_period_service(service_code: &str) -> bool {
    service_code == HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE
}

fn is_free_period_service(service_code: &str) -> bool {
    service_code == HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE
}

fn sanitize_period_week_overview(value: Option<&Value>) -> Value {
    let text = value.and_then(|v| v.get("text")).and_then(Value::as_str).unwrap_or("La période se lit comme une progression continue, avec des jours d'appui, des ajustements concrets et une consolidation finale.");
    let trajectory = value
        .and_then(|v| v.get("trajectory"))
        .and_then(Value::as_str)
        .unwrap_or("Clarifier, ajuster, puis consolider.");
    json!({
        "title": sanitize_period_public_string(value.and_then(|v| v.get("title")).and_then(Value::as_str).unwrap_or("Vue d'ensemble")),
        "text": sanitize_period_public_string(&ensure_period_explicit_personalization_text(text, "Les priorités de la semaine prennent appui sur vos repères personnels sans perdre leur rythme concret.")),
        "trajectory": sanitize_period_public_string(&ensure_period_explicit_personalization_text(trajectory, "Le mouvement relie vos repères personnels, les appuis émotionnels et les choix à consolider."))
    })
}

fn sanitize_period_advice(value: Option<&Value>) -> Value {
    json!({
        "main": sanitize_period_public_string(value.and_then(|v| v.get("main")).and_then(Value::as_str).unwrap_or("Gardez une progression simple et reliez les décisions d'un jour à l'autre.")),
        "best_use": sanitize_period_public_string(value.and_then(|v| v.get("best_use")).and_then(Value::as_str).unwrap_or("Utiliser les appuis de la semaine pour organiser, dialoguer et consolider.")),
        "avoid": sanitize_period_public_string(value.and_then(|v| v.get("avoid")).and_then(Value::as_str).unwrap_or("Éviter de transformer un signal quotidien en certitude définitive."))
    })
}

fn sanitize_period_watch_summary(value: Option<&Value>, fallback: &Value) -> Value {
    let status = fallback
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let fallback_text = fallback
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or(FREE_PERIOD_NONE_WATCH_SUMMARY);
    json!({
        "status": status,
        "text": sanitize_period_public_string(value
            .and_then(|item| item.get("text"))
            .and_then(Value::as_str)
            .filter(|text| !text.trim().is_empty())
            .unwrap_or(fallback_text)),
        "evidence_keys": string_array_value(fallback.get("evidence_keys")).unwrap_or_else(|| json!([]))
    })
}

fn sanitize_period_markers(value: Option<&Value>, fallback: &Value) -> Value {
    let generated_items = value.and_then(Value::as_array).cloned().unwrap_or_default();
    let generated_by_date = value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| Some((item.get("date")?.as_str()?.to_string(), item.clone())))
        .collect::<HashMap<_, _>>();
    let source = fallback.as_array().cloned().unwrap_or_else(Vec::new);
    Value::Array(
        source
            .into_iter()
            .enumerate()
            .map(|(index, fallback_item)| {
                let date = fallback_item
                    .get("date")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let generated_item = generated_by_date
                    .get(date)
                    .or_else(|| generated_items.get(index));
                let fallback_reason = generated_item
                    .and_then(|item| item.get("fallback_reason"))
                    .filter(|value| !value.is_null())
                    .and_then(Value::as_str)
                    .filter(|reason| !reason.trim().is_empty())
                    .or_else(|| {
                        fallback_item
                            .get("fallback_reason")
                            .filter(|value| !value.is_null())
                            .and_then(Value::as_str)
                            .filter(|reason| !reason.trim().is_empty())
                    })
                    .map_or(Value::Null, |reason| json!(reason));
                json!({
                    "date": fallback_item["date"],
                    "title": sanitize_period_public_string(
                        generated_item
                            .and_then(|item| item.get("title"))
                            .and_then(Value::as_str)
                            .or_else(|| fallback_item.get("title").and_then(Value::as_str))
                            .unwrap_or("Jour")
                    ),
                    "reason": sanitize_period_public_string(
                        generated_item
                            .and_then(|item| item.get("reason"))
                            .and_then(Value::as_str)
                            .filter(|reason| !reason.trim().is_empty())
                            .or_else(|| fallback_item.get("reason").and_then(Value::as_str))
                            .unwrap_or("Ce jour donne un repère à retenir pour ajuster une priorité de la période.")
                    ),
                    "evidence_keys": non_empty_string_array_value(fallback_item.get("evidence_keys")).unwrap_or_else(|| json!([])),
                    "fallback_reason": fallback_reason
                })
            })
            .collect(),
    )
}

fn sanitize_period_daily_timeline(value: Option<&Value>, request: &Value) -> Value {
    let by_date = value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|day| Some((day.get("date")?.as_str()?.to_string(), day.clone())))
        .collect::<HashMap<_, _>>();
    let days = request["daily_plans"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|plan| {
            let date = plan.get("date").and_then(Value::as_str).unwrap_or("");
            let generated = by_date.get(date);
            let theme = plan
                .get("theme_label")
                .and_then(Value::as_str)
                .unwrap_or("priorité");
            let fallback_text = period_public_day_text(plan, 0);
            let fallback_advice = period_public_day_advice(plan);
            json!({
                "date": date,
                "day_label": sanitize_period_public_string(generated.and_then(|day| day.get("day_label")).and_then(Value::as_str).or_else(|| plan.get("day_label").and_then(Value::as_str)).unwrap_or("Jour")),
                "theme": sanitize_period_public_string(theme),
                "tone": generated.and_then(|day| day.get("tone")).and_then(Value::as_str).unwrap_or("concentré"),
                "text": sanitize_period_public_string(&generated.and_then(|day| day.get("text")).and_then(Value::as_str).map(|text| ensure_period_personalization_text(text, &period_public_interpretive_sentence(plan))).unwrap_or(fallback_text)),
                "advice": sanitize_period_public_string(generated.and_then(|day| day.get("advice")).and_then(Value::as_str).unwrap_or(&fallback_advice)),
                "evidence_keys": string_array_value(plan.get("evidence_keys")).unwrap_or_else(|| json!([]))
            })
        })
        .collect::<Vec<_>>();
    Value::Array(days)
}

fn sanitize_period_domain_sections(value: Option<&Value>, request: &Value) -> Value {
    let generated = value.and_then(Value::as_array).cloned().unwrap_or_default();
    let fallback_sections = request["domain_sections"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let fallback_by_domain = fallback_sections
        .iter()
        .filter_map(|section| {
            let domain = section.get("domain")?.as_str()?.to_string();
            Some((domain, section.clone()))
        })
        .collect::<HashMap<_, _>>();
    let fallback_by_title = fallback_sections
        .iter()
        .filter_map(|section| {
            let title = normalized_text(section.get("title")?.as_str()?);
            Some((title, section.clone()))
        })
        .collect::<HashMap<_, _>>();
    let fallback = fallback_sections
        .iter()
        .map(|section| {
            json!({
                "domain": section["domain"],
                "title": section["title"],
                "text": period_public_domain_text(section),
                "evidence_keys": section["evidence_keys"]
            })
        })
        .collect::<Vec<_>>();
    let source = if generated.is_empty() {
        fallback
    } else {
        generated
    };
    Value::Array(
        source
            .into_iter()
            .enumerate()
            .map(|(index, section)| {
                let fallback = section
                    .get("domain")
                    .and_then(Value::as_str)
                    .and_then(|domain| fallback_by_domain.get(domain))
                    .or_else(|| {
                        section
                            .get("title")
                            .and_then(Value::as_str)
                            .and_then(|title| fallback_by_title.get(&normalized_text(title)))
                    })
                    .or_else(|| fallback_sections.get(index));
                json!({
                    "domain": sanitize_period_public_string(
                        fallback
                            .and_then(|item| item.get("domain"))
                            .and_then(Value::as_str)
                            .or_else(|| section.get("domain").and_then(Value::as_str))
                            .unwrap_or("organisation")
                    ),
                    "title": sanitize_period_public_string(
                        section
                            .get("title")
                            .and_then(Value::as_str)
                            .or_else(|| fallback.and_then(|item| item.get("title")).and_then(Value::as_str))
                            .unwrap_or("Organisation")
                    ),
                    "text": sanitize_period_public_string(
                        &section
                            .get("text")
                            .and_then(Value::as_str)
                            .map(|text| ensure_period_explicit_personalization_text(text, &period_public_domain_interpretive_sentence(fallback.unwrap_or(&section))))
                            .unwrap_or_else(|| fallback.map(period_public_domain_text).unwrap_or_else(|| period_public_domain_text(&section)))
                    ),
                    "evidence_keys": fallback
                        .and_then(|fallback| non_empty_string_array_value(fallback.get("evidence_keys")))
                        .unwrap_or_else(|| json!([]))
                })
            })
            .collect(),
    )
}

fn sanitize_period_windows(value: Option<&Value>, request: &Value, field: &str) -> Value {
    let allowed = request[field].as_array().cloned().unwrap_or_default();
    let allowed_by_key = allowed
        .iter()
        .filter_map(|window| {
            let key = period_window_identity(window)?;
            Some((key, window.clone()))
        })
        .collect::<HashMap<_, _>>();
    let generated = value.and_then(Value::as_array).cloned().unwrap_or_default();
    let mut out = Vec::new();
    for window in generated {
        let Some(identity) = period_window_identity(&window) else {
            continue;
        };
        let Some(fallback) = allowed_by_key.get(&identity) else {
            continue;
        };
        out.push(sanitize_period_window_from_fallback(
            &window, fallback, field,
        ));
    }
    Value::Array(out)
}

fn period_window_identity(window: &Value) -> Option<String> {
    let date = window.get("date")?.as_str()?;
    let keys = window
        .get("source_snapshot_keys")?
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join("|");
    if keys.is_empty() {
        return None;
    }
    Some(format!("{date}:{keys}"))
}

fn sanitize_period_window_from_fallback(window: &Value, fallback: &Value, field: &str) -> Value {
    let mut out = json!({
        "date": fallback["date"],
        "time_range_label": sanitize_period_public_string(window.get("time_range_label").and_then(Value::as_str).or_else(|| fallback.get("time_range_label").and_then(Value::as_str)).unwrap_or("")),
        "source_snapshot_keys": fallback["source_snapshot_keys"],
        "title": sanitize_period_public_string(window.get("title").and_then(Value::as_str).or_else(|| fallback.get("title").and_then(Value::as_str)).unwrap_or("Fenêtre")),
        "theme": sanitize_period_public_string(window.get("theme").and_then(Value::as_str).or_else(|| fallback.get("theme").and_then(Value::as_str)).unwrap_or("priorité")),
        "tone": sanitize_period_public_string(window.get("tone").and_then(Value::as_str).or_else(|| fallback.get("tone").and_then(Value::as_str)).unwrap_or("nuancé")),
        "evidence_keys": fallback["evidence_keys"]
    });
    if field == "best_windows" {
        out["reason"] = json!(sanitize_period_public_string(
            window
                .get("reason")
                .and_then(Value::as_str)
                .or_else(|| fallback.get("reason").and_then(Value::as_str))
                .unwrap_or("Ce créneau aide à poser une action simple et vérifiable.")
        ));
        out["best_for"] = fallback["best_for"].clone();
    } else {
        out["watch_point"] = json!(sanitize_period_public_string(
            window
                .get("watch_point")
                .and_then(Value::as_str)
                .or_else(|| fallback.get("watch_point").and_then(Value::as_str))
                .unwrap_or("Garder une marge avant de répondre.")
        ));
    }
    out
}

fn sanitize_period_strategy(value: Option<&Value>, request: &Value) -> Value {
    let fallback = &request["strategy"];
    json!({
        "title": sanitize_period_public_string(value.and_then(|v| v.get("title")).and_then(Value::as_str).or_else(|| fallback.get("title").and_then(Value::as_str)).unwrap_or("Stratégie de semaine")),
        "text": sanitize_period_public_string(value.and_then(|v| v.get("text")).and_then(Value::as_str).unwrap_or("Alterner les fenêtres favorables pour agir, les moments de vigilance pour ralentir et les temps d'intégration pour consolider les choix.")),
        "best_use": sanitize_period_public_string(value.and_then(|v| v.get("best_use")).and_then(Value::as_str).or_else(|| fallback.get("best_use").and_then(Value::as_str)).unwrap_or("Utiliser les appuis pour décider et communiquer simplement.")),
        "recovery": sanitize_period_public_string(value.and_then(|v| v.get("recovery")).and_then(Value::as_str).or_else(|| fallback.get("recovery").and_then(Value::as_str)).unwrap_or("Préserver un temps de recul après les moments plus réactifs.")),
        "evidence_keys": string_array_value(fallback.get("evidence_keys")).unwrap_or_else(|| json!([]))
    })
}

fn ensure_period_personalization_text(text: &str, personalization: &str) -> String {
    let base = sanitize_period_public_string(text);
    if period_text_has_personalization(&base) {
        base
    } else {
        format!("{base} {personalization}")
    }
}

fn ensure_period_explicit_personalization_text(text: &str, personalization: &str) -> String {
    let base = sanitize_period_public_string(text);
    if period_text_has_explicit_personal_anchor(&base) {
        base
    } else {
        format!("{base} {personalization}")
    }
}

fn period_text_has_explicit_personal_anchor(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("repères personnels")
        || lower.contains("repère personnel")
        || lower.contains("repere personnel")
}

fn period_public_day_text(day: &Value, index: usize) -> String {
    let day_label = day
        .get("day_label")
        .and_then(Value::as_str)
        .unwrap_or("Ce jour");
    let theme = day
        .get("theme_label")
        .and_then(Value::as_str)
        .or_else(|| day.get("theme").and_then(Value::as_str))
        .unwrap_or("priorité");
    let focus = period_public_focus_text(day);
    match period_style_code(day) {
        "relation" => format!(
            "{day_label} adoucit le thème {theme} en repartant de {focus}. Une attente ou une parole simple peut détendre l'échange sans chercher un accord de façade."
        ),
        "action" => format!(
            "{day_label} donne du relief au thème {theme}. En partant de {focus}, une action courte vaut mieux qu'une série de réponses dispersées."
        ),
        "clarity" => format!(
            "{day_label} aide à nommer ce qui compte dans le thème {theme}. Avec {focus}, le tri devient plus simple et les choix gagnent en lisibilité."
        ),
        "communication" => format!(
            "{day_label} remet le thème {theme} dans les mots justes. En partant de {focus}, une formulation directe peut éviter plusieurs malentendus."
        ),
        "integration" => format!(
            "{day_label} invite à relier le thème {theme} à ce qui a déjà été compris. En partant de {focus}, il devient plus simple de consolider sans ouvrir trop de nouveaux fronts."
        ),
        _ => match index {
            0 => format!(
                "{day_label} ouvre la période sur le thème {theme}. À travers {focus}, il s'agit surtout de remettre de l'ordre dans ce qui circule déjà, sans tout contrôler."
            ),
            5 => format!(
                "{day_label} ramène le thème {theme} vers une priorité réaliste. En partant de {focus}, il devient plus facile de choisir ce qui mérite d'être tenu jusqu'au bout."
            ),
            _ => format!(
                "{day_label} recentre le thème {theme}. Avec {focus}, le plus utile consiste à poser un repère clair avant d'élargir le mouvement."
            ),
        },
    }
}

fn period_public_day_advice(day: &Value) -> String {
    let focus = period_public_focus_text(day);
    match period_style_code(day) {
        "relation" => format!("Privilégiez un geste relationnel simple autour de {focus}, sans chercher à traiter tous les sujets."),
        "action" => format!("Transformez cette priorité en une action vérifiable, puis laissez le reste en attente."),
        "clarity" => format!("Nommez ce qui compte vraiment pour {focus}, même si la décision reste progressive."),
        "communication" => format!("Formulez une demande courte et vérifiable, puis écoutez la réponse sans surinterpréter."),
        "integration" => format!("Reliez ce travail d'intégration à une habitude déjà solide et consolidez-la avant d'ajouter autre chose."),
        _ => format!("Posez une priorité claire liée à {focus}, puis avancez par un geste mesuré."),
    }
}

fn period_daily_advice_expansion(index: usize) -> &'static str {
    match index % 7 {
        0 => "Gardez un geste simple et retenez une suite concrète.",
        1 => "Avancez par une décision courte, puis laissez le rythme se stabiliser.",
        2 => "Choisissez un repère utile et vérifiez-le avant d'élargir l'action.",
        3 => "Préservez une marge de recul avant de répondre trop vite.",
        4 => "Transformez l'élan du jour en action mesurable et limitée.",
        5 => "Revenez à ce qui peut vraiment être tenu jusqu'au lendemain.",
        _ => "Laissez la journée fermer une étape avant d'en ouvrir une autre.",
    }
}

fn period_public_domain_text(section: &Value) -> String {
    let domain = section
        .get("title")
        .and_then(Value::as_str)
        .or_else(|| section.get("domain").and_then(Value::as_str))
        .unwrap_or("Ce domaine");
    let focus = period_public_focus_text(section);
    format!(
        "{domain} devient un terrain concret cette semaine. Avec vos repères personnels liés à {focus}, la bonne échelle consiste à choisir une priorité lisible, agir sans raideur et garder le fil entre les journées."
    )
}

fn period_public_personalization_sentence(item: &Value) -> String {
    period_public_interpretive_sentence(item)
}

fn period_public_interpretive_sentence(item: &Value) -> String {
    let focus = period_public_focus_text(item);
    format!("Avec {focus}, la journée gagne un repère personnel concret sans devenir une explication abstraite.")
}

fn period_public_domain_personalization_sentence(item: &Value) -> String {
    period_public_domain_interpretive_sentence(item)
}

fn period_public_domain_interpretive_sentence(item: &Value) -> String {
    let focus = period_public_focus_text(item);
    format!(
        "Dans ce domaine, vos repères personnels liés à {focus} aident à choisir le bon niveau d'engagement."
    )
}

fn period_style_code(item: &Value) -> &str {
    item.get("style_variant_code")
        .and_then(Value::as_str)
        .unwrap_or_else(|| match item.get("theme_code").and_then(Value::as_str) {
            Some("relationship") => "relation",
            Some("energy") => "action",
            Some("clarity") => "clarity",
            Some("communication") => "communication",
            Some("integration") => "integration",
            _ => "anchor",
        })
}

fn period_public_focus_text(item: &Value) -> String {
    for key in [
        "personalization_hint",
        "natal_focus_label",
        "natal_focus_hint",
    ] {
        if let Some(raw) = item.get(key).and_then(Value::as_str) {
            let cleaned = period_public_focus_from_hint(raw);
            if !cleaned.trim().is_empty() {
                return cleaned;
            }
        }
    }
    "un repère personnel important".to_string()
}

fn period_public_focus_from_hint(raw: &str) -> String {
    let mut text = raw.trim().to_string();
    for prefix in [
        "Personnaliser ce signal par ",
        "Personnaliser ce signal avec ",
        "Relier ce signal à ",
        "Relier ce signal aux ",
        "Relier ce signal au ",
        "Relier ce domaine à ",
    ] {
        if let Some(rest) = text.strip_prefix(prefix) {
            text = rest.to_string();
            break;
        }
    }
    for suffix in [
        " plutôt que rester sur un conseil générique.",
        " plutôt que rester sur un conseil générique",
        ", sans jargon technique.",
        " sans jargon technique.",
    ] {
        if let Some(rest) = text.strip_suffix(suffix) {
            text = rest.to_string();
        }
    }
    text
}

fn sanitize_period_public_string(text: &str) -> String {
    reprocess_horoscope_period("fr", json!(text), None)
        .payload
        .as_str()
        .unwrap_or(text)
        .to_string()
}

fn sanitize_period_evidence_summary(value: Option<&Value>, request: &Value) -> Value {
    let generated = value.and_then(Value::as_array).cloned().unwrap_or_default();
    let fallback_items = request["evidence"]
        .as_array()
        .into_iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    let fallback_by_key = fallback_items
        .iter()
        .filter_map(|item| Some((item.get("evidence_key")?.as_str()?.to_string(), item)))
        .collect::<HashMap<_, _>>();
    let fallback_by_date = fallback_items
        .iter()
        .filter_map(|item| Some((item.get("date")?.as_str()?.to_string(), item)))
        .collect::<HashMap<_, _>>();
    let generated_by_key = generated
        .iter()
        .filter_map(|item| Some((item.get("evidence_key")?.as_str()?.to_string(), item)))
        .collect::<HashMap<_, _>>();
    let generated_by_date = generated
        .iter()
        .filter_map(|item| Some((item.get("date")?.as_str()?.to_string(), item)))
        .collect::<HashMap<_, _>>();
    let source = if generated.is_empty() {
        fallback_items.iter().take(3).collect::<Vec<_>>()
    } else {
        generated
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                item.get("evidence_key")
                    .and_then(Value::as_str)
                    .and_then(|key| fallback_by_key.get(key).copied())
                    .or_else(|| {
                        item.get("date")
                            .and_then(Value::as_str)
                            .and_then(|date| fallback_by_date.get(date).copied())
                    })
                    .or_else(|| fallback_items.get(index))
            })
            .collect::<Vec<_>>()
    };
    Value::Array(
        source
            .into_iter()
            .enumerate()
            .map(|(index, fallback)| {
                let key = fallback
                    .get("evidence_key")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let date = fallback.get("date").and_then(Value::as_str).unwrap_or("");
                let generated_item = generated
                    .get(index)
                    .or_else(|| generated_by_key.get(key).copied())
                    .or_else(|| generated_by_date.get(date).copied());
                json!({
                    "date": fallback["date"],
                    "evidence_key": fallback["evidence_key"],
                    "label": sanitize_period_public_string(
                        generated_item
                            .and_then(|item| item.get("label"))
                            .and_then(Value::as_str)
                            .filter(|label| !label.trim().is_empty())
                            .or_else(|| fallback.get("human_label").and_then(Value::as_str))
                            .unwrap_or("Repère de période")
                    )
                })
            })
            .collect(),
    )
}

fn ensure_period_response_minimum_words(request: &Value, response: &mut Value) {
    let limits = period_word_limits_for_request(request);
    trim_period_response_to_hard_limit(request, response, &limits);
    let current_words = period_public_word_count(response);
    if current_words >= limits.target_min && current_words <= limits.hard_limit {
        return;
    }
    if current_words > limits.hard_limit {
        trim_period_response_aggressively(request, response);
        let compact_words = period_public_word_count(response);
        if compact_words >= limits.target_min && compact_words <= limits.hard_limit {
            return;
        }
        if compact_words > limits.hard_limit {
            return;
        }
    }

    if let Some(text) = response.pointer_mut("/week_overview/text") {
        append_period_value_sentence(
            text,
            "La semaine gagne en cohérence quand chaque décision reste reliée à vos repères personnels et au rythme déjà engagé.",
        );
    }
    if period_public_word_count(response) >= limits.target_min {
        return;
    }

    let plans = request["daily_plans"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let day_count = response["daily_timeline"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0);
    for index in 0..day_count {
        {
            let day = &mut response["daily_timeline"][index];
            let date = day.get("date").and_then(Value::as_str).unwrap_or("");
            let plan = plans
                .iter()
                .find(|plan| plan.get("date").and_then(Value::as_str) == Some(date));
            if let Some(plan) = plan {
                if let Some(text) = day.get_mut("text") {
                    append_period_value_sentence(
                        text,
                        &period_public_personalization_sentence(plan),
                    );
                }
                if let Some(advice) = day.get_mut("advice") {
                    append_period_value_sentence(advice, period_daily_advice_expansion(index));
                }
            }
        }
        if period_public_word_count(response) >= limits.target_min {
            return;
        }
    }
    if period_public_word_count(response) >= limits.target_min {
        return;
    }

    let sections = request["domain_sections"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let section_count = response["domain_sections"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0);
    for index in 0..section_count {
        {
            let section = &mut response["domain_sections"][index];
            let domain = section.get("domain").and_then(Value::as_str).unwrap_or("");
            let plan = sections
                .iter()
                .find(|plan| plan.get("domain").and_then(Value::as_str) == Some(domain));
            if let Some(plan) = plan {
                if let Some(text) = section.get_mut("text") {
                    append_period_value_sentence(
                        text,
                        &period_public_domain_personalization_sentence(plan),
                    );
                }
            }
        }
        if period_public_word_count(response) >= limits.target_min {
            return;
        }
    }
    if period_public_word_count(response) >= limits.target_min {
        return;
    }

    if let Some(main) = response.pointer_mut("/advice/main") {
        append_period_value_sentence(
            main,
            "Utilisez ces repères comme une synthèse personnelle de période, pas comme une liste de journées isolées.",
        );
    }
    fill_period_response_to_minimum(request, response, &limits);
    if period_public_word_count(response) > limits.hard_limit {
        trim_period_response_to_hard_limit(request, response, &limits);
    }
    if period_public_word_count(response) > limits.hard_limit {
        trim_period_response_aggressively(request, response);
    }
}

fn trim_period_response_to_hard_limit(
    request: &Value,
    response: &mut Value,
    limits: &PeriodWordLimits,
) {
    if period_public_word_count(response) <= limits.hard_limit {
        return;
    }

    response["week_overview"] = json!({
        "title": "Vos 7 prochains jours",
        "text": "Vos 7 prochains jours avancent par étapes : remettre de l'ordre, retrouver un appui plus simple, puis consolider ce qui devient clair dans vos repères personnels.",
        "trajectory": "Le mouvement va des appuis initiaux vers une consolidation plus consciente."
    });
    response["advice"] = json!({
        "main": "Avancez par étapes et gardez une priorité concrète par journée.",
        "best_use": "Utiliser les jours favorables pour poser un geste clair et personnel.",
        "avoid": "Transformer un signal de période en certitude rigide."
    });

    let plans = request["daily_plans"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if let Some(days) = response["daily_timeline"].as_array_mut() {
        for day in days {
            let date = day.get("date").and_then(Value::as_str).unwrap_or("");
            let plan = plans
                .iter()
                .find(|plan| plan.get("date").and_then(Value::as_str) == Some(date));
            if let Some(plan) = plan {
                day["text"] = json!(sanitize_period_public_string(&compact_period_words(
                    &period_public_day_text(plan, 0),
                    42,
                )));
                day["advice"] = json!(sanitize_period_public_string(&compact_period_words(
                    &period_public_day_advice(plan),
                    24,
                )));
            }
        }
    }

    let sections = request["domain_sections"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if let Some(domain_sections) = response["domain_sections"].as_array_mut() {
        if domain_sections.len() > 3 {
            domain_sections.truncate(3);
        }
        for section in domain_sections {
            let domain = section.get("domain").and_then(Value::as_str).unwrap_or("");
            let plan = sections
                .iter()
                .find(|plan| plan.get("domain").and_then(Value::as_str) == Some(domain));
            if let Some(plan) = plan {
                section["text"] = json!(sanitize_period_public_string(&compact_period_words(
                    &period_public_domain_text(plan),
                    46,
                )));
            }
        }
    }

    if response["evidence_summary"]
        .as_array()
        .map(|items| items.len() > 4)
        .unwrap_or(false)
    {
        if let Some(items) = response["evidence_summary"].as_array_mut() {
            items.truncate(4);
        }
    }
}

fn trim_period_response_aggressively(request: &Value, response: &mut Value) {
    response["week_overview"] = json!({
        "title": "Vos 7 prochains jours",
        "text": "La semaine avance en reliant les échanges, les choix concrets et vos repères personnels.",
        "trajectory": "La période progresse vers des choix plus posés et personnels."
    });
    response["advice"] = json!({
        "main": "Avancez par étapes, avec une priorité concrète à la fois.",
        "best_use": "Choisir un geste utile sur les jours favorables.",
        "avoid": "Forcer une conclusion trop rapide."
    });

    let plans = request["daily_plans"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if let Some(days) = response["daily_timeline"].as_array_mut() {
        for day in days {
            let date = day.get("date").and_then(Value::as_str).unwrap_or("");
            if let Some(plan) = plans
                .iter()
                .find(|plan| plan.get("date").and_then(Value::as_str) == Some(date))
            {
                day["text"] = json!(sanitize_period_public_string(&compact_period_words(
                    &period_public_day_text(plan, 0),
                    30,
                )));
                day["advice"] = json!(sanitize_period_public_string(&compact_period_words(
                    &period_public_day_advice(plan),
                    14,
                )));
            }
        }
    }

    let sections = request["domain_sections"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if let Some(domain_sections) = response["domain_sections"].as_array_mut() {
        if domain_sections.len() > 2 {
            domain_sections.truncate(2);
        }
        for section in domain_sections {
            let domain = section.get("domain").and_then(Value::as_str).unwrap_or("");
            if let Some(plan) = sections
                .iter()
                .find(|plan| plan.get("domain").and_then(Value::as_str) == Some(domain))
            {
                section["text"] = json!(sanitize_period_public_string(&compact_period_words(
                    &period_public_domain_text(plan),
                    34,
                )));
            }
        }
    }

    for field in ["key_days", "best_days", "watch_days"] {
        if let Some(markers) = response[field].as_array_mut() {
            for marker in markers {
                if let Some(reason) = marker.get("reason").and_then(Value::as_str) {
                    marker["reason"] = json!(sanitize_period_public_string(&compact_period_words(
                        reason, 14,
                    )));
                }
            }
        }
    }

    if let Some(items) = response["evidence_summary"].as_array_mut() {
        if items.len() > 2 {
            items.truncate(2);
        }
        for item in items {
            if let Some(label) = item.get("label").and_then(Value::as_str) {
                item["label"] = json!(sanitize_period_public_string(&compact_period_words(
                    label, 18,
                )));
            }
        }
    }
}

fn fill_period_response_to_minimum(
    request: &Value,
    response: &mut Value,
    limits: &PeriodWordLimits,
) {
    if period_public_word_count(response) >= limits.target_min
        || period_public_word_count(response) > limits.hard_limit
    {
        return;
    }
    let plans = request["daily_plans"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let day_count = response["daily_timeline"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0);
    for index in 0..day_count {
        {
            let day = &mut response["daily_timeline"][index];
            let date = day.get("date").and_then(Value::as_str).unwrap_or("");
            if let Some(plan) = plans
                .iter()
                .find(|plan| plan.get("date").and_then(Value::as_str) == Some(date))
            {
                let theme = plan
                    .get("theme_label")
                    .and_then(Value::as_str)
                    .unwrap_or("ce thème");
                if let Some(text) = day.get_mut("text") {
                    append_period_value_sentence(
                        text,
                        &format!(
                            "Pour {theme}, cette indication précise la façon de choisir un rythme personnel sans isoler la journée du reste de la période."
                        ),
                    );
                    append_period_value_sentence(
                        text,
                        &period_public_personalization_sentence(plan),
                    );
                }
            }
        }
        if period_public_word_count(response) >= limits.target_min {
            return;
        }
    }
}

fn normalize_period_week_overview_repetition(response: &mut Value) {
    let phrase = "thème natal comme fil directeur";
    let week_text = format!(
        "{} {}",
        response["week_overview"]["text"].as_str().unwrap_or(""),
        response["week_overview"]["trajectory"]
            .as_str()
            .unwrap_or("")
    );
    if count_normalized_phrase(&week_text, phrase) <= 1 {
        return;
    }
    for pointer in ["/week_overview/trajectory", "/week_overview/text"] {
        if count_normalized_phrase(
            &format!(
                "{} {}",
                response["week_overview"]["text"].as_str().unwrap_or(""),
                response["week_overview"]["trajectory"]
                    .as_str()
                    .unwrap_or("")
            ),
            phrase,
        ) <= 1
        {
            return;
        }
        if let Some(value) = response
            .pointer(pointer)
            .and_then(Value::as_str)
            .map(str::to_string)
        {
            let normalized = if pointer == "/week_overview/trajectory" {
                replace_period_phrase_all(&value, phrase, "progression personnelle de la semaine")
            } else {
                replace_period_phrase_after_first(
                    &value,
                    phrase,
                    "progression personnelle de la semaine",
                )
            };
            *response.pointer_mut(pointer).unwrap() = json!(normalized);
        }
    }
}

fn normalize_period_repetitive_public_phrases(response: &mut Value) {
    let mut counts = HashMap::<&'static str, usize>::new();
    normalize_period_repetitive_value(response, &mut counts, None);
}

fn dedupe_period_daily_timeline_texts(request: &Value, response: &mut Value) {
    let plan_by_date = request["daily_plans"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|plan| Some((plan.get("date")?.as_str()?.to_string(), plan.clone())))
        .collect::<HashMap<_, _>>();
    let Some(days) = response
        .get_mut("daily_timeline")
        .and_then(Value::as_array_mut)
    else {
        return;
    };

    let mut seen = HashSet::<String>::new();
    for day in days {
        let text = day
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let normalized = normalized_text(&text);
        if normalized.is_empty() || seen.insert(normalized) {
            continue;
        }

        let date = day.get("date").and_then(Value::as_str).unwrap_or("");
        let plan = plan_by_date.get(date).unwrap_or(day);
        let day_label = day
            .get("day_label")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("Ce jour");
        let theme = plan
            .get("theme_label")
            .and_then(Value::as_str)
            .or_else(|| day.get("theme").and_then(Value::as_str))
            .unwrap_or("la priorité du jour");
        let nuance = format!(
            "{} précise ce repère par le thème {}, afin de distinguer cette étape du reste de la semaine.",
            day_label, theme
        );
        day["text"] = json!(sanitize_period_public_string(&format!(
            "{} {}",
            text.trim(),
            nuance
        )));
        seen.insert(normalized_text(day["text"].as_str().unwrap_or("")));
    }
}

fn normalize_period_repetitive_value(
    value: &mut Value,
    counts: &mut HashMap<&'static str, usize>,
    key: Option<&str>,
) {
    match value {
        Value::String(text) => {
            if !period_repetition_normalization_excluded_key(key) {
                *text = normalize_period_repetitive_text(text, counts);
            }
        }
        Value::Array(items) => {
            for item in items {
                normalize_period_repetitive_value(item, counts, key);
            }
        }
        Value::Object(map) => {
            for (child_key, child) in map {
                normalize_period_repetitive_value(child, counts, Some(child_key));
            }
        }
        _ => {}
    }
}

fn period_repetition_normalization_excluded_key(key: Option<&str>) -> bool {
    matches!(
        key,
        Some(
            "contract_version"
                | "service_code"
                | "date"
                | "evidence_key"
                | "evidence_keys"
                | "label"
                | "source_snapshot_keys"
                | "quality"
                | "period_resolution"
                | "provider"
                | "model"
                | "period_contract"
        )
    )
}

fn normalize_period_repetitive_text(
    text: &str,
    counts: &mut HashMap<&'static str, usize>,
) -> String {
    let mut normalized = text.to_string();
    for (phrase, replacements) in period_repetitive_phrase_replacements() {
        normalized = replace_period_phrase_after_allowed(&normalized, phrase, replacements, counts);
    }
    normalized
}

fn replace_period_phrase_after_allowed(
    text: &str,
    phrase: &'static str,
    replacements: &[&'static str],
    counts: &mut HashMap<&'static str, usize>,
) -> String {
    let lower = text.to_lowercase();
    let phrase_lower = phrase.to_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    while let Some(relative) = lower[cursor..].find(&phrase_lower) {
        let start = cursor + relative;
        let end = start + phrase.len();
        out.push_str(&text[cursor..start]);
        let count = counts.entry(phrase).or_insert(0);
        if *count < 2 {
            out.push_str(&text[start..end]);
        } else {
            let replacement = replacements
                .get((*count - 2) % replacements.len())
                .copied()
                .unwrap_or("préciser");
            out.push_str(replacement);
        }
        *count += 1;
        cursor = end;
    }
    out.push_str(&text[cursor..]);
    out
}

fn period_repetitive_phrase_replacements() -> &'static [(&'static str, &'static [&'static str])] {
    &[
        (
            "restez concret",
            &["gardez une prise directe", "revenez au geste utile"],
        ),
        (
            "gardez une marge",
            &["préservez un espace de recul", "laissez une respiration"],
        ),
        ("clarifier", &["rendre lisible", "mettre au net", "nommer"]),
        ("ajuster", &["réaccorder", "moduler", "reprendre"]),
        ("intégrer", &["assimiler", "relier", "consolider"]),
        (
            "met l'accent",
            &["souligne", "fait ressortir", "place l'attention"],
        ),
        (
            "choisissez une seule priorité",
            &[
                "retenez une priorité nette",
                "avancez avec une priorité lisible",
            ],
        ),
        (
            "Hiérarchisez une priorité",
            &[
                "Retenez une priorité nette",
                "Avancez avec une priorité lisible",
                "Gardez un seul axe prioritaire",
            ],
        ),
        (
            "le point d'appui concerne",
            &["l'appui principal touche", "le repère central passe par"],
        ),
        (
            "L'appui personnel vient de",
            &[
                "Le repère personnel passe par",
                "La nuance natale se lit dans",
            ],
        ),
    ]
}

fn replace_period_phrase_all(text: &str, phrase: &str, replacement: &str) -> String {
    let lower = text.to_lowercase();
    let phrase_lower = phrase.to_lowercase();
    let mut out = String::new();
    let mut cursor = 0;
    for (index, _) in lower.match_indices(&phrase_lower) {
        out.push_str(&text[cursor..index]);
        out.push_str(replacement);
        cursor = index + phrase.len();
    }
    out.push_str(&text[cursor..]);
    out
}

fn replace_period_phrase_after_first(text: &str, phrase: &str, replacement: &str) -> String {
    let lower = text.to_lowercase();
    let phrase_lower = phrase.to_lowercase();
    let mut out = String::new();
    let mut cursor = 0;
    let mut seen = false;
    for (index, _) in lower.match_indices(&phrase_lower) {
        out.push_str(&text[cursor..index]);
        let end = index + phrase.len();
        if seen {
            out.push_str(replacement);
        } else {
            out.push_str(&text[index..end]);
            seen = true;
        }
        cursor = end;
    }
    out.push_str(&text[cursor..]);
    out
}

fn compact_period_words(text: &str, max_words: usize) -> String {
    if text.split_whitespace().count() <= max_words {
        return text.to_string();
    }
    let mut out = String::new();
    for sentence in period_complete_sentences(text) {
        let candidate = if out.is_empty() {
            sentence.to_string()
        } else {
            format!("{out} {sentence}")
        };
        if candidate.split_whitespace().count() > max_words {
            break;
        }
        out = candidate;
    }
    if !out.trim().is_empty() {
        return out;
    }
    let compact = text
        .split_whitespace()
        .take(max_words)
        .collect::<Vec<_>>()
        .join(" ");
    period_trim_incomplete_tail(&compact)
}

fn period_complete_sentences(text: &str) -> Vec<&str> {
    let mut sentences = Vec::new();
    let mut start = 0;
    for (index, ch) in text.char_indices() {
        if matches!(ch, '.' | '!' | '?') {
            let end = index + ch.len_utf8();
            let sentence = text[start..end].trim();
            if !sentence.is_empty() {
                sentences.push(sentence);
            }
            start = end;
        }
    }
    sentences
}

fn period_trim_incomplete_tail(text: &str) -> String {
    let mut words = text
        .split_whitespace()
        .map(|word| word.trim_matches(|ch: char| matches!(ch, ',' | ';' | ':')))
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    while words
        .last()
        .map(|word| period_is_weak_sentence_ending(word))
        .unwrap_or(false)
    {
        words.pop();
    }
    let mut compact = words.join(" ");
    compact = compact.trim_end_matches([',', ';', ':']).to_string();
    if !compact.ends_with(['.', '!', '?']) {
        compact.push('.');
    }
    compact
}

fn period_is_weak_sentence_ending(word: &str) -> bool {
    matches!(
        word.trim_matches(|ch: char| !ch.is_alphabetic())
            .to_lowercase()
            .as_str(),
        "et" | "à"
            | "a"
            | "de"
            | "pour"
            | "avec"
            | "sans"
            | "dans"
            | "sur"
            | "vers"
            | "la"
            | "le"
            | "les"
            | "des"
            | "du"
            | "au"
            | "aux"
            | "un"
            | "une"
            | "ce"
            | "cet"
            | "cette"
            | "d"
            | "l"
            | "qu"
            | "jusqu"
            | "puisqu"
            | "lorsqu"
    )
}

fn append_period_value_sentence(value: &mut Value, sentence: &str) {
    if let Some(text) = value.as_str() {
        let mut updated = text.to_string();
        append_period_sentence(&mut updated, sentence);
        *value = json!(updated);
    }
}

fn append_period_sentence(text: &mut String, sentence: &str) {
    if sentence.trim().is_empty() || text.contains(sentence) {
        return;
    }
    if !text.trim().is_empty() && !text.ends_with(' ') {
        text.push(' ');
    }
    text.push_str(sentence.trim());
}

fn period_public_word_count(response: &Value) -> usize {
    let mut public_text = String::new();
    collect_period_daily_public_text(response, &mut public_text);
    collect_period_public_text(response, &mut public_text);
    public_text.split_whitespace().count()
}

fn string_array_value(value: Option<&Value>) -> Option<Value> {
    let items = value?
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
        .filter(|item| !item.trim().is_empty())
        .map(|item| json!(item))
        .collect::<Vec<_>>();
    Some(Value::Array(items))
}

fn non_empty_string_array_value(value: Option<&Value>) -> Option<Value> {
    let value = string_array_value(value)?;
    if value.as_array().map(Vec::is_empty).unwrap_or(true) {
        None
    } else {
        Some(value)
    }
}

pub fn validate_period_provider_public_payload(response: &Value) -> Result<(), GenerationError> {
    if response["service_code"].as_str() == Some(HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE) {
        return validate_free_period_provider_public_payload(response);
    }
    require_period_public_string(response, &["week_overview", "title"])?;
    require_period_public_string(response, &["week_overview", "text"])?;
    require_period_public_string(response, &["week_overview", "trajectory"])?;
    require_period_public_string(response, &["advice", "main"])?;
    require_period_public_string(response, &["advice", "best_use"])?;
    require_period_public_string(response, &["advice", "avoid"])?;

    let timeline = response
        .get("daily_timeline")
        .and_then(Value::as_array)
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_TIMELINE_MISSING"))?;
    if timeline.len() != 7 {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_TIMELINE_MISSING",
            json!({ "timeline_count": timeline.len() }),
        ));
    }
    for day in timeline {
        for field in ["date", "day_label", "theme", "text", "advice"] {
            require_period_public_string_in(day, field, "daily_timeline")?;
        }
    }

    require_period_public_marker_array(response, "key_days", false)?;
    require_period_public_marker_array(response, "best_days", true)?;
    require_period_public_marker_array(response, "watch_days", false)?;
    require_period_watch_summary(response)?;

    let domains = response
        .get("domain_sections")
        .and_then(Value::as_array)
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"))?;
    let domain_range = if response["service_code"].as_str()
        == Some(HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE)
    {
        3..=5
    } else {
        2..=4
    };
    if !domain_range.contains(&domains.len()) {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_EVIDENCE_MISSING",
            json!({ "field": "domain_sections", "count": domains.len() }),
        ));
    }
    for section in domains {
        for field in ["domain", "title", "text"] {
            require_period_public_string_in(section, field, "domain_sections")?;
        }
    }

    let evidence = response
        .get("evidence_summary")
        .and_then(Value::as_array)
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"))?;
    if evidence.is_empty() {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_EVIDENCE_MISSING",
            json!({ "field": "evidence_summary" }),
        ));
    }
    for item in evidence {
        require_period_public_string_in(item, "date", "evidence_summary")?;
        require_period_public_string_in(item, "evidence_key", "evidence_summary")?;
        require_period_public_string_in(item, "label", "evidence_summary")?;
    }
    if response["service_code"].as_str() == Some(HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE) {
        let best_windows = response
            .get("best_windows")
            .and_then(Value::as_array)
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_PREMIUM_WINDOWS_MISSING"))?;
        if best_windows.is_empty() {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_PREMIUM_WINDOWS_MISSING",
                json!({ "field": "best_windows" }),
            ));
        }
        for window in best_windows {
            for field in [
                "date",
                "time_range_label",
                "title",
                "theme",
                "tone",
                "reason",
            ] {
                require_period_public_string_in(window, field, "best_windows")?;
            }
        }
        let watch_windows = response
            .get("watch_windows")
            .and_then(Value::as_array)
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_PREMIUM_WINDOWS_MISSING"))?;
        for window in watch_windows {
            for field in [
                "date",
                "time_range_label",
                "title",
                "theme",
                "tone",
                "watch_point",
            ] {
                require_period_public_string_in(window, field, "watch_windows")?;
            }
        }
        let strategy = response
            .get("strategy")
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_PREMIUM_STRATEGY_MISSING"))?;
        for field in ["title", "text", "best_use", "recovery"] {
            require_period_public_string_in(strategy, field, "strategy")?;
        }
    }
    Ok(())
}

fn validate_free_period_provider_public_payload(response: &Value) -> Result<(), GenerationError> {
    validate_free_period_forbidden_leaks(response)?;
    validate_free_period_required_fields(response)?;
    require_period_public_string(response, &["summary", "title"])?;
    require_period_public_string(response, &["summary", "text"])?;
    require_period_public_string(response, &["dominant_theme", "theme"])?;
    require_period_public_string(response, &["dominant_theme", "text"])?;
    require_period_public_string(response, &["watch_summary", "text"])?;
    require_period_public_marker_array(response, "key_days", true)?;
    if response["key_days"].as_array().map(Vec::len).unwrap_or(0) > 2 {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_FREE_BEST_DAYS_LEAK",
            json!({ "field": "key_days", "count": response["key_days"].as_array().map(Vec::len).unwrap_or(0) }),
        ));
    }
    for day in response["key_days"].as_array().into_iter().flatten() {
        if day["title"].as_str() != Some("Jour à retenir") {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_FREE_KEY_DAY_TITLE_INVALID",
                json!({ "field": "key_days.title" }),
            ));
        }
    }
    validate_free_period_key_days_are_neutral_markers(response)?;
    require_period_public_string(response, &["advice"])?;
    let evidence = response
        .get("evidence_summary")
        .and_then(Value::as_array)
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_FREE_EVIDENCE_MISSING"))?;
    if evidence.is_empty() || evidence.len() > 3 {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_FREE_EVIDENCE_MISSING",
            json!({ "field": "evidence_summary", "count": evidence.len() }),
        ));
    }
    Ok(())
}

fn validate_free_period_key_days_are_neutral_markers(
    response: &Value,
) -> Result<(), GenerationError> {
    let forbidden_terms = [
        "meilleur",
        "meilleure",
        "favorabl",
        "idéal",
        "ideal",
        "opportun",
        "chance",
        "fenêtre",
        "fenetre",
        "créneau",
        "creneau",
        "optimal",
        "parfait",
        "profiter",
    ];
    let useful_terms = [
        "repère",
        "repere",
        "retenir",
        "attention",
        "thème",
        "theme",
        "priorité",
        "priorite",
        "tendance",
        "ajuster",
        "comprendre",
    ];
    for (index, day) in response["key_days"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        let text = [
            day.get("title").and_then(Value::as_str),
            day.get("reason").and_then(Value::as_str),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
        if forbidden_terms.iter().any(|term| text.contains(term)) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_FREE_KEY_DAY_BEST_DAY_LEAK",
                json!({ "field": "key_days", "index": index }),
            ));
        }
        let reason = day
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_lowercase();
        if reason.split_whitespace().count() < 8
            || !useful_terms.iter().any(|term| reason.contains(term))
        {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_FREE_KEY_DAY_TOO_THIN",
                json!({ "field": "key_days.reason", "index": index }),
            ));
        }
    }
    Ok(())
}

fn validate_free_period_required_fields(response: &Value) -> Result<(), GenerationError> {
    if free_required_string_missing(response, "/summary/title")
        || free_required_string_missing(response, "/summary/text")
    {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_FREE_MISSING_SUMMARY",
            json!({ "field": "summary.text" }),
        ));
    }
    if free_required_string_missing(response, "/dominant_theme/theme")
        || free_required_string_missing(response, "/dominant_theme/text")
    {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_FREE_MISSING_DOMINANT_THEME",
            json!({ "field": "dominant_theme.text" }),
        ));
    }
    if response
        .get("advice")
        .and_then(Value::as_str)
        .map(|text| text.trim().is_empty())
        .unwrap_or(true)
    {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_FREE_MISSING_ADVICE",
            json!({ "field": "advice" }),
        ));
    }
    if response
        .get("key_days")
        .and_then(Value::as_array)
        .map(|items| items.is_empty())
        .unwrap_or(true)
    {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_FREE_MISSING_KEY_DAY",
            json!({ "field": "key_days" }),
        ));
    }
    if response
        .get("evidence_summary")
        .and_then(Value::as_array)
        .map(|items| items.is_empty())
        .unwrap_or(true)
    {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_FREE_EVIDENCE_MISSING",
            json!({ "field": "evidence_summary" }),
        ));
    }
    Ok(())
}

fn free_required_string_missing(response: &Value, pointer: &str) -> bool {
    response
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(|text| text.trim().is_empty())
        .unwrap_or(true)
}

fn validate_free_period_forbidden_leaks(response: &Value) -> Result<(), GenerationError> {
    for forbidden in [
        "daily_timeline",
        "best_days",
        "watch_days",
        "best_windows",
        "watch_windows",
        "domain_sections",
        "strategy",
        "week_overview",
    ] {
        if response.get(forbidden).is_some() {
            return Err(quality_error(
                match forbidden {
                    "daily_timeline" => "HOROSCOPE_PERIOD_FREE_DAILY_TIMELINE_LEAK",
                    "best_days" => "HOROSCOPE_PERIOD_FREE_BEST_DAYS_LEAK",
                    "watch_days" => "HOROSCOPE_PERIOD_FREE_WATCH_DAYS_LEAK",
                    "best_windows" | "watch_windows" => "HOROSCOPE_PERIOD_FREE_WINDOWS_LEAK",
                    "domain_sections" => "HOROSCOPE_PERIOD_FREE_DOMAIN_SECTIONS_LEAK",
                    "strategy" => "HOROSCOPE_PERIOD_FREE_STRATEGY_LEAK",
                    "week_overview" => "HOROSCOPE_PERIOD_FREE_WEEK_OVERVIEW_LEAK",
                    _ => "HOROSCOPE_PERIOD_RESPONSE_INVALID",
                },
                json!({ "field": forbidden }),
            ));
        }
    }
    Ok(())
}

fn require_period_watch_summary(response: &Value) -> Result<(), GenerationError> {
    let summary = response
        .get("watch_summary")
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_RESPONSE_INVALID"))?;
    let status = summary.get("status").and_then(Value::as_str).unwrap_or("");
    if !matches!(status, "active" | "low" | "none") {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_RESPONSE_INVALID",
            json!({ "field": "watch_summary.status" }),
        ));
    }
    require_period_public_string_in(summary, "text", "watch_summary")?;
    Ok(())
}

fn require_period_public_marker_array(
    response: &Value,
    field: &str,
    require_non_empty: bool,
) -> Result<(), GenerationError> {
    let items = response
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_KEY_DAYS_MISSING"))?;
    if require_non_empty && items.is_empty() {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_KEY_DAYS_MISSING",
            json!({ "field": field }),
        ));
    }
    for item in items {
        require_period_public_string_in(item, "date", field)?;
        require_period_public_string_in(item, "title", field)?;
        require_period_public_string_in(item, "reason", field)?;
    }
    Ok(())
}

fn require_period_public_string(value: &Value, path: &[&str]) -> Result<(), GenerationError> {
    let mut cursor = value;
    for segment in path {
        cursor = cursor.get(*segment).ok_or_else(|| {
            quality_error(
                "HOROSCOPE_PERIOD_RESPONSE_INVALID",
                json!({ "field": path.join(".") }),
            )
        })?;
    }
    let text = cursor.as_str().unwrap_or("").trim();
    if text.is_empty() {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_RESPONSE_INVALID",
            json!({ "field": path.join(".") }),
        ));
    }
    Ok(())
}

fn require_period_public_string_in(
    value: &Value,
    field: &str,
    parent: &str,
) -> Result<(), GenerationError> {
    let text = value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if text.is_empty() {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_RESPONSE_INVALID",
            json!({ "field": format!("{parent}.{field}") }),
        ));
    }
    Ok(())
}

pub fn validate_period_interpretation_request_schema(value: &Value) -> Result<(), GenerationError> {
    validate_schema(
        period_interpretation_request_schema,
        "HOROSCOPE_PERIOD_RESPONSE_INVALID",
        value,
    )
}

pub fn validate_period_response_schema(value: &Value) -> Result<(), GenerationError> {
    validate_schema(
        period_response_schema,
        "HOROSCOPE_PERIOD_RESPONSE_INVALID",
        value,
    )
}

pub fn validate_period_response_evidence(
    request: &Value,
    response: &Value,
) -> Result<(), GenerationError> {
    if is_free_period_request(request) {
        validate_free_period_forbidden_leaks(response)?;
        validate_free_period_required_fields(response)?;
        validate_period_response_schema(response)?;
        return validate_free_period_response_evidence(request, response);
    }
    validate_period_response_schema(response)?;
    let included = request["period_resolution"]["included_dates"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str())
        .collect::<HashSet<_>>();
    let evidence = request["evidence"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item["evidence_key"].as_str())
        .collect::<HashSet<_>>();
    if included.len() != 7 {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH",
            json!({ "included_date_count": included.len() }),
        ));
    }
    let timeline = response["daily_timeline"]
        .as_array()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_TIMELINE_MISSING"))?;
    if timeline.len() != 7 {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_TIMELINE_MISSING",
            json!({ "timeline_count": timeline.len() }),
        ));
    }
    let mut timeline_dates = HashSet::new();
    let mut public_text = String::new();
    let mut normalized_day_texts = HashSet::new();
    for day in timeline {
        let date = day["date"]
            .as_str()
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_TIMELINE_MISSING"))?;
        if !included.contains(date) || !timeline_dates.insert(date) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH",
                json!({ "date": date }),
            ));
        }
        validate_period_evidence_keys(&evidence, day["evidence_keys"].as_array())?;
        let day_text = day["text"].as_str().unwrap_or("").trim();
        let normalized_day_text = normalized_text(day_text);
        if normalized_day_text.is_empty() || !normalized_day_texts.insert(normalized_day_text) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_REPETITIVE_DAILY_TEXT",
                json!({ "date": date }),
            ));
        }
        for key in ["day_label", "theme", "tone", "text", "advice"] {
            if let Some(value) = day.get(key).and_then(|value| value.as_str()) {
                public_text.push_str(value);
                public_text.push('\n');
            }
        }
    }
    for date in &included {
        if !timeline_dates.contains(date) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH",
                json!({ "missing_date": date }),
            ));
        }
    }
    collect_period_public_text(response, &mut public_text);
    validate_period_day_markers(request, response, "key_days", &included, &evidence)?;
    validate_period_day_markers(request, response, "best_days", &included, &evidence)?;
    validate_period_day_markers(request, response, "watch_days", &included, &evidence)?;
    validate_period_watch_summary(response, &evidence)?;
    validate_period_domain_sections(response, &evidence)?;
    validate_period_evidence_summary(response, &included, &evidence)?;
    if is_premium_period_request(request) {
        validate_period_premium_windows(request, response, &included, &evidence)?;
        validate_period_premium_strategy(response, &evidence)?;
        validate_period_premium_detail(response)?;
    }
    validate_period_marker_date_overlaps(response)?;
    validate_period_public_text(&public_text)?;
    validate_period_public_tones(response)?;
    validate_period_public_word_count(request, response, &public_text)?;
    validate_period_public_personalization(response)?;
    validate_period_repeated_vocabulary(&public_text)?;
    validate_period_not_seven_daily(response)?;
    Ok(())
}

fn validate_free_period_response_evidence(
    request: &Value,
    response: &Value,
) -> Result<(), GenerationError> {
    validate_free_period_provider_public_payload(response)?;
    let included = request["period_resolution"]["included_dates"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<HashSet<_>>();
    let evidence = request["evidence"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item["evidence_key"].as_str())
        .collect::<HashSet<_>>();
    if included.len() != 7 {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH",
            json!({ "included_date_count": included.len() }),
        ));
    }
    validate_period_day_markers(request, response, "key_days", &included, &evidence)?;
    let watch = &response["watch_summary"];
    let status = watch["status"]
        .as_str()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_FREE_MISSING_ADVICE"))?;
    if !matches!(status, "none" | "low" | "present") {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_RESPONSE_INVALID",
            json!({ "field": "watch_summary.status" }),
        ));
    }
    if status == "none" {
        if watch["evidence_keys"]
            .as_array()
            .map(|keys| !keys.is_empty())
            .unwrap_or(false)
        {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_FREE_EVIDENCE_MISSING",
                json!({ "field": "watch_summary.evidence_keys" }),
            ));
        }
        if watch["text"]
            .as_str()
            .map(|text| text.split_whitespace().count() < 14)
            .unwrap_or(true)
        {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_FREE_WATCH_SUMMARY_TOO_THIN",
                json!({ "field": "watch_summary.text" }),
            ));
        }
    } else {
        validate_period_evidence_keys(&evidence, watch["evidence_keys"].as_array())?;
    }
    validate_period_evidence_summary(response, &included, &evidence)?;
    if response["evidence_summary"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0)
        > 3
    {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_FREE_EVIDENCE_MISSING",
            json!({ "field": "evidence_summary" }),
        ));
    }
    let mut public_text = String::new();
    collect_period_public_text(response, &mut public_text);
    validate_period_public_text(&public_text)?;
    validate_free_period_not_too_generic(response)?;
    let words = public_text.split_whitespace().count();
    let limits = period_word_limits_for_request(request);
    if response["quality"]["provider"].as_str() != Some("fake") && words < limits.target_min {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_FREE_TOO_SHORT",
            json!({ "word_count": words, "target_words_min": limits.target_min, "hard_limit_words": limits.hard_limit }),
        ));
    }
    if response["quality"]["provider"].as_str() != Some("fake") && words > limits.hard_limit {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_FREE_TOO_LONG",
            json!({ "word_count": words, "target_words_min": limits.target_min, "hard_limit_words": limits.hard_limit }),
        ));
    }
    if explicit_date_count(response["summary"]["text"].as_str().unwrap_or("")) > 2 {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_FREE_SUMMARY_TOO_MANY_EXPLICIT_DATES",
            Value::Null,
        ));
    }
    Ok(())
}

fn validate_free_period_not_too_generic(response: &Value) -> Result<(), GenerationError> {
    let text = [
        response.pointer("/summary/text").and_then(Value::as_str),
        response
            .pointer("/dominant_theme/text")
            .and_then(Value::as_str),
        response.get("advice").and_then(Value::as_str),
        response
            .pointer("/watch_summary/text")
            .and_then(Value::as_str),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n")
    .to_lowercase();
    let has_specific_anchor = [
        "lune",
        "mars",
        "venus",
        "mercure",
        "soleil",
        "jupiter",
        "saturne",
        "thème",
        "theme",
        "organisation",
        "relations",
        "énergie",
        "energie",
        "communication",
        "clarté",
        "clarte",
        "intégration",
        "integration",
        "routine",
    ]
    .iter()
    .any(|needle| text.contains(needle));
    if has_specific_anchor {
        Ok(())
    } else {
        Err(quality_error(
            "HOROSCOPE_PERIOD_FREE_TOO_GENERIC",
            json!({ "reason": "missing_free_specific_anchor" }),
        ))
    }
}

fn validate_period_watch_summary(
    response: &Value,
    evidence: &HashSet<&str>,
) -> Result<(), GenerationError> {
    let summary = &response["watch_summary"];
    let status = summary["status"]
        .as_str()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_RESPONSE_INVALID"))?;
    let watch_count = response["watch_days"].as_array().map(Vec::len).unwrap_or(0);
    if !matches!(status, "none" | "low" | "active") {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_BEST_WATCH_MISSING",
            json!({ "status": status }),
        ));
    }
    if (status == "none" && watch_count > 0) || (status == "active" && watch_count == 0) {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_BEST_WATCH_MISSING",
            json!({ "status": status, "watch_count": watch_count }),
        ));
    }
    if status == "none" {
        if summary["evidence_keys"]
            .as_array()
            .map(|keys| !keys.is_empty())
            .unwrap_or(false)
        {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_EVIDENCE_MISSING",
                json!({ "field": "watch_summary.evidence_keys" }),
            ));
        }
        return Ok(());
    }
    validate_period_evidence_keys(evidence, summary["evidence_keys"].as_array())
}

fn validate_period_day_markers(
    _request: &Value,
    response: &Value,
    field: &str,
    included: &HashSet<&str>,
    evidence: &HashSet<&str>,
) -> Result<(), GenerationError> {
    let mut dates = HashSet::new();
    for marker in response[field].as_array().into_iter().flatten() {
        let date = marker["date"]
            .as_str()
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_KEY_DAYS_MISSING"))?;
        if !dates.insert(date) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_DUPLICATE_DAY_MARKER",
                json!({ "field": field, "date": date }),
            ));
        }
        if !included.contains(date) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH",
                json!({ "field": field, "date": date }),
            ));
        }
        if marker
            .get("fallback_reason")
            .and_then(Value::as_str)
            .map(|reason| reason.trim().is_empty())
            .unwrap_or(false)
        {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_EVIDENCE_MISSING",
                json!({ "field": field, "date": date, "reason": "empty_fallback_reason" }),
            ));
        }
        let keys = marker["evidence_keys"].as_array();
        if keys.map(|items| items.is_empty()).unwrap_or(true)
            && marker
                .get("fallback_reason")
                .and_then(|v| v.as_str())
                .is_none()
        {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_EVIDENCE_MISSING",
                json!({ "field": field, "date": date }),
            ));
        }
        validate_period_evidence_keys(evidence, keys)?;
    }
    Ok(())
}

fn validate_period_evidence_keys(
    allowed: &HashSet<&str>,
    keys: Option<&Vec<Value>>,
) -> Result<(), GenerationError> {
    let Some(keys) = keys else {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_EVIDENCE_MISSING",
            Value::Null,
        ));
    };
    if keys.is_empty() {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_EVIDENCE_MISSING",
            Value::Null,
        ));
    }
    for key in keys {
        let Some(key) = key.as_str() else {
            return Err(horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"));
        };
        if !allowed.contains(key) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_EVIDENCE_MISSING",
                json!({ "evidence_key": key }),
            ));
        }
    }
    Ok(())
}

fn validate_period_marker_date_overlaps(response: &Value) -> Result<(), GenerationError> {
    let key = response["key_days"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item["date"].as_str())
        .collect::<HashSet<_>>();
    let best = response["best_days"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item["date"].as_str())
        .collect::<HashSet<_>>();
    for date in &best {
        if key.contains(date) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_KEY_DAYS_MISSING",
                json!({ "reason": "best_day_overlaps_key_day", "overlap_date": date }),
            ));
        }
    }
    for date in response["watch_days"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item["date"].as_str())
    {
        if best.contains(date) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_BEST_WATCH_MISSING",
                json!({ "overlap_date": date }),
            ));
        }
    }
    Ok(())
}

fn validate_period_domain_sections(
    response: &Value,
    evidence: &HashSet<&str>,
) -> Result<(), GenerationError> {
    let sections = response["domain_sections"]
        .as_array()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"))?;
    let is_premium =
        response["service_code"].as_str() == Some(HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE);
    let valid_range = if is_premium { 3..=5 } else { 2..=4 };
    if !valid_range.contains(&sections.len()) {
        return Err(quality_error(
            if is_premium {
                "HOROSCOPE_PERIOD_PREMIUM_DOMAIN_DEPTH_MISSING"
            } else {
                "HOROSCOPE_PERIOD_EVIDENCE_MISSING"
            },
            json!({ "field": "domain_sections", "count": sections.len() }),
        ));
    }
    let mut section_evidence_sets = HashSet::new();
    let mut section_domains = HashSet::new();
    for section in sections {
        let domain = section.get("domain").and_then(Value::as_str).unwrap_or("");
        if !section_domains.insert(domain.to_lowercase()) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_EVIDENCE_MISSING",
                json!({ "field": "domain_sections", "reason": "duplicate_domain", "domain": domain }),
            ));
        }
        validate_period_evidence_keys(evidence, section["evidence_keys"].as_array())?;
        let joined = section["evidence_keys"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|value| value.as_str())
            .collect::<Vec<_>>()
            .join("|");
        section_evidence_sets.insert(joined);
    }
    if sections.len() > 1 && section_evidence_sets.len() == 1 {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_EVIDENCE_MISSING",
            json!({ "reason": "domain_sections_share_same_evidence" }),
        ));
    }
    Ok(())
}

fn validate_period_premium_windows(
    request: &Value,
    response: &Value,
    included: &HashSet<&str>,
    evidence: &HashSet<&str>,
) -> Result<(), GenerationError> {
    let snapshot_keys = request["scan_plan"]["snapshots"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|snapshot| snapshot["snapshot_key"].as_str())
        .collect::<HashSet<_>>();
    let best = response["best_windows"]
        .as_array()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_PREMIUM_WINDOWS_MISSING"))?;
    if best.is_empty() {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_PREMIUM_WINDOWS_MISSING",
            json!({ "field": "best_windows" }),
        ));
    }
    validate_period_window_array("best_windows", best, included, evidence, &snapshot_keys)?;
    validate_period_best_windows_not_generic(best)?;
    let watch = response["watch_windows"]
        .as_array()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_PREMIUM_WINDOWS_MISSING"))?;
    if watch.is_empty() && !matches!(response["watch_summary"]["status"].as_str(), Some("none")) {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_PREMIUM_WINDOWS_MISSING",
            json!({ "field": "watch_windows" }),
        ));
    }
    if !watch.is_empty() {
        validate_period_window_array("watch_windows", watch, included, evidence, &snapshot_keys)?;
    }
    let best_identities = best
        .iter()
        .filter_map(period_window_identity)
        .collect::<HashSet<_>>();
    for window in watch {
        if let Some(identity) = period_window_identity(window) {
            if best_identities.contains(&identity) {
                return Err(quality_error(
                    "HOROSCOPE_PERIOD_PREMIUM_WINDOW_OVERLAP",
                    json!({ "window": identity }),
                ));
            }
        }
    }
    Ok(())
}

fn validate_period_best_windows_not_generic(windows: &[Value]) -> Result<(), GenerationError> {
    let titles = windows
        .iter()
        .filter_map(|window| window["title"].as_str())
        .map(normalized_text)
        .collect::<HashSet<_>>();
    let best_for_sets = windows
        .iter()
        .filter_map(|window| window["best_for"].as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(normalized_text)
                .collect::<Vec<_>>()
                .join("|")
        })
        .collect::<HashSet<_>>();
    let generic_titles = windows
        .iter()
        .filter_map(|window| window["title"].as_str())
        .filter(|title| normalized_text(title) == "fenêtre favorable")
        .count();
    if generic_titles > 0 || (windows.len() >= 3 && (titles.len() < 2 || best_for_sets.len() < 2)) {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_PREMIUM_WINDOWS_TOO_GENERIC",
            json!({
                "title_count": titles.len(),
                "best_for_count": best_for_sets.len(),
                "generic_titles": generic_titles
            }),
        ));
    }
    Ok(())
}

fn validate_period_window_array(
    field: &str,
    windows: &[Value],
    included: &HashSet<&str>,
    evidence: &HashSet<&str>,
    snapshot_keys: &HashSet<&str>,
) -> Result<(), GenerationError> {
    for window in windows {
        let date = window["date"]
            .as_str()
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_PREMIUM_WINDOWS_MISSING"))?;
        if !included.contains(date) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH",
                json!({ "field": field, "date": date }),
            ));
        }
        for text_field in ["time_range_label", "title", "theme", "tone"] {
            require_period_public_string_in(window, text_field, field)?;
        }
        if field == "best_windows" {
            require_period_public_string_in(window, "reason", field)?;
        } else {
            require_period_public_string_in(window, "watch_point", field)?;
        }
        let sources = window["source_snapshot_keys"].as_array().ok_or_else(|| {
            quality_error(
                "HOROSCOPE_PERIOD_PREMIUM_WINDOWS_MISSING",
                json!({ "field": format!("{field}.source_snapshot_keys") }),
            )
        })?;
        if sources.is_empty() {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_PREMIUM_WINDOWS_MISSING",
                json!({ "field": format!("{field}.source_snapshot_keys") }),
            ));
        }
        for source in sources {
            let Some(source) = source.as_str() else {
                return Err(horoscope_error("HOROSCOPE_PERIOD_PREMIUM_WINDOWS_MISSING"));
            };
            if !snapshot_keys.contains(source) {
                return Err(quality_error(
                    "HOROSCOPE_PERIOD_PREMIUM_WINDOWS_MISSING",
                    json!({ "field": field, "source_snapshot_key": source }),
                ));
            }
        }
        let keys = window["evidence_keys"].as_array();
        if keys.map(|items| items.is_empty()).unwrap_or(true) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_PREMIUM_WINDOW_EVIDENCE_MISSING",
                json!({ "field": field, "date": date }),
            ));
        }
        validate_period_evidence_keys(evidence, keys).map_err(|_| {
            quality_error(
                "HOROSCOPE_PERIOD_PREMIUM_WINDOW_EVIDENCE_MISSING",
                json!({ "field": field, "date": date }),
            )
        })?;
    }
    Ok(())
}

fn validate_period_premium_strategy(
    response: &Value,
    evidence: &HashSet<&str>,
) -> Result<(), GenerationError> {
    let strategy = response
        .get("strategy")
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_PREMIUM_STRATEGY_MISSING"))?;
    for field in ["title", "text", "best_use", "recovery"] {
        require_period_public_string_in(strategy, field, "strategy").map_err(|_| {
            quality_error(
                "HOROSCOPE_PERIOD_PREMIUM_STRATEGY_MISSING",
                json!({ "field": field }),
            )
        })?;
    }
    validate_period_evidence_keys(evidence, strategy["evidence_keys"].as_array())
}

fn validate_period_premium_detail(response: &Value) -> Result<(), GenerationError> {
    if response["best_windows"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0)
        == 0
        || response.get("strategy").is_none()
        || response["domain_sections"]
            .as_array()
            .map(Vec::len)
            .unwrap_or(0)
            < 3
        || response["daily_timeline"]
            .as_array()
            .map(Vec::len)
            .unwrap_or(0)
            != 7
        || response["evidence_summary"]
            .as_array()
            .map(Vec::len)
            .unwrap_or(0)
            == 0
    {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_PREMIUM_INSUFFICIENT_DETAIL",
            Value::Null,
        ));
    }
    let advice_and_strategy_text = [
        response.pointer("/advice/main").and_then(Value::as_str),
        response.pointer("/advice/best_use").and_then(Value::as_str),
        response.pointer("/advice/avoid").and_then(Value::as_str),
        response.pointer("/strategy/text").and_then(Value::as_str),
        response
            .pointer("/strategy/best_use")
            .and_then(Value::as_str),
        response
            .pointer("/strategy/recovery")
            .and_then(Value::as_str),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ");
    if explicit_date_count(&advice_and_strategy_text) > 0 {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_PREMIUM_ADVICE_RECALENDARIZED",
            Value::Null,
        ));
    }
    Ok(())
}

fn is_premium_period_request(request: &Value) -> bool {
    request["service_code"].as_str() == Some(HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE)
}

fn is_free_period_request(request: &Value) -> bool {
    request["service_code"].as_str() == Some(HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE)
}

fn validate_period_evidence_summary(
    response: &Value,
    included: &HashSet<&str>,
    evidence: &HashSet<&str>,
) -> Result<(), GenerationError> {
    let items = response["evidence_summary"]
        .as_array()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"))?;
    if items.is_empty() {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_EVIDENCE_MISSING",
            json!({ "field": "evidence_summary" }),
        ));
    }
    for item in items {
        let date = item["date"]
            .as_str()
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"))?;
        if !included.contains(date) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH",
                json!({ "field": "evidence_summary", "date": date }),
            ));
        }
        let key = item["evidence_key"]
            .as_str()
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"))?;
        if !evidence.contains(key) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_EVIDENCE_MISSING",
                json!({ "field": "evidence_summary", "evidence_key": key }),
            ));
        }
        if item["label"].as_str().unwrap_or("").trim().is_empty() {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_EVIDENCE_MISSING",
                json!({ "field": "evidence_summary.label" }),
            ));
        }
    }
    Ok(())
}

fn is_period_major_aspect(aspect: &str) -> bool {
    matches!(
        aspect,
        "conjunction" | "sextile" | "square" | "trine" | "opposition"
    )
}

fn period_max_major_aspect_orb_deg() -> f64 {
    serde_json::from_str::<Value>(ORB_BANDS_JSON)
        .ok()
        .and_then(|value| value.get("data").and_then(Value::as_array).cloned())
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("max_orb_deg").and_then(Value::as_f64))
        .filter(|orb| orb.is_finite() && *orb > 0.0)
        .max_by(|left, right| left.total_cmp(right))
        .expect("json_db/horoscope_orb_weight_bands.json must define positive max_orb_deg values")
}

fn validate_period_public_text(public_text: &str) -> Result<(), GenerationError> {
    let lower = public_text.to_lowercase();
    for forbidden in [
        "personnaliser ce signal",
        "relier ce signal",
        "relier ce domaine",
        "plutôt que rester sur un conseil générique",
        "donne le relief principal",
        "devient plus lisible",
        "deviennent plus lisibles",
        " en prose utilisateur",
        "writer",
        "summary_hint",
        "advice_hint",
        "personalization_hint",
        "natal_focus_hint",
    ] {
        if lower.contains(forbidden) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_INTERNAL_GUIDANCE_LEAK",
                json!({ "forbidden": forbidden }),
            ));
        }
    }
    if contains_period_theme_instruction(&lower) {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_INTERNAL_GUIDANCE_LEAK",
            json!({ "forbidden": "date_theme_instruction" }),
        ));
    }
    if let Some(fragment) = period_broken_sentence_fragment(public_text) {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_BROKEN_SENTENCE",
            json!({ "fragment": fragment }),
        ));
    }
    if let Some(fragment) = period_lowercase_sentence_start(public_text) {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_BROKEN_SENTENCE",
            json!({ "fragment": fragment }),
        ));
    }
    if let Some(fragment) = period_broken_french_fragment(public_text) {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_BROKEN_FRENCH_FRAGMENT",
            json!({ "fragment": fragment }),
        ));
    }
    for forbidden in [
        "plus personnel que générique",
        "conseil générique",
        "ce qui rend le conseil",
        "cette nuance reste liée",
        "avec un écho personnel autour de",
        "secteur personnel activé",
        "adaptez le geste au secteur personnel",
        "la lecture relie",
        "zones personnelles déjà mises en évidence",
        "zones personnelles",
        "zones natales activées",
        "secteurs personnels",
        "thème natal comme fil directeur",
        "le point d'appui concerne",
    ] {
        if lower.contains(forbidden) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_META_PERSONALIZATION_LEAK",
                json!({ "forbidden": forbidden }),
            ));
        }
    }
    if period_has_bad_french_colon_spacing(public_text) {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_FRENCH_TYPOGRAPHY_FAILED",
            json!({ "reason": "colon_spacing" }),
        ));
    }
    for forbidden in [
        "slot:",
        "slot_",
        "[morning]",
        "[afternoon]",
        "[evening]",
        "raw_transits",
        "period:",
        "natal_",
        "fake_",
        "theme_code",
        "evidence_key",
        "snapshot",
        "transit_exact",
        "transit_active",
        "moon_house_by_day",
        "organization",
        "relationship",
        "energy",
        "clarity",
        "integration",
    ] {
        if lower.contains(forbidden) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_TECHNICAL_CODE_LEAK",
                json!({ "forbidden": forbidden }),
            ));
        }
    }
    for forbidden in [
        "focused",
        "focus",
        "supportive",
        "careful",
        "active",
        "mixed",
        "fluid",
        "tense",
    ] {
        if contains_ascii_token(&lower, forbidden) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_TECHNICAL_CODE_LEAK",
                json!({ "forbidden": forbidden }),
            ));
        }
    }
    Ok(())
}

fn period_broken_french_fragment(public_text: &str) -> Option<String> {
    let lower = public_text.to_lowercase();
    for fragment in [
        "s’dynamique",
        "s'dynamique",
        "tout s’dynamique",
        "tout s'dynamique",
        "d’accélère",
        "d'accélère",
        "rédynamique",
        "redynamique",
        "l’organiser",
        "l'organiser",
    ] {
        if let Some(index) = lower.find(fragment) {
            return Some(public_text[index..].chars().take(48).collect::<String>());
        }
    }
    None
}

fn period_has_bad_french_colon_spacing(public_text: &str) -> bool {
    let chars = public_text.chars().collect::<Vec<_>>();
    for (index, ch) in chars.iter().enumerate() {
        if *ch != ':' {
            continue;
        }
        let before = index.checked_sub(1).and_then(|idx| chars.get(idx)).copied();
        let after = chars.get(index + 1).copied();
        if before.map(|ch| ch.is_ascii_digit()).unwrap_or(false)
            && after.map(|ch| ch.is_ascii_digit()).unwrap_or(false)
        {
            continue;
        }
        if before.map(|ch| !ch.is_whitespace()).unwrap_or(false)
            || after.map(|ch| !ch.is_whitespace()).unwrap_or(false)
        {
            return true;
        }
    }
    false
}

fn contains_period_theme_instruction(lower: &str) -> bool {
    lower
        .split(['.', '!', '?', '\n'])
        .any(|sentence| sentence.contains(", le thème ") && sentence.contains(" donne "))
}

fn period_broken_sentence_fragment(public_text: &str) -> Option<String> {
    for sentence in public_text.split(['.', '!', '?']) {
        let trimmed = sentence.trim();
        if trimmed.is_empty() {
            continue;
        }
        let tail = trimmed
            .split_whitespace()
            .rev()
            .take(3)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join(" ");
        if period_is_broken_sentence_tail(&tail) {
            return Some(tail);
        }
    }
    None
}

fn period_lowercase_sentence_start(public_text: &str) -> Option<String> {
    for (index, ch) in public_text.char_indices() {
        if !matches!(ch, '.' | '!' | '?') {
            continue;
        }
        let rest = public_text[index + ch.len_utf8()..].trim_start();
        let mut words = rest.split_whitespace();
        let first = words.next().unwrap_or("");
        let second = words.next().unwrap_or("");
        let first_is_lower = first
            .chars()
            .next()
            .map(|ch| ch.is_lowercase())
            .unwrap_or(false);
        let second_is_lower = second
            .chars()
            .next()
            .map(|ch| ch.is_lowercase())
            .unwrap_or(false);
        if first_is_lower {
            match first.trim_matches(|ch: char| !ch.is_alphabetic()) {
                "votre" | "vos" => {
                    return Some(rest.chars().take(32).collect::<String>());
                }
                "le" | "la" | "un" | "une" if second_is_lower => {
                    return Some(rest.chars().take(32).collect::<String>());
                }
                _ => {}
            }
        }
    }
    None
}

fn period_is_broken_sentence_tail(tail: &str) -> bool {
    let normalized = tail
        .trim()
        .trim_matches(|ch: char| matches!(ch, ',' | ';' | ':' | '\'' | '’' | '“' | '”' | '"'))
        .to_lowercase();
    let words = normalized.split_whitespace().collect::<Vec<_>>();
    match words.as_slice() {
        [] => false,
        [last] => period_is_weak_sentence_ending(last),
        [.., "à", "la" | "l" | "l'"] => true,
        [.., "de", "la" | "l" | "l'"] => true,
        [.., last] => period_is_weak_sentence_ending(last),
    }
}

fn validate_period_public_personalization(response: &Value) -> Result<(), GenerationError> {
    let mut count = 0;
    for day in response["daily_timeline"].as_array().into_iter().flatten() {
        if period_text_has_personalization(day["text"].as_str().unwrap_or("")) {
            count += 1;
        }
    }
    let week_text = format!(
        "{} {}",
        response["week_overview"]["text"].as_str().unwrap_or(""),
        response["week_overview"]["trajectory"]
            .as_str()
            .unwrap_or("")
    );
    for phrase in ["thème natal comme fil directeur", "relations directes"] {
        if count_normalized_phrase(&week_text, phrase) > 1 {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_OVERVIEW_REPETITION",
                json!({ "phrase": phrase }),
            ));
        }
    }
    if !period_text_has_personalization(&week_text) {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_EVIDENCE_MISSING",
            json!({ "reason": "week_overview_missing_natal_personalization" }),
        ));
    }
    if count < 4 {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_EVIDENCE_MISSING",
            json!({ "reason": "daily_timeline_missing_natal_personalization", "count": count }),
        ));
    }
    Ok(())
}

fn count_normalized_phrase(text: &str, phrase: &str) -> usize {
    text.to_lowercase().matches(&phrase.to_lowercase()).count()
}

fn period_text_has_personalization(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "thème natal",
        "vous-même",
        "vous meme",
        "pour vous",
        "vos priorités",
        "vos priorites",
        "votre agenda",
        "repères personnels",
        "repère personnel",
        "zone natale",
        "zones natales",
        "maison",
        "sensibilité",
        "besoins émotionnels",
        "communiquer",
        "penser",
        "attachement",
        "plaisir",
        "agir",
        "énergie",
        "responsabilité",
        "limites",
        "relations directes",
        "besoin de sens",
        "habitudes",
        "rythme de travail",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn validate_period_repeated_vocabulary(public_text: &str) -> Result<(), GenerationError> {
    let lower = public_text.to_lowercase();
    for phrase in [
        "restez concret",
        "gardez une marge",
        "clarifier",
        "ajuster",
        "intégrer",
        "met l'accent",
        "choisissez une seule priorité",
        "le point d'appui concerne",
    ] {
        let count = lower.matches(phrase).count();
        if count > 2 {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_REPETITIVE_DAILY_TEXT",
                json!({ "phrase": phrase, "count": count }),
            ));
        }
    }
    Ok(())
}

fn collect_period_daily_public_text(response: &Value, public_text: &mut String) {
    for day in response["daily_timeline"].as_array().into_iter().flatten() {
        for key in ["day_label", "theme", "tone", "text", "advice"] {
            if let Some(value) = day.get(key).and_then(|value| value.as_str()) {
                public_text.push_str(value);
                public_text.push('\n');
            }
        }
    }
}

fn contains_ascii_token(text: &str, token: &str) -> bool {
    text.match_indices(token).any(|(idx, _)| {
        let before = text[..idx].chars().next_back();
        let after = text[idx + token.len()..].chars().next();
        before
            .map(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
            .unwrap_or(true)
            && after
                .map(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
                .unwrap_or(true)
    })
}

fn collect_period_public_text(response: &Value, public_text: &mut String) {
    for pointer in [
        "/summary/title",
        "/summary/text",
        "/dominant_theme/theme",
        "/dominant_theme/text",
        "/week_overview/title",
        "/week_overview/text",
        "/week_overview/trajectory",
        "/watch_summary/text",
        "/advice",
        "/advice/main",
        "/advice/best_use",
        "/advice/avoid",
    ] {
        if let Some(value) = response.pointer(pointer).and_then(|value| value.as_str()) {
            public_text.push_str(value);
            public_text.push('\n');
        }
    }
    for field in [
        "key_days",
        "best_days",
        "watch_days",
        "best_windows",
        "watch_windows",
        "domain_sections",
        "evidence_summary",
    ] {
        for item in response[field].as_array().into_iter().flatten() {
            for key in [
                "title",
                "reason",
                "watch_point",
                "theme",
                "tone",
                "domain",
                "text",
                "label",
            ] {
                if let Some(value) = item.get(key).and_then(|value| value.as_str()) {
                    public_text.push_str(value);
                    public_text.push('\n');
                }
            }
        }
    }
    for pointer in [
        "/strategy/title",
        "/strategy/text",
        "/strategy/best_use",
        "/strategy/recovery",
    ] {
        if let Some(value) = response.pointer(pointer).and_then(Value::as_str) {
            public_text.push_str(value);
            public_text.push('\n');
        }
    }
}

fn explicit_date_count(text: &str) -> usize {
    let tokens = text
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|ch: char| {
                !ch.is_alphanumeric() && !matches!(ch, '-' | '/' | 'û' | 'é')
            })
            .to_lowercase()
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let mut count = tokens
        .iter()
        .filter(|word| is_explicit_numeric_date(word))
        .count();
    for pair in tokens.windows(2) {
        if is_day_number(&pair[0]) && is_french_month_name(&pair[1]) {
            count += 1;
        }
    }
    count
}

fn is_explicit_numeric_date(token: &str) -> bool {
    if chrono::NaiveDate::parse_from_str(token, "%Y-%m-%d").is_ok()
        || chrono::NaiveDate::parse_from_str(token, "%d/%m/%Y").is_ok()
    {
        return true;
    }
    let parts = token.split('/').collect::<Vec<_>>();
    if parts.len() == 2 {
        return is_day_number(parts[0]) && is_month_number(parts[1]);
    }
    false
}

fn is_day_number(token: &str) -> bool {
    token
        .parse::<u32>()
        .map(|value| (1..=31).contains(&value))
        .unwrap_or(false)
}

fn is_month_number(token: &str) -> bool {
    token
        .parse::<u32>()
        .map(|value| (1..=12).contains(&value))
        .unwrap_or(false)
}

fn is_french_month_name(token: &str) -> bool {
    matches!(
        token,
        "janvier"
            | "fevrier"
            | "février"
            | "mars"
            | "avril"
            | "mai"
            | "juin"
            | "juillet"
            | "aout"
            | "août"
            | "septembre"
            | "octobre"
            | "novembre"
            | "decembre"
            | "décembre"
    )
}

fn validate_period_not_seven_daily(response: &Value) -> Result<(), GenerationError> {
    if response.get("week_overview").is_none()
        || response.get("domain_sections").is_none()
        || response.get("key_days").is_none()
    {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_REPETITIVE_DAILY_TEXT",
            json!({ "reason": "missing_period_level_sections" }),
        ));
    }
    Ok(())
}

fn period_theme_public_label(theme_code: &str) -> &'static str {
    match theme_code {
        "organization" => "organisation",
        "relationship" => "relations",
        "energy" => "énergie",
        "clarity" => "clarté",
        "communication" => "communication",
        "integration" => "intégration",
        "routine" => "routine",
        _ => "priorité principale",
    }
}

fn period_domain_title(theme_code: &str) -> &'static str {
    match theme_code {
        "organization" | "routine" => "Organisation",
        "relationship" => "Relations",
        "energy" => "Énergie",
        "clarity" => "Clarté",
        "communication" => "Communication",
        "integration" => "Intégration",
        _ => "Priorité",
    }
}

fn period_domain_focus(theme_code: &str, personalization: &str) -> String {
    let focus = match theme_code {
        "relationship" => "Observer la qualité du lien, les attentes implicites et la manière de répondre sans suradapter.",
        "energy" => "Canaliser l'élan dans des choix courts, assumés et compatibles avec le rythme personnel.",
        "communication" => "Privilégier les messages utiles, les décisions formulées clairement et les échanges qui font avancer.",
        "clarity" => "Mettre en lumière ce qui compte vraiment avant de conclure ou de promettre.",
        "integration" => "Relier les avancées de la semaine à une base plus mature et plus stable.",
        "routine" => "Rendre les habitudes plus soutenables sans rigidifier toute la semaine.",
        _ => "Installer des repères simples et hiérarchiser ce qui mérite vraiment de l'attention.",
    };
    format!("{focus} {personalization}")
}

#[derive(Clone)]
struct PeriodNatalFocus {
    label: String,
    hint: String,
}

fn period_natal_focus_code(fact: &Value) -> String {
    if let Some(target) = fact.get("natal_target").and_then(Value::as_str) {
        if !target.trim().is_empty() {
            return target.to_string();
        }
    }
    if let Some(house) = fact.get("natal_house").and_then(Value::as_i64) {
        if (1..=12).contains(&house) {
            return format!("natal_house_{house}");
        }
    }
    "natal_house_6".to_string()
}

fn period_natal_focus(code: &str) -> PeriodNatalFocus {
    period_natal_focus_labels()
        .get(code)
        .cloned()
        .unwrap_or_else(|| PeriodNatalFocus {
            label: "un repère personnel important".to_string(),
            hint: "Relier ce signal à une priorité personnelle concrète, sans jargon technique."
                .to_string(),
        })
}

fn period_natal_focus_labels() -> &'static HashMap<String, PeriodNatalFocus> {
    static LABELS: OnceLock<HashMap<String, PeriodNatalFocus>> = OnceLock::new();
    LABELS.get_or_init(|| {
        serde_json::from_str::<Value>(NATAL_FOCUS_LABELS_JSON)
            .ok()
            .and_then(|value| value.get("data").and_then(Value::as_array).cloned())
            .into_iter()
            .flatten()
            .filter(|row| {
                row.get("is_active")
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
            })
            .filter_map(|row| {
                Some((
                    row.get("focus_code")?.as_str()?.to_string(),
                    PeriodNatalFocus {
                        label: row.get("label_fr")?.as_str()?.to_string(),
                        hint: row.get("hint_fr")?.as_str()?.to_string(),
                    },
                ))
            })
            .collect()
    })
}

#[derive(Clone)]
struct PeriodStyleVariant {
    code: String,
    avoid_terms: Value,
}

fn period_style_variant_for_theme(theme: &str) -> PeriodStyleVariant {
    let code = match theme {
        "relationship" => "relation",
        "energy" => "action",
        "communication" => "communication",
        "clarity" => "clarity",
        "integration" => "integration",
        "routine" => "perspective",
        _ => "anchor",
    };
    period_style_variants()
        .get(code)
        .cloned()
        .unwrap_or_else(|| PeriodStyleVariant {
            code: code.to_string(),
            avoid_terms: json!(["restez concret", "gardez une marge"]),
        })
}

fn period_style_variants() -> &'static HashMap<String, PeriodStyleVariant> {
    static VARIANTS: OnceLock<HashMap<String, PeriodStyleVariant>> = OnceLock::new();
    VARIANTS.get_or_init(|| {
        serde_json::from_str::<Value>(PERIOD_STYLE_VARIANTS_JSON)
            .ok()
            .and_then(|value| value.get("data").and_then(Value::as_array).cloned())
            .into_iter()
            .flatten()
            .filter(|row| {
                row.get("is_active")
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
            })
            .filter_map(|row| {
                let code = row.get("style_variant_code")?.as_str()?.to_string();
                Some((
                    code.clone(),
                    PeriodStyleVariant {
                        code,
                        avoid_terms: row.get("avoid_terms").cloned().unwrap_or_else(|| json!([])),
                    },
                ))
            })
            .collect()
    })
}

fn period_event_personalization_hint(event: &Value) -> &str {
    event["theme_code"].as_str().map_or(
        "Relier ce signal à une priorité personnelle concrète.",
        |theme| match theme {
            "relationship" => "Nuancer ce jour par votre manière personnelle de chercher du lien et de répondre aux attentes.",
            "energy" => "Nuancer ce jour par votre manière personnelle d'agir, de décider et de doser l'effort.",
            "communication" => "Nuancer ce jour par votre manière personnelle de penser, parler et arbitrer rapidement.",
            "clarity" => "Nuancer ce jour par ce qui vous aide à reconnaître ce qui a vraiment de la valeur.",
            "integration" => "Nuancer ce jour par votre rapport aux limites utiles, au temps et à la consolidation.",
            _ => "Nuancer ce jour par la maison natale activée et par vos repères personnels.",
        },
    )
}

fn period_advice_hint(theme: &str, natal_focus_hint: &str) -> String {
    let advice = match theme {
        "relationship" => {
            "Cherchez une réponse relationnelle précise plutôt qu'un accord de façade."
        }
        "energy" => "Choisissez une action courte, assumée et proportionnée.",
        "communication" => "Formulez le message utile avant d'élargir la discussion.",
        "clarity" => "Nommez ce qui compte avant de décider.",
        "integration" => "Reliez ce qui avance à une limite ou un engagement réaliste.",
        "routine" => "Allégez une habitude avant d'en ajouter une autre.",
        _ => "Hiérarchisez une priorité et laissez le reste au second plan.",
    };
    format!("{advice} {natal_focus_hint}")
}

fn period_tone_public_label(tone_code: &str) -> String {
    period_tone_labels()
        .get(tone_code)
        .cloned()
        .unwrap_or_else(|| "nuancé".to_string())
}

fn period_tone_public_label_if_code(tone: &str) -> String {
    period_tone_labels()
        .get(tone)
        .cloned()
        .unwrap_or_else(|| tone.to_string())
}

fn period_public_tone_labels() -> &'static HashSet<String> {
    static PUBLIC_TONE_LABELS: OnceLock<HashSet<String>> = OnceLock::new();
    PUBLIC_TONE_LABELS.get_or_init(|| period_tone_labels().values().cloned().collect())
}

fn period_tone_labels() -> &'static HashMap<String, String> {
    static TONE_LABELS: OnceLock<HashMap<String, String>> = OnceLock::new();
    TONE_LABELS.get_or_init(|| {
        serde_json::from_str::<Value>(TONE_LABELS_JSON)
            .ok()
            .and_then(|value| value.get("data").and_then(Value::as_array).cloned())
            .into_iter()
            .flatten()
            .filter(|row| {
                row.get("is_active")
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
            })
            .filter_map(|row| {
                Some((
                    row.get("tone_code")?.as_str()?.to_string(),
                    row.get("label_fr")?.as_str()?.to_string(),
                ))
            })
            .collect::<HashMap<_, _>>()
    })
}

fn normalize_period_public_tones(request: &Value, response: &mut Value) {
    let tone_by_date = request["daily_plans"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|day| {
            Some((
                day.get("date")?.as_str()?.to_string(),
                period_tone_public_label(day.get("tone")?.as_str()?),
            ))
        })
        .collect::<HashMap<_, _>>();
    if let Some(days) = response
        .get_mut("daily_timeline")
        .and_then(Value::as_array_mut)
    {
        for day in days {
            if let Some(date) = day.get("date").and_then(Value::as_str) {
                if let Some(label) = tone_by_date.get(date) {
                    day["tone"] = json!(label);
                    continue;
                }
            }
            if let Some(tone) = day.get("tone").and_then(Value::as_str) {
                day["tone"] = json!(period_tone_public_label_if_code(tone));
            }
        }
    }
}

fn validate_period_public_tones(response: &Value) -> Result<(), GenerationError> {
    let allowed = period_public_tone_labels();
    for day in response["daily_timeline"].as_array().into_iter().flatten() {
        let tone = day["tone"]
            .as_str()
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_TECHNICAL_CODE_LEAK"))?;
        if !allowed.contains(tone) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_TECHNICAL_CODE_LEAK",
                json!({ "field": "daily_timeline.tone", "tone": tone }),
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct PeriodWordLimits {
    target_min: usize,
    target_max: usize,
    hard_limit: usize,
}

#[derive(Debug, Clone, Copy)]
struct PeriodDetailProfile {
    max_main_events: usize,
    max_evidence: usize,
    max_key_days: usize,
    max_best_days: usize,
    max_watch_days: usize,
    max_domain_sections: usize,
    max_best_windows: usize,
    max_watch_windows: usize,
    include_best_days: bool,
    include_watch_days: bool,
    include_daily_timeline: bool,
    include_domain_sections: bool,
    include_best_windows: bool,
    include_watch_windows: bool,
    include_strategy_section: bool,
    word_limits: PeriodWordLimits,
}

fn period_detail_profile(
    detail_profile_code: &str,
) -> Result<PeriodDetailProfile, GenerationError> {
    let row = rows(DETAIL_PROFILES_JSON)?
        .into_iter()
        .find(|row| {
            row.get("detail_profile_code").and_then(Value::as_str) == Some(detail_profile_code)
                && row
                    .get("is_enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
        })
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_PROFILE_UNSUPPORTED"))?;
    let max_domain_sections = if detail_profile_code == "premium_rich" {
        5
    } else {
        4
    };
    Ok(PeriodDetailProfile {
        max_main_events: row
            .get("max_main_events")
            .and_then(Value::as_u64)
            .unwrap_or(8) as usize,
        max_evidence: row
            .get("max_evidence")
            .and_then(Value::as_u64)
            .unwrap_or(20) as usize,
        max_key_days: row.get("max_key_days").and_then(Value::as_u64).unwrap_or(2) as usize,
        max_best_days: row
            .get("max_best_days")
            .and_then(Value::as_u64)
            .unwrap_or(2) as usize,
        max_watch_days: row
            .get("max_watch_days")
            .and_then(Value::as_u64)
            .unwrap_or(2) as usize,
        max_domain_sections,
        max_best_windows: row
            .get("max_best_windows")
            .and_then(Value::as_u64)
            .unwrap_or(3) as usize,
        max_watch_windows: row
            .get("max_watch_windows")
            .and_then(Value::as_u64)
            .unwrap_or(3) as usize,
        include_best_days: row
            .get("include_best_days")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        include_watch_days: row
            .get("include_watch_days")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        include_daily_timeline: row
            .get("include_daily_timeline")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        include_domain_sections: row
            .get("include_domain_sections")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        include_best_windows: row
            .get("include_best_windows")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        include_watch_windows: row
            .get("include_watch_windows")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        include_strategy_section: row
            .get("include_strategy_section")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        word_limits: PeriodWordLimits {
            target_min: row
                .get("target_words_min")
                .and_then(Value::as_u64)
                .unwrap_or(800) as usize,
            target_max: row
                .get("target_words_max")
                .and_then(Value::as_u64)
                .unwrap_or(1200) as usize,
            hard_limit: row
                .get("hard_limit_words")
                .and_then(Value::as_u64)
                .unwrap_or(1500) as usize,
        },
    })
}

fn period_basic_word_limits() -> PeriodWordLimits {
    period_detail_profile("basic_standard")
        .map(|profile| profile.word_limits)
        .expect("json_db/horoscope_detail_profiles.json must define basic_standard word limits")
}

fn period_word_limits_for_request(request: &Value) -> PeriodWordLimits {
    request["detail_profile_code"]
        .as_str()
        .and_then(|code| period_detail_profile(code).ok())
        .map(|profile| profile.word_limits)
        .unwrap_or_else(period_basic_word_limits)
}

fn period_writer_max_output_tokens(request: &Value) -> u32 {
    let limits = period_word_limits_for_request(request);
    ((limits.hard_limit as u32).saturating_mul(3)).saturating_add(500)
}

fn validate_period_public_word_count(
    request: &Value,
    response: &Value,
    public_text: &str,
) -> Result<(), GenerationError> {
    if response["quality"]["provider"].as_str() == Some("fake") {
        return Ok(());
    }
    let limits = period_word_limits_for_request(request);
    let word_count = public_text.split_whitespace().count();
    if word_count < limits.target_min || word_count > limits.hard_limit {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_WORD_COUNT_OUT_OF_RANGE",
            json!({
                "word_count": word_count,
                "target_words_min": limits.target_min,
                "target_words_max": limits.target_max,
                "hard_limit_words": limits.hard_limit
            }),
        ));
    }
    Ok(())
}

fn period_object_public_label(object_code: &str) -> &'static str {
    match object_code {
        "sun" => "le Soleil",
        "moon" => "la Lune",
        "mercury" => "Mercure",
        "venus" => "Vénus",
        "mars" => "Mars",
        "jupiter" => "Jupiter",
        "saturn" => "Saturne",
        _ => "un facteur astrologique",
    }
}

fn public_day_label(date: &str) -> String {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .ok()
        .map(|date| {
            let label = match date.weekday() {
                chrono::Weekday::Mon => "Lundi",
                chrono::Weekday::Tue => "Mardi",
                chrono::Weekday::Wed => "Mercredi",
                chrono::Weekday::Thu => "Jeudi",
                chrono::Weekday::Fri => "Vendredi",
                chrono::Weekday::Sat => "Samedi",
                chrono::Weekday::Sun => "Dimanche",
            };
            format!("{label} {}", date.format("%d/%m"))
        })
        .unwrap_or_else(|| date.to_string())
}

fn premium_period(
    public: &HoroscopePublicRequest,
    service_code: &str,
    calculation: &Value,
) -> Value {
    let mut period = calculation.get("period").cloned().unwrap_or_else(|| {
        json!({
            "date": public.date,
            "timezone": public.timezone
        })
    });
    if service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
        if let Some(label) = public
            .location
            .as_ref()
            .and_then(|location| location.label.as_ref())
            .filter(|label| !label.trim().is_empty())
        {
            period["location_label"] = json!(label);
        }
    }
    period
}

fn build_best_slots(request: &Value) -> Vec<Value> {
    premium_ranked_slots(request, false)
}

fn build_watch_slots(request: &Value) -> Vec<Value> {
    premium_ranked_slots(request, true)
}

fn premium_ranked_slots(request: &Value, watch: bool) -> Vec<Value> {
    let slots = request
        .get("slots")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let mut ranked = slots
        .iter()
        .copied()
        .filter(|slot| {
            let tone = slot.get("tone").and_then(|v| v.as_str()).unwrap_or("");
            if watch {
                tone.contains("tense") || tone.contains("careful")
            } else {
                !tone.contains("tense") && !tone.contains("careful")
            }
        })
        .take(3)
        .collect::<Vec<_>>();
    if ranked.is_empty() {
        ranked = slots.iter().rev().copied().take(3).collect();
    }
    ranked
        .into_iter()
        .map(|slot| premium_slot_summary(slot, watch))
        .collect()
}

async fn daily_writer_response(
    use_case: &GenerateReadingUseCase,
    request: &Value,
) -> Result<Value, GenerationError> {
    let defaults = horoscope_writer_engine_defaults(use_case);
    if defaults.provider == ProviderKind::Fake {
        return fake_writer_response(request);
    }

    let schema = daily_response_provider_schema(request)?;
    let provider_request = ProviderGenerationRequest {
        model: defaults.model.clone(),
        messages: daily_writer_messages(request)?,
        structured_schema: Some(schema),
        reasoning_effort: None,
        temperature: Some(0.4),
        max_output_tokens: Some(daily_writer_max_output_tokens(request)),
        safety_mode: SafetyMode::PlatformRulesOnly,
        timeout: StdDuration::from_secs(180),
        metadata: GenerationMetadata {
            run_id: uuid::Uuid::new_v4().to_string(),
            request_id: None,
            product_code: request["service_code"]
                .as_str()
                .unwrap_or(HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE)
                .to_string(),
            chapter_code: None,
        },
    };

    let routed = use_case
        .router
        .generate(
            provider_request,
            defaults.provider.clone(),
            &defaults.model,
            false,
            true,
            ModelRouteContext::PrimaryReading,
        )
        .await?;
    if routed.used_provider == ProviderKind::Fake {
        return Err(quality_error(
            "HOROSCOPE_DAILY_REAL_PROVIDER_REQUIRED",
            json!({ "provider": "fake" }),
        ));
    }

    let mut response = routed
        .response
        .parsed_json
        .or_else(|| parse_period_provider_json(&routed.response.raw_text))
        .ok_or_else(|| {
            GenerationError::with_details(
                GenerationErrorCode::PostSafetyValidationFailed,
                format!(
                    "HOROSCOPE_RESPONSE_INVALID: provider_response_not_json raw_text_len={}",
                    routed.response.raw_text.len()
                ),
                json!({
                    "reason": "provider_response_not_json",
                    "raw_text_len": routed.response.raw_text.len()
                }),
            )
        })?;
    if !response
        .get("quality")
        .map_or(false, |value| value.is_object())
    {
        response["quality"] = json!({});
    }
    response["quality"]["provider"] = json!(routed.used_provider.as_str());
    response["quality"]["model"] = json!(routed.response.model_used);
    response["quality"]["fallback_used"] = json!(routed.fallback_used);
    repair_daily_response_shape(request, &mut response);
    repair_premium_daily_editorial_repetition(&mut response);
    Ok(reprocess_horoscope_daily_payload(response))
}

fn horoscope_writer_engine_defaults(use_case: &GenerateReadingUseCase) -> EngineDefaults {
    let mut defaults = use_case.engine_defaults().clone();
    let Some(policy) = use_case.catalog().product_policy(HOROSCOPE_PRODUCT_CODE) else {
        return defaults;
    };
    if let Some(provider) = policy.default_provider.clone() {
        defaults.provider = provider;
    }
    if let Some(model) = policy
        .default_model
        .as_ref()
        .map(|m| m.trim())
        .filter(|m| !m.is_empty())
    {
        defaults.model = model.to_string();
    }
    defaults
}

fn daily_response_provider_schema(request: &Value) -> Result<Value, GenerationError> {
    let schema: Value = serde_json::from_str(RESPONSE_SCHEMA_JSON).map_err(|err| {
        GenerationError::with_details(
            GenerationErrorCode::SchemaValidationFailed,
            format!("HOROSCOPE_RESPONSE_INVALID: {err}"),
            Value::Null,
        )
    })?;
    let service_code = request
        .get("service_code")
        .and_then(Value::as_str)
        .unwrap_or(HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE);
    let branch_index = match service_code {
        HOROSCOPE_FREE_DAILY_SERVICE_CODE => 1,
        HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE => 2,
        _ => 0,
    };
    let branch = schema
        .get("oneOf")
        .and_then(Value::as_array)
        .and_then(|branches| branches.get(branch_index))
        .cloned()
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let mut required = branch.get("required").cloned().unwrap_or_else(|| json!([]));
    if let Some(items) = required.as_array_mut() {
        items.retain(|item| item.as_str() != Some("quality"));
    }
    let mut properties = branch
        .get("properties")
        .cloned()
        .unwrap_or_else(|| json!({}));
    if let Some(object) = properties.as_object_mut() {
        object.remove("quality");
    }
    let mut schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "horoscope_response_v1",
        "definitions": schema.get("definitions").cloned().unwrap_or_else(|| json!({})),
        "type": "object",
        "required": required,
        "additionalProperties": false,
        "properties": properties
    });
    if branch_index == 0 {
        schema["properties"]["watch_points"] = json!({
            "type": "array",
            "items": { "type": "string" }
        });
        schema["properties"]["opportunities"] = json!({
            "type": "array",
            "items": { "type": "string" }
        });
        schema["properties"]["evidence_summary"] = json!({
            "type": "array",
            "items": {
                "type": "object",
                "required": ["evidence_key", "theme_code"],
                "additionalProperties": false,
                "properties": {
                    "evidence_key": { "type": "string" },
                    "theme_code": { "type": "string" }
                }
            }
        });
    }
    if branch_index == 2 {
        schema["properties"]["evidence_summary"] = json!({
            "type": "array",
            "items": {
                "type": "object",
                "required": ["evidence_key", "theme_code"],
                "additionalProperties": false,
                "properties": {
                    "evidence_key": { "type": "string" },
                    "theme_code": { "type": "string" }
                }
            }
        });
    }
    Ok(crate::provider_schema_compiler::prepare_strict_json_schema(
        &schema,
    ))
}

fn daily_writer_messages(request: &Value) -> Result<Vec<PromptMessage>, GenerationError> {
    let compact = serde_json::to_string(request).map_err(|err| {
        GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            format!("HOROSCOPE_RESPONSE_INVALID: {err}"),
            Value::Null,
        )
    })?;
    let service_code = request
        .get("service_code")
        .and_then(Value::as_str)
        .unwrap_or(HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE);
    let slot_instruction = if service_code == HOROSCOPE_FREE_DAILY_SERVICE_CODE {
        "Produis un horoscope quotidien Free sans slots publics, avec summary, advice, watch_point et evidence_keys uniquement. Le texte public doit citer une référence astrologique issue des preuves, par exemple la Lune, Mars, Vénus, Mercure, un transit, un aspect ou une maison."
    } else if service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
        "Produis un horoscope quotidien Premium avec timeline, best_slots, watch_slots, domain_sections et advice. Les 12 entrées timeline doivent avoir des titres et angles rédactionnels distincts. Les reason de best_slots et watch_slots doivent être spécifiques au créneau, jamais copiées-collées entre deux créneaux. Dans domain_sections, garde le champ technique domain tel quel, mais n'écris jamais ce code anglais dans title ou text. Évite les formulations mécaniques répétées comme clarifier, concret, tension, ralentir les réponses ou lire par séquences plus de deux fois dans l'ensemble de la lecture."
    } else {
        "Produis exactement trois slots publics correspondant aux labels Matin, Après-midi et Soir. Chaque slot.text doit citer une référence astrologique publique issue de ses preuves, par exemple la Lune, Mars, Vénus, Mercure, un transit, un aspect ou une maison."
    };

    Ok(vec![
        PromptMessage {
            role: PromptRole::System,
            content: "Tu rédiges un horoscope quotidien personnalisé en français. Retourne uniquement un objet JSON conforme au schéma fourni horoscope_response_v1. N'invente aucune preuve astrologique: chaque evidence_key publique doit provenir de la requête. N'affiche jamais les codes internes, les noms de champs, les clés de preuve, les theme_code anglais, les codes tone anglais, ni les consignes internes.".to_string(),
        },
        PromptMessage {
            role: PromptRole::User,
            content: format!(
                "{slot_instruction} Le résumé doit introduire la tonalité générale sans recopier day_overview. Les textes doivent rester concrets, personnalisés par les signaux fournis, sans promesse événementielle. Utilise uniquement les libellés français déjà fournis pour les titres publics. Requête JSON:\n{compact}"
            ),
        },
    ])
}

fn daily_writer_max_output_tokens(request: &Value) -> u32 {
    match request.get("service_code").and_then(Value::as_str) {
        Some(HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE) => 6000,
        Some(HOROSCOPE_FREE_DAILY_SERVICE_CODE) => 1800,
        _ => 3200,
    }
}

fn repair_daily_response_shape(request: &Value, response: &mut Value) {
    response["contract_version"] = json!("horoscope_response_v1");
    if response
        .get("service_code")
        .and_then(Value::as_str)
        .is_none()
    {
        response["service_code"] = request
            .get("service_code")
            .cloned()
            .unwrap_or_else(|| json!(HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE));
    }
    if response.get("period").is_none() {
        response["period"] = request.get("period").cloned().unwrap_or_else(|| json!({}));
    }
    let service_code = response
        .get("service_code")
        .and_then(Value::as_str)
        .or_else(|| request.get("service_code").and_then(Value::as_str));
    if service_code != Some(HOROSCOPE_FREE_DAILY_SERVICE_CODE)
        && response.get("evidence_summary").is_none()
    {
        response["evidence_summary"] = request
            .get("evidence")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|item| {
                json!({
                    "evidence_key": item.get("evidence_key").cloned().unwrap_or(Value::Null),
                    "theme_code": item.get("theme_code").cloned().unwrap_or(Value::Null)
                })
            })
            .collect::<Vec<_>>()
            .into();
    }
    repair_daily_free_astro_reference(request, response);
    repair_daily_basic_astro_references(request, response);
}

fn repair_premium_daily_editorial_repetition(response: &mut Value) {
    if response.get("service_code").and_then(Value::as_str)
        != Some(HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE)
    {
        return;
    }
    let timeline_text_by_label = response
        .get("timeline")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|slot| {
            let label = slot.get("slot_label")?.as_str()?.to_string();
            let candidates = ["text", "advice", "fallback_reason"]
                .into_iter()
                .filter_map(|key| slot.get(key).and_then(Value::as_str))
                .filter_map(first_public_sentence)
                .collect::<Vec<_>>();
            Some((label, candidates))
        })
        .collect::<HashMap<_, _>>();

    repair_premium_slot_summary_reasons(response, "best_slots", &timeline_text_by_label);
    repair_premium_slot_summary_reasons(response, "watch_slots", &timeline_text_by_label);
}

fn repair_premium_slot_summary_reasons(
    response: &mut Value,
    field: &str,
    timeline_text_by_label: &HashMap<String, Vec<String>>,
) {
    let Some(slots) = response.get_mut(field).and_then(Value::as_array_mut) else {
        return;
    };
    let mut used = HashSet::new();
    for slot in slots {
        let reason = slot
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let normalized = normalize_editorial_sentence(&reason);
        if normalized.is_empty() || used.insert(normalized) {
            continue;
        }
        let Some(label) = slot.get("slot_label").and_then(Value::as_str) else {
            continue;
        };
        let Some(replacement) = timeline_text_by_label.get(label).and_then(|candidates| {
            candidates
                .iter()
                .find(|candidate| used.insert(normalize_editorial_sentence(candidate)))
        }) else {
            continue;
        };
        slot["reason"] = json!(replacement);
    }
}

fn first_public_sentence(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let end = trimmed
        .char_indices()
        .find_map(|(idx, ch)| matches!(ch, '.' | '!' | '?').then_some(idx + ch.len_utf8()))
        .unwrap_or(trimmed.len());
    let sentence = trimmed[..end].trim();
    if sentence.split_whitespace().count() < 5 {
        None
    } else {
        Some(sentence.to_string())
    }
}

fn normalize_editorial_sentence(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn repair_daily_free_astro_reference(request: &Value, response: &mut Value) {
    if request.get("service_code").and_then(Value::as_str)
        != Some(HOROSCOPE_FREE_DAILY_SERVICE_CODE)
    {
        return;
    }
    let public_text = free_public_text(response);
    if daily_text_has_astrological_reference(&public_text) {
        return;
    }
    let Some(prefix) = daily_response_astro_reference_prefix(request, response) else {
        return;
    };
    let current = response
        .get("summary")
        .and_then(|summary| summary.get("text"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    response["summary"]["text"] = if current.is_empty() {
        json!(prefix)
    } else {
        json!(format!("{prefix} {current}"))
    };
}

fn repair_daily_basic_astro_references(request: &Value, response: &mut Value) {
    if request.get("service_code").and_then(Value::as_str)
        != Some(HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE)
    {
        return;
    }
    let Some(slots) = response.get_mut("slots").and_then(Value::as_array_mut) else {
        return;
    };
    for slot in slots {
        let text = slot.get("text").and_then(Value::as_str).unwrap_or("");
        if daily_text_has_astrological_reference(text) {
            continue;
        }
        let Some(prefix) = daily_response_astro_reference_prefix(request, slot) else {
            continue;
        };
        let repaired = if text.trim().is_empty() {
            prefix
        } else {
            format!("{prefix} {}", text.trim())
        };
        slot["text"] = json!(repaired);
    }
}

fn daily_text_has_astrological_reference(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "lune",
        "mars",
        "vénus",
        "venus",
        "mercure",
        "aspect",
        "maison",
        "transit",
        "astrologique",
        "natal",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn daily_response_astro_reference_prefix(request: &Value, response: &Value) -> Option<String> {
    let evidence_keys = response
        .get("evidence_keys")
        .or_else(|| response.get("required_evidence_keys"))
        .and_then(Value::as_array)?;
    let first_key = evidence_keys.iter().find_map(Value::as_str)?;
    let evidence = request.get("evidence").and_then(Value::as_array)?;
    let signal = evidence
        .iter()
        .find(|item| item.get("evidence_key").and_then(Value::as_str) == Some(first_key))?;
    let object = signal
        .get("transiting_object")
        .and_then(Value::as_str)
        .map(public_astro_object_label)
        .unwrap_or("Un transit");
    if object == "Un transit" {
        Some("Un transit astrologique donne le repère du créneau.".to_string())
    } else {
        Some(format!("{object} donne le repère astrologique du créneau."))
    }
}

fn public_astro_object_label(code: &str) -> &'static str {
    match code {
        "sun" => "Le Soleil",
        "moon" => "La Lune",
        "mercury" => "Mercure",
        "venus" => "Vénus",
        "mars" => "Mars",
        "jupiter" => "Jupiter",
        "saturn" => "Saturne",
        "uranus" => "Uranus",
        "neptune" => "Neptune",
        "pluto" => "Pluton",
        _ => "Un transit",
    }
}

fn premium_slot_summary(slot: &Value, watch: bool) -> Value {
    let label = slot
        .get("slot_label")
        .cloned()
        .unwrap_or_else(|| json!("Moment"));
    let evidence_keys = slot
        .get("required_evidence_keys")
        .cloned()
        .unwrap_or_else(|| json!([]));
    json!({
        "slot_label": label,
        "title": if watch { "Créneau de vigilance" } else { "Créneau favorable" },
        "reason": premium_slot_summary_reason(slot, watch),
        "best_for": slot.get("best_for").cloned().unwrap_or_else(|| json!([])),
        "avoid": if watch { json!(["réponse impulsive"]) } else { json!([]) },
        "evidence_keys": evidence_keys
    })
}

fn premium_slot_summary_reason(slot: &Value, watch: bool) -> String {
    let label = slot
        .get("slot_label")
        .and_then(Value::as_str)
        .unwrap_or("ce créneau");
    if watch {
        format!("{label} demande de filtrer les réactions et de garder une réponse proportionnée.")
    } else {
        format!("{label} soutient une action simple, utile et facile à vérifier.")
    }
}

fn build_domain_sections(request: &Value) -> Vec<Value> {
    let evidence = request
        .get("evidence")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .take(4)
        .filter_map(|item| item.get("evidence_key").and_then(|v| v.as_str()))
        .map(str::to_string)
        .collect::<Vec<_>>();
    premium_domain_rows()
        .unwrap_or_default()
        .into_iter()
        .map(|(domain, title)| {
            json!({
                "domain": domain,
                "title": title,
                "text": "Cette section relie les meilleurs repères horaires aux preuves astrologiques retenues, sans promettre d'événement.",
                "evidence_keys": evidence
            })
        })
        .collect()
}

fn premium_domain_rows() -> Result<Vec<(String, String)>, GenerationError> {
    let mut rows = rows(DOMAIN_SCORE_MAPPINGS_JSON)?
        .into_iter()
        .filter(|row| {
            row.get("service_code").and_then(|v| v.as_str())
                == Some(HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE)
        })
        .filter_map(|row| {
            Some((
                row.get("domain_code")?.as_str()?.to_string(),
                row.get("domain_title")?.as_str()?.to_string(),
                row.get("sort_order")?.as_i64()?,
            ))
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|(_, _, sort_order)| *sort_order);
    Ok(rows
        .into_iter()
        .map(|(domain, title, _)| (domain, title))
        .collect())
}

pub fn validate_response_evidence(
    request: &Value,
    response: &Value,
) -> Result<(), GenerationError> {
    validate_horoscope_response_schema(response)?;
    let service_code = request
        .get("service_code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    if response.get("contract_version").and_then(|v| v.as_str()) != Some("horoscope_response_v1")
        || response.get("service_code").and_then(|v| v.as_str()) != Some(service_code)
    {
        return Err(horoscope_error("HOROSCOPE_RESPONSE_INVALID"));
    }
    let allowed = request
        .get("evidence")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("evidence_key").and_then(|v| v.as_str()))
        .collect::<HashSet<_>>();
    let request_slots = request
        .get("slots")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    if allowed.is_empty() {
        return Err(horoscope_error("HOROSCOPE_RESPONSE_INVALID"));
    }
    if service_code == HOROSCOPE_FREE_DAILY_SERVICE_CODE {
        return validate_free_response_evidence(request, response, &allowed);
    }
    if service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
        return validate_premium_response_evidence(request, response, &allowed);
    }
    if service_code != HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE {
        return Err(horoscope_error("HOROSCOPE_RESPONSE_INVALID"));
    }
    let slots = response
        .get("slots")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    if slots.len() != 3 {
        return Err(horoscope_error("HOROSCOPE_RESPONSE_INVALID"));
    }
    validate_day_overview_not_copied(request, slots)?;
    let mut texts = Vec::new();
    let mut advices = Vec::new();
    let mut best_for_sets = Vec::new();
    for slot in slots {
        let slot_code = slot
            .get("slot_code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
        let keys = slot
            .get("evidence_keys")
            .and_then(|v| v.as_array())
            .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
        if keys.is_empty() {
            return Err(GenerationError::with_details(
                GenerationErrorCode::PostSafetyValidationFailed,
                "HOROSCOPE_EVIDENCE_MISMATCH",
                json!({ "reason": "slot_without_evidence" }),
            ));
        }
        if keys.iter().any(|key| key.as_str().is_none()) {
            return Err(GenerationError::with_details(
                GenerationErrorCode::PostSafetyValidationFailed,
                "HOROSCOPE_EVIDENCE_MISMATCH",
                json!({ "reason": "non_string_evidence_key" }),
            ));
        }
        let request_slot = request_slots
            .iter()
            .find(|item| item.get("slot_code").and_then(|v| v.as_str()) == Some(slot_code))
            .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
        validate_slot_specificity(request_slot)?;
        validate_slot_evidence_alignment(request_slot, keys)?;
        validate_public_slot_text(slot)?;
        let text = slot.get("text").and_then(|v| v.as_str()).unwrap_or("");
        validate_astrological_reference(slot_code, text, request_slot)?;
        texts.push(text.to_string());
        advices.push(
            slot.get("advice")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        );
        best_for_sets.push(
            slot.get("best_for")
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str())
                .map(str::to_string)
                .collect::<Vec<_>>(),
        );
    }
    validate_slot_diversity(&texts)?;
    validate_distinct_strings(&advices, "HOROSCOPE_SLOT_ADVICE_DUPLICATED")?;
    validate_distinct_best_for(&best_for_sets)?;
    let mut cited = Vec::new();
    collect_evidence_keys(response, &mut cited);
    let invented = cited
        .into_iter()
        .filter(|key| !allowed.contains(key.as_str()))
        .collect::<Vec<_>>();
    if invented.is_empty() {
        Ok(())
    } else {
        Err(GenerationError::with_details(
            GenerationErrorCode::PostSafetyValidationFailed,
            "HOROSCOPE_EVIDENCE_MISMATCH",
            json!({ "invented_evidence_keys": invented }),
        ))
    }
}

fn validate_premium_calculation_local_chart(
    service_code: &str,
    calculation: &Value,
) -> Result<(), GenerationError> {
    if service_code != HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
        return Ok(());
    }
    let slots = calculation
        .get("slots")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_CALCULATION_FAILED"))?;
    if slots.len() != 12 {
        return Err(quality_error(
            "HOROSCOPE_PREMIUM_TIMELINE_MISSING",
            json!({ "reason": "premium_calculation_must_have_12_slots" }),
        ));
    }
    for slot in slots {
        let local_chart = slot
            .get("local_chart")
            .and_then(|v| v.as_object())
            .ok_or_else(|| quality_error("HOROSCOPE_PREMIUM_LOCAL_CHART_MISSING", Value::Null))?;
        if !local_chart.contains_key("ascendant")
            || !local_chart.contains_key("midheaven")
            || !local_chart.contains_key("houses")
        {
            return Err(quality_error(
                "HOROSCOPE_PREMIUM_LOCAL_CHART_MISSING",
                json!({ "reason": "ascendant_midheaven_or_houses_missing" }),
            ));
        }
        if !local_chart
            .get("ascendant")
            .and_then(|v| v.as_object())
            .is_some_and(|angle| angle.contains_key("sign") && angle.contains_key("longitude_deg"))
            || !local_chart
                .get("midheaven")
                .and_then(|v| v.as_object())
                .is_some_and(|angle| {
                    angle.contains_key("sign") && angle.contains_key("longitude_deg")
                })
            || local_chart
                .get("houses")
                .and_then(|v| v.as_array())
                .map(|houses| houses.len() != 12)
                .unwrap_or(true)
        {
            return Err(quality_error(
                "HOROSCOPE_PREMIUM_LOCAL_CHART_MISSING",
                json!({ "reason": "local_chart_shape_invalid" }),
            ));
        }
    }
    Ok(())
}

fn fake_writer_response(request: &Value) -> Result<Value, GenerationError> {
    let service_code = request
        .get("service_code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    if service_code == HOROSCOPE_FREE_DAILY_SERVICE_CODE {
        return fake_writer_free_response(request);
    }
    if service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
        return fake_writer_premium_response(request);
    }
    let period = request
        .get("period")
        .cloned()
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let evidence = request
        .get("evidence")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let slots = request
        .get("slots")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let rendered_slots = slots
        .iter()
        .map(render_fake_slot)
        .collect::<Result<Vec<_>, _>>()?;
    let response = json!({
        "contract_version": "horoscope_response_v1",
        "service_code": HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE,
        "period": period,
        "summary": {
            "title": "Une journée à ajuster avec précision",
            "text": "La journée avance en trois temps distincts : organiser le cadre, ralentir les réactions, puis rouvrir une parole plus souple. Les preuves astrologiques retenues dessinent une progression concrète sans transformer le climat du jour en promesse événementielle."
        },
        "slots": rendered_slots,
        "watch_points": ["Réactivité émotionnelle en milieu de journée"],
        "opportunities": ["Conversation plus fluide en fin de journée"],
        "evidence_summary": evidence.iter().map(|item| json!({
            "evidence_key": item.get("evidence_key").cloned().unwrap_or(Value::Null),
            "theme_code": item.get("theme_code").cloned().unwrap_or(Value::Null)
        })).collect::<Vec<_>>(),
        "quality": {
            "provider": "fake",
            "evidence_guard": "passed",
            "evidence_coverage": 1.0,
            "slot_diversity_passed": true,
            "french_typography_passed": true,
            "generic_language_passed": true
        }
    });
    Ok(reprocess_horoscope_daily_payload(response))
}

fn validate_free_response_evidence(
    request: &Value,
    response: &Value,
    allowed: &HashSet<&str>,
) -> Result<(), GenerationError> {
    if response.get("slots").is_some() {
        return Err(horoscope_error("HOROSCOPE_RESPONSE_INVALID"));
    }
    let request_slots = request
        .get("slots")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    if request_slots.len() != 1
        || request_slots[0].get("slot_code").and_then(|v| v.as_str()) != Some("day")
    {
        return Err(horoscope_error("HOROSCOPE_RESPONSE_INVALID"));
    }
    let evidence_keys = response
        .get("evidence_keys")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    if evidence_keys.is_empty() {
        return Err(quality_error(
            "HOROSCOPE_EVIDENCE_MISMATCH",
            json!({ "reason": "free_without_evidence" }),
        ));
    }
    validate_slot_evidence_alignment(&request_slots[0], evidence_keys)?;

    let mut cited = Vec::new();
    collect_evidence_keys(response, &mut cited);
    let invented = cited
        .into_iter()
        .filter(|key| !allowed.contains(key.as_str()))
        .collect::<Vec<_>>();
    if !invented.is_empty() {
        return Err(GenerationError::with_details(
            GenerationErrorCode::PostSafetyValidationFailed,
            "HOROSCOPE_EVIDENCE_MISMATCH",
            json!({ "invented_evidence_keys": invented }),
        ));
    }

    let public_text = free_public_text(response);
    validate_public_text_no_technical_codes(&public_text)?;
    validate_free_text_quality(&public_text, response)?;
    validate_astrological_reference("day", &public_text, &request_slots[0])?;
    Ok(())
}

fn fake_writer_premium_response(request: &Value) -> Result<Value, GenerationError> {
    let period = request
        .get("period")
        .cloned()
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let slots = request
        .get("slots")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let timeline = slots
        .iter()
        .enumerate()
        .map(|(idx, slot)| render_fake_premium_timeline_slot(slot, idx))
        .collect::<Result<Vec<_>, _>>()?;
    let best_slots = request
        .get("best_slots")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let watch_slots = request
        .get("watch_slots")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let domain_sections = request
        .get("domain_sections")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .map(|section| {
            json!({
                "domain": section.get("domain").cloned().unwrap_or_else(|| json!("daily")),
                "title": section.get("title").cloned().unwrap_or_else(|| json!("Repères du jour")),
                "text": "Les preuves astrologiques retenues donnent un repère pratique pour organiser ce domaine sans annoncer d'événement certain.",
                "evidence_keys": section.get("evidence_keys").cloned().unwrap_or_else(|| json!([]))
            })
        })
        .collect::<Vec<_>>();
    let evidence = request
        .get("evidence")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .map(|item| {
            json!({
                "evidence_key": item.get("evidence_key").cloned().unwrap_or(Value::Null),
                "theme_code": item.get("theme_code").cloned().unwrap_or(Value::Null)
            })
        })
        .collect::<Vec<_>>();

    let response = json!({
        "contract_version": "horoscope_response_v1",
        "service_code": HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
        "period": period,
        "summary": {
            "title": "Votre météo astrologique détaillée",
            "text": "La journée se lit par créneaux courts : certains moments favorisent l'organisation, d'autres demandent de ralentir la réponse émotionnelle. Les repères ci-dessous s'appuient sur les preuves astrologiques sélectionnées et restent des indications pratiques, non des promesses d'événements."
        },
        "best_slots": best_slots,
        "watch_slots": watch_slots,
        "timeline": timeline,
        "domain_sections": domain_sections,
        "advice": {
            "main": "Utilisez les créneaux les plus fluides pour les décisions concrètes et gardez les moments tendus pour observer avant d'agir.",
            "best_use": "Planifier, prioriser et formuler les échanges importants quand la tonalité est plus claire.",
            "avoid": "Transformer un signal bref en certitude ou répondre trop vite pendant un créneau de vigilance."
        },
        "evidence_summary": evidence,
        "quality": {
            "provider": "fake",
            "evidence_guard": "passed",
            "timeline_count": 12,
            "timeline_order_passed": true,
            "premium_rich_bounds": "fake_structural_only"
        }
    });
    Ok(reprocess_horoscope_daily_payload(response))
}

fn render_fake_premium_timeline_slot(slot: &Value, index: usize) -> Result<Value, GenerationError> {
    let label = slot
        .get("slot_label")
        .and_then(|v| v.as_str())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let evidence_keys = slot
        .get("required_evidence_keys")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let tone = slot.get("tone").and_then(|v| v.as_str()).unwrap_or("mixed");
    let best_for = slot
        .get("best_for")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let theme_code = slot
        .get("theme_code")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let internal_watch_point = slot.get("watch_point").and_then(|v| v.as_str());
    let watch_point = public_watch_point_for_theme(theme_code)?
        .or_else(|| {
            internal_watch_point.and_then(|value| {
                if value.contains("avoid_") {
                    None
                } else {
                    Some(value.to_string())
                }
            })
        })
        .unwrap_or_else(|| "Gardez un repère simple et vérifiable.".to_string());
    Ok(json!({
        "slot_label": label,
        "title": premium_timeline_title(index),
        "theme": premium_timeline_theme(index),
        "tone": tone,
        "text": premium_timeline_text(index),
        "advice": premium_timeline_advice(index),
        "best_for": best_for,
        "watch_point": watch_point,
        "evidence_keys": evidence_keys
    }))
}

fn premium_timeline_title(index: usize) -> &'static str {
    match index % 4 {
        0 => "Clarté pratique",
        1 => "Rythme à canaliser",
        2 => "Réactivité à modérer",
        _ => "Dialogue à simplifier",
    }
}

fn premium_timeline_theme(index: usize) -> &'static str {
    match index % 4 {
        0 => "Organisation",
        1 => "Énergie",
        2 => "Émotion",
        _ => "Relation",
    }
}

fn premium_timeline_text(index: usize) -> &'static str {
    match index % 4 {
        0 => "La Lune donne un repère concret pour organiser une priorité sans disperser l'attention.",
        1 => "Le climat du créneau soutient une action courte, à condition de garder un cadre mesurable.",
        2 => "Mars rend la réaction plus vive : mieux vaut vérifier le détail avant de répondre.",
        _ => "Vénus adoucit l'échange si vous revenez à un sujet précis plutôt qu'à toute l'histoire.",
    }
}

fn premium_timeline_advice(index: usize) -> &'static str {
    match index % 4 {
        0 => "Choisissez une tâche utile et terminez-la avant d'en ouvrir une autre.",
        1 => "Gardez le mouvement, mais limitez le nombre de décisions simultanées.",
        2 => "Respirez avant de répondre et reformulez ce qui manque.",
        _ => "Préférez une phrase simple à une explication trop longue.",
    }
}

fn validate_premium_response_evidence(
    request: &Value,
    response: &Value,
    allowed: &HashSet<&str>,
) -> Result<(), GenerationError> {
    let timeline = response
        .get("timeline")
        .and_then(|v| v.as_array())
        .ok_or_else(|| quality_error("HOROSCOPE_PREMIUM_TIMELINE_MISSING", Value::Null))?;
    let request_slots = request
        .get("slots")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    if timeline.len() != 12 || request_slots.len() != 12 {
        return Err(quality_error(
            "HOROSCOPE_PREMIUM_TIMELINE_MISSING",
            json!({ "reason": "timeline_must_have_exactly_12_entries" }),
        ));
    }
    for (idx, (response_slot, request_slot)) in timeline.iter().zip(request_slots).enumerate() {
        let expected_label = request_slot
            .get("slot_label")
            .and_then(|v| v.as_str())
            .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
        let received_label = response_slot
            .get("slot_label")
            .and_then(|v| v.as_str())
            .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
        if received_label != expected_label {
            return Err(quality_error(
                "HOROSCOPE_PREMIUM_TIMELINE_MISSING",
                json!({
                    "reason": "timeline_label_order_mismatch",
                    "index": idx,
                    "expected": expected_label,
                    "received": received_label
                }),
            ));
        }
        validate_public_slot_text(response_slot)?;
        let keys = response_slot
            .get("evidence_keys")
            .and_then(|v| v.as_array())
            .ok_or_else(|| quality_error("HOROSCOPE_PREMIUM_SLOT_EVIDENCE_MISSING", Value::Null))?;
        if keys.is_empty() {
            return Err(quality_error(
                "HOROSCOPE_PREMIUM_SLOT_EVIDENCE_MISSING",
                json!({ "slot_label": expected_label }),
            ));
        }
        validate_slot_evidence_alignment(request_slot, keys)?;
    }

    let request_by_label = request_slots
        .iter()
        .filter_map(|slot| Some((slot.get("slot_label")?.as_str()?, slot)))
        .collect::<HashMap<_, _>>();
    let best = response
        .get("best_slots")
        .and_then(|v| v.as_array())
        .ok_or_else(|| quality_error("HOROSCOPE_PREMIUM_BEST_SLOTS_MISSING", Value::Null))?;
    let watch = response
        .get("watch_slots")
        .and_then(|v| v.as_array())
        .ok_or_else(|| quality_error("HOROSCOPE_PREMIUM_WATCH_SLOTS_MISSING", Value::Null))?;
    if best.is_empty() {
        return Err(quality_error(
            "HOROSCOPE_PREMIUM_BEST_SLOTS_MISSING",
            Value::Null,
        ));
    }
    if watch.is_empty() {
        return Err(quality_error(
            "HOROSCOPE_PREMIUM_WATCH_SLOTS_MISSING",
            Value::Null,
        ));
    }
    validate_premium_slot_summaries(best, &request_by_label, "best_slots")?;
    validate_premium_slot_summaries(watch, &request_by_label, "watch_slots")?;
    validate_premium_slot_summary_reason_diversity(best, "best_slots")?;
    validate_premium_slot_summary_reason_diversity(watch, "watch_slots")?;
    let best_labels = best
        .iter()
        .filter_map(|slot| slot.get("slot_label").and_then(|v| v.as_str()))
        .collect::<HashSet<_>>();
    let watch_labels = watch
        .iter()
        .filter_map(|slot| slot.get("slot_label").and_then(|v| v.as_str()))
        .collect::<HashSet<_>>();
    if best_labels.iter().any(|label| watch_labels.contains(label)) {
        return Err(quality_error(
            "HOROSCOPE_PREMIUM_CONTRADICTORY_SLOT_CLASSIFICATION",
            json!({ "reason": "slot_in_best_and_watch" }),
        ));
    }

    let domain_sections = response
        .get("domain_sections")
        .and_then(|v| v.as_array())
        .ok_or_else(|| quality_error("HOROSCOPE_PREMIUM_DOMAIN_SECTION_MISSING", Value::Null))?;
    if domain_sections.is_empty() {
        return Err(quality_error(
            "HOROSCOPE_PREMIUM_DOMAIN_SECTION_MISSING",
            Value::Null,
        ));
    }

    let mut cited = Vec::new();
    collect_evidence_keys(response, &mut cited);
    let invented = cited
        .into_iter()
        .filter(|key| !allowed.contains(key.as_str()))
        .collect::<Vec<_>>();
    if invented.is_empty() {
        Ok(())
    } else {
        Err(GenerationError::with_details(
            GenerationErrorCode::PostSafetyValidationFailed,
            "HOROSCOPE_EVIDENCE_MISMATCH",
            json!({ "invented_evidence_keys": invented }),
        ))
    }
}

fn validate_premium_slot_summaries(
    slots: &[Value],
    request_by_label: &HashMap<&str, &Value>,
    field: &str,
) -> Result<(), GenerationError> {
    let mut seen = HashSet::new();
    for slot in slots {
        let label = slot
            .get("slot_label")
            .and_then(|v| v.as_str())
            .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
        if !seen.insert(label) {
            return Err(quality_error(
                "HOROSCOPE_PREMIUM_DUPLICATED_SLOT_CLASSIFICATION",
                json!({ "field": field, "slot_label": label }),
            ));
        }
        let request_slot = request_by_label.get(label).ok_or_else(|| {
            quality_error(
                "HOROSCOPE_PREMIUM_UNKNOWN_SLOT_CLASSIFICATION",
                json!({ "field": field, "slot_label": label }),
            )
        })?;
        let keys = slot
            .get("evidence_keys")
            .and_then(|v| v.as_array())
            .ok_or_else(|| quality_error("HOROSCOPE_PREMIUM_SLOT_EVIDENCE_MISSING", Value::Null))?;
        if keys.is_empty() {
            return Err(quality_error(
                "HOROSCOPE_PREMIUM_SLOT_EVIDENCE_MISSING",
                json!({ "field": field, "slot_label": label }),
            ));
        }
        validate_slot_evidence_alignment(request_slot, keys)?;
        validate_premium_summary_public_text(slot)?;
    }
    Ok(())
}

fn validate_premium_slot_summary_reason_diversity(
    slots: &[Value],
    field: &str,
) -> Result<(), GenerationError> {
    let mut seen = HashSet::new();
    for slot in slots {
        let reason = slot.get("reason").and_then(Value::as_str).unwrap_or("");
        let normalized = normalize_editorial_sentence(reason);
        if normalized.is_empty() || seen.insert(normalized) {
            continue;
        }
        return Err(quality_error(
            "HOROSCOPE_PREMIUM_REPETITIVE_SLOT_REASON",
            json!({ "field": field, "reason": reason }),
        ));
    }
    Ok(())
}

fn validate_premium_summary_public_text(slot: &Value) -> Result<(), GenerationError> {
    let mut public_text = String::new();
    for key in ["slot_label", "title", "reason"] {
        if let Some(value) = slot.get(key).and_then(|v| v.as_str()) {
            public_text.push_str(value);
            public_text.push('\n');
        }
    }
    for key in ["best_for", "avoid"] {
        for value in slot
            .get(key)
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str())
        {
            public_text.push_str(value);
            public_text.push('\n');
        }
    }
    validate_public_text_no_technical_codes(&public_text)
}

fn fake_writer_free_response(request: &Value) -> Result<Value, GenerationError> {
    let period = request
        .get("period")
        .cloned()
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let slot = request
        .get("slots")
        .and_then(|v| v.as_array())
        .and_then(|items| items.first())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let evidence_keys = slot
        .get("required_evidence_keys")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let response = json!({
        "contract_version": "horoscope_response_v1",
        "service_code": HOROSCOPE_FREE_DAILY_SERVICE_CODE,
        "period": period,
        "summary": {
            "title": "Votre tendance du jour",
            "text": "La Lune met l'accent sur l'organisation, les priorités simples et les gestes utiles. La journée gagne à rester concrète : choisir une tâche mesurable, clarifier ce qui doit vraiment avancer, puis éviter de multiplier les intentions."
        },
        "advice": "Choisissez une action vérifiable et avancez étape par étape.",
        "watch_point": "Ne cherchez pas à tout régler en même temps.",
        "evidence_keys": evidence_keys,
        "quality": {
            "provider": "fake",
            "evidence_guard": "passed",
            "evidence_coverage": 1.0,
            "slot_diversity_passed": "not_applicable",
            "french_typography_passed": true,
            "generic_language_passed": true
        }
    });
    Ok(reprocess_horoscope_daily_payload(response))
}

fn score_fact(
    refs: &ReferenceData,
    slot_id: &str,
    fact: &Value,
) -> Result<ScoredSignal, GenerationError> {
    let evidence_key = fact_string(fact, "evidence_key")?;
    let transiting_object = fact_string(fact, "transiting_object")?;
    if !refs.supported_objects.contains(&transiting_object) {
        return Err(horoscope_error("HOROSCOPE_SCORING_FAILED"));
    }
    let natal_target = fact
        .get("natal_target")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let aspect = fact
        .get("aspect")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    if let Some(aspect) = &aspect {
        if !refs.supported_aspects.contains(aspect) {
            return Err(horoscope_error("HOROSCOPE_SCORING_FAILED"));
        }
    }
    let orb_deg = fact.get("orb_deg").and_then(|v| v.as_f64());
    let object_weight = refs.weight(&refs.object_weights, &transiting_object);
    let target_weight = natal_target
        .as_deref()
        .map(|target| refs.weight(&refs.target_weights, target))
        .unwrap_or(1.0);
    let aspect_weight = aspect
        .as_deref()
        .map(|aspect| refs.weight(&refs.aspect_weights, aspect))
        .unwrap_or(1.0);
    let orb_weight = refs.orb_weight(orb_deg.unwrap_or(6.0));
    let exactness_bonus = if orb_deg.unwrap_or(9.0) <= refs.scoring.exact_orb_bonus_max_deg {
        refs.scoring.exactness_bonus
    } else {
        0.0
    };
    let priority_score =
        object_weight * target_weight * aspect_weight * orb_weight + exactness_bonus;
    let theme_code = refs.theme_for(
        &transiting_object,
        aspect.as_deref(),
        natal_target.as_deref(),
    );
    let tone = aspect
        .as_deref()
        .and_then(|aspect| refs.aspect_tones.get(aspect))
        .cloned()
        .unwrap_or_else(|| "mixed".into());

    Ok(ScoredSignal {
        evidence_key,
        fact_type: fact
            .get("fact_type")
            .and_then(|v| v.as_str())
            .unwrap_or("transit_to_natal")
            .into(),
        slot_id: slot_id.into(),
        source: fact
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("calculator")
            .into(),
        transiting_object,
        natal_target,
        aspect,
        orb_deg,
        natal_house: fact.get("natal_house").and_then(|v| v.as_i64()),
        theme_code,
        priority_score: round2(priority_score),
        intensity: refs.intensity(priority_score),
        tone,
        duration_class: refs.scoring.default_duration_class.clone(),
        confidence_score: refs.scoring.default_confidence_score,
        human_label: "Preuve astrologique retenue pour l'horoscope quotidien".into(),
        score_breakdown: json!({
            "transiting_object_weight": object_weight,
            "natal_target_weight": target_weight,
            "aspect_weight": aspect_weight,
            "orb_weight": orb_weight,
            "house_weight": refs.scoring.default_house_weight,
            "theme_repetition_bonus": 0.0,
            "exactness_bonus": exactness_bonus,
            "weak_signal_penalty": 0.0
        }),
    })
}

fn build_slot_plans(
    refs: &ReferenceData,
    calculation: &Value,
    signals: &[ScoredSignal],
) -> Result<Vec<SlotInterpretationPlan>, GenerationError> {
    let slots = calculation
        .get("slots")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_CALCULATION_FAILED"))?;
    let mut plans = Vec::new();
    for slot in slots {
        let slot_code = slot
            .get("slot_code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| horoscope_error("HOROSCOPE_CALCULATION_FAILED"))?;
        let public_label = refs.slot_label(slot_code);
        let mut slot_signals = signals
            .iter()
            .filter(|signal| {
                signal.slot_id == slot_code
                    && signal.priority_score >= refs.shortlist.min_priority_score
            })
            .cloned()
            .collect::<Vec<_>>();
        slot_signals.sort_by(|a, b| {
            b.priority_score
                .partial_cmp(&a.priority_score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.evidence_key.cmp(&b.evidence_key))
        });
        slot_signals.truncate(refs.shortlist.max_main_signals_per_slot);

        if slot_signals.is_empty() {
            plans.push(SlotInterpretationPlan {
                slot_code: slot_code.to_string(),
                slot_label: public_label,
                specificity: "fallback".into(),
                theme_code: None,
                tone: None,
                intensity: None,
                main_signal_keys: Vec::new(),
                required_evidence_keys: Vec::new(),
                advice_axis: None,
                avoid_axis: None,
                watch_point: None,
                best_for: Vec::new(),
                fallback_reason: Some("no_slot_specific_signal_above_threshold".into()),
            });
            continue;
        }

        let primary = &slot_signals[0];
        let axis = refs.advice_axis(&primary.theme_code);
        let evidence_keys = slot_signals
            .iter()
            .take(refs.shortlist.max_required_evidence_per_slot)
            .map(|signal| signal.evidence_key.clone())
            .collect::<Vec<_>>();
        plans.push(SlotInterpretationPlan {
            slot_code: slot_code.to_string(),
            slot_label: public_label,
            specificity: "specific".into(),
            theme_code: Some(primary.theme_code.clone()),
            tone: Some(
                axis.tone_hint
                    .clone()
                    .unwrap_or_else(|| primary.tone.clone()),
            ),
            intensity: Some(primary.intensity.clone()),
            main_signal_keys: evidence_keys.clone(),
            required_evidence_keys: evidence_keys,
            advice_axis: Some(axis.advice_axis.clone()),
            avoid_axis: axis.avoid_axis.clone(),
            watch_point: axis.watch_point.clone(),
            best_for: axis.best_for.clone(),
            fallback_reason: None,
        });
    }
    let expected = if refs.service_code == HOROSCOPE_FREE_DAILY_SERVICE_CODE {
        1
    } else if refs.service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
        12
    } else {
        3
    };
    if plans.len() != expected {
        return Err(horoscope_error("HOROSCOPE_CALCULATION_FAILED"));
    }
    Ok(plans)
}

fn render_fake_slot(slot: &Value) -> Result<Value, GenerationError> {
    let slot_code = slot
        .get("slot_code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let title = slot
        .get("slot_label")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| slot_label(slot_code));
    let theme_code = slot
        .get("theme_code")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let tone = slot.get("tone").and_then(|v| v.as_str()).unwrap_or("mixed");
    let evidence_keys = slot
        .get("required_evidence_keys")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let best_for = slot
        .get("best_for")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let internal_watch_point = slot
        .get("watch_point")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let watch_point = public_watch_point_for_theme(theme_code)?
        .or_else(|| {
            if internal_watch_point.contains("avoid_") {
                None
            } else {
                Some(internal_watch_point.to_string())
            }
        })
        .unwrap_or_default();
    let (theme, text, advice) = match slot_code {
        "morning" => (
            "Organisation",
            "La Lune met l'accent sur l'organisation et les gestes utiles. C'est un bon moment pour clarifier une priorité concrète, ranger une tâche ou reprendre un point simple sans ouvrir trop de sujets à la fois.",
            "Choisissez une action vérifiable et terminez-la avant de passer à la suivante.",
        ),
        "afternoon" => (
            "Limites émotionnelles",
            "Un contact tendu entre Mars et la Lune natale peut rendre l'après-midi plus réactif. Ce créneau demande de ralentir les réponses, surtout si une discussion devient imprécise ou chargée.",
            "Si une tension monte, reformulez d'abord ce que vous avez compris avant de répondre.",
        ),
        "evening" => (
            "Dialogue",
            "Vénus soutient Mercure natal et adoucit le climat relationnel du soir. L'enjeu n'est pas de tout résoudre, mais de rouvrir un échange simple, concret et moins défensif.",
            "Revenez sur un point précis plutôt que sur toute l'histoire.",
        ),
        _ => (
            "Repère du jour",
            "Le climat astrologique du slot donne un repère simple pour ajuster le rythme sans surinterpréter la journée.",
            "Gardez une action courte, observable et reliée au moment.",
        ),
    };
    Ok(json!({
        "slot_code": slot_code,
        "title": title,
        "theme": if theme_code.is_empty() { theme } else { theme },
        "tone": tone,
        "text": text,
        "advice": advice,
        "best_for": best_for,
        "watch_point": watch_point,
        "evidence_keys": evidence_keys
    }))
}

pub fn public_watch_point_for_theme(theme_code: &str) -> Result<Option<String>, GenerationError> {
    if theme_code.trim().is_empty() {
        return Ok(None);
    }
    Ok(advice_axes()?
        .get(theme_code)
        .and_then(|axis| axis.public_watch_point.clone()))
}

#[derive(Clone)]
struct ReferenceData {
    service_code: String,
    service_profile: ServiceProfile,
    slot_profiles: Vec<SlotProfile>,
    object_weights: HashMap<String, f64>,
    target_weights: HashMap<String, f64>,
    aspect_weights: HashMap<String, f64>,
    aspect_tones: HashMap<String, String>,
    supported_objects: HashSet<String>,
    supported_aspects: HashSet<String>,
    orb_bands: Vec<OrbBand>,
    theme_mappings: Vec<ThemeMapping>,
    intensity_bands: Vec<IntensityBand>,
    duration_classes: HashSet<String>,
    advice_axes: HashMap<String, ThemeAdviceAxis>,
    scoring: ScoringParameters,
    shortlist: ShortlistProfile,
}

#[derive(Clone)]
struct ServiceProfile {
    house_system_code: Option<String>,
    period_profile_code: Option<String>,
    detail_profile_code: Option<String>,
    scan_profile_code: Option<String>,
}

#[derive(Clone)]
struct ScanProfile {
    granularity: String,
    reference_time_local: String,
    expected_snapshots_per_day: usize,
}

#[derive(Clone)]
struct OrbBand {
    min: f64,
    max: f64,
    weight: f64,
}

#[derive(Clone)]
struct IntensityBand {
    code: String,
    min: f64,
    max: f64,
}

#[derive(Clone)]
struct ThemeMapping {
    object: String,
    aspect: Option<String>,
    target: Option<String>,
    theme: String,
}

#[derive(Clone)]
struct ShortlistProfile {
    max_main_signals: usize,
    max_main_signals_per_slot: usize,
    max_dominant_themes: usize,
    max_evidence: usize,
    max_required_evidence_per_slot: usize,
    min_priority_score: f64,
}

#[derive(Clone)]
struct ThemeAdviceAxis {
    advice_axis: String,
    avoid_axis: Option<String>,
    best_for: Vec<String>,
    watch_point: Option<String>,
    public_watch_point: Option<String>,
    tone_hint: Option<String>,
}

#[derive(Clone)]
struct ScoringParameters {
    exact_orb_bonus_max_deg: f64,
    exactness_bonus: f64,
    default_house_weight: f64,
    default_confidence_score: f64,
    default_duration_class: String,
    unknown_orb_weight: f64,
}

impl ReferenceData {
    fn load(service_code: &str) -> Result<Self, GenerationError> {
        validate_supported_service_code(service_code)?;
        let aspect_rows = rows(ASPECT_WEIGHTS_JSON)?;
        let refs = Self {
            service_code: service_code.to_string(),
            service_profile: service_profile(service_code)?,
            slot_profiles: slot_profiles(service_code)?,
            object_weights: weight_map(OBJECT_WEIGHTS_JSON, "object_code")?,
            target_weights: weight_map(TARGET_WEIGHTS_JSON, "target_code")?,
            supported_objects: enabled_codes(SUPPORTED_OBJECTS_JSON, "object_code")?,
            supported_aspects: enabled_codes(SUPPORTED_ASPECTS_JSON, "aspect_code")?,
            aspect_weights: aspect_rows
                .iter()
                .filter_map(|row| {
                    Some((
                        row.get("aspect_code")?.as_str()?.to_string(),
                        row.get("weight")?.as_f64()?,
                    ))
                })
                .collect(),
            aspect_tones: aspect_rows
                .iter()
                .filter_map(|row| {
                    Some((
                        row.get("aspect_code")?.as_str()?.to_string(),
                        row.get("tone")?.as_str()?.to_string(),
                    ))
                })
                .collect(),
            orb_bands: rows(ORB_BANDS_JSON)?
                .into_iter()
                .filter_map(|row| {
                    Some(OrbBand {
                        min: row.get("min_orb_deg")?.as_f64()?,
                        max: row.get("max_orb_deg")?.as_f64()?,
                        weight: row.get("weight")?.as_f64()?,
                    })
                })
                .collect(),
            theme_mappings: rows(THEME_MAPPINGS_JSON)?
                .into_iter()
                .filter_map(|row| {
                    Some(ThemeMapping {
                        object: row.get("match_object")?.as_str()?.to_string(),
                        aspect: row
                            .get("match_aspect")
                            .and_then(|v| v.as_str())
                            .map(str::to_string),
                        target: row
                            .get("match_natal_target")
                            .and_then(|v| v.as_str())
                            .map(str::to_string),
                        theme: row.get("theme_code")?.as_str()?.to_string(),
                    })
                })
                .collect(),
            intensity_bands: rows(INTENSITY_JSON)?
                .into_iter()
                .filter_map(|row| {
                    Some(IntensityBand {
                        code: row.get("band_code")?.as_str()?.to_string(),
                        min: row.get("min_score")?.as_f64()?,
                        max: row.get("max_score")?.as_f64()?,
                    })
                })
                .collect(),
            duration_classes: rows(DURATION_CLASSES_JSON)?
                .into_iter()
                .filter_map(|row| row.get("duration_class")?.as_str().map(str::to_string))
                .collect(),
            advice_axes: advice_axes()?,
            scoring: scoring_parameters(service_code)?,
            shortlist: shortlist_profile(service_code)?,
        };
        refs.validate()?;
        Ok(refs)
    }

    fn weight(&self, map: &HashMap<String, f64>, key: &str) -> f64 {
        map.get(key).copied().unwrap_or(1.0)
    }

    fn orb_weight(&self, orb: f64) -> f64 {
        self.orb_bands
            .iter()
            .find(|band| orb >= band.min && orb <= band.max)
            .map(|band| band.weight)
            .unwrap_or(self.scoring.unknown_orb_weight)
    }

    fn intensity(&self, score: f64) -> String {
        self.intensity_bands
            .iter()
            .find(|band| score >= band.min && score < band.max)
            .map(|band| band.code.clone())
            .unwrap_or_else(|| "medium".into())
    }

    fn theme_for(&self, object: &str, aspect: Option<&str>, target: Option<&str>) -> String {
        self.theme_mappings
            .iter()
            .find(|mapping| {
                mapping.object == object
                    && mapping.aspect.as_deref() == aspect
                    && mapping.target.as_deref() == target
            })
            .or_else(|| {
                self.theme_mappings.iter().find(|mapping| {
                    mapping.object == object
                        && mapping.aspect.is_none()
                        && mapping.target.as_deref() == target
                })
            })
            .map(|mapping| mapping.theme.clone())
            .unwrap_or_else(|| "daily_focus".into())
    }

    fn advice_axis(&self, theme_code: &str) -> ThemeAdviceAxis {
        self.advice_axes
            .get(theme_code)
            .cloned()
            .unwrap_or_else(|| ThemeAdviceAxis {
                advice_axis: "observe_before_acting".into(),
                avoid_axis: Some("overgeneralizing_the_day".into()),
                best_for: vec!["orientation".into()],
                watch_point: Some("avoid_turning_a_small_signal_into_a_prediction".into()),
                public_watch_point: Some(
                    "Évitez de transformer un signal bref en prédiction.".into(),
                ),
                tone_hint: Some("measured".into()),
            })
    }

    fn validate(&self) -> Result<(), GenerationError> {
        if !self
            .duration_classes
            .contains(&self.scoring.default_duration_class)
        {
            return Err(horoscope_error("HOROSCOPE_SCORING_FAILED"));
        }
        if self.advice_axes.values().any(|axis| {
            axis.watch_point
                .as_deref()
                .is_some_and(|value| !value.is_empty())
                && axis
                    .public_watch_point
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or("")
                    .is_empty()
        }) {
            return Err(horoscope_error("HOROSCOPE_SCORING_FAILED"));
        }
        Ok(())
    }

    fn slot_label(&self, slot_code: &str) -> String {
        self.slot_profiles
            .iter()
            .find(|slot| slot.slot_code == slot_code)
            .and_then(|slot| slot.slot_label.clone())
            .unwrap_or_else(|| slot_label(slot_code).to_string())
    }
}

fn slot_profiles(service_code: &str) -> Result<Vec<SlotProfile>, GenerationError> {
    let mut slots = rows(TIME_SLOTS_JSON)?
        .into_iter()
        .filter(|row| row.get("service_code").and_then(|v| v.as_str()) == Some(service_code))
        .map(serde_json::from_value)
        .collect::<Result<Vec<SlotProfile>, _>>()
        .map_err(|err| GenerationError::new(GenerationErrorCode::InvalidInput, err.to_string()))?;
    slots.sort_by_key(|slot| slot.sort_order);
    Ok(slots)
}

fn service_profile(service_code: &str) -> Result<ServiceProfile, GenerationError> {
    let row = rows(SERVICES_JSON)?
        .into_iter()
        .find(|row| row.get("service_code").and_then(|v| v.as_str()) == Some(service_code))
        .ok_or_else(|| horoscope_error("HOROSCOPE_SERVICE_NOT_IMPLEMENTED"))?;
    Ok(ServiceProfile {
        house_system_code: row
            .get("house_system_code")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        period_profile_code: row
            .get("period_profile_code")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        detail_profile_code: row
            .get("detail_profile_code")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        scan_profile_code: row
            .get("scan_profile_code")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    })
}

fn period_service_profile(service_code: &str) -> Result<ServiceProfile, GenerationError> {
    let profile = service_profile(service_code)?;
    if profile.period_profile_code.as_deref() != Some("next_7_days")
        || profile.detail_profile_code.is_none()
        || profile.scan_profile_code.is_none()
    {
        return Err(horoscope_error("HOROSCOPE_PERIOD_PROFILE_UNSUPPORTED"));
    }
    Ok(profile)
}

fn scan_profile(scan_profile_code: &str) -> Result<ScanProfile, GenerationError> {
    let row = rows(SCAN_PROFILES_JSON)?
        .into_iter()
        .find(|row| {
            row.get("scan_profile_code").and_then(Value::as_str) == Some(scan_profile_code)
                && row
                    .get("is_enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
        })
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"))?;
    Ok(ScanProfile {
        granularity: row
            .get("granularity")
            .and_then(Value::as_str)
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"))?
            .to_string(),
        reference_time_local: row
            .get("reference_time_local")
            .and_then(Value::as_str)
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"))?
            .to_string(),
        expected_snapshots_per_day: row
            .get("expected_snapshots_per_day")
            .and_then(Value::as_u64)
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"))?
            as usize,
    })
}

impl ScanProfile {
    fn reference_times(&self) -> Result<Vec<NaiveTime>, GenerationError> {
        let times = self
            .reference_time_local
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| {
                NaiveTime::parse_from_str(value, "%H:%M")
                    .map_err(|_| horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if times.len() != self.expected_snapshots_per_day || times.is_empty() {
            return Err(horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"));
        }
        Ok(times)
    }
}

fn shortlist_profile(service_code: &str) -> Result<ShortlistProfile, GenerationError> {
    let row = rows(SHORTLIST_JSON)?
        .into_iter()
        .find(|row| row.get("service_code").and_then(|v| v.as_str()) == Some(service_code))
        .ok_or_else(|| horoscope_error("HOROSCOPE_SCORING_FAILED"))?;
    Ok(ShortlistProfile {
        max_main_signals: row
            .get("max_main_signals")
            .and_then(|v| v.as_u64())
            .unwrap_or(6) as usize,
        max_main_signals_per_slot: row
            .get("max_main_signals_per_slot")
            .and_then(|v| v.as_u64())
            .unwrap_or(2) as usize,
        max_dominant_themes: row
            .get("max_dominant_themes")
            .and_then(|v| v.as_u64())
            .unwrap_or(4) as usize,
        max_evidence: row
            .get("max_evidence")
            .and_then(|v| v.as_u64())
            .unwrap_or(8) as usize,
        max_required_evidence_per_slot: row
            .get("max_required_evidence_per_slot")
            .and_then(|v| v.as_u64())
            .unwrap_or(2) as usize,
        min_priority_score: row
            .get("min_priority_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5),
    })
}

fn advice_axes() -> Result<HashMap<String, ThemeAdviceAxis>, GenerationError> {
    Ok(rows(THEME_ADVICE_AXES_JSON)?
        .into_iter()
        .filter_map(|row| {
            let theme_code = row.get("theme_code")?.as_str()?.to_string();
            let best_for = row
                .get("best_for")
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str())
                .map(str::to_string)
                .collect::<Vec<_>>();
            Some((
                theme_code,
                ThemeAdviceAxis {
                    advice_axis: row.get("advice_axis")?.as_str()?.to_string(),
                    avoid_axis: row
                        .get("avoid_axis")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    best_for,
                    watch_point: row
                        .get("watch_point")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    public_watch_point: row
                        .get("public_watch_point")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    tone_hint: row
                        .get("tone_hint")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                },
            ))
        })
        .collect())
}

fn scoring_parameters(service_code: &str) -> Result<ScoringParameters, GenerationError> {
    let scoring_rows = rows(SCORING_PARAMETERS_JSON)?;
    let row = scoring_rows
        .iter()
        .find(|row| row.get("service_code").and_then(|v| v.as_str()) == Some(service_code))
        .cloned()
        .or_else(|| {
            scoring_rows.into_iter().find(|row| {
                row.get("service_code").and_then(|v| v.as_str())
                    == Some(HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE)
            })
        })
        .ok_or_else(|| horoscope_error("HOROSCOPE_SCORING_FAILED"))?;
    Ok(ScoringParameters {
        exact_orb_bonus_max_deg: row
            .get("exact_orb_bonus_max_deg")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| horoscope_error("HOROSCOPE_SCORING_FAILED"))?,
        exactness_bonus: row
            .get("exactness_bonus")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| horoscope_error("HOROSCOPE_SCORING_FAILED"))?,
        default_house_weight: row
            .get("default_house_weight")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| horoscope_error("HOROSCOPE_SCORING_FAILED"))?,
        default_confidence_score: row
            .get("default_confidence_score")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| horoscope_error("HOROSCOPE_SCORING_FAILED"))?,
        default_duration_class: row
            .get("default_duration_class")
            .and_then(|v| v.as_str())
            .ok_or_else(|| horoscope_error("HOROSCOPE_SCORING_FAILED"))?
            .to_string(),
        unknown_orb_weight: row
            .get("unknown_orb_weight")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| horoscope_error("HOROSCOPE_SCORING_FAILED"))?,
    })
}

fn weight_map(raw: &str, key: &str) -> Result<HashMap<String, f64>, GenerationError> {
    Ok(rows(raw)?
        .into_iter()
        .filter_map(|row| {
            Some((
                row.get(key)?.as_str()?.to_string(),
                row.get("weight")?.as_f64()?,
            ))
        })
        .collect())
}

fn enabled_codes(raw: &str, key: &str) -> Result<HashSet<String>, GenerationError> {
    Ok(rows(raw)?
        .into_iter()
        .filter(|row| row.get("is_enabled_v1").and_then(|v| v.as_bool()) == Some(true))
        .filter_map(|row| row.get(key)?.as_str().map(str::to_string))
        .collect())
}

fn validate_slot_specificity(slot: &Value) -> Result<(), GenerationError> {
    let specificity = slot
        .get("specificity")
        .and_then(|v| v.as_str())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let required = slot
        .get("required_evidence_keys")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let fallback_reason = slot.get("fallback_reason").and_then(|v| v.as_str());
    match specificity {
        "specific" => {
            if required.is_empty() {
                return Err(quality_error(
                    "HOROSCOPE_SLOT_EVIDENCE_MISSING",
                    json!({ "reason": "specific_without_required_evidence" }),
                ));
            }
        }
        "shared" => {
            if required.is_empty() {
                return Err(quality_error(
                    "HOROSCOPE_SLOT_EVIDENCE_MISSING",
                    json!({ "reason": "shared_without_required_evidence" }),
                ));
            }
            let has_differentiator = ["tone", "intensity", "advice_axis", "watch_point"]
                .iter()
                .any(|key| slot.get(*key).and_then(|v| v.as_str()).is_some())
                || slot
                    .get("best_for")
                    .and_then(|v| v.as_array())
                    .map(|items| !items.is_empty())
                    .unwrap_or(false);
            if !has_differentiator {
                return Err(quality_error(
                    "HOROSCOPE_SLOT_THEME_DUPLICATED",
                    json!({ "reason": "shared_without_differentiator" }),
                ));
            }
        }
        "fallback" => {
            if !required.is_empty() || fallback_reason.unwrap_or("").trim().is_empty() {
                return Err(quality_error(
                    "HOROSCOPE_SLOT_FALLBACK_INVALID",
                    json!({ "reason": "fallback_requires_empty_evidence_and_reason" }),
                ));
            }
        }
        _ => return Err(horoscope_error("HOROSCOPE_RESPONSE_INVALID")),
    }
    Ok(())
}

fn validate_slot_evidence_alignment(
    request_slot: &Value,
    response_keys: &[Value],
) -> Result<(), GenerationError> {
    let required = request_slot
        .get("required_evidence_keys")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str())
        .collect::<HashSet<_>>();
    let specificity = request_slot
        .get("specificity")
        .and_then(|v| v.as_str())
        .unwrap_or("specific");
    if specificity != "fallback" {
        for key in response_keys.iter().filter_map(|v| v.as_str()) {
            if !required.contains(key) {
                return Err(quality_error(
                    "HOROSCOPE_EVIDENCE_MISMATCH",
                    json!({ "reason": "slot_uses_unplanned_evidence", "evidence_key": key }),
                ));
            }
        }
    }
    Ok(())
}

fn validate_public_slot_text(slot: &Value) -> Result<(), GenerationError> {
    let mut public_text = String::new();
    for key in ["title", "theme", "tone", "text", "advice", "watch_point"] {
        if let Some(value) = slot.get(key).and_then(|v| v.as_str()) {
            public_text.push_str(value);
            public_text.push('\n');
        }
    }
    for forbidden in [
        "[morning]",
        "[afternoon]",
        "[evening]",
        "[day]",
        "slot:morning",
        "slot:afternoon",
        "slot:evening",
        "slot:day",
        "slot_",
        "avoid_",
    ] {
        if public_text.contains(forbidden) {
            return Err(quality_error(
                "HOROSCOPE_PUBLIC_SLOT_CODE_LEAK",
                json!({ "forbidden": forbidden }),
            ));
        }
    }
    for generic in [
        "les signaux du jour invitent",
        "rester concret et nuance",
        "l'elan du moment",
        "l’énergie du moment",
        "lecture reste volontairement synthétique",
        "preuve astrologique centrale",
        "découpage horaire",
    ] {
        if public_text.to_lowercase().contains(generic) {
            return Err(quality_error(
                "HOROSCOPE_SLOT_TOO_GENERIC",
                json!({ "forbidden": generic }),
            ));
        }
    }
    if public_text.contains("Apres-midi")
        || public_text.contains("Repondez")
        || public_text.contains("Conseil:")
        || !french_elision_violations(&public_text).is_empty()
    {
        return Err(quality_error(
            "HOROSCOPE_FRENCH_TYPOGRAPHY_FAILED",
            json!({ "reason": "known_french_typography_violation" }),
        ));
    }
    Ok(())
}

fn validate_public_text_no_technical_codes(public_text: &str) -> Result<(), GenerationError> {
    let lower = public_text.to_lowercase();
    for forbidden in [
        "[morning]",
        "[afternoon]",
        "[evening]",
        "[day]",
        "slot:morning",
        "slot:afternoon",
        "slot:evening",
        "slot:day",
        "slot technique",
        "slot_code",
        "slot_",
        "avoid_",
    ] {
        if lower.contains(forbidden) {
            return Err(quality_error(
                "HOROSCOPE_PUBLIC_SLOT_CODE_LEAK",
                json!({ "forbidden": forbidden }),
            ));
        }
    }
    if normalized_text(public_text)
        .split_whitespace()
        .any(|token| token == "day")
    {
        return Err(quality_error(
            "HOROSCOPE_PUBLIC_SLOT_CODE_LEAK",
            json!({ "forbidden": "day" }),
        ));
    }
    Ok(())
}

fn validate_free_text_quality(public_text: &str, response: &Value) -> Result<(), GenerationError> {
    for key in ["advice", "watch_point"] {
        if response
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .is_empty()
        {
            return Err(quality_error(
                "HOROSCOPE_RESPONSE_INVALID",
                json!({ "reason": format!("missing_{key}") }),
            ));
        }
    }
    validate_public_text_no_technical_codes(public_text)?;
    let word_count = public_text.split_whitespace().count();
    if !(40..=190).contains(&word_count) {
        return Err(quality_error(
            "HOROSCOPE_FREE_LENGTH_INVALID",
            json!({ "word_count": word_count }),
        ));
    }
    for generic in [
        "les signaux du jour invitent",
        "rester concret et nuance",
        "l'elan du moment",
        "l’énergie du moment",
        "lecture reste volontairement synthétique",
        "preuve astrologique centrale",
        "découpage horaire",
    ] {
        let lower = public_text.to_lowercase();
        let normalized = normalized_text(public_text);
        if lower.contains(generic) || normalized.contains(generic) {
            return Err(quality_error(
                "HOROSCOPE_SLOT_TOO_GENERIC",
                json!({ "forbidden": generic }),
            ));
        }
    }
    if public_text.contains("Conseil:")
        || public_text.contains("Repondez")
        || !french_elision_violations(public_text).is_empty()
    {
        return Err(quality_error(
            "HOROSCOPE_FRENCH_TYPOGRAPHY_FAILED",
            json!({ "reason": "known_french_typography_violation" }),
        ));
    }
    Ok(())
}

fn validate_astrological_reference(
    slot_code: &str,
    text: &str,
    request_slot: &Value,
) -> Result<(), GenerationError> {
    if request_slot.get("specificity").and_then(|v| v.as_str()) == Some("fallback") {
        return Ok(());
    }
    let lower = text.to_lowercase();
    let has_astro = [
        "lune",
        "mars",
        "vénus",
        "venus",
        "mercure",
        "aspect",
        "maison",
        "transit",
        "astrologique",
        "natal",
    ]
    .iter()
    .any(|needle| lower.contains(needle));
    if has_astro {
        Ok(())
    } else {
        Err(quality_error(
            "HOROSCOPE_SLOT_ASTRO_REFERENCE_MISSING",
            json!({ "slot_code": slot_code }),
        ))
    }
}

fn validate_day_overview_not_copied(
    request: &Value,
    response_slots: &[Value],
) -> Result<(), GenerationError> {
    let overview = request
        .get("day_overview")
        .and_then(|v| v.get("summary_hint"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if overview.is_empty() {
        return Ok(());
    }
    for slot in response_slots {
        let text = slot.get("text").and_then(|v| v.as_str()).unwrap_or("");
        if normalized_text(text).contains(&normalized_text(overview)) {
            return Err(quality_error(
                "HOROSCOPE_SLOT_REPETITION_FAILED",
                json!({ "reason": "day_overview_copied_into_slot" }),
            ));
        }
    }
    Ok(())
}

fn validate_slot_diversity(texts: &[String]) -> Result<(), GenerationError> {
    for i in 0..texts.len() {
        for j in (i + 1)..texts.len() {
            let a = meaningful_words(&texts[i]);
            let b = meaningful_words(&texts[j]);
            let shared = a.intersection(&b).count();
            let denom = a.len().min(b.len()).max(1);
            if shared as f64 / denom as f64 > 0.60 {
                return Err(quality_error(
                    "HOROSCOPE_SLOT_REPETITION_FAILED",
                    json!({ "reason": "slot_word_overlap_too_high" }),
                ));
            }
            if first_words(&texts[i], 3) == first_words(&texts[j], 3) {
                return Err(quality_error(
                    "HOROSCOPE_SLOT_REPETITION_FAILED",
                    json!({ "reason": "same_opening_trigram" }),
                ));
            }
        }
    }
    Ok(())
}

fn validate_distinct_strings(items: &[String], code: &str) -> Result<(), GenerationError> {
    let normalized = items
        .iter()
        .map(|item| normalized_text(item))
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    let unique = normalized.iter().collect::<HashSet<_>>();
    if unique.len() != normalized.len() {
        return Err(quality_error(code, json!({ "reason": "duplicate_text" })));
    }
    Ok(())
}

fn validate_distinct_best_for(items: &[Vec<String>]) -> Result<(), GenerationError> {
    let normalized = items
        .iter()
        .map(|set| {
            let mut values = set
                .iter()
                .map(|value| normalized_text(value))
                .collect::<Vec<_>>();
            values.sort();
            values.join("|")
        })
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let unique = normalized.iter().collect::<HashSet<_>>();
    if unique.len() != normalized.len() {
        return Err(quality_error(
            "HOROSCOPE_SLOT_THEME_DUPLICATED",
            json!({ "reason": "best_for_duplicated" }),
        ));
    }
    Ok(())
}

fn meaningful_words(text: &str) -> HashSet<String> {
    let stopwords = [
        "le", "la", "les", "un", "une", "des", "de", "du", "et", "ou", "a", "à", "ce", "c", "est",
        "sur", "pour", "plus", "dans", "avec", "sans", "du", "au", "aux", "en",
    ]
    .into_iter()
    .collect::<HashSet<_>>();
    normalized_text(text)
        .split_whitespace()
        .filter(|word| word.len() > 2 && !stopwords.contains(*word))
        .map(str::to_string)
        .collect()
}

fn first_words(text: &str, count: usize) -> Vec<String> {
    normalized_text(text)
        .split_whitespace()
        .take(count)
        .map(str::to_string)
        .collect()
}

fn normalized_text(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || ch.is_whitespace() {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn rows(raw: &str) -> Result<Vec<Value>, GenerationError> {
    let value: Value = serde_json::from_str(raw)
        .map_err(|err| GenerationError::new(GenerationErrorCode::InvalidInput, err.to_string()))?;
    Ok(value
        .get("data")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default())
}

pub fn validate_interpretation_request_schema(value: &Value) -> Result<(), GenerationError> {
    validate_schema(
        interpretation_request_schema,
        "HOROSCOPE_RESPONSE_INVALID",
        value,
    )
}

pub fn validate_horoscope_response_schema(value: &Value) -> Result<(), GenerationError> {
    validate_schema(response_schema, "HOROSCOPE_RESPONSE_INVALID", value)
}

fn validate_schema(
    schema: fn() -> &'static JSONSchema,
    code: &str,
    value: &Value,
) -> Result<(), GenerationError> {
    schema().validate(value).map_err(|errors| {
        let errors = errors.map(|err| err.to_string()).collect::<Vec<_>>();
        let message = if errors.is_empty() {
            code.to_string()
        } else {
            format!("{code}: {}", errors.join("; "))
        };
        GenerationError::with_details(
            GenerationErrorCode::SchemaValidationFailed,
            message,
            json!({ "errors": errors }),
        )
    })
}

fn interpretation_request_schema() -> &'static JSONSchema {
    static SCHEMA: OnceLock<JSONSchema> = OnceLock::new();
    SCHEMA.get_or_init(|| compile_schema(INTERPRETATION_REQUEST_SCHEMA_JSON))
}

fn response_schema() -> &'static JSONSchema {
    static SCHEMA: OnceLock<JSONSchema> = OnceLock::new();
    SCHEMA.get_or_init(|| compile_schema(RESPONSE_SCHEMA_JSON))
}

fn period_interpretation_request_schema() -> &'static JSONSchema {
    static SCHEMA: OnceLock<JSONSchema> = OnceLock::new();
    SCHEMA.get_or_init(|| compile_schema(PERIOD_INTERPRETATION_REQUEST_SCHEMA_JSON))
}

fn period_response_schema() -> &'static JSONSchema {
    static SCHEMA: OnceLock<JSONSchema> = OnceLock::new();
    SCHEMA.get_or_init(|| compile_schema(PERIOD_RESPONSE_SCHEMA_JSON))
}

fn compile_schema(raw: &str) -> JSONSchema {
    let schema: Value = serde_json::from_str(raw).expect("horoscope schema json is valid");
    JSONSchema::compile(&schema).expect("horoscope schema compiles")
}

fn collect_evidence_keys(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            if let Some(items) = map.get("evidence_keys").and_then(|v| v.as_array()) {
                out.extend(items.iter().filter_map(|v| v.as_str().map(str::to_string)));
            }
            for child in map.values() {
                collect_evidence_keys(child, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_evidence_keys(item, out);
            }
        }
        _ => {}
    }
}

fn free_public_text(response: &Value) -> String {
    let mut out = String::new();
    if let Some(summary) = response.get("summary") {
        for key in ["title", "text"] {
            if let Some(value) = summary.get(key).and_then(|v| v.as_str()) {
                out.push_str(value);
                out.push('\n');
            }
        }
    }
    for key in ["advice", "watch_point"] {
        if let Some(value) = response.get(key).and_then(|v| v.as_str()) {
            out.push_str(value);
            out.push('\n');
        }
    }
    out
}

fn service_code_from_value(value: &Value) -> Result<&str, GenerationError> {
    let service_code = value
        .get("service_code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| horoscope_error("HOROSCOPE_CALCULATION_FAILED"))?;
    validate_supported_service_code(service_code)?;
    Ok(service_code)
}

fn validate_supported_service_code(service_code: &str) -> Result<(), GenerationError> {
    if matches!(
        service_code,
        HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE
            | HOROSCOPE_FREE_DAILY_SERVICE_CODE
            | HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE
    ) {
        return Ok(());
    }
    Err(horoscope_error("HOROSCOPE_SERVICE_NOT_IMPLEMENTED"))
}

fn validate_period_service_code(service_code: &str) -> Result<(), GenerationError> {
    if matches!(
        service_code,
        HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE
            | HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE
            | HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE
    ) {
        return Ok(());
    }
    Err(horoscope_error("HOROSCOPE_SERVICE_NOT_IMPLEMENTED"))
}

fn fact_string(fact: &Value, key: &str) -> Result<String, GenerationError> {
    fact.get(key)
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| horoscope_error("HOROSCOPE_CALCULATION_FAILED"))
}

fn horoscope_error(code: &str) -> GenerationError {
    GenerationError::with_details(GenerationErrorCode::InvalidInput, code, Value::Null)
}

fn quality_error(code: &str, details: Value) -> GenerationError {
    GenerationError::with_details(
        GenerationErrorCode::PostSafetyValidationFailed,
        code,
        details,
    )
}

fn default_audience() -> String {
    "general".into()
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn slot_label(slot_code: &str) -> &'static str {
    match slot_code {
        "morning" => "Matin",
        "afternoon" => "Après-midi",
        "evening" => "Soir",
        _ => "Moment",
    }
}
