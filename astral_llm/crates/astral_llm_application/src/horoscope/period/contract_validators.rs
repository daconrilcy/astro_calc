use super::*;
pub fn validate_period_response_contract_gates_v2(
    request: &Value,
    response: &Value,
) -> Result<(), GenerationError> {
    validate_period_response_schema(response)?;
    validate_period_response_identity_contract_v2(request, response)?;
    let included = period_included_dates_from_request(request)?;
    let evidence = period_evidence_keys_from_request(request)?;
    let snapshot_keys = period_snapshot_keys_from_request(request)?;
    validate_period_timeline_contract_v2(response, &included, &evidence)?;
    validate_period_day_markers_contract_v2(response, "key_days", &included, &evidence)?;
    validate_period_day_markers_contract_v2(response, "best_days", &included, &evidence)?;
    validate_period_day_markers_contract_v2(response, "watch_days", &included, &evidence)?;
    validate_period_marker_date_overlaps(response)?;
    validate_period_watch_summary(response, &evidence)?;
    validate_period_domain_sections(response, &evidence)?;
    validate_period_evidence_summary(response, &included, &evidence)?;
    if is_premium_period_request(request) {
        validate_period_premium_windows_contract_v2(
            response,
            &included,
            &evidence,
            &snapshot_keys,
        )?;
        validate_period_premium_strategy(response, &evidence)?;
        validate_period_premium_detail_structure(response)?;
    }
    let public_text = collect_period_v2_public_text(response);
    validate_period_v2_public_text_forbidden_technical_leaks(&public_text)?;
    validate_period_public_word_count(request, response, &public_text)?;
    Ok(())
}
pub(crate) fn validate_period_response_identity_contract_v2(
    request: &Value,
    response: &Value,
) -> Result<(), GenerationError> {
    let request_service = request["service_code"]
        .as_str()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_RESPONSE_INVALID"))?;
    let response_service = response["service_code"]
        .as_str()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_RESPONSE_INVALID"))?;
    if response_service != request_service {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_RESPONSE_INVALID",
            json!({                "field": "service_code",                "expected": request_service,                "actual": response_service            }),
        ));
    }
    if response["period_resolution"] != request["period_resolution"] {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH",
            json!({ "field": "period_resolution" }),
        ));
    }
    Ok(())
}
pub(crate) fn period_included_dates_from_request(
    request: &Value,
) -> Result<HashSet<&str>, GenerationError> {
    let included = request["period_resolution"]["included_dates"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<HashSet<_>>();
    if included.len() != 7 {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH",
            json!({ "included_date_count": included.len() }),
        ));
    }
    Ok(included)
}
pub(crate) fn period_evidence_keys_from_request(
    request: &Value,
) -> Result<HashSet<&str>, GenerationError> {
    let evidence = request["evidence"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item["evidence_key"].as_str())
        .collect::<HashSet<_>>();
    if evidence.is_empty() {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_EVIDENCE_MISSING",
            json!({ "field": "evidence" }),
        ));
    }
    Ok(evidence)
}
pub(crate) fn period_snapshot_keys_from_request(
    request: &Value,
) -> Result<HashSet<&str>, GenerationError> {
    let snapshot_keys = request["scan_plan"]["snapshots"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|snapshot| snapshot["snapshot_key"].as_str())
        .collect::<HashSet<_>>();
    if snapshot_keys.is_empty() {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_PREMIUM_WINDOWS_MISSING",
            json!({ "field": "scan_plan.snapshots" }),
        ));
    }
    Ok(snapshot_keys)
}
pub(crate) fn validate_period_timeline_contract_v2(
    response: &Value,
    included: &HashSet<&str>,
    evidence: &HashSet<&str>,
) -> Result<(), GenerationError> {
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
        validate_period_evidence_keys(evidence, day["evidence_keys"].as_array())?;
    }
    for date in included {
        if !timeline_dates.contains(date) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH",
                json!({ "missing_date": date }),
            ));
        }
    }
    Ok(())
}
pub(crate) fn validate_period_day_markers_contract_v2(
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
        validate_period_evidence_keys(evidence, marker["evidence_keys"].as_array())?;
    }
    Ok(())
}
