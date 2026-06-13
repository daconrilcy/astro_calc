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
    validate_period_service_code(service_code)?;
    let profile = period_service_profile(service_code)?;
    let period_resolution = resolve_horoscope_period_window(service_code, public)?;
    let scan_plan = build_scan_plan(
        &period_resolution,
        profile
            .scan_profile_code
            .as_deref()
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"))?,
    )?;
    validate_scan_plan(&period_resolution, &scan_plan)?;
    Ok(
        json!({        "contract_version": "horoscope_period_calculation_request",        "service_code": service_code,        "chart_calculation_id": public.chart_calculation_id,        "period_resolution": period_resolution,        "scan_plan": scan_plan    }),
    )
}
pub(crate) fn resolve_horoscope_period_window(
    service_code: &str,
    public: &HoroscopePeriodPublicRequest,
) -> Result<Value, GenerationError> {
    let service_profile = period_service_profile(service_code)?;
    let period_profile_code = service_profile
        .period_profile_code
        .as_deref()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_PROFILE_UNSUPPORTED"))?;
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
        period_profile_code: period_profile_code.to_string(),
        anchor_date: public.anchor_date.clone(),
        timezone: public.timezone.clone(),
        custom_start_date: None,
        custom_end_date: None,
    };
    let resolved = resolver.resolve(&request).map_err(period_window_error)?;
    let start_utc = resolved.start_datetime_utc().map_err(period_window_error)?;
    let end_utc = resolved.end_datetime_utc().map_err(period_window_error)?;
    let included_dates = resolved.included_dates();
    Ok(
        json!({        "period_profile_code": period_profile_code,        "anchor_date": public.anchor_date,        "timezone": public.timezone,        "start_datetime_local": resolved.start_datetime_local.format("%Y-%m-%dT%H:%M:%S").to_string(),        "end_datetime_local": resolved.end_datetime_local.format("%Y-%m-%dT%H:%M:%S").to_string(),        "start_datetime_utc": start_utc,        "end_datetime_utc": end_utc,        "end_exclusive": resolved.end_exclusive,        "duration_days": resolved.duration_days,        "included_dates": included_dates,        "included_days": resolved.included_days    }),
    )
}

fn period_window_error(err: astral_time_window::PeriodWindowError) -> GenerationError {
    let code = match err {
        astral_time_window::PeriodWindowError::InvalidTimezone(_) => {
            "HOROSCOPE_PERIOD_TIMEZONE_REQUIRED"
        }
        astral_time_window::PeriodWindowError::InvalidDate {
            field: "anchor_date",
            ..
        } => "HOROSCOPE_PERIOD_ANCHOR_DATE_REQUIRED",
        astral_time_window::PeriodWindowError::InvalidDate { .. } => {
            "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"
        }
        astral_time_window::PeriodWindowError::AmbiguousLocalDateTime { .. }
        | astral_time_window::PeriodWindowError::NonexistentLocalDateTime { .. } => {
            "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"
        }
        astral_time_window::PeriodWindowError::UnknownProfile(_)
        | astral_time_window::PeriodWindowError::DisabledProfile(_)
        | astral_time_window::PeriodWindowError::MissingCustomDateRange
        | astral_time_window::PeriodWindowError::InvalidCustomDateRange
        | astral_time_window::PeriodWindowError::InvalidProfileDefinition { .. } => {
            "HOROSCOPE_PERIOD_PROFILE_UNSUPPORTED"
        }
    };
    GenerationError::with_details(
        GenerationErrorCode::InvalidInput,
        format!("{code}: {err}"),
        Value::Null,
    )
}
pub(crate) fn build_scan_plan(
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
            snapshots.push(json!({                "snapshot_key": format!("{date}:{key_suffix}"),                "date": date,                "reference_time_local": time_label,                "reference_datetime_local": local.format("%Y-%m-%dT%H:%M:%S").to_string(),                "reference_datetime_utc": utc            }));
        }
    }
    let duration_days = period_resolution["duration_days"]
        .as_u64()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"))?
        as usize;
    if snapshots.len() != duration_days * scan_profile.expected_snapshots_per_day {
        return Err(horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"));
    }
    Ok(
        json!({        "scan_profile_code": scan_profile_code,        "granularity": scan_profile.granularity,        "snapshot_count": snapshots.len(),        "snapshots": snapshots    }),
    )
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
pub(crate) fn require_canonical_utc_offset(raw: &str) -> Result<(), GenerationError> {
    let parsed = chrono::DateTime::parse_from_rfc3339(raw)
        .map_err(|_| horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"))?;
    if parsed.with_timezone(&chrono::Utc).to_rfc3339() != raw {
        return Err(horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"));
    }
    Ok(())
}
pub(crate) fn local_to_utc(tz: Tz, local: NaiveDateTime) -> Result<String, GenerationError> {
    tz.from_local_datetime(&local)
        .single()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"))
        .map(|value| value.with_timezone(&chrono::Utc).to_rfc3339())
}
