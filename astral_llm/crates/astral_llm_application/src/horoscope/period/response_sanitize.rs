use super::*;

pub(crate) fn sanitize_period_public_string(text: &str) -> String {
    let reprocessed = reprocess_horoscope_period("fr", json!(text), None)
        .payload
        .as_str()
        .unwrap_or(text)
        .to_string();
    let (reprocessed, _) = restore_french_glued_compounds(&reprocessed);
    let repaired = repair_period_truncated_public_tail(&reprocessed);
    repair_period_mechanical_public_fragments(&repaired)
}

pub(crate) fn ensure_period_personalization_text(text: &str, personalization: &str) -> String {
    let base = sanitize_period_public_string(text);
    if period_text_has_personalization(&base) {
        return base;
    }
    let personalization = sanitize_period_public_string(personalization);
    if personalization.trim().is_empty() {
        return base;
    }
    sanitize_period_public_string(&format!("{} {}", base.trim(), personalization.trim()))
}

pub(crate) fn period_public_focus_text(item: &Value) -> String {
    item.get("focus")
        .and_then(Value::as_str)
        .or_else(|| item.get("summary_hint").and_then(Value::as_str))
        .or_else(|| item.get("theme_label").and_then(Value::as_str))
        .or_else(|| item.get("human_label").and_then(Value::as_str))
        .unwrap_or("garder un cadre simple et vérifiable")
        .to_string()
}

pub(crate) fn period_public_day_text(item: &Value, _index: usize) -> String {
    let day_label = item
        .get("day_label")
        .and_then(Value::as_str)
        .or_else(|| item.get("date").and_then(Value::as_str))
        .unwrap_or("Ce jour");
    let theme = item
        .get("theme_label")
        .and_then(Value::as_str)
        .or_else(|| item.get("theme").and_then(Value::as_str))
        .unwrap_or("vos priorités");
    format!(
        "{day_label} met surtout l'accent sur {theme} : avancez par un geste court, concret et vérifiable."
    )
}

pub(crate) fn period_public_day_advice(item: &Value) -> String {
    let theme = item
        .get("theme_label")
        .and_then(Value::as_str)
        .or_else(|| item.get("theme").and_then(Value::as_str))
        .unwrap_or("ce sujet");
    format!("Utilisez {theme} pour choisir une action simple plutôt qu'une réaction large.")
}

pub(crate) fn period_public_domain_text(item: &Value) -> String {
    let domain = item
        .get("domain")
        .and_then(Value::as_str)
        .or_else(|| item.get("title").and_then(Value::as_str))
        .unwrap_or("ce domaine");
    let focus = period_public_focus_text(item);
    format!("{domain} sert de repère transversal dans la période. {focus}")
}

pub(crate) fn period_focus_parts(focus: &str, max_parts: usize) -> Vec<String> {
    focus
        .split([',', ';'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .take(max_parts)
        .map(ToOwned::to_owned)
        .collect()
}

pub(crate) fn naturalize_period_focus(focus: &str) -> String {
    let parts = period_focus_parts(focus, 3);
    match parts.as_slice() {
        [one] => format!("Le geste utile consiste à {one}."),
        [one, two] => format!("Le geste utile consiste à {one}, puis à {two}."),
        [one, two, three, ..] => {
            format!("Le geste utile consiste à {one}, à {two} ou à {three}.")
        }
        _ => "Choisissez un geste simple et vérifiable.".to_string(),
    }
}

pub(crate) fn repair_period_truncated_public_tail(text: &str) -> String {
    let trimmed = text.trim();
    let lower = trimmed.to_lowercase();
    for marker in ["(par ex.", "(par exemple", "(ex."] {
        if let Some(index) = lower.rfind(marker) {
            if !trimmed[index..].contains(')') {
                let mut repaired = trimmed[..index]
                    .trim_end()
                    .trim_end_matches([',', ';', ':'])
                    .to_string();
                if !repaired.ends_with(['.', '!', '?']) {
                    repaired.push('.');
                }
                return repaired;
            }
        }
    }
    trimmed.to_string()
}

pub(crate) fn repair_period_mechanical_public_fragments(text: &str) -> String {
    let mut repaired = text.to_string();
    for (pattern, replacement) in period_mechanical_public_fragment_replacements() {
        repaired = pattern.replace_all(&repaired, *replacement).into_owned();
    }
    repaired
}

pub(crate) fn period_mechanical_public_fragment_replacements() -> &'static [(Regex, &'static str)] {
    static REPLACEMENTS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    REPLACEMENTS
        .get_or_init(|| {
            [
                (r"(?i)\bvérifiez\s+vérifier\b", "vérifiez puis ajustez"),
                (r"(?i)\bautour\s+de\s+vérifier\b", "pour vérifier"),
                (r"(?i)\bautour\s+d['’]attendre\b", "avant d'attendre"),
                (r"(?i)\bautour\s+de\s+attendre\b", "avant d'attendre"),
                (r"(?i):\s*appuis\s+concrets\s+aide\b", " : cet appui aide"),
                (r"(?i)\bappui\s+concret\s*:", "Point d'appui :"),
                (r"(?i)\brevint\b", "revient"),
                (r"\.\s+\.", "."),
                (r"\.\s*,", ","),
                (r"\s+\.", "."),
            ]
            .into_iter()
            .map(|(pattern, replacement)| {
                (
                    Regex::new(pattern).expect("period mechanical fragment regex"),
                    replacement,
                )
            })
            .collect()
        })
        .as_slice()
}
