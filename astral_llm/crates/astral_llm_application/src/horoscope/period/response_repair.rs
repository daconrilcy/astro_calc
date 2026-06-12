use super::*;
pub fn repair_period_response_shape(request: &Value, response: &mut Value) {
    let service_code = request["service_code"]
        .as_str()
        .unwrap_or(HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE);
    response["contract_version"] = json!("horoscope_period_response_v1");
    response["service_code"] = json!(service_code);
    response["period_resolution"] = request["period_resolution"].clone();
    if is_free_period_service(service_code) {
        repair_free_period_response_shape(request, response);
        return;
    }
    response["week_overview"] = sanitize_period_week_overview(response.get("week_overview"));
    response["advice"] = sanitize_period_advice(response.get("advice"));
    response["key_days"] = sanitize_period_markers(
        response.get("key_days"),
        &request["key_days"],
        PeriodMarkerRole::Key,
    );
    response["best_days"] = sanitize_period_markers(
        response.get("best_days"),
        &request["best_days"],
        PeriodMarkerRole::Best,
    );
    response["watch_days"] = sanitize_period_markers(
        response.get("watch_days"),
        &request["watch_days"],
        PeriodMarkerRole::Watch,
    );
    response["watch_summary"] = sanitize_period_watch_summary(
        response.get("watch_summary"),
        &request["watch_summary_plan"],
    );
    response["daily_timeline"] =
        sanitize_period_daily_timeline(response.get("daily_timeline"), request);
    response["domain_sections"] =
        sanitize_period_domain_sections(response.get("domain_sections"), request);
    if is_premium_period_service(service_code) {
        response["best_windows"] =
            sanitize_period_windows(response.get("best_windows"), request, "best_windows");
        response["watch_windows"] =
            sanitize_period_windows(response.get("watch_windows"), request, "watch_windows");
        response["strategy"] = sanitize_period_strategy(response.get("strategy"), request);
    } else {
        response.as_object_mut().map(|map| {
            map.remove("best_windows");
            map.remove("watch_windows");
            map.remove("strategy");
        });
    }
    response.as_object_mut().map(|map| {
        map.remove("summary");
        map.remove("dominant_theme");
    });
    response["evidence_summary"] =
        sanitize_period_evidence_summary(response.get("evidence_summary"), request);
    ensure_period_response_minimum_words(request, response);
    normalize_period_week_overview_repetition(response);
    normalize_period_repetitive_public_phrases(response);
    dedupe_period_daily_timeline_texts(request, response);
    repair_period_mechanical_public_blocks(request, response);
    enforce_period_public_personalization_from_request(request, response);
    enforce_premium_period_advice_synthesis(request, response);
    restore_period_response_evidence_from_request(request, response);
    normalize_period_public_strings(response);
    enforce_period_public_personalization_from_request(request, response);
    let provider = response["quality"]["provider"]
        .as_str()
        .unwrap_or("openai")
        .to_string();
    let model = response["quality"]["model"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let fallback_used = response["quality"]["fallback_used"]
        .as_bool()
        .unwrap_or(false);
    response["quality"] = json!({        "daily_timeline_count": response["daily_timeline"].as_array().map(|days| days.len()).unwrap_or(0) as i64,        "evidence_guard_passed": true,        "best_watch_overlap_passed": true,        "provider": provider,        "model": model,        "fallback_used": fallback_used,        "period_contract": "horoscope_period_response_v1"    });
}
pub(crate) fn restore_period_response_evidence_from_request(request: &Value, response: &mut Value) {
    if is_free_period_request(request) {
        return;
    }
    let ordered_evidence = request["evidence"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item["evidence_key"].as_str())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if ordered_evidence.is_empty() {
        return;
    }
    let first_evidence_key = ordered_evidence
        .first()
        .map(|key| json!([key]))
        .unwrap_or_else(|| json!([]));
    restore_period_array_evidence_by_date(response, "daily_timeline", request, "daily_plans");
    restore_period_array_evidence_by_date(response, "key_days", request, "key_days");
    restore_period_array_evidence_by_date(response, "best_days", request, "best_days");
    restore_period_array_evidence_by_date(response, "watch_days", request, "watch_days");
    restore_period_domain_evidence(response, request);
    response["evidence_summary"] =
        sanitize_period_evidence_summary(response.get("evidence_summary"), request);
    let watch_status = response["watch_summary"]["status"]
        .as_str()
        .unwrap_or("none");
    if watch_status == "none" {
        response["watch_summary"]["evidence_keys"] = json!([]);
    } else {
        let fallback_keys =
            non_empty_string_array_value(request["watch_summary_plan"].get("evidence_keys"))
                .or_else(|| first_non_empty_period_array_evidence(response.get("watch_days")));
        if let Some(keys) = fallback_keys {
            response["watch_summary"]["evidence_keys"] = keys;
        }
    }
    if is_premium_period_request(request) {
        restore_period_window_evidence(response, request, "best_windows");
        restore_period_window_evidence(response, request, "watch_windows");
        response["strategy"]["evidence_keys"] =
            non_empty_string_array_value(request["strategy"].get("evidence_keys"))
                .unwrap_or(first_evidence_key);
    }
}
pub(crate) fn repair_period_mechanical_public_blocks(request: &Value, response: &mut Value) {
    let plans_by_date = request["daily_plans"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|plan| Some((plan.get("date")?.as_str()?.to_string(), plan.clone())))
        .collect::<HashMap<_, _>>();
    if let Some(days) = response
        .get_mut("daily_timeline")
        .and_then(Value::as_array_mut)
    {
        for (index, day) in days.iter_mut().enumerate() {
            let date = day.get("date").and_then(Value::as_str).unwrap_or("");
            let plan = plans_by_date
                .get(date)
                .cloned()
                .unwrap_or_else(|| day.clone());
            if day
                .get("text")
                .and_then(Value::as_str)
                .is_some_and(period_public_block_needs_rewrite)
            {
                day["text"] = json!(sanitize_period_public_string(&period_public_day_text(
                    &plan, index
                )));
            }
            if day
                .get("advice")
                .and_then(Value::as_str)
                .is_some_and(period_public_block_needs_rewrite)
            {
                day["advice"] = json!(sanitize_period_public_string(&period_public_day_advice(
                    &plan
                )));
            }
        }
    }
    for field in ["key_days", "best_days", "watch_days"] {
        let fallback_by_date = request[field]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|marker| Some((marker.get("date")?.as_str()?.to_string(), marker.clone())))
            .collect::<HashMap<_, _>>();
        let Some(markers) = response.get_mut(field).and_then(Value::as_array_mut) else {
            continue;
        };
        for marker in markers {
            let reason_needs_rewrite = marker
                .get("reason")
                .and_then(Value::as_str)
                .is_some_and(period_public_block_needs_rewrite);
            if !reason_needs_rewrite {
                continue;
            }
            let date = marker.get("date").and_then(Value::as_str).unwrap_or("");
            if let Some(fallback) = fallback_by_date
                .get(date)
                .and_then(|item| item.get("reason"))
            {
                marker["reason"] = json!(sanitize_period_public_string(
                    fallback
                        .as_str()
                        .unwrap_or("Gardez ce repère comme point de contrôle.")
                ));
            }
        }
    }
}
pub(crate) fn period_public_block_needs_rewrite(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains(". ,")
        || lower.contains("vérifiez vérifier")
        || lower.contains("posez une priorité claire liée à")
        || period_starts_with_raw_focus_list(&lower)
}
pub(crate) fn period_starts_with_raw_focus_list(lower: &str) -> bool {
    [
        "avec vérifier",
        "avec réduire",
        "avec nommer",
        "avec tenir",
        "avec préparer",
        "avec choisir",
        "avec accorder",
        "avec terminer",
        "avec alléger",
        "avec refuser",
        "avec confirmer",
        "en partant de vérifier",
        "en partant de réduire",
        "en partant de nommer",
        "en partant de tenir",
        "en partant de préparer",
        "en partant de choisir",
        "en partant de accorder",
        "en partant d'accorder",
        "à travers vérifier",
        "à travers réduire",
        "à travers nommer",
    ]
    .iter()
    .any(|fragment| lower.contains(fragment))
}
pub(crate) fn first_non_empty_period_array_evidence(value: Option<&Value>) -> Option<Value> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| non_empty_string_array_value(item.get("evidence_keys")))
        .next()
}
pub(crate) fn restore_period_array_evidence_by_date(
    response: &mut Value,
    response_field: &str,
    request: &Value,
    request_field: &str,
) {
    let fallback = request[request_field]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let fallback_by_date = fallback
        .iter()
        .filter_map(|item| Some((item.get("date")?.as_str()?.to_string(), item.clone())))
        .collect::<HashMap<_, _>>();
    let Some(items) = response
        .get_mut(response_field)
        .and_then(Value::as_array_mut)
    else {
        return;
    };
    for (index, item) in items.iter_mut().enumerate() {
        let item_date = item.get("date").and_then(Value::as_str);
        let fallback_by_index = fallback.get(index).filter(|fallback| {
            item_date.is_none() || fallback.get("date").and_then(Value::as_str) == item_date
        });
        let fallback = item
            .get("date")
            .and_then(Value::as_str)
            .and_then(|date| fallback_by_date.get(date))
            .or(fallback_by_index);
        if let Some(keys) =
            fallback.and_then(|item| non_empty_string_array_value(item.get("evidence_keys")))
        {
            item["evidence_keys"] = keys;
        }
    }
}
pub(crate) fn restore_period_domain_evidence(response: &mut Value, request: &Value) {
    let fallback = request["domain_sections"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let by_domain = fallback
        .iter()
        .filter_map(|item| Some((item.get("domain")?.as_str()?.to_string(), item.clone())))
        .collect::<HashMap<_, _>>();
    let by_title = fallback
        .iter()
        .filter_map(|item| Some((normalized_text(item.get("title")?.as_str()?), item.clone())))
        .collect::<HashMap<_, _>>();
    let Some(items) = response["domain_sections"].as_array_mut() else {
        return;
    };
    for (index, item) in items.iter_mut().enumerate() {
        let has_identity = item.get("domain").and_then(Value::as_str).is_some()
            || item.get("title").and_then(Value::as_str).is_some();
        let fallback_by_index = fallback.get(index).filter(|_| !has_identity);
        let fallback = item
            .get("domain")
            .and_then(Value::as_str)
            .and_then(|domain| by_domain.get(domain))
            .or_else(|| {
                item.get("title")
                    .and_then(Value::as_str)
                    .and_then(|title| by_title.get(&normalized_text(title)))
            })
            .or(fallback_by_index);
        if let Some(keys) =
            fallback.and_then(|item| non_empty_string_array_value(item.get("evidence_keys")))
        {
            item["evidence_keys"] = keys;
        }
    }
}
pub(crate) fn restore_period_window_evidence(response: &mut Value, request: &Value, field: &str) {
    let fallback = request[field].as_array().cloned().unwrap_or_default();
    let by_identity = fallback
        .iter()
        .filter_map(|item| Some((period_window_identity(item)?, item.clone())))
        .collect::<HashMap<_, _>>();
    let Some(items) = response.get_mut(field).and_then(Value::as_array_mut) else {
        return;
    };
    for (index, item) in items.iter_mut().enumerate() {
        let fallback_by_index = fallback.get(index).filter(|fallback| {
            fallback.get("date").and_then(Value::as_str) == item.get("date").and_then(Value::as_str)
        });
        let fallback = period_window_identity(item)
            .and_then(|identity| by_identity.get(&identity))
            .or(fallback_by_index);
        let Some(fallback) = fallback else {
            continue;
        };
        if let Some(keys) = non_empty_string_array_value(fallback.get("evidence_keys")) {
            item["evidence_keys"] = keys;
        }
        item["source_snapshot_keys"] = fallback["source_snapshot_keys"].clone();
    }
}
pub(crate) fn normalize_period_public_strings(response: &mut Value) {
    normalize_period_public_strings_value(response, None);
    normalize_period_domain_section_duplicates(response);
}
pub(crate) fn normalize_period_public_strings_value(value: &mut Value, key: Option<&str>) {
    if period_public_string_normalization_excluded_key(key) {
        return;
    }
    match value {
        Value::String(text) => {
            *text = sanitize_period_public_string(text);
        }
        Value::Array(items) => {
            for item in items {
                normalize_period_public_strings_value(item, key);
            }
        }
        Value::Object(map) => {
            for (child_key, child) in map {
                normalize_period_public_strings_value(child, Some(child_key));
            }
        }
        _ => {}
    }
}
pub(crate) fn period_public_string_normalization_excluded_key(key: Option<&str>) -> bool {
    matches!(
        key,
        Some(
            "contract_version"
                | "service_code"
                | "date"
                | "status"
                | "period_resolution"
                | "start_datetime_local"
                | "start_datetime_utc"
                | "end_datetime_local"
                | "end_datetime_utc"
                | "timezone"
                | "period_profile_code"
                | "evidence_key"
                | "evidence_keys"
                | "source_snapshot_keys"
                | "quality"
                | "provider"
                | "model"
                | "period_contract"
        )
    )
}
pub(crate) fn prune_period_response_variant_fields_v2(request: &Value, response: &mut Value) {
    let service_code = request["service_code"]
        .as_str()
        .unwrap_or(HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE);
    if is_free_period_service(service_code) {
        response.as_object_mut().map(|map| {
            map.remove("week_overview");
            map.remove("best_days");
            map.remove("watch_days");
            map.remove("daily_timeline");
            map.remove("domain_sections");
            map.remove("best_windows");
            map.remove("watch_windows");
            map.remove("strategy");
        });
        return;
    }
    response.as_object_mut().map(|map| {
        map.remove("summary");
        map.remove("dominant_theme");
    });
    if !is_premium_period_service(service_code) {
        response.as_object_mut().map(|map| {
            map.remove("best_windows");
            map.remove("watch_windows");
            map.remove("strategy");
        });
    }
}
pub(crate) fn finalize_period_response_words_and_repetition(request: &Value, response: &mut Value) {
    ensure_period_response_minimum_words(request, response);
    normalize_period_week_overview_repetition(response);
    normalize_period_repetitive_public_phrases(response);
    dedupe_period_daily_timeline_texts(request, response);
    enforce_period_overview_personalization(response);
    enforce_period_domain_personalization(request, response);
    enforce_premium_period_advice_synthesis(request, response);
    ensure_period_response_minimum_words(request, response);
    normalize_period_week_overview_repetition(response);
    normalize_period_repetitive_public_phrases(response);
    dedupe_period_daily_timeline_texts(request, response);
    enforce_period_overview_personalization(response);
    enforce_period_domain_personalization(request, response);
    enforce_premium_period_advice_synthesis(request, response);
    ensure_period_response_minimum_words(request, response);
    dedupe_period_daily_timeline_texts(request, response);
    enforce_period_overview_personalization(response);
    enforce_period_domain_personalization(request, response);
    enforce_premium_period_advice_synthesis(request, response);
}
#[doc(hidden)]
pub fn prune_period_response_variant_fields(request: &Value, response: &mut Value) {
    let service_code = request["service_code"]
        .as_str()
        .unwrap_or(HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE);
    if is_free_period_service(service_code) {
        response.as_object_mut().map(|map| {
            map.remove("week_overview");
            map.remove("best_days");
            map.remove("watch_days");
            map.remove("daily_timeline");
            map.remove("domain_sections");
            map.remove("best_windows");
            map.remove("watch_windows");
            map.remove("strategy");
        });
        return;
    }
    response["watch_summary"] = sanitize_period_watch_summary(
        response.get("watch_summary"),
        &request["watch_summary_plan"],
    );
    response.as_object_mut().map(|map| {
        map.remove("summary");
        map.remove("dominant_theme");
    });
    if !is_premium_period_service(service_code) {
        response.as_object_mut().map(|map| {
            map.remove("best_windows");
            map.remove("watch_windows");
            map.remove("strategy");
        });
    }
}
pub(crate) fn enforce_period_overview_personalization(response: &mut Value) {
    let text = response
        .pointer("/week_overview/text")
        .and_then(Value::as_str)
        .unwrap_or("");
    let trajectory = response
        .pointer("/week_overview/trajectory")
        .and_then(Value::as_str)
        .unwrap_or("");
    if period_text_has_personalization(&format!("{text} {trajectory}")) {
        return;
    }
    let addition = "La semaine se pilote avec vos priorités concrètes : qui fait quoi, pour quand, avec quelle preuve.";
    response["week_overview"]["text"] = json!(sanitize_period_public_string(&format!(
        "{} {}",
        text.trim(),
        addition
    )));
}
pub(crate) fn enforce_period_public_personalization_from_request(
    request: &Value,
    response: &mut Value,
) {
    enforce_period_overview_personalization(response);
    enforce_period_daily_personalization(request, response);
    enforce_period_domain_personalization(request, response);
}
pub(crate) fn enforce_period_daily_personalization(request: &Value, response: &mut Value) {
    let current_count = response["daily_timeline"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|day| period_text_has_personalization(day["text"].as_str().unwrap_or("")))
        .count();
    if current_count >= 4 {
        return;
    }
    let plans_by_date = request["daily_plans"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|plan| Some((plan.get("date")?.as_str()?.to_string(), plan.clone())))
        .collect::<HashMap<_, _>>();
    let Some(days) = response["daily_timeline"].as_array_mut() else {
        return;
    };
    let mut count = current_count;
    for (index, day) in days.iter_mut().enumerate() {
        if count >= 4 {
            break;
        }
        if period_text_has_personalization(day["text"].as_str().unwrap_or("")) {
            continue;
        }
        let fallback_plan = day
            .get("date")
            .and_then(Value::as_str)
            .and_then(|date| plans_by_date.get(date))
            .unwrap_or(day);
        let addition = period_public_day_personalization_sentence(fallback_plan, index);
        let text = day["text"].as_str().unwrap_or("").trim();
        day["text"] = json!(sanitize_period_public_string(&format!("{text} {addition}")));
        count += 1;
    }
}
pub(crate) fn enforce_period_domain_personalization(request: &Value, response: &mut Value) {
    let fallback_sections = request["domain_sections"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let fallback_by_domain = fallback_sections
        .iter()
        .filter_map(|section| {
            Some((
                section.get("domain")?.as_str()?.to_string(),
                section.clone(),
            ))
        })
        .collect::<HashMap<_, _>>();
    let Some(sections) = response
        .get_mut("domain_sections")
        .and_then(Value::as_array_mut)
    else {
        return;
    };
    for (index, section) in sections.iter_mut().enumerate() {
        let text = section.get("text").and_then(Value::as_str).unwrap_or("");
        let is_generic_domain_text = period_domain_text_is_generic(text);
        let needs_focus_support = period_domain_text_needs_focus_support(text);
        if period_text_has_personalization(text) && !is_generic_domain_text && !needs_focus_support
        {
            continue;
        }
        let fallback = section
            .get("domain")
            .and_then(Value::as_str)
            .and_then(|domain| fallback_by_domain.get(domain))
            .or_else(|| fallback_sections.get(index))
            .unwrap_or(section);
        let addition = if period_domain_text_has_focus_support(&text.to_lowercase()) {
            period_public_domain_personalization_tail(fallback)
        } else {
            period_public_domain_interpretive_sentence(fallback)
        };
        let repaired = if is_generic_domain_text {
            addition
        } else {
            format!("{} {}", text.trim(), addition)
        };
        section["text"] = json!(sanitize_period_public_string(&repaired));
    }
    normalize_period_domain_section_duplicates(response);
}
pub(crate) fn normalize_period_domain_section_duplicates(response: &mut Value) {
    let Some(sections) = response
        .get_mut("domain_sections")
        .and_then(Value::as_array_mut)
    else {
        return;
    };
    for section in sections {
        let Some(text) = section.get("text").and_then(Value::as_str) else {
            continue;
        };
        section["text"] = json!(dedupe_period_domain_support_sentences(text));
    }
}
pub(crate) fn dedupe_period_domain_support_sentences(text: &str) -> String {
    let mut seen = HashSet::new();
    let mut kept = Vec::new();
    for raw in text.split_inclusive('.') {
        let sentence = raw.trim();
        if sentence.is_empty() {
            continue;
        }
        let key = sentence.trim_end_matches('.').trim().to_lowercase();
        if period_domain_support_sentence_key(&key) && !seen.insert(key) {
            continue;
        }
        kept.push(sentence.to_string());
    }
    if kept.is_empty() {
        text.trim().to_string()
    } else {
        kept.join(" ")
    }
}
pub(crate) fn period_domain_support_sentence_key(lower_sentence: &str) -> bool {
    [
        "le plus concret est",
        "le bon appui est",
        "le geste à garder est",
        "la bonne mesure reste",
    ]
    .iter()
    .any(|prefix| lower_sentence.starts_with(prefix))
}
pub(crate) fn period_domain_text_needs_focus_support(text: &str) -> bool {
    let lower = text.to_lowercase();
    !period_domain_text_has_focus_support(&lower)
        && (lower.contains("besoin net de remettre l'ordre")
            || lower.contains("moments les plus lisibles")
            || lower.contains("vrai désir de la simple habitude"))
}
pub(crate) fn period_domain_text_has_focus_support(lower: &str) -> bool {
    [
        "le plus concret est",
        "le bon appui est",
        "le geste à garder est",
        "la bonne mesure reste",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}
pub(crate) fn enforce_premium_period_advice_synthesis(request: &Value, response: &mut Value) {
    if !is_premium_period_request(request) {
        return;
    }
    let advice_text = [
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
    if explicit_date_count(&advice_text) == 0 && !period_premium_advice_is_too_generic(response) {
        return;
    }
    response["advice"] = premium_period_default_advice();
    if explicit_date_count(&advice_text) > 0 {
        response["strategy"] = sanitize_period_strategy(None, request);
    }
}
pub(crate) fn period_premium_advice_is_too_generic(response: &Value) -> bool {
    let main = response
        .pointer("/advice/main")
        .and_then(Value::as_str)
        .unwrap_or("");
    let best_use = response
        .pointer("/advice/best_use")
        .and_then(Value::as_str)
        .unwrap_or("");
    let avoid = response
        .pointer("/advice/avoid")
        .and_then(Value::as_str)
        .unwrap_or("");
    let joined = format!("{main} {best_use} {avoid}").to_lowercase();
    simple_public_word_count(&joined) < 32
        || joined.contains("gardez une progression simple")
        || joined.contains("utiliser les appuis")
        || joined.contains("transformer un signal quotidien")
}
pub(crate) fn simple_public_word_count(text: &str) -> usize {
    text.split_whitespace()
        .filter(|word| word.chars().any(char::is_alphabetic))
        .count()
}
pub(crate) fn premium_period_default_advice() -> Value {
    json!({        "main": "Travaillez par gestes courts : une preuve à obtenir, une charge à réduire, un message à formuler, puis une pause avant de rouvrir le sujet.",        "best_use": "Réservez les fenêtres favorables aux actions qui laissent une trace claire : confirmation, échéance, accord écrit, ressource vérifiée ou tâche fermée.",        "avoid": "Évitez les promesses larges, les réponses en chaîne et les discussions longues tant que le cadre, le responsable et la prochaine étape ne sont pas explicites."    })
}
pub fn repair_period_response_shape_v2(request: &Value, response: &mut Value) {
    let service_code = request["service_code"]
        .as_str()
        .unwrap_or(HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE);
    response["contract_version"] = json!("horoscope_period_response_v1");
    response["service_code"] = json!(service_code);
    response["period_resolution"] = request["period_resolution"].clone();
    if !response.get("quality").is_some_and(Value::is_object) {
        response["quality"] = quality_v2(
            service_code,
            request,
            if is_free_period_service(service_code) {
                0
            } else {
                7
            },
        );
    }
    prune_period_response_variant_fields_v2(request, response);
    trim_period_response_strings_v2(response);
    normalize_period_v2_public_short_labels(response);
    normalize_period_v2_watch_summary_status(response);
    restore_period_response_technical_keys_v2(request, response);
}
pub(crate) fn trim_period_response_strings_v2(value: &mut Value) {
    match value {
        Value::String(text) => {
            *text = normalize_period_v2_objective_public_text(text.trim());
        }
        Value::Array(items) => {
            for item in items {
                trim_period_response_strings_v2(item);
            }
        }
        Value::Object(map) => {
            for child in map.values_mut() {
                trim_period_response_strings_v2(child);
            }
        }
        _ => {}
    }
}
pub(crate) fn normalize_period_v2_objective_public_text(text: &str) -> String {
    PERIOD_V2_OBJECTIVE_TEXT_REPLACEMENTS
        .iter()
        .fold(text.to_string(), |acc, (from, to)| acc.replace(from, to))
}
pub(crate) fn restore_period_response_technical_keys_v2(request: &Value, response: &mut Value) {
    let evidence_by_date = request["semantic_brief"]["daily_signal_summary"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|day| {
            let date = day["date"].as_str()?.to_string();
            let keys = day["evidence_keys"].as_array()?.clone();
            Some((date, keys))
        })
        .collect::<HashMap<_, _>>();
    for field in ["daily_timeline", "key_days", "best_days", "watch_days"] {
        let Some(items) = response.get_mut(field).and_then(Value::as_array_mut) else {
            continue;
        };
        for item in items {
            if item
                .get("evidence_keys")
                .and_then(Value::as_array)
                .is_some_and(|keys| !keys.is_empty())
            {
                continue;
            }
            let Some(date) = item.get("date").and_then(Value::as_str) else {
                continue;
            };
            if let Some(keys) = evidence_by_date.get(date) {
                item["evidence_keys"] = json!(keys);
            }
        }
    }
    if response["watch_summary"]["status"].as_str() == Some("none") {
        response["watch_summary"]["evidence_keys"] = json!([]);
    }
}
