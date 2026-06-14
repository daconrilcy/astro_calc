use super::*;

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
    let payload = serde_json::to_value(public).map_err(|err| {
        GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            format!("HOROSCOPE_PAYLOAD_INVALID: {err}"),
            Value::Null,
        )
    })?;
    let calculation_request =
        astral_calculator::horoscope::build_horoscope_period_calculation_request_from_public(
            service_code,
            &payload,
        )
        .map_err(|message| {
            GenerationError::with_details(GenerationErrorCode::InvalidInput, message, Value::Null)
        })?;
    serde_json::to_value(calculation_request).map_err(|err| {
        GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            format!("HOROSCOPE_PERIOD_CALCULATION_FAILED: {err}"),
            Value::Null,
        )
    })
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
