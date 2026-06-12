use super::*;

pub fn validate_period_interpretation_request_schema(value: &Value) -> Result<(), GenerationError> {
    validate_schema(
        period_interpretation_request_schema,
        "HOROSCOPE_PERIOD_RESPONSE_INVALID",
        value,
    )
}
#[doc(hidden)]
pub fn validate_period_writer_request_v2_schema(value: &Value) -> Result<(), GenerationError> {
    validate_schema(
        period_writer_request_v2_schema,
        "HOROSCOPE_PERIOD_WRITER_REQUEST_INVALID",
        value,
    )?;
    validate_semantic_brief_is_atomic(value)
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
    if is_period_writer_request_v2(request) {
        return validate_period_response_contract_gates_v2(request, response);
    }
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
        validate_period_premium_public_not_meta(&public_text)?;
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
pub(crate) fn validate_free_period_response_evidence(
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
pub(crate) fn validate_free_period_not_too_generic(
    response: &Value,
) -> Result<(), GenerationError> {
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
pub(crate) fn validate_period_watch_summary(
    response: &Value,
    evidence: &HashSet<&str>,
) -> Result<(), GenerationError> {
    let summary = &response["watch_summary"];
    let status = summary["status"]
        .as_str()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_RESPONSE_INVALID"))?;
    let watch_count = response["watch_days"].as_array().map(Vec::len).unwrap_or(0);
    let watch_window_count = response["watch_windows"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0);
    if !matches!(status, "none" | "low" | "active") {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_BEST_WATCH_MISSING",
            json!({ "status": status }),
        ));
    }
    if (status == "none" && (watch_count > 0 || watch_window_count > 0))
        || (status == "active" && watch_count == 0 && watch_window_count == 0)
    {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_BEST_WATCH_MISSING",
            json!({                "status": status,                "watch_count": watch_count,                "watch_window_count": watch_window_count            }),
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
pub(crate) fn validate_period_day_markers(
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
        if field == "best_days"
            && marker["reason"]
                .as_str()
                .map(|reason| {
                    reason
                        .to_lowercase()
                        .contains("avant de promettre davantage")
                })
                .unwrap_or(false)
        {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_MECHANICAL_PUBLIC_TEXT",
                json!({ "field": field, "date": date, "reason": "best_day_uses_watch_wording" }),
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
pub(crate) fn validate_period_evidence_keys(
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
pub(crate) fn validate_period_marker_date_overlaps(
    response: &Value,
) -> Result<(), GenerationError> {
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
pub(crate) fn validate_period_domain_sections(
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
pub(crate) fn validate_period_premium_windows(
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
        validate_period_watch_windows_not_meta(watch)?;
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
pub(crate) fn validate_period_premium_windows_contract_v2(
    response: &Value,
    included: &HashSet<&str>,
    evidence: &HashSet<&str>,
    snapshot_keys: &HashSet<&str>,
) -> Result<(), GenerationError> {
    let best = response["best_windows"]
        .as_array()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_PREMIUM_WINDOWS_MISSING"))?;
    if best.is_empty() {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_PREMIUM_WINDOWS_MISSING",
            json!({ "field": "best_windows" }),
        ));
    }
    validate_period_window_array("best_windows", best, included, evidence, snapshot_keys)?;
    let watch = response["watch_windows"]
        .as_array()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_PREMIUM_WINDOWS_MISSING"))?;
    if !watch.is_empty() {
        validate_period_window_array("watch_windows", watch, included, evidence, snapshot_keys)?;
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
pub(crate) fn validate_period_watch_windows_not_meta(
    windows: &[Value],
) -> Result<(), GenerationError> {
    for window in windows {
        let text = [
            window.get("title").and_then(Value::as_str),
            window.get("watch_point").and_then(Value::as_str),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
        for forbidden in period_editorial_meta_forbidden_terms() {
            if text.contains(forbidden) {
                return Err(quality_error(
                    "HOROSCOPE_PERIOD_PREMIUM_WINDOW_META_LEAK",
                    json!({ "forbidden": forbidden }),
                ));
            }
        }
    }
    Ok(())
}
pub(crate) fn validate_period_premium_public_not_meta(
    public_text: &str,
) -> Result<(), GenerationError> {
    let lower = public_text.to_lowercase();
    for forbidden in period_editorial_meta_forbidden_terms() {
        if lower.contains(forbidden) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_PREMIUM_PUBLIC_META_LEAK",
                json!({ "forbidden": forbidden }),
            ));
        }
    }
    Ok(())
}
pub(crate) fn period_editorial_meta_forbidden_terms() -> &'static [&'static str] {
    &[
        "nouvelle facette",
        "répéter le même conseil",
        "repeter le meme conseil",
        "fonction narrative",
        "changer l'usage",
        "changer l’usage",
    ]
}
pub(crate) fn validate_period_best_windows_not_generic(
    windows: &[Value],
) -> Result<(), GenerationError> {
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
    let generic_reasons = windows
        .iter()
        .filter_map(|window| window["reason"].as_str())
        .filter(|reason| period_best_window_reason_is_generic(reason))
        .count();
    if generic_titles > 0
        || generic_reasons > 0
        || (windows.len() >= 3 && (titles.len() < 2 || best_for_sets.len() < 2))
    {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_PREMIUM_WINDOWS_TOO_GENERIC",
            json!({                "title_count": titles.len(),                "best_for_count": best_for_sets.len(),                "generic_titles": generic_titles,                "generic_reasons": generic_reasons            }),
        ));
    }
    Ok(())
}
pub(crate) fn validate_period_window_array(
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
pub(crate) fn validate_period_premium_strategy(
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
pub(crate) fn validate_period_premium_detail(response: &Value) -> Result<(), GenerationError> {
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
pub(crate) fn is_premium_period_request(request: &Value) -> bool {
    request["service_code"].as_str() == Some(HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE)
}
pub(crate) fn is_free_period_request(request: &Value) -> bool {
    request["service_code"].as_str() == Some(HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE)
}
pub(crate) fn is_period_writer_request_v2(request: &Value) -> bool {
    request["contract_version"].as_str() == Some("horoscope_period_writer_request_v2")
}
pub(crate) fn collect_period_v2_public_text_only(response: &Value, public_text: &mut String) {
    for pointer in [
        "/week_overview/title",
        "/week_overview/text",
        "/week_overview/trajectory",
        "/advice/main",
        "/advice/best_use",
        "/advice/avoid",
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
    for field in [
        "key_days",
        "best_days",
        "watch_days",
        "daily_timeline",
        "domain_sections",
        "best_windows",
        "watch_windows",
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
                "summary",
                "advice",
            ] {
                if let Some(value) = item.get(key).and_then(Value::as_str) {
                    public_text.push_str(value);
                    public_text.push('\n');
                }
            }
        }
    }
}
pub(crate) fn validate_period_evidence_summary(
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
pub(crate) fn is_period_major_aspect(aspect: &str) -> bool {
    matches!(
        aspect,
        "conjunction" | "sextile" | "square" | "trine" | "opposition"
    )
}
pub(crate) fn period_max_major_aspect_orb_deg() -> f64 {
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
