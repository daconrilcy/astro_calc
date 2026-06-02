use std::collections::HashSet;

use crate::domain::BasicSignal;

use super::{angles::normalized_pair, json};

pub(super) fn is_structural_axis_aspect_signal(
    signal: &BasicSignal,
    structural_axis_pairs: &HashSet<(String, String)>,
) -> bool {
    let is_marked_structural_axis = signal
        .aspect_context
        .as_ref()
        .and_then(|context| context.get("is_structural_axis"))
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
        || signal
            .evidence
            .as_ref()
            .and_then(|evidence| evidence.get("is_structural_axis"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false);

    signal.signal_key.starts_with("aspect:")
        && (is_marked_structural_axis
            || (aspect_code(signal) == Some("opposition")
                && object_pair_from_aspect_signal(signal)
                    .is_some_and(|pair| structural_axis_pairs.contains(&pair))))
}

pub(super) fn is_angle_to_angle_aspect_signal(
    signal: &BasicSignal,
    angle_object_codes: &HashSet<String>,
) -> bool {
    signal.signal_key.starts_with("aspect:")
        && object_pair_from_aspect_signal(signal).is_some_and(|(source, target)| {
            angle_object_codes.contains(&source) && angle_object_codes.contains(&target)
        })
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
        && json::has_text_value(context.get("writing_guidance"))
}

fn has_any_aspect_effect(context: &serde_json::Value) -> bool {
    ["primary_valence", "intensity_modifier", "secondary_effect"]
        .into_iter()
        .any(|key| json::has_text_value(context.get(key)))
}

fn aspect_code(signal: &BasicSignal) -> Option<&str> {
    signal
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.get("aspect_code"))
        .and_then(|value| value.as_str())
        .or_else(|| signal.signal_key.split(':').nth(3))
}

fn object_pair_from_aspect_signal(signal: &BasicSignal) -> Option<(String, String)> {
    let evidence_pair = signal.evidence.as_ref().and_then(|evidence| {
        let source = evidence
            .get("source_object_code")
            .and_then(|value| value.as_str())?;
        let target = evidence
            .get("target_object_code")
            .and_then(|value| value.as_str())?;
        Some(normalized_pair(source, target))
    });
    if evidence_pair.is_some() {
        return evidence_pair;
    }

    let parts = signal.signal_key.split(':').collect::<Vec<_>>();
    if parts.len() >= 4 {
        Some(normalized_pair(parts[1], parts[2]))
    } else {
        None
    }
}
