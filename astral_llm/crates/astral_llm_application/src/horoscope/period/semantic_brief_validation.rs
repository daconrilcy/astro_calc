use super::*;

pub fn validate_semantic_brief_references_only(value: &Value) -> Result<(), GenerationError> {
    let semantic = value.get("semantic_brief").ok_or_else(|| {
        period_v2_request_error(
            "HOROSCOPE_PERIOD_WRITER_REQUEST_INVALID",
            json!({ "missing": "semantic_brief" }),
        )
    })?;
    let included_dates = value["period_resolution"]["included_dates"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<HashSet<_>>();
    let snapshot_keys = value["scan_plan"]["snapshots"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|snapshot| snapshot["snapshot_key"].as_str())
        .collect::<HashSet<_>>();
    let evidence_items = value["evidence"].as_array().ok_or_else(|| {
        period_v2_request_error(
            "HOROSCOPE_PERIOD_EVIDENCE_MISSING",
            json!({ "field": "evidence" }),
        )
    })?;
    let mut evidence_keys = HashSet::new();
    for item in evidence_items {
        let Some(key) = item["evidence_key"].as_str() else {
            return Err(period_v2_request_error(
                "HOROSCOPE_PERIOD_EVIDENCE_MISSING",
                json!({ "field": "evidence.evidence_key" }),
            ));
        };
        if !evidence_keys.insert(key) {
            return Err(period_v2_request_error(
                "HOROSCOPE_PERIOD_EVIDENCE_DUPLICATE",
                json!({ "evidence_key": key }),
            ));
        }
    }
    validate_semantic_brief_references(
        semantic,
        "",
        &included_dates,
        &snapshot_keys,
        &evidence_keys,
    )
}
pub(crate) fn period_v2_request_error(message: &str, details: Value) -> GenerationError {
    GenerationError::with_details(GenerationErrorCode::InvalidInput, message, details)
}
pub(crate) fn validate_semantic_brief_references(
    value: &Value,
    field_name: &str,
    included_dates: &HashSet<&str>,
    snapshot_keys: &HashSet<&str>,
    evidence_keys: &HashSet<&str>,
) -> Result<(), GenerationError> {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                validate_semantic_brief_references(
                    child,
                    key,
                    included_dates,
                    snapshot_keys,
                    evidence_keys,
                )?;
            }
        }
        Value::Array(items) => {
            if matches!(field_name, "evidence_keys" | "source_snapshot_keys") {
                let mut seen = HashSet::new();
                for item in items {
                    let Some(raw) = item.as_str() else {
                        return Err(period_v2_request_error(
                            "HOROSCOPE_PERIOD_WRITER_REQUEST_V2_INVALID_REFERENCE",
                            json!({ "field": field_name }),
                        ));
                    };
                    if !seen.insert(raw) {
                        return Err(period_v2_request_error(
                            "HOROSCOPE_PERIOD_WRITER_REQUEST_V2_DUPLICATE_REFERENCE",
                            json!({ "field": field_name, "value": raw }),
                        ));
                    }
                    let allowed = if field_name == "evidence_keys" {
                        evidence_keys.contains(raw)
                    } else {
                        snapshot_keys.contains(raw)
                    };
                    if !allowed {
                        return Err(period_v2_request_error(
                            "HOROSCOPE_PERIOD_WRITER_REQUEST_V2_UNKNOWN_REFERENCE",
                            json!({ "field": field_name, "value": raw }),
                        ));
                    }
                }
            } else {
                for item in items {
                    validate_semantic_brief_references(
                        item,
                        field_name,
                        included_dates,
                        snapshot_keys,
                        evidence_keys,
                    )?;
                }
            }
        }
        Value::String(raw) if field_name == "date" => {
            if !included_dates.contains(raw.as_str()) {
                return Err(period_v2_request_error(
                    "HOROSCOPE_PERIOD_WRITER_REQUEST_V2_DATE_OUTSIDE_PERIOD",
                    json!({ "date": raw }),
                ));
            }
        }
        _ => {}
    }
    Ok(())
}
