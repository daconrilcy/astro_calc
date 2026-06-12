use super::*;

pub(crate) fn validate_period_public_text(public_text: &str) -> Result<(), GenerationError> {
    let lower = public_text.to_lowercase();
    for forbidden in [
        "personnaliser ce signal",
        "relier ce signal",
        "relier ce domaine",
        "plutôt que rester sur un conseil générique",
        "donne le relief principal",
        "donne une direction claire",
        "devient plus lisible",
        "deviennent plus lisibles",
        "vos repères personnels aident ici",
        "ce domaine donne une manière d'utiliser la semaine",
        " en prose utilisateur",
        "writer",
        "summary_hint",
        "advice_hint",
        "personalization_hint",
        "natal_focus_hint",
    ] {
        if lower.contains(forbidden) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_INTERNAL_GUIDANCE_LEAK",
                json!({ "forbidden": forbidden }),
            ));
        }
    }
    if contains_period_theme_instruction(&lower) {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_INTERNAL_GUIDANCE_LEAK",
            json!({ "forbidden": "date_theme_instruction" }),
        ));
    }
    if let Some(fragment) = period_broken_sentence_fragment(public_text) {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_BROKEN_SENTENCE",
            json!({ "fragment": fragment }),
        ));
    }
    if let Some(fragment) = period_truncated_example_fragment(public_text) {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_BROKEN_SENTENCE",
            json!({ "fragment": fragment }),
        ));
    }
    if let Some(fragment) = period_lowercase_sentence_start(public_text) {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_BROKEN_SENTENCE",
            json!({ "fragment": fragment }),
        ));
    }
    if let Some(fragment) = period_broken_french_fragment(public_text) {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_BROKEN_FRENCH_FRAGMENT",
            json!({ "fragment": fragment }),
        ));
    }
    for forbidden in [
        "plus personnel que générique",
        "conseil générique",
        "ce qui rend le conseil",
        "cette nuance reste liée",
        "avec un écho personnel autour de",
        "secteur personnel activé",
        "adaptez le geste au secteur personnel",
        "la lecture relie",
        "zones personnelles déjà mises en évidence",
        "zones personnelles",
        "zones natales activées",
        "secteurs personnels",
        "thème natal comme fil directeur",
        "le point d'appui concerne",
        "repère personnel concret sans devenir",
        "vos repères personnels liés à",
    ] {
        if lower.contains(forbidden) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_META_PERSONALIZATION_LEAK",
                json!({ "forbidden": forbidden }),
            ));
        }
    }
    if !french_elision_violations(public_text).is_empty() {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_FRENCH_TYPOGRAPHY_FAILED",
            json!({ "reason": "french_elision_violation" }),
        ));
    }
    if !french_glued_compound_violations(public_text).is_empty() {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_FRENCH_TYPOGRAPHY_FAILED",
            json!({ "reason": "french_glued_compound" }),
        ));
    }
    if period_has_bad_french_colon_spacing(public_text) {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_FRENCH_TYPOGRAPHY_FAILED",
            json!({ "reason": "colon_spacing" }),
        ));
    }
    if lower.contains(". .") || lower.contains(". ,") {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_FRENCH_TYPOGRAPHY_FAILED",
            json!({ "reason": "double_punctuation" }),
        ));
    }
    if lower.contains("vérifiez vérifier") {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_MECHANICAL_PUBLIC_TEXT",
            json!({ "reason": "repeated_verification_verb" }),
        ));
    }
    if period_marker_reason_is_suspect(public_text) {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_MECHANICAL_PUBLIC_TEXT",
            json!({ "reason": "serialized_situation_hint" }),
        ));
    }
    if lower
        .matches("cette énergie devient utile quand elle sert à")
        .count()
        > 1
    {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_MECHANICAL_PUBLIC_TEXT",
            json!({ "reason": "repeated_domain_template" }),
        ));
    }
    if let Some(fragment) = period_domain_template_fragment(public_text) {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_META_PERSONALIZATION_LEAK",
            json!({ "forbidden": fragment }),
        ));
    }
    for forbidden in [
        "slot:",
        "slot_",
        "[morning]",
        "[afternoon]",
        "[evening]",
        "raw_transits",
        "period:",
        "natal_",
        "fake_",
        "theme_code",
        "evidence_key",
        "snapshot",
        "transit_exact",
        "transit_active",
        "moon_house_by_day",
        "organization",
        "relationship",
        "energy",
        "clarity",
        "integration",
    ] {
        if lower.contains(forbidden) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_TECHNICAL_CODE_LEAK",
                json!({ "forbidden": forbidden }),
            ));
        }
    }
    for forbidden in [
        "focused",
        "focus",
        "supportive",
        "careful",
        "mixed",
        "fluid",
        "tense",
    ] {
        if contains_ascii_token(&lower, forbidden) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_TECHNICAL_CODE_LEAK",
                json!({ "forbidden": forbidden }),
            ));
        }
    }
    Ok(())
}
pub(crate) fn period_broken_french_fragment(public_text: &str) -> Option<String> {
    let lower = public_text.to_lowercase();
    for fragment in [
        "s’dynamique",
        "s'dynamique",
        "tout s’dynamique",
        "tout s'dynamique",
        "d’accélère",
        "d'accélère",
        "rédynamique",
        "redynamique",
        "l’organiser",
        "l'organiser",
        "consiste à de",
        " est de de ",
        " est d'de ",
        "allègerez",
        "consolider nommer",
        "consolider vérifier",
        "rendre concret tenir",
        "soleil dynamique un",
        "mars dynamique",
        "mercure dynamique",
        "et suspendre la discussion",
        "visible trancher et prouver vérifier",
    ] {
        if let Some(index) = lower.find(fragment) {
            return Some(public_text[index..].chars().take(48).collect::<String>());
        }
    }
    None
}
pub(crate) fn period_has_bad_french_colon_spacing(public_text: &str) -> bool {
    let chars = public_text.chars().collect::<Vec<_>>();
    for (index, ch) in chars.iter().enumerate() {
        if *ch != ':' {
            continue;
        }
        let before = index.checked_sub(1).and_then(|idx| chars.get(idx)).copied();
        let after = chars.get(index + 1).copied();
        if before.map(|ch| ch.is_ascii_digit()).unwrap_or(false)
            && after.map(|ch| ch.is_ascii_digit()).unwrap_or(false)
        {
            continue;
        }
        if before.map(|ch| !ch.is_whitespace()).unwrap_or(false)
            || after.map(|ch| !ch.is_whitespace()).unwrap_or(false)
        {
            return true;
        }
    }
    false
}
pub(crate) fn contains_period_theme_instruction(lower: &str) -> bool {
    lower
        .split(['.', '!', '?', '\n'])
        .any(|sentence| sentence.contains(", le thème ") && sentence.contains(" donne "))
}
pub(crate) fn period_broken_sentence_fragment(public_text: &str) -> Option<String> {
    for sentence in public_text.split(['.', '!', '?']) {
        let trimmed = sentence.trim();
        if trimmed.is_empty() {
            continue;
        }
        let tail = trimmed
            .split_whitespace()
            .rev()
            .take(3)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join(" ");
        if period_is_broken_sentence_tail(&tail) {
            return Some(tail);
        }
    }
    None
}
pub(crate) fn period_truncated_example_fragment(public_text: &str) -> Option<String> {
    let lower = public_text.to_lowercase();
    for marker in ["(par ex.", "(par exemple", "(ex."] {
        if let Some(index) = lower.rfind(marker) {
            let tail = &public_text[index..];
            if !tail.contains(')') {
                return Some(tail.chars().take(48).collect::<String>());
            }
        }
    }
    None
}
pub(crate) fn period_lowercase_sentence_start(public_text: &str) -> Option<String> {
    for (index, ch) in public_text.char_indices() {
        if !matches!(ch, '.' | '!' | '?') {
            continue;
        }
        let rest = public_text[index + ch.len_utf8()..].trim_start();
        let mut words = rest.split_whitespace();
        let first = words.next().unwrap_or("");
        let second = words.next().unwrap_or("");
        let first_is_lower = first
            .chars()
            .next()
            .map(|ch| ch.is_lowercase())
            .unwrap_or(false);
        let second_is_lower = second
            .chars()
            .next()
            .map(|ch| ch.is_lowercase())
            .unwrap_or(false);
        if first_is_lower {
            match first.trim_matches(|ch: char| !ch.is_alphabetic()) {
                "votre" | "vos" => {
                    return Some(rest.chars().take(32).collect::<String>());
                }
                "le" | "la" | "un" | "une" if second_is_lower => {
                    return Some(rest.chars().take(32).collect::<String>());
                }
                _ => {}
            }
        }
    }
    None
}
pub(crate) fn period_is_broken_sentence_tail(tail: &str) -> bool {
    let normalized = tail
        .trim()
        .trim_matches(|ch: char| matches!(ch, ',' | ';' | ':' | '\'' | '’' | '“' | '”' | '"'))
        .to_lowercase();
    let words = normalized.split_whitespace().collect::<Vec<_>>();
    match words.as_slice() {
        [] => false,
        [last] => period_is_weak_sentence_ending(last),
        [.., "à", "la" | "l" | "l'"] => true,
        [.., "de", "la" | "l" | "l'"] => true,
        [.., last] => period_is_weak_sentence_ending(last),
    }
}
pub(crate) fn validate_period_public_personalization(
    response: &Value,
) -> Result<(), GenerationError> {
    let mut count = 0;
    for day in response["daily_timeline"].as_array().into_iter().flatten() {
        if period_text_has_personalization(day["text"].as_str().unwrap_or("")) {
            count += 1;
        }
    }
    let week_text = format!(
        "{} {}",
        response["week_overview"]["text"].as_str().unwrap_or(""),
        response["week_overview"]["trajectory"]
            .as_str()
            .unwrap_or("")
    );
    for phrase in ["thème natal comme fil directeur", "relations directes"] {
        if count_normalized_phrase(&week_text, phrase) > 1 {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_OVERVIEW_REPETITION",
                json!({ "phrase": phrase }),
            ));
        }
    }
    if !period_text_has_personalization(&week_text) {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_EVIDENCE_MISSING",
            json!({ "reason": "week_overview_missing_natal_personalization" }),
        ));
    }
    if count < 4 {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_EVIDENCE_MISSING",
            json!({ "reason": "daily_timeline_missing_natal_personalization", "count": count }),
        ));
    }
    Ok(())
}
pub(crate) fn count_normalized_phrase(text: &str, phrase: &str) -> usize {
    text.to_lowercase().matches(&phrase.to_lowercase()).count()
}
pub(crate) fn period_text_has_personalization(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "thème natal",
        "vous-même",
        "vous meme",
        "pour vous",
        "vos priorités",
        "vos priorites",
        "votre agenda",
        "zone natale",
        "zones natales",
        "natal",
        "natale",
        "maison",
        "lune",
        "soleil",
        "vénus",
        "venus",
        "mars",
        "mercure",
        "jupiter",
        "saturne",
        "carré",
        "carre",
        "opposition",
        "opposé",
        "oppose",
        "sensibilité",
        "besoins émotionnels",
        "communiquer",
        "penser",
        "attachement",
        "plaisir",
        "agir",
        "énergie",
        "responsabilité",
        "limites",
        "relations directes",
        "besoin de sens",
        "habitudes",
        "rythme de travail",
        "qui fait quoi",
        "quelle preuve",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}
