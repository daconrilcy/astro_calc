use super::*;

pub(crate) fn period_text_has_personalization(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "vous",
        "vos",
        "votre",
        "priorités",
        "priorites",
        "rythme",
        "relations directes",
        "besoins émotionnels",
        "besoins emotionnels",
        "besoin de sens",
        "attachement",
        "habitudes",
        "responsabilité",
        "responsabilite",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

pub(crate) fn explicit_date_count(text: &str) -> usize {
    static DATE_RE: OnceLock<Regex> = OnceLock::new();
    DATE_RE
        .get_or_init(|| Regex::new(r"\b\d{1,2}/\d{1,2}\b").expect("explicit date regex"))
        .find_iter(text)
        .count()
}
