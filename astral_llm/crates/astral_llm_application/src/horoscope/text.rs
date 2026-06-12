use super::*;
pub(crate) fn validate_slot_specificity(slot: &Value) -> Result<(), GenerationError> {
    let specificity = slot
        .get("specificity")
        .and_then(|v| v.as_str())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let required = slot
        .get("required_evidence_keys")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let fallback_reason = slot.get("fallback_reason").and_then(|v| v.as_str());
    match specificity {
        "specific" => {
            if required.is_empty() {
                return Err(quality_error(
                    "HOROSCOPE_SLOT_EVIDENCE_MISSING",
                    json!({ "reason": "specific_without_required_evidence" }),
                ));
            }
        }
        "shared" => {
            if required.is_empty() {
                return Err(quality_error(
                    "HOROSCOPE_SLOT_EVIDENCE_MISSING",
                    json!({ "reason": "shared_without_required_evidence" }),
                ));
            }
            let has_differentiator = ["tone", "intensity", "advice_axis", "watch_point"]
                .iter()
                .any(|key| slot.get(*key).and_then(|v| v.as_str()).is_some())
                || slot
                    .get("best_for")
                    .and_then(|v| v.as_array())
                    .map(|items| !items.is_empty())
                    .unwrap_or(false);
            if !has_differentiator {
                return Err(quality_error(
                    "HOROSCOPE_SLOT_THEME_DUPLICATED",
                    json!({ "reason": "shared_without_differentiator" }),
                ));
            }
        }
        "fallback" => {
            if !required.is_empty() || fallback_reason.unwrap_or("").trim().is_empty() {
                return Err(quality_error(
                    "HOROSCOPE_SLOT_FALLBACK_INVALID",
                    json!({ "reason": "fallback_requires_empty_evidence_and_reason" }),
                ));
            }
        }
        _ => return Err(horoscope_error("HOROSCOPE_RESPONSE_INVALID")),
    }
    Ok(())
}
pub(crate) fn validate_slot_evidence_alignment(
    request_slot: &Value,
    response_keys: &[Value],
) -> Result<(), GenerationError> {
    let required = request_slot
        .get("required_evidence_keys")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str())
        .collect::<HashSet<_>>();
    let specificity = request_slot
        .get("specificity")
        .and_then(|v| v.as_str())
        .unwrap_or("specific");
    if specificity != "fallback" {
        for key in response_keys.iter().filter_map(|v| v.as_str()) {
            if !required.contains(key) {
                return Err(quality_error(
                    "HOROSCOPE_EVIDENCE_MISMATCH",
                    json!({ "reason": "slot_uses_unplanned_evidence", "evidence_key": key }),
                ));
            }
        }
    }
    Ok(())
}
pub(crate) fn validate_public_slot_text(slot: &Value) -> Result<(), GenerationError> {
    let mut public_text = String::new();
    for key in ["title", "theme", "tone", "text", "advice", "watch_point"] {
        if let Some(value) = slot.get(key).and_then(|v| v.as_str()) {
            public_text.push_str(value);
            public_text.push('\n');
        }
    }
    for forbidden in [
        "[morning]",
        "[afternoon]",
        "[evening]",
        "[day]",
        "slot:morning",
        "slot:afternoon",
        "slot:evening",
        "slot:day",
        "slot_",
        "avoid_",
    ] {
        if public_text.contains(forbidden) {
            return Err(quality_error(
                "HOROSCOPE_PUBLIC_SLOT_CODE_LEAK",
                json!({ "forbidden": forbidden }),
            ));
        }
    }
    for generic in [
        "les signaux du jour invitent",
        "rester concret et nuance",
        "l'elan du moment",
        "l’énergie du moment",
        "lecture reste volontairement synthétique",
        "preuve astrologique centrale",
        "découpage horaire",
    ] {
        if public_text.to_lowercase().contains(generic) {
            return Err(quality_error(
                "HOROSCOPE_SLOT_TOO_GENERIC",
                json!({ "forbidden": generic }),
            ));
        }
    }
    if public_text.contains("Apres-midi")
        || public_text.contains("Repondez")
        || public_text.contains("Conseil:")
        || !french_elision_violations(&public_text).is_empty()
    {
        return Err(quality_error(
            "HOROSCOPE_FRENCH_TYPOGRAPHY_FAILED",
            json!({ "reason": "known_french_typography_violation" }),
        ));
    }
    Ok(())
}
pub(crate) fn validate_public_text_no_technical_codes(
    public_text: &str,
) -> Result<(), GenerationError> {
    let lower = public_text.to_lowercase();
    for forbidden in [
        "[morning]",
        "[afternoon]",
        "[evening]",
        "[day]",
        "slot:morning",
        "slot:afternoon",
        "slot:evening",
        "slot:day",
        "slot technique",
        "slot_code",
        "slot_",
        "avoid_",
    ] {
        if lower.contains(forbidden) {
            return Err(quality_error(
                "HOROSCOPE_PUBLIC_SLOT_CODE_LEAK",
                json!({ "forbidden": forbidden }),
            ));
        }
    }
    if normalized_text(public_text)
        .split_whitespace()
        .any(|token| token == "day")
    {
        return Err(quality_error(
            "HOROSCOPE_PUBLIC_SLOT_CODE_LEAK",
            json!({ "forbidden": "day" }),
        ));
    }
    Ok(())
}
pub(crate) fn validate_free_text_quality(
    public_text: &str,
    response: &Value,
) -> Result<(), GenerationError> {
    for key in ["advice", "watch_point"] {
        if response
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .is_empty()
        {
            return Err(quality_error(
                "HOROSCOPE_RESPONSE_INVALID",
                json!({ "reason": format!("missing_{key}") }),
            ));
        }
    }
    validate_public_text_no_technical_codes(public_text)?;
    let word_count = public_text.split_whitespace().count();
    if !(40..=190).contains(&word_count) {
        return Err(quality_error(
            "HOROSCOPE_FREE_LENGTH_INVALID",
            json!({ "word_count": word_count }),
        ));
    }
    for generic in [
        "les signaux du jour invitent",
        "rester concret et nuance",
        "l'elan du moment",
        "l’énergie du moment",
        "lecture reste volontairement synthétique",
        "preuve astrologique centrale",
        "découpage horaire",
    ] {
        let lower = public_text.to_lowercase();
        let normalized = normalized_text(public_text);
        if lower.contains(generic) || normalized.contains(generic) {
            return Err(quality_error(
                "HOROSCOPE_SLOT_TOO_GENERIC",
                json!({ "forbidden": generic }),
            ));
        }
    }
    if public_text.contains("Conseil:")
        || public_text.contains("Repondez")
        || !french_elision_violations(public_text).is_empty()
    {
        return Err(quality_error(
            "HOROSCOPE_FRENCH_TYPOGRAPHY_FAILED",
            json!({ "reason": "known_french_typography_violation" }),
        ));
    }
    Ok(())
}
pub(crate) fn validate_astrological_reference(
    slot_code: &str,
    text: &str,
    request_slot: &Value,
) -> Result<(), GenerationError> {
    if request_slot.get("specificity").and_then(|v| v.as_str()) == Some("fallback") {
        return Ok(());
    }
    let lower = text.to_lowercase();
    let has_astro = [
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
    .any(|needle| lower.contains(needle));
    if has_astro {
        Ok(())
    } else {
        Err(quality_error(
            "HOROSCOPE_SLOT_ASTRO_REFERENCE_MISSING",
            json!({ "slot_code": slot_code }),
        ))
    }
}
pub(crate) fn validate_day_overview_not_copied(
    request: &Value,
    response_slots: &[Value],
) -> Result<(), GenerationError> {
    let overview = request
        .get("day_overview")
        .and_then(|v| v.get("summary_hint"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if overview.is_empty() {
        return Ok(());
    }
    for slot in response_slots {
        let text = slot.get("text").and_then(|v| v.as_str()).unwrap_or("");
        if normalized_text(text).contains(&normalized_text(overview)) {
            return Err(quality_error(
                "HOROSCOPE_SLOT_REPETITION_FAILED",
                json!({ "reason": "day_overview_copied_into_slot" }),
            ));
        }
    }
    Ok(())
}
pub(crate) fn validate_slot_diversity(texts: &[String]) -> Result<(), GenerationError> {
    for i in 0..texts.len() {
        for j in (i + 1)..texts.len() {
            let a = meaningful_words(&texts[i]);
            let b = meaningful_words(&texts[j]);
            let shared = a.intersection(&b).count();
            let denom = a.len().min(b.len()).max(1);
            if shared as f64 / denom as f64 > 0.60 {
                return Err(quality_error(
                    "HOROSCOPE_SLOT_REPETITION_FAILED",
                    json!({ "reason": "slot_word_overlap_too_high" }),
                ));
            }
            if first_words(&texts[i], 3) == first_words(&texts[j], 3) {
                return Err(quality_error(
                    "HOROSCOPE_SLOT_REPETITION_FAILED",
                    json!({ "reason": "same_opening_trigram" }),
                ));
            }
        }
    }
    Ok(())
}
pub(crate) fn validate_distinct_strings(
    items: &[String],
    code: &str,
) -> Result<(), GenerationError> {
    let normalized = items
        .iter()
        .map(|item| normalized_text(item))
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    let unique = normalized.iter().collect::<HashSet<_>>();
    if unique.len() != normalized.len() {
        return Err(quality_error(code, json!({ "reason": "duplicate_text" })));
    }
    Ok(())
}
pub(crate) fn validate_distinct_best_for(items: &[Vec<String>]) -> Result<(), GenerationError> {
    let normalized = items
        .iter()
        .map(|set| {
            let mut values = set
                .iter()
                .map(|value| normalized_text(value))
                .collect::<Vec<_>>();
            values.sort();
            values.join("|")
        })
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let unique = normalized.iter().collect::<HashSet<_>>();
    if unique.len() != normalized.len() {
        return Err(quality_error(
            "HOROSCOPE_SLOT_THEME_DUPLICATED",
            json!({ "reason": "best_for_duplicated" }),
        ));
    }
    Ok(())
}
pub(crate) fn meaningful_words(text: &str) -> HashSet<String> {
    let stopwords = [
        "le", "la", "les", "un", "une", "des", "de", "du", "et", "ou", "a", "à", "ce", "c", "est",
        "sur", "pour", "plus", "dans", "avec", "sans", "du", "au", "aux", "en",
    ]
    .into_iter()
    .collect::<HashSet<_>>();
    normalized_text(text)
        .split_whitespace()
        .filter(|word| word.len() > 2 && !stopwords.contains(*word))
        .map(str::to_string)
        .collect()
}
pub(crate) fn first_words(text: &str, count: usize) -> Vec<String> {
    normalized_text(text)
        .split_whitespace()
        .take(count)
        .map(str::to_string)
        .collect()
}
pub(crate) fn normalized_text(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || ch.is_whitespace() {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn collect_evidence_keys(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            if let Some(items) = map.get("evidence_keys").and_then(|v| v.as_array()) {
                out.extend(items.iter().filter_map(|v| v.as_str().map(str::to_string)));
            }
            for child in map.values() {
                collect_evidence_keys(child, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_evidence_keys(item, out);
            }
        }
        _ => {}
    }
}

pub(crate) fn free_public_text(response: &Value) -> String {
    let mut out = String::new();
    if let Some(summary) = response.get("summary") {
        for key in ["title", "text"] {
            if let Some(value) = summary.get(key).and_then(|v| v.as_str()) {
                out.push_str(value);
                out.push('\n');
            }
        }
    }
    for key in ["advice", "watch_point"] {
        if let Some(value) = response.get(key).and_then(|v| v.as_str()) {
            out.push_str(value);
            out.push('\n');
        }
    }
    out
}
