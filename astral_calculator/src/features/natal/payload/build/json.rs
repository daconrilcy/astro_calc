//! Module astral_calculator\src\features\natal\payload\build\json.rs du moteur astral_calculator.

use crate::domain::InterpretationSignalRow;
use crate::domain::ObjectPositionFact;
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
    let context = position.context()?;
    match key {
        "sign_context" => context
            .sign_context
            .and_then(|value| serde_json::to_value(value).ok()),
        "house_context" => context
            .house_context
            .and_then(|value| serde_json::to_value(value).ok()),
        "house_modality" => context
            .house_modality
            .and_then(|value| serde_json::to_value(value).ok()),
        "object_context" => context
            .object_context
            .and_then(|value| serde_json::to_value(value).ok()),
        "motion_context" => context
            .motion_context
            .and_then(|value| serde_json::to_value(value).ok()),
        "angle_context" => context
            .angle_context
            .and_then(|value| serde_json::to_value(value).ok()),
        "visibility_context" => context
            .visibility_context
            .and_then(|value| serde_json::to_value(value).ok()),
        _ => None,
    }
}

pub(super) fn position_sign_context(position: &ObjectPositionFact) -> Option<serde_json::Value> {
    position
        .context()
        .and_then(|context| context.sign_context)
        .and_then(|context| serde_json::to_value(context).ok())
}

pub(super) fn position_house_context(position: &ObjectPositionFact) -> Option<serde_json::Value> {
    position
        .context()
        .and_then(|context| context.house_context)
        .and_then(|context| serde_json::to_value(context).ok())
}

pub(super) fn position_house_modality(position: &ObjectPositionFact) -> Option<serde_json::Value> {
    position
        .context()
        .and_then(|context| context.house_modality)
        .and_then(|context| serde_json::to_value(context).ok())
}

pub(super) fn position_object_context(position: &ObjectPositionFact) -> Option<serde_json::Value> {
    position
        .context()
        .and_then(|context| context.object_context)
        .and_then(|context| serde_json::to_value(context).ok())
}

pub(super) fn position_motion_context(position: &ObjectPositionFact) -> Option<serde_json::Value> {
    position
        .context()
        .and_then(|context| context.motion_context)
        .and_then(|context| serde_json::to_value(context).ok())
}
