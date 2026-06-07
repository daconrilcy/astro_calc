use std::collections::HashSet;

use crate::domain::{AspectFact, InterpretationSignalDraft};

use super::constants::{
    SIGNAL_PREFIX_ASPECT, SIGNAL_PREFIX_CLUSTER, SUPPRESSION_ACTIVE, SUPPRESSION_SUPPRESSED,
};
use super::relations::normalized_pair;

pub(super) fn suppress_over_basic_limit(
    signals: &mut [InterpretationSignalDraft],
    max_active_signals: usize,
) {
    let mut active_count = 0;
    for signal in signals {
        if signal.suppression_state != SUPPRESSION_ACTIVE {
            continue;
        }

        active_count += 1;
        if active_count > max_active_signals {
            signal.suppression_state = SUPPRESSION_SUPPRESSED.to_string();
        }
    }
}

pub(super) fn preserve_strong_tension_aspect(
    signals: &mut [InterpretationSignalDraft],
    angle_object_codes: &HashSet<String>,
) {
    if signals.iter().any(|signal| {
        signal.suppression_state == SUPPRESSION_ACTIVE
            && is_strong_tension_signal(signal, angle_object_codes)
    }) {
        return;
    }

    let Some(tension_index) = signals
        .iter()
        .enumerate()
        .filter(|(_, signal)| is_strong_tension_signal(signal, angle_object_codes))
        .max_by(|(_, left), (_, right)| {
            left.priority_score
                .partial_cmp(&right.priority_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(index, _)| index)
    else {
        return;
    };

    let Some(replacement_index) = signals
        .iter()
        .enumerate()
        .rev()
        .find(|(_, signal)| {
            signal.suppression_state == SUPPRESSION_ACTIVE && !is_basic_required_signal(signal)
        })
        .map(|(index, _)| index)
    else {
        return;
    };

    signals[replacement_index].suppression_state = SUPPRESSION_SUPPRESSED.to_string();
    signals[tension_index].suppression_state = SUPPRESSION_ACTIVE.to_string();
}

pub(super) fn preserve_strong_non_structural_aspect(
    signals: &mut [InterpretationSignalDraft],
    angle_object_codes: &HashSet<String>,
) {
    if signals.iter().any(|signal| {
        signal.suppression_state == SUPPRESSION_ACTIVE
            && is_strong_non_structural_aspect_signal(signal, angle_object_codes)
    }) {
        return;
    }

    let Some(aspect_index) = signals
        .iter()
        .enumerate()
        .filter(|(_, signal)| is_strong_non_structural_aspect_signal(signal, angle_object_codes))
        .max_by(|(_, left), (_, right)| {
            left.priority_score
                .partial_cmp(&right.priority_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(index, _)| index)
    else {
        return;
    };

    let Some(replacement_index) = signals
        .iter()
        .enumerate()
        .rev()
        .find(|(_, signal)| {
            signal.suppression_state == SUPPRESSION_ACTIVE && !is_basic_required_signal(signal)
        })
        .map(|(index, _)| index)
    else {
        return;
    };

    signals[replacement_index].suppression_state = SUPPRESSION_SUPPRESSED.to_string();
    signals[aspect_index].suppression_state = SUPPRESSION_ACTIVE.to_string();
}

fn is_strong_non_structural_aspect_signal(
    signal: &InterpretationSignalDraft,
    angle_object_codes: &HashSet<String>,
) -> bool {
    signal.signal_key.starts_with(SIGNAL_PREFIX_ASPECT)
        && !is_structural_axis_signal(signal)
        && !is_angle_to_angle_aspect_signal(signal, angle_object_codes)
        && aspect_strength_score(signal) >= 0.75
}

fn is_strong_tension_signal(
    signal: &InterpretationSignalDraft,
    angle_object_codes: &HashSet<String>,
) -> bool {
    if !signal.signal_key.starts_with(SIGNAL_PREFIX_ASPECT) {
        return false;
    }
    if is_structural_axis_signal(signal) {
        return false;
    }
    if is_angle_to_angle_aspect_signal(signal, angle_object_codes) {
        return false;
    }

    let Some(evidence) = signal
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get("evidence"))
    else {
        return false;
    };

    let aspect_code = evidence.get("aspect_code").and_then(|value| value.as_str());
    let strength_score = evidence
        .get("strength_score")
        .and_then(|value| value.as_f64())
        .unwrap_or(signal.priority_score / 80.0);

    matches!(aspect_code, Some("square" | "opposition")) && strength_score >= 0.75
}

fn aspect_strength_score(signal: &InterpretationSignalDraft) -> f64 {
    signal
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get("evidence"))
        .and_then(|evidence| evidence.get("strength_score"))
        .and_then(|value| value.as_f64())
        .unwrap_or(signal.priority_score / 80.0)
}

fn is_basic_required_signal(signal: &InterpretationSignalDraft) -> bool {
    if signal.signal_key.starts_with(SIGNAL_PREFIX_CLUSTER) {
        return true;
    }

    let Some(object_code) = signal
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get("evidence"))
        .and_then(|evidence| evidence.get("object_code"))
        .and_then(|value| value.as_str())
    else {
        return false;
    };

    matches!(
        object_code,
        "sun" | "moon" | "ascendant" | "mc" | "mercury" | "venus" | "mars"
    )
}

pub(super) fn fill_basic_active_limit(
    signals: &mut [InterpretationSignalDraft],
    angle_object_codes: &HashSet<String>,
    max_active_signals: usize,
    aspect_min_strength: f64,
) {
    let mut active_count = signals
        .iter()
        .filter(|signal| signal.suppression_state == SUPPRESSION_ACTIVE)
        .count();

    if active_count >= max_active_signals {
        return;
    }

    for signal in signals {
        if active_count >= max_active_signals {
            break;
        }

        if signal.suppression_state == SUPPRESSION_SUPPRESSED
            && is_basic_fill_eligible(signal, angle_object_codes, aspect_min_strength)
        {
            signal.suppression_state = SUPPRESSION_ACTIVE.to_string();
            active_count += 1;
        }
    }
}

fn is_basic_fill_eligible(
    signal: &InterpretationSignalDraft,
    angle_object_codes: &HashSet<String>,
    aspect_min_strength: f64,
) -> bool {
    !is_weak_aspect_signal(signal, aspect_min_strength)
        && !is_structural_axis_signal(signal)
        && !is_angle_to_angle_aspect_signal(signal, angle_object_codes)
}

fn is_weak_aspect_signal(signal: &InterpretationSignalDraft, aspect_min_strength: f64) -> bool {
    if !signal.signal_key.starts_with(SIGNAL_PREFIX_ASPECT) {
        return false;
    }

    signal
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get("evidence"))
        .and_then(|evidence| evidence.get("strength_score"))
        .and_then(|value| value.as_f64())
        .is_some_and(|strength_score| strength_score < aspect_min_strength)
}

