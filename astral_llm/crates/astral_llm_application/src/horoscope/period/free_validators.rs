use super::*;
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
pub(crate) fn validate_free_period_provider_public_payload(
    response: &Value,
) -> Result<(), GenerationError> {
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
pub(crate) fn validate_free_period_key_days_are_neutral_markers(
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
        if reason.split_whitespace().count() < 6 {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_FREE_KEY_DAY_TOO_THIN",
                json!({ "field": "key_days.reason", "index": index }),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_period_accepts_non_literal_key_day_title_when_marker_stays_neutral() {
        let response = json!({
            "service_code": HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
            "summary": {
                "title": "Vos 7 prochains jours",
                "text": "Une tendance générale se dessine sans transformer la semaine en programme rigide."
            },
            "dominant_theme": {
                "theme": "Relations",
                "text": "Le climat dominant aide à prioriser les échanges utiles."
            },
            "key_days": [{
                "date": "2026-06-14",
                "title": "Point d'appui de la semaine",
                "reason": "Ce repère aide à cadrer une décision simple sans en faire un verdict.",
                "evidence_keys": ["signal:1"]
            }],
            "advice": "Choisissez une action simple puis ajustez selon les retours.",
            "watch_summary": {
                "status": "low",
                "text": "Une vigilance légère suffit pour éviter les réactions trop rapides.",
                "evidence_keys": ["signal:1"]
            },
            "evidence_summary": [{
                "date": "2026-06-14",
                "evidence_key": "signal:1",
                "label": "Climat relationnel plus sensible"
            }]
        });

        assert!(validate_free_period_provider_public_payload(&response).is_ok());
    }

    #[test]
    fn free_period_rejects_key_day_reason_that_is_too_thin() {
        let response = json!({
            "key_days": [{
                "date": "2026-06-14",
                "title": "Point d'appui",
                "reason": "Ralentir avant de conclure."
            }]
        });

        let error = validate_free_period_key_days_are_neutral_markers(&response).unwrap_err();
        assert_eq!(
            error.detail().message,
            "HOROSCOPE_PERIOD_FREE_KEY_DAY_TOO_THIN"
        );
    }
}
pub(crate) fn validate_free_period_required_fields(
    response: &Value,
) -> Result<(), GenerationError> {
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
pub(crate) fn free_required_string_missing(response: &Value, pointer: &str) -> bool {
    response
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(|text| text.trim().is_empty())
        .unwrap_or(true)
}
pub(crate) fn validate_free_period_forbidden_leaks(
    response: &Value,
) -> Result<(), GenerationError> {
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
pub(crate) fn require_period_watch_summary(response: &Value) -> Result<(), GenerationError> {
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
pub(crate) fn require_period_public_marker_array(
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
pub(crate) fn require_period_public_string(
    value: &Value,
    path: &[&str],
) -> Result<(), GenerationError> {
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
pub(crate) fn require_period_public_string_in(
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
