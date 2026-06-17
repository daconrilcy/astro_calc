use std::collections::HashSet;

use crate::domain::BasicSignal;

use super::json;

pub(super) fn is_structural_axis_aspect_signal(
    signal: &BasicSignal,
    structural_axis_pairs: &HashSet<(String, String)>,
) -> bool {
    crate::natal::payload::rules::angles::is_structural_axis_aspect_signal(
        signal,
        structural_axis_pairs,
    )
}

pub(super) fn is_angle_to_angle_aspect_signal(
    signal: &BasicSignal,
    angle_object_codes: &HashSet<String>,
) -> bool {
    crate::natal::payload::rules::angles::is_angle_to_angle_aspect_signal(
        signal,
        angle_object_codes,
    )
}

pub(super) fn has_current_aspect_context(signal: &BasicSignal) -> bool {
    if !signal.signal_key.starts_with("aspect:") {
        return true;
    }

    let Some(context) = signal.aspect_context.as_ref() else {
        return false;
    };

    json::has_text_value(context.get("aspect_family"))
        && context.get("primary_valence").is_some()
        && context.get("intensity_modifier").is_some()
        && context.get("secondary_effect").is_some()
        && has_any_aspect_effect(context)
        && json::has_text_value(context.get("dynamic_quality"))
        && json::has_text_value(context.get("phase_state"))
        && json::has_text_value(context.get("valence_family"))
        && json::has_bool_value(context.get("is_tonal_valence"))
        && json::has_bool_value(context.get("is_intensity_modifier"))
}

fn has_any_aspect_effect(context: &serde_json::Value) -> bool {
    ["primary_valence", "intensity_modifier", "secondary_effect"]
        .into_iter()
        .any(|key| json::has_text_value(context.get(key)))
}
