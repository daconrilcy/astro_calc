use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use crate::french_typography::french_elision_violations;

use astral_llm_domain::{
    model_usage_tier::ModelRouteContext, GenerationError, GenerationErrorCode, ProviderKind,
    SafetyMode,
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
pub const HOROSCOPE_SERVICE_CODE: &str = HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE;

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
        payload: &Value,
    ) -> Result<serde_json::Value, GenerationError> {
        HoroscopeDailyNatalOrchestrator::execute(
            HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE,
            calculator,
            payload,
        )
        .await
    }
}

impl HoroscopeFreeDailyOrchestrator {
    pub async fn execute(
        calculator: &astral_llm_infra::CalculatorClient,
        payload: &Value,
    ) -> Result<serde_json::Value, GenerationError> {
        HoroscopeDailyNatalOrchestrator::execute(
            HOROSCOPE_FREE_DAILY_SERVICE_CODE,
            calculator,
            payload,
        )
        .await
    }
}

impl HoroscopePremiumDailyLocalOrchestrator {
    pub async fn execute(
        calculator: &astral_llm_infra::CalculatorClient,
        payload: &Value,
    ) -> Result<serde_json::Value, GenerationError> {
        HoroscopeDailyNatalOrchestrator::execute(
            HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
            calculator,
            payload,
        )
        .await
    }
}

