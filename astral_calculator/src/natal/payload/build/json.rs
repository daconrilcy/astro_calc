use crate::domain::ObjectPositionFact;
use crate::domain::InterpretationSignalRow;
pub(super) fn payload_value(
    signal: &InterpretationSignalRow,
    key: &str,
) -> Option<serde_json::Value> {
    signal
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get(key).cloned())
}

pub(super) fn payload_aspect_context(
    signal: &InterpretationSignalRow,
) -> Option<serde_json::Value> {
    let mut context = payload_value(signal, "aspect_context")?;
    if let Some(object) = context.as_object_mut() {
        object.remove("writing_guidance");
    }
    Some(context)
}

pub(super) fn payload_string(signal: &InterpretationSignalRow, key: &str) -> Option<String> {
    signal
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get(key))
        .and_then(|value| value.as_str())
        .map(ToString::to_string)
}

pub(super) fn payload_f64(signal: &InterpretationSignalRow, key: &str) -> Option<f64> {
    signal
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get(key))
        .and_then(|value| value.as_f64())
}

pub(super) fn payload_string_array(signal: &InterpretationSignalRow, key: &str) -> Vec<String> {
    signal
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get(key))
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn position_context(
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