pub(super) fn is_structural_axis_aspect(
    aspect: &AspectFact,
    structural_axis_pairs: &HashSet<(String, String)>,
) -> bool {
    aspect
        .calculation_notes_json
        .as_ref()
        .and_then(|notes| notes.get("is_structural_axis"))
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
        || (aspect.aspect_code == "opposition"
            && structural_axis_pairs.contains(&normalized_pair(
                &aspect.source_object_code,
                &aspect.target_object_code,
            )))
}

fn is_structural_axis_signal(signal: &InterpretationSignalDraft) -> bool {
    signal
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get("aspect_context"))
        .and_then(|context| context.get("is_structural_axis"))
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
        || signal
            .payload_json
            .as_ref()
            .and_then(|payload| payload.get("evidence"))
            .and_then(|evidence| evidence.get("is_structural_axis"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
}

fn is_angle_to_angle_aspect_signal(
    signal: &InterpretationSignalDraft,
    angle_object_codes: &HashSet<String>,
) -> bool {
    if !signal.signal_key.starts_with(SIGNAL_PREFIX_ASPECT) {
        return false;
    }

    object_pair_from_signal(signal).is_some_and(|(source, target)| {
        angle_object_codes.contains(&source) && angle_object_codes.contains(&target)
    })
}

pub(super) fn is_angle_to_angle_aspect(
    aspect: &AspectFact,
    angle_object_codes: &HashSet<String>,
) -> bool {
    angle_object_codes.contains(&aspect.source_object_code)
        && angle_object_codes.contains(&aspect.target_object_code)
}

fn object_pair_from_signal(signal: &InterpretationSignalDraft) -> Option<(String, String)> {
    let evidence_pair = signal
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get("evidence"))
        .and_then(|evidence| {
            let source = evidence
                .get("source_object_code")
                .and_then(|value| value.as_str())?;
            let target = evidence
                .get("target_object_code")
                .and_then(|value| value.as_str())?;
            Some((source.to_string(), target.to_string()))
        });
    if evidence_pair.is_some() {
        return evidence_pair;
    }

    let parts = signal.signal_key.split(':').collect::<Vec<_>>();
    if parts.len() >= 4 && parts[0] == "aspect" {
        Some((parts[1].to_string(), parts[2].to_string()))
    } else {
        None
    }
}