pub(crate) fn period_domain_text_is_generic(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("ce domaine donne une manière d'utiliser la semaine")
        || lower.contains("il donne une manière d'utiliser la semaine")
        || lower.contains("sans disperser l'énergie ni isoler les décisions")
}
pub(crate) fn validate_period_repeated_vocabulary(
    public_text: &str,
) -> Result<(), GenerationError> {
    let lower = public_text.to_lowercase();
    for phrase in [
        "restez concret",
        "gardez une marge",
        "clarifier",
        "ajuster",
        "intégrer",
        "met l'accent",
        "choisissez une seule priorité",
        "le point d'appui concerne",
    ] {
        let count = lower.matches(phrase).count();
        if count > 2 {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_REPETITIVE_DAILY_TEXT",
                json!({ "phrase": phrase, "count": count }),
            ));
        }
    }
    Ok(())
}
pub(crate) fn collect_period_daily_public_text(response: &Value, public_text: &mut String) {
    for day in response["daily_timeline"].as_array().into_iter().flatten() {
        for key in ["day_label", "theme", "tone", "text", "advice"] {
            if let Some(value) = day.get(key).and_then(|value| value.as_str()) {
                public_text.push_str(value);
                public_text.push('\n');
            }
        }
    }
}
pub(crate) fn contains_ascii_token(text: &str, token: &str) -> bool {
    text.match_indices(token).any(|(idx, _)| {
        let before = text[..idx].chars().next_back();
        let after = text[idx + token.len()..].chars().next();
        before
            .map(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
            .unwrap_or(true)
            && after
                .map(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
                .unwrap_or(true)
    })
}
pub(crate) fn collect_period_public_text(response: &Value, public_text: &mut String) {
    for pointer in [
        "/summary/title",
        "/summary/text",
        "/dominant_theme/theme",
        "/dominant_theme/text",
        "/week_overview/title",
        "/week_overview/text",
        "/week_overview/trajectory",
        "/watch_summary/text",
        "/advice",
        "/advice/main",
        "/advice/best_use",
        "/advice/avoid",
    ] {
        if let Some(value) = response.pointer(pointer).and_then(|value| value.as_str()) {
            public_text.push_str(value);
            public_text.push('\n');
        }
    }
    for field in [
        "key_days",
        "best_days",
        "watch_days",
        "best_windows",
        "watch_windows",
        "domain_sections",
        "evidence_summary",
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
            ] {
                if let Some(value) = item.get(key).and_then(|value| value.as_str()) {
                    public_text.push_str(value);
                    public_text.push('\n');
                }
            }
        }
    }
    for pointer in [
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
}
pub(crate) fn explicit_date_count(text: &str) -> usize {
    let tokens = text
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|ch: char| {
                !ch.is_alphanumeric() && !matches!(ch, '-' | '/' | 'û' | 'é')
            })
            .to_lowercase()
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let mut count = tokens
        .iter()
        .filter(|word| is_explicit_numeric_date(word))
        .count();
    for pair in tokens.windows(2) {
        if is_day_number(&pair[0]) && is_french_month_name(&pair[1]) {
            count += 1;
        }
    }
    count
}
pub(crate) fn is_explicit_numeric_date(token: &str) -> bool {
    if chrono::NaiveDate::parse_from_str(token, "%Y-%m-%d").is_ok()
        || chrono::NaiveDate::parse_from_str(token, "%d/%m/%Y").is_ok()
    {
        return true;
    }
    let parts = token.split('/').collect::<Vec<_>>();
    if parts.len() == 2 {
        return is_day_number(parts[0]) && is_month_number(parts[1]);
    }
    false
}
pub(crate) fn is_day_number(token: &str) -> bool {
    token
        .parse::<u32>()
        .map(|value| (1..=31).contains(&value))
        .unwrap_or(false)
}
pub(crate) fn is_month_number(token: &str) -> bool {
    token
        .parse::<u32>()
        .map(|value| (1..=12).contains(&value))
        .unwrap_or(false)
}
pub(crate) fn is_french_month_name(token: &str) -> bool {
    matches!(
        token,
        "janvier"
            | "fevrier"
            | "février"
            | "mars"
            | "avril"
            | "mai"
            | "juin"
            | "juillet"
            | "aout"
            | "août"
            | "septembre"
            | "octobre"
            | "novembre"
            | "decembre"
            | "décembre"
    )
}
pub(crate) fn validate_period_not_seven_daily(response: &Value) -> Result<(), GenerationError> {
    if response.get("week_overview").is_none()
        || response.get("domain_sections").is_none()
        || response.get("key_days").is_none()
    {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_REPETITIVE_DAILY_TEXT",
            json!({ "reason": "missing_period_level_sections" }),
        ));
    }
    Ok(())
}
