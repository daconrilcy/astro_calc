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
    let mut response = reprocess_horoscope_period_payload(response);
    prune_period_response_variant_fields(request, &mut response);
    finalize_period_response_words_and_repetition(request, &mut response);
    prune_period_response_variant_fields(request, &mut response);
    response
}
pub fn postprocess_period_provider_response_v2(request: &Value, response: Value) -> Value {
    let mut response = response;
    prune_period_response_variant_fields_v2(request, &mut response);
    trim_period_response_strings_v2(&mut response);
    normalize_period_v2_public_short_labels(&mut response);
    prune_period_v2_overlapping_watch_windows(&mut response);
    normalize_period_v2_watch_summary_status(&mut response);
    prune_period_response_variant_fields_v2(request, &mut response);
    response
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
