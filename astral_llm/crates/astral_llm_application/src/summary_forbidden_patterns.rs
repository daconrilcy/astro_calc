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
