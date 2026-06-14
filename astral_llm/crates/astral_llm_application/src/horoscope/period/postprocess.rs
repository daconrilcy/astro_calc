use super::*;
pub(crate) fn reprocess_horoscope_daily_payload(response: Value) -> Value {
    reprocess_horoscope_daily("fr", response, None).payload
}
#[doc(hidden)]
pub fn reprocess_horoscope_period_payload(response: Value) -> Value {
    reprocess_horoscope_period("fr", response, None).payload
}
#[doc(hidden)]
pub fn postprocess_period_provider_response(request: &Value, response: Value) -> Value {
    let mut response = response;
    repair_period_response_shape(request, &mut response);
    trim_period_response_strings_v2(&mut response);
    normalize_period_v2_public_short_labels(&mut response);
    prune_period_v2_overlapping_watch_windows(&mut response);
    normalize_period_v2_watch_summary_status(&mut response);
    restore_period_response_technical_keys_v2(request, &mut response);
    normalize_free_period_key_days(&mut response);
    expand_free_period_if_too_short(request, &mut response);
    response
}

pub(crate) fn expand_free_period_if_too_short(request: &Value, response: &mut Value) {
    if response["service_code"].as_str() != Some(HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE) {
        return;
    }
    let limits = period_word_limits_for_request(request);
    let mut public_text = collect_period_v2_public_text(response);
    while simple_public_word_count(&public_text) < limits.target_min {
        append_free_period_summary_expansion(response);
        public_text = collect_period_v2_public_text(response);
        if simple_public_word_count(&public_text) >= limits.target_min
            || simple_public_word_count(&public_text) > limits.hard_limit
        {
            break;
        }
    }
}

fn append_free_period_summary_expansion(response: &mut Value) {
    let theme = response["dominant_theme"]["theme"]
        .as_str()
        .unwrap_or("le thème dominant");
    let addition = format!(
        " Dans cette version courte, {theme} sert surtout de boussole: observez ce qui revient, gardez une priorité simple et laissez une marge pour ajuster le rythme. L'objectif n'est pas de prévoir chaque journée, mais de reconnaître le fil utile de la période, puis de choisir une action concrète, mesurable et réversible."
    );
    let current = response["summary"]["text"].as_str().unwrap_or("").trim();
    response["summary"]["text"] = if current.is_empty() {
        json!(addition.trim())
    } else {
        json!(format!("{current}{addition}"))
    };
}

pub(crate) fn normalize_free_period_key_days(response: &mut Value) {
    if response["service_code"].as_str() != Some(HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE) {
        return;
    }
    let Some(key_days) = response.get_mut("key_days").and_then(Value::as_array_mut) else {
        return;
    };
    for day in key_days {
        let public_text = [
            day.get("title").and_then(Value::as_str),
            day.get("reason").and_then(Value::as_str),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
        if !free_period_key_day_contains_best_day_language(&public_text) {
            continue;
        }
        day["title"] = json!("Jour à retenir");
        day["reason"] = json!(
            "Ce jour sert de repère pour observer le thème dominant sans en faire une promesse."
        );
    }
}

fn free_period_key_day_contains_best_day_language(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
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
    ]
    .iter()
    .any(|term| lower.contains(term))
}
pub(crate) fn prune_period_v2_overlapping_watch_windows(response: &mut Value) {
    let best_identities = response["best_windows"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(period_window_identity)
        .collect::<HashSet<_>>();
    if best_identities.is_empty() {
        return;
    }
    if let Some(watch_windows) = response
        .get_mut("watch_windows")
        .and_then(Value::as_array_mut)
    {
        watch_windows.retain(|window| {
            period_window_identity(window)
                .map(|identity| !best_identities.contains(&identity))
                .unwrap_or(true)
        });
    }
}
pub(crate) fn normalize_period_v2_public_short_labels(response: &mut Value) {
    for array_key in [
        "key_days",
        "best_days",
        "watch_days",
        "daily_timeline",
        "domain_sections",
        "best_windows",
        "watch_windows",
    ] {
        if let Some(items) = response.get_mut(array_key).and_then(Value::as_array_mut) {
            for item in items {
                normalize_period_v2_public_short_label_item(item);
            }
        }
    }
}
pub(crate) fn normalize_period_v2_public_short_label_item(item: &mut Value) {
    if let Some(object) = item.as_object_mut() {
        for field in ["theme", "domain"] {
            if let Some(value) = object.get(field).and_then(Value::as_str) {
                object.insert(
                    field.to_string(),
                    json!(period_theme_public_label_if_code(value)),
                );
            }
        }
        if let Some(value) = object.get("tone").and_then(Value::as_str) {
            object.insert(
                "tone".to_string(),
                json!(period_tone_public_label_if_code(value)),
            );
        }
    }
}
pub(crate) fn normalize_period_v2_watch_summary_status(response: &mut Value) {
    let watch_days_count = response["watch_days"].as_array().map(Vec::len).unwrap_or(0);
    let watch_windows_count = response["watch_windows"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0);
    let original_status = response["watch_summary"]["status"]
        .as_str()
        .unwrap_or("")
        .to_string();
    if original_status == "active" && watch_days_count == 0 && watch_windows_count > 0 {
        response["watch_summary"]["status"] = json!("low");
    }
    if matches!(original_status.as_str(), "active" | "low")
        && watch_days_count == 0
        && watch_windows_count == 0
    {
        response["watch_summary"]["status"] = json!("none");
        response["watch_summary"]["evidence_keys"] = json!([]);
    }
    if original_status == "none" {
        response["watch_summary"]["evidence_keys"] = json!([]);
    }
    if original_status == "none"
        && response["watch_summary"]["status"].as_str() == Some("none")
        && watch_days_count == 0
        && watch_windows_count == 0
    {
        response["watch_summary"]["evidence_keys"] = json!([]);
    }
}
pub(crate) fn period_window_title_conflicts_with_time(time_range_label: &str, title: &str) -> bool {
    let lower_title = title.to_lowercase();
    if !lower_title.contains("matin") {
        return false;
    }
    period_window_start_hour(time_range_label).is_some_and(|hour| hour >= 12)
}
pub(crate) fn period_window_start_hour(time_range_label: &str) -> Option<u32> {
    let start = time_range_label
        .split(['–', '-', '—'])
        .next()
        .unwrap_or("")
        .trim();
    start
        .split(':')
        .next()
        .and_then(|hour| hour.trim().parse::<u32>().ok())
}
