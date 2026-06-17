use std::collections::HashSet;

use crate::domain::BasicSignal;

pub(crate) fn normalized_pair(left: &str, right: &str) -> (String, String) {
    if left <= right {
        (left.to_string(), right.to_string())
    } else {
        (right.to_string(), left.to_string())
    }
}

pub(crate) fn structural_axis_pairs(
    angle_positions: &[(String, String)],
) -> HashSet<(String, String)> {
    let mut pairs = HashSet::new();

    for left_index in 0..angle_positions.len() {
        for right_index in (left_index + 1)..angle_positions.len() {
            let (left_axis, left_code) = &angle_positions[left_index];
            let (right_axis, right_code) = &angle_positions[right_index];
            if !left_axis.trim().is_empty() && left_axis == right_axis {
                pairs.insert(normalized_pair(left_code, right_code));
            }
        }
    }

    pairs
}

pub(crate) fn angle_object_codes(angle_codes: &[String]) -> HashSet<String> {
    angle_codes.iter().cloned().collect()
}

pub(crate) fn aspect_code(signal: &BasicSignal) -> Option<&str> {
    signal
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.get("aspect_code"))
        .and_then(|value| value.as_str())
        .or_else(|| signal.signal_key.split(':').nth(3))
}

pub(crate) fn object_pair_from_aspect_signal(signal: &BasicSignal) -> Option<(String, String)> {
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

pub(crate) fn is_marked_structural_axis(signal: &BasicSignal) -> bool {
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
