use crate::domain::BasicSignal;
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
    signal
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
            .unwrap_or(false)
}

pub(super) fn is_structural_axis_signal_for_pairs(
    signal: &BasicSignal,
    structural_axis_pairs: &HashSet<(String, String)>,
) -> bool {
    if !signal.signal_key.starts_with("aspect:") {
        return false;
    }
    if is_structural_axis_signal(signal) {
        return true;
    }
    if aspect_code(signal) != Some("opposition") {
        return false;
    }

    object_pair_from_aspect_signal(signal).is_some_and(|pair| structural_axis_pairs.contains(&pair))
}

pub(super) fn is_angle_to_angle_aspect_signal(
    signal: &BasicSignal,
    angle_object_codes: &HashSet<String>,
) -> bool {
    if !signal.signal_key.starts_with("aspect:") {
        return false;
    }

    object_pair_from_aspect_signal(signal).is_some_and(|(source, target)| {
        angle_object_codes.contains(&source) && angle_object_codes.contains(&target)
    })
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

pub(super) fn normalized_pair(left: &str, right: &str) -> (String, String) {
    if left <= right {
        (left.to_string(), right.to_string())
    } else {
        (right.to_string(), left.to_string())
    }
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
