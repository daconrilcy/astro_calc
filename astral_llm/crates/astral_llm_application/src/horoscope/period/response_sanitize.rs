use super::*;

pub(crate) fn period_public_day_text(day: &Value, _index: usize) -> String {
    day.get("text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub(crate) fn period_public_focus_text(day: &Value) -> String {
    day.get("focus")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub(crate) fn naturalize_period_focus(text: &str) -> String {
    text.to_string()
}

pub(crate) fn ensure_period_personalization_text(base: &str, personalization: &str) -> String {
    let base = base.trim();
    let personalization = personalization.trim();
    match (base.is_empty(), personalization.is_empty()) {
        (true, true) => String::new(),
        (true, false) => personalization.to_string(),
        (false, true) => base.to_string(),
        (false, false) => format!("{base} {personalization}"),
    }
}

pub(crate) fn period_public_day_advice(day: &Value) -> String {
    day.get("advice")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub(crate) fn period_public_domain_text(section: &Value) -> String {
    section
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}