impl HoroscopePeriodNatalOrchestrator {
    pub async fn execute(
        calculator: &astral_llm_infra::CalculatorClient,
        use_case: &GenerateReadingUseCase,
        payload: &Value,
    ) -> Result<serde_json::Value, GenerationError> {
        let public = validate_period_public_request(payload)?;
        let calculation_request = build_period_calculation_request(&public)?;
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
        let response = period_writer_response(use_case, &interpretation).await?;
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
        let response = fake_writer_response(&interpretation)?;
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
    let period_resolution = resolve_period(public)?;
    let scan_plan = build_scan_plan(&period_resolution)?;
    validate_scan_plan(&period_resolution, &scan_plan)?;
    Ok(json!({
        "contract_version": "horoscope_period_calculation_request_v1",
        "service_code": HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE,
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

fn build_scan_plan(period_resolution: &Value) -> Result<Value, GenerationError> {
    let tz = period_resolution["timezone"]
        .as_str()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"))?
        .parse::<Tz>()
        .map_err(|_| horoscope_error("HOROSCOPE_PERIOD_TIMEZONE_REQUIRED"))?;
    let dates = period_resolution["included_dates"]
        .as_array()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"))?;
    let noon = NaiveTime::from_hms_opt(12, 0, 0).expect("noon is valid");
    let snapshots = dates
        .iter()
        .map(|value| {
            let date = value
                .as_str()
                .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"))?;
            let parsed = NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .map_err(|_| horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"))?;
            let local = parsed.and_time(noon);
            let utc = local_to_utc(tz, local)?;
            Ok(json!({
                "snapshot_key": format!("{date}:noon"),
                "date": date,
                "reference_time_local": "12:00",
                "reference_datetime_local": local.format("%Y-%m-%dT%H:%M:%S").to_string(),
                "reference_datetime_utc": utc
            }))
        })
        .collect::<Result<Vec<_>, GenerationError>>()?;
    Ok(json!({
        "scan_profile_code": "daily_noon_7_days",
        "granularity": "daily_noon",
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
    let included = period_resolution["included_dates"]
        .as_array()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"))?;
    let snapshots = scan_plan["snapshots"]
        .as_array()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"))?;
    if scan_plan["snapshot_count"].as_u64() != Some(snapshots.len() as u64)
        || snapshots.len() != included.len()
    {
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
    if snapshots.len() != 7 || included_dates.len() != 7 {
        return Err(horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"));
    }

    let evidence = period_evidence_from_snapshots(snapshots)?;
    if evidence.is_empty() {
        return Err(horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"));
    }
    let events = build_period_events(&evidence, &period_resolution)?;
    let daily_plans = build_daily_plans(&included_dates, &events)?;
    let key_days = build_period_day_markers(&daily_plans, 3, "Jour clé");
    let best_days = build_period_day_markers(
        &daily_plans
            .iter()
            .filter(|day| day.get("tone").and_then(|v| v.as_str()) != Some("careful"))
            .cloned()
            .collect::<Vec<_>>(),
        2,
        "Jour favorable",
    );
    let watch_days = build_period_day_markers(
        &daily_plans
            .iter()
            .filter(|day| day.get("tone").and_then(|v| v.as_str()) == Some("careful"))
            .cloned()
            .collect::<Vec<_>>(),
        2,
        "Jour de vigilance",
    );
    let request = json!({
        "contract_version": "horoscope_period_interpretation_request_v1",
        "service_code": HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        "period_resolution": period_resolution,
        "scan_plan": scan_plan,
        "target_language": public.target_language,
        "week_overview_plan": {
            "dominant_theme": events.first().and_then(|event| event["theme_code"].as_str()).unwrap_or("weekly_focus"),
            "tone": events.first().and_then(|event| event["tone"].as_str()).unwrap_or("constructive"),
            "trajectory_hint": "Construire une trajectoire de semaine, pas sept lectures quotidiennes independantes.",
            "evidence_keys": evidence.iter().take(4).filter_map(|item| item["evidence_key"].as_str()).collect::<Vec<_>>()
        },
        "period_events": events.clone(),
        "main_events": events.iter().take(8).cloned().collect::<Vec<_>>(),
        "key_days": key_days,
        "best_days": best_days,
        "watch_days": watch_days,
        "daily_plans": daily_plans,
        "domain_sections": build_period_domain_sections(&evidence),
        "evidence": evidence.into_iter().take(20).collect::<Vec<_>>()
    });
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
            let tone = match aspect {
                Some("square") | Some("opposition") => "careful",
                Some("trine") | Some("sextile") => "supportive",
                Some("conjunction") => "active",
                _ => "focused",
            };
            out.push(json!({
                "evidence_key": key,
                "date": date,
                "fact_type": fact_type,
                "source": fact["source"].as_str().unwrap_or("calculator"),
                "transiting_object": object,
                "natal_target": fact.get("natal_target").cloned().unwrap_or(Value::Null),
                "aspect": fact.get("aspect").cloned().unwrap_or(Value::Null),
                "orb_deg": fact.get("orb_deg").cloned().unwrap_or(Value::Null),
                "natal_house": fact.get("natal_house").cloned().unwrap_or(Value::Null),
                "theme_code": theme,
                "tone": tone,
                "human_label": format!(
                    "{} met en avant le thème {}",
                    period_object_public_label(object),
                    period_theme_public_label(theme)
                )
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
    let mut out = Vec::new();
    for item in evidence {
        let date = item["date"]
            .as_str()
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"))?;
        if !included.contains(date) {
            return Err(horoscope_error("HOROSCOPE_PERIOD_EVENT_OUTSIDE_WINDOW"));
        }
        let evidence_key = item["evidence_key"].as_str().unwrap_or("");
        out.push(json!({
            "event_key": format!("event:{evidence_key}"),
            "event_type": if item["fact_type"].as_str() == Some("moon_house_by_day") {
                "moon_house_by_day"
            } else if item.get("orb_deg").and_then(|v| v.as_f64()).unwrap_or(9.0) <= 1.0 {
                "transit_exact"
            } else {
                "transit_active"
            },
            "date": date,
            "theme_code": item["theme_code"],
            "tone": item["tone"],
            "score": 1.0,
            "evidence_keys": [evidence_key]
        }));
    }
    Ok(out)
}

fn build_daily_plans(
    included_dates: &[String],
    events: &[Value],
) -> Result<Vec<Value>, GenerationError> {
    let mut out = Vec::new();
    for date in included_dates {
        let event = events
            .iter()
            .find(|event| event["date"].as_str() == Some(date.as_str()))
            .or_else(|| events.first())
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"))?;
        let theme = event["theme_code"].as_str().unwrap_or("organization");
        let theme_label = period_theme_public_label(theme);
        let tone = event["tone"].as_str().unwrap_or("focused");
        let evidence_keys = event["evidence_keys"].clone();
        out.push(json!({
            "date": date,
            "day_label": public_day_label(date),
            "theme_code": theme,
            "theme_label": theme_label,
            "tone": tone,
            "summary_hint": format!("Le {date}, le thème {theme_label} donne le relief principal de la journée."),
            "advice_hint": format!("Restez concret sur {theme_label} et gardez une marge d'ajustement."),
            "evidence_keys": evidence_keys
        }));
    }
    Ok(out)
}

fn build_period_day_markers(days: &[Value], limit: usize, title: &str) -> Vec<Value> {
    days.iter()
        .take(limit)
        .map(|day| {
            json!({
                "date": day["date"],
                "title": title,
                "reason": format!(
                    "{} ressort par le thème {}.",
                    day["day_label"].as_str().unwrap_or("Ce jour"),
                    period_theme_public_label(day["theme_code"].as_str().unwrap_or("principal"))
                ),
                "evidence_keys": day["evidence_keys"],
                "fallback_reason": ""
            })
        })
        .collect()
}

fn build_period_domain_sections(evidence: &[Value]) -> Vec<Value> {
    let first_key = evidence
        .first()
        .and_then(|item| item["evidence_key"].as_str())
        .unwrap_or("period:fallback");
    let second_key = evidence
        .iter()
        .skip(1)
        .find_map(|item| item["evidence_key"].as_str())
        .unwrap_or(first_key);
    vec![
        json!({
            "domain": "organisation",
            "title": "Organisation",
            "focus": "Installer une progression simple plutôt que multiplier les priorités.",
            "evidence_keys": [first_key]
        }),
        json!({
            "domain": "relations",
            "title": "Relations",
            "focus": "Garder des échanges courts, précis et réparateurs.",
            "evidence_keys": [second_key]
        }),
    ]
}

async fn period_writer_response(
    use_case: &GenerateReadingUseCase,
    request: &Value,
) -> Result<Value, GenerationError> {
    let defaults = use_case.engine_defaults();
    if defaults.provider == ProviderKind::Fake {
        return fake_period_writer_response(request);
    }

    let schema: Value = serde_json::from_str(PERIOD_RESPONSE_SCHEMA_JSON).map_err(|err| {
        GenerationError::with_details(
            GenerationErrorCode::SchemaValidationFailed,
            format!("HOROSCOPE_PERIOD_RESPONSE_INVALID: {err}"),
            Value::Null,
        )
    })?;
    let provider_request = ProviderGenerationRequest {
        model: defaults.model.clone(),
        messages: period_writer_messages(request)?,
        structured_schema: Some(schema),
        reasoning_effort: None,
        temperature: Some(0.4),
        max_output_tokens: Some(2200),
        safety_mode: SafetyMode::PlatformRulesOnly,
        timeout: StdDuration::from_secs(180),
        metadata: GenerationMetadata {
            run_id: uuid::Uuid::new_v4().to_string(),
            request_id: None,
            product_code: HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE.to_string(),
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
        .or_else(|| serde_json::from_str::<Value>(&routed.response.raw_text).ok())
        .ok_or_else(|| {
            quality_error(
                "HOROSCOPE_PERIOD_RESPONSE_INVALID",
                json!({ "reason": "provider_response_not_json" }),
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
    normalize_period_public_tones(&mut response);
    Ok(response)
}

fn period_writer_messages(request: &Value) -> Result<Vec<PromptMessage>, GenerationError> {
    let compact = serde_json::to_string(request).map_err(|err| {
        GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            format!("HOROSCOPE_PERIOD_RESPONSE_INVALID: {err}"),
            Value::Null,
        )
    })?;
    Ok(vec![
        PromptMessage {
            role: PromptRole::System,
            content: "Tu écris une lecture d'horoscope de période en français. Retourne uniquement un objet JSON conforme au schéma fourni. N'invente aucune preuve: chaque evidence_key publique doit provenir de la requête. N'affiche jamais les codes internes, les clés de preuve, les noms techniques de transits, les theme_code anglais, ni les codes tone anglais. La timeline doit couvrir exactement les 7 dates, avec des formulations variées et une trajectoire globale.".to_string(),
        },
        PromptMessage {
            role: PromptRole::User,
            content: format!(
                "Construis horoscope_period_response_v1 pour cette requête d'interprétation. Utilise les libellés français déjà présents, pas les codes internes. Requête JSON:\n{compact}"
            ),
        },
    ])
}

fn fake_period_writer_response(request: &Value) -> Result<Value, GenerationError> {
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
            let text = match index {
                0 => format!("{} ouvre la période sur {theme_label}, avec une priorité simple à poser avant d'élargir le mouvement.", day["day_label"].as_str().unwrap_or("Ce jour")),
                1 => format!("{} aide à ajuster {theme_label} sans perdre le fil installé en début de semaine.", day["day_label"].as_str().unwrap_or("Ce jour")),
                2 => format!("{} demande plus de tri autour de {theme_label}; mieux vaut choisir une action nette qu'accumuler les réponses rapides.", day["day_label"].as_str().unwrap_or("Ce jour")),
                3 => format!("{} donne un point d'appui pour clarifier {theme_label} et consolider ce qui tient déjà.", day["day_label"].as_str().unwrap_or("Ce jour")),
                4 => format!("{} remet {theme_label} au centre des échanges, avec intérêt pour les formulations courtes et précises.", day["day_label"].as_str().unwrap_or("Ce jour")),
                5 => format!("{} ramène {theme_label} vers des choix pratiques, utiles pour préparer la fin de période.", day["day_label"].as_str().unwrap_or("Ce jour")),
                _ => format!("{} referme la période sur {theme_label}, en reliant les appuis et les vigilances des jours précédents.", day["day_label"].as_str().unwrap_or("Ce jour")),
            };
            json!({
                "date": day["date"],
                "day_label": day["day_label"],
                "theme": theme_label,
                "tone": period_tone_public_label(day["tone"].as_str().unwrap_or("focused")),
                "text": text,
                "advice": day["advice_hint"],
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
                "text": section["focus"],
                "evidence_keys": section["evidence_keys"]
            })
        })
        .collect::<Vec<_>>();
    let response = json!({
        "contract_version": "horoscope_period_response_v1",
        "service_code": HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        "period_resolution": request["period_resolution"],
        "week_overview": {
            "title": "Vos 7 prochains jours",
            "text": "La période se lit comme une progression continue : d'abord clarifier les priorités, puis ajuster les échanges et terminer sur une intégration plus posée.",
            "trajectory": "Une trajectoire globale relie les jours clés, les appuis et les moments de vigilance."
        },
        "key_days": request["key_days"],
        "best_days": request["best_days"],
        "watch_days": request["watch_days"],
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
    Ok(response)
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
    validate_period_domain_sections(response, &evidence)?;
    validate_best_watch_no_overlap(response)?;
    validate_period_public_text(&public_text)?;
    validate_period_not_seven_daily(response)?;
    Ok(())
}

fn validate_period_day_markers(
    _request: &Value,
    response: &Value,
    field: &str,
    included: &HashSet<&str>,
    evidence: &HashSet<&str>,
) -> Result<(), GenerationError> {
    for marker in response[field].as_array().into_iter().flatten() {
        let date = marker["date"]
            .as_str()
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_KEY_DAYS_MISSING"))?;
        if !included.contains(date) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH",
                json!({ "field": field, "date": date }),
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

fn validate_best_watch_no_overlap(response: &Value) -> Result<(), GenerationError> {
    let best = response["best_days"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item["date"].as_str())
        .collect::<HashSet<_>>();
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
    if sections.len() < 2 {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_EVIDENCE_MISSING",
            json!({ "field": "domain_sections" }),
        ));
    }
    let mut section_evidence_sets = HashSet::new();
    for section in sections {
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
        "/week_overview/title",
        "/week_overview/text",
        "/week_overview/trajectory",
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
        "domain_sections",
        "evidence_summary",
    ] {
        for item in response[field].as_array().into_iter().flatten() {
            for key in ["title", "reason", "domain", "text", "label"] {
                if let Some(value) = item.get(key).and_then(|value| value.as_str()) {
                    public_text.push_str(value);
                    public_text.push('\n');
                }
            }
        }
    }
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

fn normalize_period_public_tones(response: &mut Value) {
    if let Some(days) = response
        .get_mut("daily_timeline")
        .and_then(Value::as_array_mut)
    {
        for day in days {
            if let Some(tone) = day.get("tone").and_then(Value::as_str) {
                day["tone"] = json!(period_tone_public_label_if_code(tone));
            }
        }
    }
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
        "reason": if watch {
            "La tension du signal principal invite à ralentir les réponses."
        } else {
            "La tonalité du signal principal favorise une action simple et utile."
        },
        "best_for": slot.get("best_for").cloned().unwrap_or_else(|| json!([])),
        "avoid": if watch { json!(["réponse impulsive"]) } else { json!([]) },
        "evidence_keys": evidence_keys
    })
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
    Ok(json!({
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
    }))
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

    Ok(json!({
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
    }))
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
    Ok(json!({
        "slot_label": label,
        "title": premium_timeline_title(index),
        "theme": premium_timeline_theme(index),
        "tone": tone,
        "text": premium_timeline_text(index),
        "advice": premium_timeline_advice(index),
        "best_for": best_for,
        "watch_point": slot.get("watch_point").and_then(|v| v.as_str()).unwrap_or("Gardez un repère simple et vérifiable."),
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
    Ok(json!({
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
    }))
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
    let watch_point = slot
        .get("watch_point")
        .and_then(|v| v.as_str())
        .unwrap_or("");
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
    })
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
        GenerationError::with_details(
            GenerationErrorCode::SchemaValidationFailed,
            code,
            json!({ "errors": errors.map(|err| err.to_string()).collect::<Vec<_>>() }),
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
