use crate::domain::ObjectPositionFact;

pub(super) fn placement_context_object(
    position: &ObjectPositionFact,
    key: &str,
) -> Option<serde_json::Value> {
    position
        .facts_json
        .as_ref()
        .and_then(|facts| facts.get(key))
        .filter(|value| !value.is_null())
        .cloned()
}

pub(super) fn placement_context_value<'a>(
    position: &'a ObjectPositionFact,
    context_key: &str,
    value_key: &str,
) -> Option<&'a serde_json::Value> {
    position
        .facts_json
        .as_ref()
        .and_then(|facts| facts.get(context_key))
        .and_then(|context| context.get(value_key))
        .filter(|value| !value.is_null())
}

pub(super) fn placement_context_str<'a>(
    position: &'a ObjectPositionFact,
    context_key: &str,
    value_key: &str,
) -> Option<&'a str> {
    placement_context_value(position, context_key, value_key).and_then(|value| value.as_str())
}
