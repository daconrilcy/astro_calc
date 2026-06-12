use super::*;

pub fn validate_semantic_brief_is_atomic(value: &Value) -> Result<(), GenerationError> {
    if !is_period_writer_request_v2(value) {
        return Ok(());
    }
    if value["service_code"].as_str() != Some(HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE) {
        return Err(period_v2_request_error(
            "HOROSCOPE_PERIOD_WRITER_V2_PREMIUM_ONLY",
            json!({ "service_code": value["service_code"] }),
        ));
    }
    if contains_key_recursive(value, "human_label") {
        return Err(period_v2_request_error(
            "HOROSCOPE_PERIOD_WRITER_REQUEST_V2_LEGACY_FIELD",
            json!({ "field": "human_label" }),
        ));
    }
    let semantic = value.get("semantic_brief").ok_or_else(|| {
        period_v2_request_error(
            "HOROSCOPE_PERIOD_WRITER_REQUEST_V2_INVALID",
            json!({ "missing": "semantic_brief" }),
        )
    })?;
    validate_semantic_brief_forbidden_keys(semantic)?;
    validate_semantic_brief_strings(semantic, "")?;
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
pub(crate) fn validate_semantic_brief_forbidden_keys(value: &Value) -> Result<(), GenerationError> {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if PERIOD_V2_FORBIDDEN_WRITER_KEYS.contains(&key.as_str()) {
                    return Err(period_v2_request_error(
                        "HOROSCOPE_PERIOD_WRITER_REQUEST_V2_LEGACY_FIELD",
                        json!({ "field": key }),
                    ));
                }
                validate_semantic_brief_forbidden_keys(child)?;
            }
        }
        Value::Array(items) => {
            for item in items {
                validate_semantic_brief_forbidden_keys(item)?;
            }
        }
        _ => {}
    }
    Ok(())
}
pub(crate) fn validate_semantic_brief_strings(
    value: &Value,
    field_name: &str,
) -> Result<(), GenerationError> {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                validate_semantic_brief_strings(child, key)?;
            }
        }
        Value::Array(items) => {
            for item in items {
                validate_semantic_brief_strings(item, field_name)?;
            }
        }
        Value::String(text) => {
            if field_name != "time_range_label"
                && field_name != "signature_code"
                && text.chars().count() > 100
            {
                return Err(period_v2_request_error(
                    "HOROSCOPE_PERIOD_WRITER_REQUEST_V2_PROSE_LEAK",
                    json!({ "field": field_name, "reason": "string_too_long" }),
                ));
            }
            if is_period_v2_keyword_field(field_name) {
                validate_period_v2_keyword_fragment(field_name, text)?;
            }
        }
        _ => {}
    }
    Ok(())
}
pub(crate) fn is_period_v2_keyword_field(field_name: &str) -> bool {
    matches!(
        field_name,
        "keywords"
            | "usage_keywords"
            | "dominant_keywords"
            | "period_arc_keywords"
            | "opportunity_keywords"
            | "risk_keywords"
            | "avoid_keywords"
    )
}
pub(crate) fn validate_period_v2_keyword_fragment(
    field_name: &str,
    text: &str,
) -> Result<(), GenerationError> {
    if text
        .chars()
        .any(|ch| matches!(ch, '.' | '!' | '?' | ':' | ';' | '\n' | '\r'))
    {
        return Err(period_v2_request_error(
            "HOROSCOPE_PERIOD_WRITER_REQUEST_V2_KEYWORD_PROSE",
            json!({ "field": field_name, "keyword": text, "reason": "punctuation" }),
        ));
    }
    let word_count = text.split_whitespace().count();
    let lower = text.to_lowercase();
    let padded = format!(" {lower} ");
    let likely_public_sentence = word_count > 5
        && [
            " donne ",
            " apporte ",
            " permet ",
            " invite ",
            " consiste ",
            " vérifiez ",
            " aide ",
            " soutient ",
            " demande ",
            " ouvre ",
        ]
        .iter()
        .any(|needle| padded.contains(needle));
    if likely_public_sentence {
        return Err(period_v2_request_error(
            "HOROSCOPE_PERIOD_WRITER_REQUEST_V2_KEYWORD_PROSE",
            json!({ "field": field_name, "keyword": text, "reason": "likely_sentence" }),
        ));
    }
    Ok(())
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
pub(crate) fn contains_key_recursive(value: &Value, needle: &str) -> bool {
    match value {
        Value::Object(object) => object
            .iter()
            .any(|(key, child)| key == needle || contains_key_recursive(child, needle)),
        Value::Array(items) => items
            .iter()
            .any(|item| contains_key_recursive(item, needle)),
        _ => false,
    }
}
