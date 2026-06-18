//! Module astral_calculator\src\features\natal\payload\shared\text.rs du moteur astral_calculator.

pub(crate) fn has_text(value: &str) -> bool {
    !value.trim().is_empty()
}

pub(crate) fn has_unique_non_empty_strings(values: &[String]) -> bool {
    let mut seen = std::collections::HashSet::new();
    values
        .iter()
        .all(|value| has_text(value) && seen.insert(value.as_str()))
}

pub(crate) fn push_unique(target: &mut Vec<String>, value: String) {
    if !target.iter().any(|existing| existing == &value) {
        target.push(value);
    }
}

pub(crate) fn is_normalized_score(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}
