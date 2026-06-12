use super::*;

pub(crate) fn repair_daily_response_shape(request: &Value, response: &mut Value) {
    response["contract_version"] = json!("horoscope_response_v1");
    if response
        .get("service_code")
        .and_then(Value::as_str)
        .is_none()
    {
        response["service_code"] = request
            .get("service_code")
            .cloned()
            .unwrap_or_else(|| json!(HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE));
    }
    if response.get("period").is_none() {
        response["period"] = request.get("period").cloned().unwrap_or_else(|| json!({}));
    }
    let service_code = response
        .get("service_code")
        .and_then(Value::as_str)
        .or_else(|| request.get("service_code").and_then(Value::as_str));
    if service_code != Some(HOROSCOPE_FREE_DAILY_SERVICE_CODE)
        && response.get("evidence_summary").is_none()
    {
        response["evidence_summary"] = request
            .get("evidence")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|item| {
                json!({
                    "evidence_key": item.get("evidence_key").cloned().unwrap_or(Value::Null),
                    "theme_code": item.get("theme_code").cloned().unwrap_or(Value::Null)
                })
            })
            .collect::<Vec<_>>()
            .into();
    }
    repair_daily_free_astro_reference(request, response);
    repair_daily_basic_astro_references(request, response);
}

pub(crate) fn repair_premium_daily_editorial_repetition(response: &mut Value) {
    if response.get("service_code").and_then(Value::as_str)
        != Some(HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE)
    {
        return;
    }
    let timeline_text_by_label = response
        .get("timeline")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|slot| {
            let label = slot.get("slot_label")?.as_str()?.to_string();
            let candidates = ["text", "advice", "fallback_reason"]
                .into_iter()
                .filter_map(|key| slot.get(key).and_then(Value::as_str))
                .filter_map(first_public_sentence)
                .collect::<Vec<_>>();
            Some((label, candidates))
        })
        .collect::<HashMap<_, _>>();

    repair_premium_slot_summary_reasons(response, "best_slots", &timeline_text_by_label);
    repair_premium_slot_summary_reasons(response, "watch_slots", &timeline_text_by_label);
}

pub(crate) fn repair_premium_slot_summary_reasons(
    response: &mut Value,
    field: &str,
    timeline_text_by_label: &HashMap<String, Vec<String>>,
) {
    let Some(slots) = response.get_mut(field).and_then(Value::as_array_mut) else {
        return;
    };
    let mut used = HashSet::new();
    for slot in slots {
        let reason = slot
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let normalized = normalize_editorial_sentence(&reason);
        if normalized.is_empty() || used.insert(normalized) {
            continue;
        }
        let Some(label) = slot.get("slot_label").and_then(Value::as_str) else {
            continue;
        };
        let Some(replacement) = timeline_text_by_label.get(label).and_then(|candidates| {
            candidates
                .iter()
                .find(|candidate| used.insert(normalize_editorial_sentence(candidate)))
        }) else {
            continue;
        };
        slot["reason"] = json!(replacement);
    }
}

pub(crate) fn first_public_sentence(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let end = trimmed
        .char_indices()
        .find_map(|(idx, ch)| matches!(ch, '.' | '!' | '?').then_some(idx + ch.len_utf8()))
        .unwrap_or(trimmed.len());
    let sentence = trimmed[..end].trim();
    if sentence.split_whitespace().count() < 5 {
        None
    } else {
        Some(sentence.to_string())
    }
}

pub(crate) fn normalize_editorial_sentence(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn repair_daily_free_astro_reference(request: &Value, response: &mut Value) {
    if request.get("service_code").and_then(Value::as_str)
        != Some(HOROSCOPE_FREE_DAILY_SERVICE_CODE)
    {
        return;
    }
    let public_text = free_public_text(response);
    if daily_text_has_astrological_reference(&public_text) {
        return;
    }
    let Some(prefix) = daily_response_astro_reference_prefix(request, response) else {
        return;
    };
    let current = response
        .get("summary")
        .and_then(|summary| summary.get("text"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    response["summary"]["text"] = if current.is_empty() {
        json!(prefix)
    } else {
        json!(format!("{prefix} {current}"))
    };
}

pub(crate) fn repair_daily_basic_astro_references(request: &Value, response: &mut Value) {
    if request.get("service_code").and_then(Value::as_str)
        != Some(HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE)
    {
        return;
    }
    let Some(slots) = response.get_mut("slots").and_then(Value::as_array_mut) else {
        return;
    };
    for slot in slots {
        let text = slot.get("text").and_then(Value::as_str).unwrap_or("");
        if daily_text_has_astrological_reference(text) {
            continue;
        }
        let Some(prefix) = daily_response_astro_reference_prefix(request, slot) else {
            continue;
        };
        let repaired = if text.trim().is_empty() {
            prefix
        } else {
            format!("{prefix} {}", text.trim())
        };
        slot["text"] = json!(repaired);
    }
}

pub(crate) fn daily_text_has_astrological_reference(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "lune",
        "mars",
        "vénus",
        "venus",
        "mercure",
        "aspect",
        "maison",
        "transit",
        "astrologique",
        "natal",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

pub(crate) fn daily_response_astro_reference_prefix(
    request: &Value,
    response: &Value,
) -> Option<String> {
    let evidence_keys = response
        .get("evidence_keys")
        .or_else(|| response.get("required_evidence_keys"))
        .and_then(Value::as_array)?;
    let first_key = evidence_keys.iter().find_map(Value::as_str)?;
    let evidence = request.get("evidence").and_then(Value::as_array)?;
    let signal = evidence
        .iter()
        .find(|item| item.get("evidence_key").and_then(Value::as_str) == Some(first_key))?;
    let object = signal
        .get("transiting_object")
        .and_then(Value::as_str)
        .map(public_astro_object_label)
        .unwrap_or("Un transit");
    if object == "Un transit" {
        Some("Un transit astrologique donne le repère du créneau.".to_string())
    } else {
        Some(format!("{object} donne le repère astrologique du créneau."))
    }
}

pub(crate) fn public_astro_object_label(code: &str) -> &'static str {
    match code {
        "sun" => "Le Soleil",
        "moon" => "La Lune",
        "mercury" => "Mercure",
        "venus" => "Vénus",
        "mars" => "Mars",
        "jupiter" => "Jupiter",
        "saturn" => "Saturne",
        "uranus" => "Uranus",
        "neptune" => "Neptune",
        "pluto" => "Pluton",
        _ => "Un transit",
    }
}

pub(crate) fn premium_slot_summary(slot: &Value, watch: bool) -> Value {
    let label = slot
        .get("slot_label")
        .cloned()
        .unwrap_or_else(|| json!("Moment"));
    let evidence_keys = slot
        .get("required_evidence_keys")
        .cloned()
        .unwrap_or_else(|| json!([]));
    json!({
        "slot_label": label,
        "title": if watch { "Créneau de vigilance" } else { "Créneau favorable" },
        "reason": premium_slot_summary_reason(slot, watch),
        "best_for": slot.get("best_for").cloned().unwrap_or_else(|| json!([])),
        "avoid": if watch { json!(["réponse impulsive"]) } else { json!([]) },
        "evidence_keys": evidence_keys
    })
}

pub(crate) fn premium_slot_summary_reason(slot: &Value, watch: bool) -> String {
    let label = slot
        .get("slot_label")
        .and_then(Value::as_str)
        .unwrap_or("ce créneau");
    if watch {
        format!("{label} demande de filtrer les réactions et de garder une réponse proportionnée.")
    } else {
        format!("{label} soutient une action simple, utile et facile à vérifier.")
    }
}
