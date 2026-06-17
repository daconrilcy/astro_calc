pub(super) fn option_json_has_text(value: &Option<serde_json::Value>, key: &str) -> bool {
    value
        .as_ref()
        .and_then(|value| value.get(key))
        .is_some_and(json_value_has_text)
}

pub(super) fn nested_json_has_text(
    value: &serde_json::Value,
    context_key: &str,
    key: &str,
) -> bool {
    value
        .get(context_key)
        .and_then(|context| context.get(key))
        .is_some_and(json_value_has_text)
}

pub(super) fn has_text_value(value: Option<&serde_json::Value>) -> bool {
    value.is_some_and(json_value_has_text)
}

pub(super) fn has_bool_value(value: Option<&serde_json::Value>) -> bool {
    value.is_some_and(serde_json::Value::is_boolean)
}

fn json_value_has_text(value: &serde_json::Value) -> bool {
    value.as_str().is_some_and(|text| !text.trim().is_empty())
}
