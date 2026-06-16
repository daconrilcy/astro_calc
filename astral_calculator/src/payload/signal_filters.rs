use crate::domain::BasicSignal;
use crate::payload_shared::aspect::is_marked_structural_axis;
use std::collections::HashSet;
pub(super) fn is_interpretive_tension_aspect(signal: &BasicSignal) -> bool {
    if !is_interpretive_aspect_signal(signal) {
        return false;
    }

    let dynamic_quality = aspect_context_str(signal, "dynamic_quality");
    let primary_valence = aspect_context_str(signal, "primary_valence");
    let strength_score = aspect_strength_score(signal);

    matches!(dynamic_quality, Some("tension"))
        || matches!(
            primary_valence,
            Some("dynamic_challenging" | "polarizing" | "minor_friction" | "indirect_tension")
        ) && strength_score >= 0.75
}

pub(super) fn is_interpretive_support_aspect(signal: &BasicSignal) -> bool {
    if !is_interpretive_aspect_signal(signal) {
        return false;
    }

    matches!(aspect_context_str(signal, "dynamic_quality"), Some("flow"))
        || matches!(
            aspect_context_str(signal, "primary_valence"),
            Some("supportive" | "harmonious")
        )
}

pub(super) fn is_interpretive_aspect_signal(signal: &BasicSignal) -> bool {
    signal.signal_key.starts_with("aspect:") && !is_structural_axis_signal(signal)
}

pub(super) fn is_structural_axis_signal(signal: &BasicSignal) -> bool {
    is_marked_structural_axis(signal)
}

pub(super) fn is_structural_axis_signal_for_pairs(
    signal: &BasicSignal,
    structural_axis_pairs: &HashSet<(String, String)>,
) -> bool {
    crate::payload_rules::angles::is_structural_axis_aspect_signal(signal, structural_axis_pairs)
}

pub(super) fn is_angle_to_angle_aspect_signal(
    signal: &BasicSignal,
    angle_object_codes: &HashSet<String>,
) -> bool {
    crate::payload_rules::angles::is_angle_to_angle_aspect_signal(signal, angle_object_codes)
}

fn aspect_context_str<'a>(signal: &'a BasicSignal, key: &str) -> Option<&'a str> {
    signal
        .aspect_context
        .as_ref()
        .and_then(|context| context.get(key))
        .and_then(|value| value.as_str())
}

pub(super) fn aspect_strength_score(signal: &BasicSignal) -> f64 {
    signal
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.get("strength_score"))
        .and_then(|value| value.as_f64())
        .unwrap_or(signal.priority_score / 80.0)
}
