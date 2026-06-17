use super::*;
use astral_calculator::features::horoscope::{
    HoroscopePeriodCalculationRequest, HoroscopeScanPlan,
};
use chrono::TimeZone;

const TIME_PERIOD_PROFILES_JSON: &str =
    include_str!("../../../../../../json_db/astral_time_period_profiles.json");

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
    validate_period_calculation_public_request(public)?;
    let service_profile = period_service_profile(service_code)?;
    let period_profile_code = service_profile
        .period_profile_code
        .as_deref()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_PROFILE_UNSUPPORTED"))?;
    let scan_profile_code = service_profile
        .scan_profile_code
        .as_deref()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"))?;
    let period_resolution =
        resolve_period_window(period_profile_code, &public.anchor_date, &public.timezone)?;
    let scan_plan = build_scan_plan(&period_resolution, scan_profile_code)?;
    validate_scan_plan(&period_resolution, &scan_plan)?;

    let calculation_request = HoroscopePeriodCalculationRequest {
        contract_version: "horoscope_period_calculation_request".into(),
        service_code: service_code.to_string(),
        chart_calculation_id: public.chart_calculation_id.clone(),
        period_resolution,
        scan_plan: serde_json::from_value::<HoroscopeScanPlan>(scan_plan).map_err(|err| {
            GenerationError::with_details(
                GenerationErrorCode::InvalidInput,
                format!("HOROSCOPE_PERIOD_CALCULATION_FAILED: {err}"),
                Value::Null,
            )
        })?,
    };

    serde_json::to_value(calculation_request).map_err(|err| {
        GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            format!("HOROSCOPE_PERIOD_CALCULATION_FAILED: {err}"),
            Value::Null,
        )
    })
}

fn validate_period_calculation_public_request(
    public: &HoroscopePeriodPublicRequest,
) -> Result<(), GenerationError> {
    if public.chart_calculation_id.trim().is_empty() {
        return Err(horoscope_error("HOROSCOPE_PERIOD_NATAL_CHART_REQUIRED"));
    }
    chrono::NaiveDate::parse_from_str(&public.anchor_date, "%Y-%m-%d")
        .map_err(|_| horoscope_error("HOROSCOPE_PERIOD_ANCHOR_DATE_REQUIRED"))?;
    public
        .timezone
        .parse::<Tz>()
        .map_err(|_| horoscope_error("HOROSCOPE_PERIOD_TIMEZONE_REQUIRED"))?;
    Ok(())
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
    for date in included.iter().filter_map(Value::as_str) {
        if !dates.contains(date) {
            return Err(horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"));
        }
    }
    Ok(())
}

pub(crate) fn require_canonical_utc_offset(raw: &str) -> Result<(), GenerationError> {
    let parsed = chrono::DateTime::parse_from_rfc3339(raw)
        .map_err(|_| horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"))?;
    if parsed.with_timezone(&chrono::Utc).to_rfc3339() != raw {
        return Err(horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"));
    }
    Ok(())
}

fn resolve_period_window(
    period_profile_code: &str,
    anchor_date: &str,
    timezone: &str,
) -> Result<Value, GenerationError> {
    let profiles = period_profiles()?;
    let resolver = astral_time_window::PeriodWindowResolver::new(profiles);
    let request = astral_time_window::PeriodWindowRequest {
        period_profile_code: period_profile_code.to_string(),
        anchor_date: anchor_date.to_string(),
        timezone: timezone.to_string(),
        custom_start_date: None,
        custom_end_date: None,
    };
    let resolved = resolver
        .resolve(&request)
        .map_err(map_period_window_error)?;
    let start_utc = resolved
        .start_datetime_utc()
        .map_err(map_period_window_error)?;
    let end_utc = resolved
        .end_datetime_utc()
        .map_err(map_period_window_error)?;
    Ok(json!({
        "period_profile_code": period_profile_code,
        "anchor_date": anchor_date,
        "timezone": timezone,
        "start_datetime_local": resolved.start_datetime_local.format("%Y-%m-%dT%H:%M:%S").to_string(),
        "end_datetime_local": resolved.end_datetime_local.format("%Y-%m-%dT%H:%M:%S").to_string(),
        "start_datetime_utc": start_utc,
        "end_datetime_utc": end_utc,
        "end_exclusive": resolved.end_exclusive,
        "duration_days": resolved.duration_days,
        "included_dates": resolved.included_dates(),
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
    let reference_times = parse_reference_times(
        &scan_profile.reference_time_local,
        scan_profile.expected_snapshots_per_day,
    )?;
    let mut snapshots = Vec::new();
    for value in dates {
        let date = value
            .as_str()
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"))?;
        let parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
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

fn parse_reference_times(
    raw: &str,
    expected_snapshots_per_day: usize,
) -> Result<Vec<chrono::NaiveTime>, GenerationError> {
    let times = raw
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            chrono::NaiveTime::parse_from_str(value, "%H:%M")
                .map_err(|_| horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if times.len() != expected_snapshots_per_day || times.is_empty() {
        return Err(horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"));
    }
    Ok(times)
}

fn local_to_utc(tz: Tz, local: chrono::NaiveDateTime) -> Result<String, GenerationError> {
    match tz.from_local_datetime(&local) {
        chrono::LocalResult::Single(value) => Ok(value.with_timezone(&chrono::Utc).to_rfc3339()),
        chrono::LocalResult::Ambiguous(_, _) | chrono::LocalResult::None => {
            Err(horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"))
        }
    }
}

fn period_profiles() -> Result<Vec<astral_time_window::PeriodProfileDefinition>, GenerationError> {
    let value: Value = serde_json::from_str(TIME_PERIOD_PROFILES_JSON)
        .map_err(|err| GenerationError::new(GenerationErrorCode::InvalidInput, err.to_string()))?;
    let items = value
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    items
        .into_iter()
        .map(|mut item| {
            if let Value::Object(map) = &mut item {
                map.remove("sort_order");
            }
            serde_json::from_value(item)
        })
        .collect::<Result<Vec<astral_time_window::PeriodProfileDefinition>, _>>()
        .map_err(|err| GenerationError::new(GenerationErrorCode::InvalidInput, err.to_string()))
}

fn map_period_window_error(err: astral_time_window::PeriodWindowError) -> GenerationError {
    match err {
        astral_time_window::PeriodWindowError::InvalidTimezone(_) => {
            horoscope_error("HOROSCOPE_PERIOD_TIMEZONE_REQUIRED")
        }
        astral_time_window::PeriodWindowError::InvalidDate {
            field: "anchor_date",
            ..
        } => horoscope_error("HOROSCOPE_PERIOD_ANCHOR_DATE_REQUIRED"),
        astral_time_window::PeriodWindowError::InvalidDate { .. }
        | astral_time_window::PeriodWindowError::AmbiguousLocalDateTime { .. }
        | astral_time_window::PeriodWindowError::NonexistentLocalDateTime { .. } => {
            horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH")
        }
        _ => horoscope_error("HOROSCOPE_PERIOD_PROFILE_UNSUPPORTED"),
    }
}
