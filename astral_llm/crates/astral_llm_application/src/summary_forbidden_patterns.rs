//! Patterns lexicaux interdits dans `summary.title` et `summary.short_text` uniquement.

use regex::Regex;
use std::sync::OnceLock;

static SUMMARY_FORBIDDEN_RE: OnceLock<Regex> = OnceLock::new();

pub fn summary_forbidden_regex() -> &'static Regex {
    SUMMARY_FORBIDDEN_RE.get_or_init(|| {
        Regex::new(
            r"(?iu)\b(oracle|oracles|tirage|tirages|cartes?\s+tir[ée]es?|consultations?\s+divinatoires?|liane\s+de\s+constance|tendance\s+invite)\b",
        )
        .expect("valid summary forbidden regex")
    })
}

pub fn find_forbidden_summary_patterns(text: &str) -> Vec<String> {
    summary_forbidden_regex()
        .find_iter(text)
        .map(|m| m.as_str().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_oracle_singular_and_plural() {
        assert!(find_forbidden_summary_patterns("oracle_model").is_empty());
        assert!(!find_forbidden_summary_patterns("un oracle.").is_empty());
        assert!(!find_forbidden_summary_patterns("ces oracles symboliques").is_empty());
    }

    #[test]
    fn matches_tirage_singular_and_plural() {
        assert!(find_forbidden_summary_patterns("retirage").is_empty());
        assert!(!find_forbidden_summary_patterns("ce tirage évoque").is_empty());
        assert!(!find_forbidden_summary_patterns("ces tirages évoquent").is_empty());
    }

    #[test]
    fn matches_cartes_tirees() {
        assert!(!find_forbidden_summary_patterns("les cartes tirées indiquent").is_empty());
        assert!(!find_forbidden_summary_patterns("les cartes tirees indiquent").is_empty());
        assert!(!find_forbidden_summary_patterns("la carte tiree indique").is_empty());
    }

    #[test]
    fn matches_consultation_divinatoire() {
        assert!(!find_forbidden_summary_patterns("une consultation divinatoire").is_empty());
        assert!(!find_forbidden_summary_patterns("des consultations divinatoires").is_empty());
    }

    #[test]
    fn matches_tendance_invite_phrase() {
        assert!(!find_forbidden_summary_patterns("tendance invite a avancer").is_empty());
    }
}
