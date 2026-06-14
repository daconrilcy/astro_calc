use super::*;
use crate::text_reprocessing::normalize_em_dashes;

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
    sanitize_period_provider_artifacts(&mut response);
    response = reprocess_horoscope_period("fr", response, None).payload;
    repair_period_response_shape(request, &mut response);
    repair_period_week_overview_trajectory(request, &mut response);
    prune_period_v2_overlapping_watch_windows(&mut response);
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

fn sanitize_period_provider_artifacts(response: &mut Value) {
    sanitize_period_public_value(response, None);
}

fn sanitize_period_public_value(value: &mut Value, parent_key: Option<&str>) {
    match value {
        Value::Object(map) => {
            let keys = map.keys().cloned().collect::<Vec<_>>();
            for key in keys {
                if let Some(child) = map.get_mut(&key) {
                    sanitize_period_public_value(child, Some(&key));
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                sanitize_period_public_value(item, parent_key);
            }
        }
        Value::String(text) if parent_key.is_some_and(is_period_public_text_key) => {
            let cleaned = sanitize_period_public_text(text);
            if cleaned != *text {
                *text = cleaned;
            }
        }
        _ => {}
    }
}

fn is_period_public_text_key(key: &str) -> bool {
    matches!(
        key,
        "title"
            | "text"
            | "trajectory"
            | "theme"
            | "tone"
            | "domain"
            | "label"
            | "summary"
            | "reason"
            | "watch_point"
            | "advice"
            | "main"
            | "best_use"
            | "avoid"
            | "recovery"
    )
}

fn sanitize_period_public_text(text: &str) -> String {
    let normalized = normalize_em_dashes(&text.trim().replace("\r\n", "\n")).replace('_', " ");
    let cutoff = period_provider_artifact_markers()
        .iter()
        .filter_map(|marker| find_case_insensitive(&normalized, marker))
        .min();
    let cleaned = cutoff
        .map(|index| normalized[..index].trim_end())
        .unwrap_or_else(|| normalized.trim_end());
    let cleaned = cleaned.trim_end_matches(|ch: char| {
        ch.is_whitespace() || matches!(ch, '"' | '\'' | '`' | '{' | '}' | '[' | ']' | '<' | '>')
    });
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn period_provider_artifact_markers() -> &'static [&'static str] {
    &[
        "</structured_reading>",
        "<structured_reading",
        "pmid:invalid.json",
        "the output seems malformed",
        "i accidentally included",
        "need produce valid json",
        "must output valid",
        "time's up",
        "i'll output minimal",
        "sorry. (but must output valid.)",
        "(removed)",
    ]
}

fn find_case_insensitive(text: &str, needle: &str) -> Option<usize> {
    text.to_lowercase().find(&needle.to_lowercase())
}

fn repair_period_week_overview_trajectory(request: &Value, response: &mut Value) {
    let Some(trajectory) = response
        .pointer("/week_overview/trajectory")
        .and_then(Value::as_str)
    else {
        return;
    };
    if trajectory_needs_fallback(trajectory) {
        response["week_overview"]["trajectory"] = json!(fallback_period_trajectory(request));
    }
}

fn trajectory_needs_fallback(trajectory: &str) -> bool {
    let lower = trajectory.to_lowercase();
    let phase_list_like = lower.contains("ouverture")
        && lower.contains("mise en mouvement")
        && lower.contains("pivot")
        && lower.contains("consolidation")
        && (lower.contains("clôture") || lower.contains("cloture"));
    lower.contains('{')
        || lower.contains('}')
        || lower.contains("(removed)")
        || lower.contains(" removed")
        || lower.contains("mise_en_")
        || phase_list_like
}

fn fallback_period_trajectory(request: &Value) -> String {
    let final_tone = request["semantic_brief"]["daily_signal_summary"]
        .as_array()
        .and_then(|days| days.last())
        .and_then(|day| day["tone_codes"].as_array())
        .and_then(|tones| tones.first())
        .and_then(Value::as_str)
        .unwrap_or("focused");
    let closing = if final_tone == "careful" {
        "une clôture prudente"
    } else {
        "une clôture claire"
    };
    format!(
        "La semaine suit une ouverture posée, un passage à l'action progressif, un pivot de clarification, une consolidation, puis {}.",
        closing
    )
}
