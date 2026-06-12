use super::*;

pub(crate) fn ensure_period_response_minimum_words(request: &Value, response: &mut Value) {
    let limits = period_word_limits_for_request(request);
    trim_period_response_to_hard_limit(request, response, &limits);
    let current_words = period_public_word_count(response);
    if current_words >= limits.target_min && current_words <= limits.hard_limit {
        return;
    }
    if current_words > limits.hard_limit {
        trim_period_response_aggressively(request, response);
        let compact_words = period_public_word_count(response);
        if compact_words >= limits.target_min && compact_words <= limits.hard_limit {
            return;
        }
        if compact_words > limits.hard_limit {
            return;
        }
    }
    if let Some(text) = response.pointer_mut("/week_overview/text") {
        append_period_value_sentence(            text,            "La semaine gagne en cohérence quand chaque décision précise qui fait quoi, pour quand, et avec quelle preuve.",        );
    }
    if period_public_word_count(response) >= limits.target_min {
        return;
    }
    let plans = request["daily_plans"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let day_count = response["daily_timeline"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0);
    for index in 0..day_count {
        {
            let day = &mut response["daily_timeline"][index];
            let date = day.get("date").and_then(Value::as_str).unwrap_or("");
            let plan = plans
                .iter()
                .find(|plan| plan.get("date").and_then(Value::as_str) == Some(date));
            if let Some(plan) = plan {
                if let Some(text) = day.get_mut("text") {
                    append_period_value_sentence(
                        text,
                        &period_public_personalization_sentence(plan),
                    );
                }
                if let Some(advice) = day.get_mut("advice") {
                    append_period_value_sentence(advice, period_daily_advice_expansion(index));
                }
            }
        }
        if period_public_word_count(response) >= limits.target_min {
            return;
        }
    }
    if period_public_word_count(response) >= limits.target_min {
        return;
    }
    let sections = request["domain_sections"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let section_count = response["domain_sections"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0);
    for index in 0..section_count {
        {
            let section = &mut response["domain_sections"][index];
            let domain = section.get("domain").and_then(Value::as_str).unwrap_or("");
            let plan = sections
                .iter()
                .find(|plan| plan.get("domain").and_then(Value::as_str) == Some(domain));
            if let Some(plan) = plan {
                if let Some(text) = section.get_mut("text") {
                    if !text.as_str().is_some_and(period_text_has_personalization) {
                        append_period_value_sentence(
                            text,
                            &period_public_domain_personalization_sentence(plan),
                        );
                    }
                }
            }
        }
        if period_public_word_count(response) >= limits.target_min {
            return;
        }
    }
    if period_public_word_count(response) >= limits.target_min {
        return;
    }
    if let Some(main) = response.pointer_mut("/advice/main") {
        append_period_value_sentence(            main,            "Utilisez ces repères comme une synthèse personnelle de période, pas comme une liste de journées isolées.",        );
    }
    fill_period_response_to_minimum(request, response, &limits);
    if period_public_word_count(response) > limits.hard_limit {
        trim_period_response_to_hard_limit(request, response, &limits);
    }
    if period_public_word_count(response) > limits.hard_limit {
        trim_period_response_aggressively(request, response);
    }
}
pub(crate) fn trim_period_response_to_hard_limit(
    request: &Value,
    response: &mut Value,
    limits: &PeriodWordLimits,
) {
    if period_public_word_count(response) <= limits.hard_limit {
        return;
    }
    response["week_overview"] = json!({        "title": "Vos 7 prochains jours",        "text": "Vos 7 prochains jours avancent par étapes : remettre de l'ordre, retrouver un appui plus simple, puis consolider ce qui devient clair dans vos priorités.",        "trajectory": "Le mouvement va des appuis initiaux vers une consolidation plus consciente."    });
    response["advice"] = json!({        "main": "Avancez par étapes et gardez une priorité concrète par journée.",        "best_use": "Utiliser les jours favorables pour poser un geste clair et personnel.",        "avoid": "Transformer un signal de période en certitude rigide."    });
    let plans = request["daily_plans"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if let Some(days) = response["daily_timeline"].as_array_mut() {
        for day in days {
            let date = day.get("date").and_then(Value::as_str).unwrap_or("");
            let plan = plans
                .iter()
                .find(|plan| plan.get("date").and_then(Value::as_str) == Some(date));
            if let Some(plan) = plan {
                day["text"] = json!(sanitize_period_public_string(&compact_period_words(
                    &period_public_day_text(plan, 0),
                    42,
                )));
                day["advice"] = json!(sanitize_period_public_string(&compact_period_words(
                    &period_public_day_advice(plan),
                    24,
                )));
            }
        }
    }
    let sections = request["domain_sections"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if let Some(domain_sections) = response["domain_sections"].as_array_mut() {
        if domain_sections.len() > 3 {
            domain_sections.truncate(3);
        }
        for section in domain_sections {
            let domain = section.get("domain").and_then(Value::as_str).unwrap_or("");
            let plan = sections
                .iter()
                .find(|plan| plan.get("domain").and_then(Value::as_str) == Some(domain));
            if let Some(plan) = plan {
                section["text"] = json!(sanitize_period_public_string(&compact_period_words(
                    &period_public_domain_text(plan),
                    46,
                )));
            }
        }
    }
    if response["evidence_summary"]
        .as_array()
        .map(|items| items.len() > 4)
        .unwrap_or(false)
    {
        if let Some(items) = response["evidence_summary"].as_array_mut() {
            items.truncate(4);
        }
    }
}
pub(crate) fn trim_period_response_aggressively(request: &Value, response: &mut Value) {
    response["week_overview"] = json!({        "title": "Vos 7 prochains jours",        "text": "La semaine avance en reliant les échanges, les choix concrets et votre agenda réel.",        "trajectory": "La période progresse vers des choix plus posés et personnels."    });
    response["advice"] = json!({        "main": "Avancez par étapes, avec une priorité concrète à la fois.",        "best_use": "Choisir un geste utile sur les jours favorables.",        "avoid": "Forcer une conclusion trop rapide."    });
    let plans = request["daily_plans"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if let Some(days) = response["daily_timeline"].as_array_mut() {
        for day in days {
            let date = day.get("date").and_then(Value::as_str).unwrap_or("");
            if let Some(plan) = plans
                .iter()
                .find(|plan| plan.get("date").and_then(Value::as_str) == Some(date))
            {
                day["text"] = json!(sanitize_period_public_string(&compact_period_words(
                    &period_public_day_text(plan, 0),
                    30,
                )));
                day["advice"] = json!(sanitize_period_public_string(&compact_period_words(
                    &period_public_day_advice(plan),
                    14,
                )));
            }
        }
    }
    let sections = request["domain_sections"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if let Some(domain_sections) = response["domain_sections"].as_array_mut() {
        if domain_sections.len() > 2 {
            domain_sections.truncate(2);
        }
        for section in domain_sections {
            let domain = section.get("domain").and_then(Value::as_str).unwrap_or("");
            if let Some(plan) = sections
                .iter()
                .find(|plan| plan.get("domain").and_then(Value::as_str) == Some(domain))
            {
                section["text"] = json!(sanitize_period_public_string(&compact_period_words(
                    &period_public_domain_text(plan),
                    34,
                )));
            }
        }
    }
    for field in ["key_days", "best_days", "watch_days"] {
        if let Some(markers) = response[field].as_array_mut() {
            for marker in markers {
                if let Some(reason) = marker.get("reason").and_then(Value::as_str) {
                    marker["reason"] = json!(sanitize_period_public_string(&compact_period_words(
                        reason, 14,
                    )));
                }
            }
        }
    }
    if let Some(items) = response["evidence_summary"].as_array_mut() {
        if items.len() > 2 {
            items.truncate(2);
        }
        for item in items {
            if let Some(label) = item.get("label").and_then(Value::as_str) {
                item["label"] = json!(sanitize_period_public_string(&compact_period_words(
                    label, 18,
                )));
            }
        }
    }
}
pub(crate) fn fill_period_response_to_minimum(
    request: &Value,
    response: &mut Value,
    limits: &PeriodWordLimits,
) {
    if period_public_word_count(response) >= limits.target_min
        || period_public_word_count(response) > limits.hard_limit
    {
        return;
    }
    let plans = request["daily_plans"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let day_count = response["daily_timeline"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0);
    for index in 0..day_count {
        {
            let day = &mut response["daily_timeline"][index];
            let date = day.get("date").and_then(Value::as_str).unwrap_or("");
            if let Some(plan) = plans
                .iter()
                .find(|plan| plan.get("date").and_then(Value::as_str) == Some(date))
            {
                let theme = plan
                    .get("theme_label")
                    .and_then(Value::as_str)
                    .unwrap_or("ce thème");
                if let Some(text) = day.get_mut("text") {
                    append_period_value_sentence(                        text,                        &format!(                            "Pour {theme}, cette indication précise la façon de choisir un rythme personnel sans isoler la journée du reste de la période."                        ),                    );
                    append_period_value_sentence(
                        text,
                        &period_public_personalization_sentence(plan),
                    );
                }
            }
        }
        if period_public_word_count(response) >= limits.target_min {
            return;
        }
    }
}
pub(crate) fn normalize_period_week_overview_repetition(response: &mut Value) {
    let phrase = "thème natal comme fil directeur";
    let week_text = format!(
        "{} {}",
        response["week_overview"]["text"].as_str().unwrap_or(""),
        response["week_overview"]["trajectory"]
            .as_str()
            .unwrap_or("")
    );
    if count_normalized_phrase(&week_text, phrase) <= 1 {
        return;
    }
    for pointer in ["/week_overview/trajectory", "/week_overview/text"] {
        if count_normalized_phrase(
            &format!(
                "{} {}",
                response["week_overview"]["text"].as_str().unwrap_or(""),
                response["week_overview"]["trajectory"]
                    .as_str()
                    .unwrap_or("")
            ),
            phrase,
        ) <= 1
        {
            return;
        }
        if let Some(value) = response
            .pointer(pointer)
            .and_then(Value::as_str)
            .map(str::to_string)
        {
            let normalized = if pointer == "/week_overview/trajectory" {
                replace_period_phrase_all(&value, phrase, "progression personnelle de la semaine")
            } else {
                replace_period_phrase_after_first(
                    &value,
                    phrase,
                    "progression personnelle de la semaine",
                )
            };
            *response.pointer_mut(pointer).unwrap() = json!(normalized);
        }
    }
}
pub(crate) fn normalize_period_repetitive_public_phrases(response: &mut Value) {
    let mut counts = HashMap::<&'static str, usize>::new();
    normalize_period_repetitive_value(response, &mut counts, None);
}
pub(crate) fn dedupe_period_daily_timeline_texts(request: &Value, response: &mut Value) {
    let plan_by_date = request["daily_plans"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|plan| Some((plan.get("date")?.as_str()?.to_string(), plan.clone())))
        .collect::<HashMap<_, _>>();
    let Some(days) = response
        .get_mut("daily_timeline")
        .and_then(Value::as_array_mut)
    else {
        return;
    };
    let mut seen = HashSet::<String>::new();
    for day in days {
        let text = day
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let normalized = normalized_text(&text);
        if normalized.is_empty() || seen.insert(normalized) {
            continue;
        }
        let date = day.get("date").and_then(Value::as_str).unwrap_or("");
        let plan = plan_by_date.get(date).unwrap_or(day);
        let day_label = day
            .get("day_label")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("Ce jour");
        let theme = plan
            .get("theme_label")
            .and_then(Value::as_str)
            .or_else(|| day.get("theme").and_then(Value::as_str))
            .unwrap_or("la priorité du jour");
        let nuance = format!(            "{} précise ce repère par le thème {}, afin de distinguer cette étape du reste de la semaine.",            day_label, theme        );
        day["text"] = json!(sanitize_period_public_string(&format!(
            "{} {}",
            text.trim(),
            nuance
        )));
        seen.insert(normalized_text(day["text"].as_str().unwrap_or("")));
    }
}
pub(crate) fn normalize_period_repetitive_value(
    value: &mut Value,
    counts: &mut HashMap<&'static str, usize>,
    key: Option<&str>,
) {
    match value {
        Value::String(text) => {
            if !period_repetition_normalization_excluded_key(key) {
                *text = normalize_period_repetitive_text(text, counts);
            }
        }
        Value::Array(items) => {
            for item in items {
                normalize_period_repetitive_value(item, counts, key);
            }
        }
        Value::Object(map) => {
            for (child_key, child) in map {
                normalize_period_repetitive_value(child, counts, Some(child_key));
            }
        }
        _ => {}
    }
}
pub(crate) fn period_repetition_normalization_excluded_key(key: Option<&str>) -> bool {
    matches!(
        key,
        Some(
            "contract_version"
                | "service_code"
                | "date"
                | "evidence_key"
                | "evidence_keys"
                | "label"
                | "source_snapshot_keys"
                | "quality"
                | "period_resolution"
                | "provider"
                | "model"
                | "period_contract"
        )
    )
}
pub(crate) fn normalize_period_repetitive_text(
    text: &str,
    counts: &mut HashMap<&'static str, usize>,
) -> String {
    let mut normalized = text.to_string();
    for (phrase, replacements) in period_repetitive_phrase_replacements() {
        normalized = replace_period_phrase_after_allowed(&normalized, phrase, replacements, counts);
    }
    normalized
}
pub(crate) fn replace_period_phrase_after_allowed(
    text: &str,
    phrase: &'static str,
    replacements: &[&'static str],
    counts: &mut HashMap<&'static str, usize>,
) -> String {
    let lower = text.to_lowercase();
    let phrase_lower = phrase.to_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    while let Some(relative) = lower[cursor..].find(&phrase_lower) {
        let start = cursor + relative;
        let end = start + phrase.len();
        out.push_str(&text[cursor..start]);
        let count = counts.entry(phrase).or_insert(0);
        if *count < 2 {
            out.push_str(&text[start..end]);
        } else {
            let replacement = replacements
                .get((*count - 2) % replacements.len())
                .copied()
                .unwrap_or("préciser");
            out.push_str(replacement);
        }
        *count += 1;
        cursor = end;
    }
    out.push_str(&text[cursor..]);
    out
}
pub(crate) fn period_repetitive_phrase_replacements(
) -> &'static [(&'static str, &'static [&'static str])] {
    &[
        (
            "restez concret",
            &["gardez une prise directe", "revenez au geste utile"],
        ),
        (
            "gardez une marge",
            &["préservez un espace de recul", "laissez une respiration"],
        ),
        ("clarifier", &["rendre lisible", "mettre au net", "nommer"]),
        ("ajuster", &["réaccorder", "moduler", "reprendre"]),
        ("intégrer", &["assimiler", "relier", "consolider"]),
        (
            "met l'accent",
            &["souligne", "fait ressortir", "place l'attention"],
        ),
        (
            "choisissez une seule priorité",
            &[
                "retenez une priorité nette",
                "avancez avec une priorité lisible",
            ],
        ),
        (
            "Hiérarchisez une priorité",
            &[
                "Retenez une priorité nette",
                "Avancez avec une priorité lisible",
                "Gardez un seul axe prioritaire",
            ],
        ),
        (
            "le point d'appui concerne",
            &["l'appui principal touche", "le repère central passe par"],
        ),
        (
            "L'appui personnel vient de",
            &["L'appui concret passe par", "La nuance natale se lit dans"],
        ),
    ]
}
pub(crate) fn replace_period_phrase_all(text: &str, phrase: &str, replacement: &str) -> String {
    let lower = text.to_lowercase();
    let phrase_lower = phrase.to_lowercase();
    let mut out = String::new();
    let mut cursor = 0;
    for (index, _) in lower.match_indices(&phrase_lower) {
        out.push_str(&text[cursor..index]);
        out.push_str(replacement);
        cursor = index + phrase.len();
    }
    out.push_str(&text[cursor..]);
    out
}
pub(crate) fn replace_period_phrase_after_first(
    text: &str,
    phrase: &str,
    replacement: &str,
) -> String {
    let lower = text.to_lowercase();
    let phrase_lower = phrase.to_lowercase();
    let mut out = String::new();
    let mut cursor = 0;
    let mut seen = false;
    for (index, _) in lower.match_indices(&phrase_lower) {
        out.push_str(&text[cursor..index]);
        let end = index + phrase.len();
        if seen {
            out.push_str(replacement);
        } else {
            out.push_str(&text[index..end]);
            seen = true;
        }
        cursor = end;
    }
    out.push_str(&text[cursor..]);
    out
}
pub(crate) fn compact_period_words(text: &str, max_words: usize) -> String {
    if text.split_whitespace().count() <= max_words {
        return text.to_string();
    }
    let mut out = String::new();
    for sentence in period_complete_sentences(text) {
        let candidate = if out.is_empty() {
            sentence.to_string()
        } else {
            format!("{out} {sentence}")
        };
        if candidate.split_whitespace().count() > max_words {
            break;
        }
        out = candidate;
    }
    if !out.trim().is_empty() {
        return out;
    }
    let compact = text
        .split_whitespace()
        .take(max_words)
        .collect::<Vec<_>>()
        .join(" ");
    period_trim_incomplete_tail(&compact)
}
pub(crate) fn period_complete_sentences(text: &str) -> Vec<&str> {
    let mut sentences = Vec::new();
    let mut start = 0;
    for (index, ch) in text.char_indices() {
        if matches!(ch, '.' | '!' | '?') {
            let end = index + ch.len_utf8();
            let sentence = text[start..end].trim();
            if !sentence.is_empty() {
                sentences.push(sentence);
            }
            start = end;
        }
    }
    sentences
}
pub(crate) fn period_trim_incomplete_tail(text: &str) -> String {
    let mut words = text
        .split_whitespace()
        .map(|word| word.trim_matches(|ch: char| matches!(ch, ',' | ';' | ':')))
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    while words
        .last()
        .map(|word| period_is_weak_sentence_ending(word))
        .unwrap_or(false)
    {
        words.pop();
    }
    let mut compact = words.join(" ");
    compact = compact.trim_end_matches([',', ';', ':']).to_string();
    if !compact.ends_with(['.', '!', '?']) {
        compact.push('.');
    }
    compact
}
pub(crate) fn period_is_weak_sentence_ending(word: &str) -> bool {
    matches!(
        word.trim_matches(|ch: char| !ch.is_alphabetic())
            .to_lowercase()
            .as_str(),
        "et" | "à"
            | "a"
            | "de"
            | "pour"
            | "avec"
            | "sans"
            | "dans"
            | "sur"
            | "vers"
            | "la"
            | "le"
            | "les"
            | "des"
            | "du"
            | "au"
            | "aux"
            | "un"
            | "une"
            | "ce"
            | "cet"
            | "cette"
            | "d"
            | "l"
            | "qu"
            | "jusqu"
            | "puisqu"
            | "lorsqu"
    )
}
pub(crate) fn append_period_value_sentence(value: &mut Value, sentence: &str) {
    if let Some(text) = value.as_str() {
        let mut updated = text.to_string();
        append_period_sentence(&mut updated, sentence);
        *value = json!(updated);
    }
}
pub(crate) fn append_period_sentence(text: &mut String, sentence: &str) {
    if sentence.trim().is_empty() || text.contains(sentence) {
        return;
    }
    if !text.trim().is_empty() && !text.ends_with(' ') {
        text.push(' ');
    }
    text.push_str(sentence.trim());
}
pub(crate) fn period_public_word_count(response: &Value) -> usize {
    let mut public_text = String::new();
    collect_period_daily_public_text(response, &mut public_text);
    collect_period_public_text(response, &mut public_text);
    public_text.split_whitespace().count()
}
pub(crate) fn string_array_value(value: Option<&Value>) -> Option<Value> {
    let items = value?
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
        .filter(|item| !item.trim().is_empty())
        .map(|item| json!(item))
        .collect::<Vec<_>>();
    Some(Value::Array(items))
}
pub(crate) fn non_empty_string_array_value(value: Option<&Value>) -> Option<Value> {
    let value = string_array_value(value)?;
    if value.as_array().map(Vec::is_empty).unwrap_or(true) {
        None
    } else {
        Some(value)
    }
}
