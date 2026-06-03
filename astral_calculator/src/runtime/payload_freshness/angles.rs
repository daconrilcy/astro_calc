use std::collections::{HashMap, HashSet};

use crate::domain::{BasicPayload, BasicSignal};

pub(super) fn has_current_angles(payload: &BasicPayload) -> bool {
    let angles_by_code: HashMap<&str, &crate::domain::BasicAngleFact> = payload
        .angles
        .iter()
        .map(|angle| (angle.angle_code.as_str(), angle))
        .collect();

    angles_by_code.len() == 4
        && canonical_angle_is_valid(&angles_by_code, "ascendant", "descendant", "horizontal")
        && canonical_angle_is_valid(&angles_by_code, "descendant", "ascendant", "horizontal")
        && canonical_angle_is_valid(&angles_by_code, "mc", "ic", "vertical")
        && canonical_angle_is_valid(&angles_by_code, "ic", "mc", "vertical")
        && payload.angles.iter().all(|angle| {
            !angle.angle_code.trim().is_empty()
                && !angle.angle_name.trim().is_empty()
                && !angle.axis.trim().is_empty()
                && !angle.opposite_angle_code.trim().is_empty()
                && !angle.sign_code.trim().is_empty()
                && !angle.sign_name.trim().is_empty()
                && (1..=12).contains(&angle.house_number)
                && angle.longitude_deg >= 0.0
                && angle.longitude_deg < 360.0
        })
        && payload.signals.iter().any(|signal| {
            signal.signal_key.starts_with("angle:ascendant:sign:")
                && signal
                    .evidence
                    .as_ref()
                    .and_then(|evidence| evidence.get("fact_type"))
                    .and_then(|value| value.as_str())
                    == Some("chart_angle")
        })
}

pub(super) fn has_current_angle_evidence(payload: &BasicPayload, signal: &BasicSignal) -> bool {
    if !signal.signal_key.starts_with("angle:") {
        return true;
    }

    let Some(evidence) = signal.evidence.as_ref() else {
        return false;
    };
    if evidence.get("fact_type").and_then(|value| value.as_str()) != Some("chart_angle") {
        return false;
    }

    let Some(angle_code) = evidence.get("angle_code").and_then(|value| value.as_str()) else {
        return false;
    };
    let Some(expected_opposite) = payload
        .angles
        .iter()
        .find(|angle| angle.angle_code == angle_code)
        .map(|angle| angle.opposite_angle_code.as_str())
    else {
        return false;
    };

    if evidence
        .get("opposite_angle_code")
        .and_then(|value| value.as_str())
        .is_none_or(|code| code.trim().is_empty())
    {
        return false;
    }

    evidence
        .get("opposite_angle_object_code")
        .and_then(|value| value.as_str())
        == Some(expected_opposite)
}

pub(super) fn structural_axis_pairs_from_payload(
    payload: &BasicPayload,
) -> HashSet<(String, String)> {
    let angle_positions = payload
        .angles
        .iter()
        .map(|angle| (angle.axis.clone(), angle.angle_code.clone()))
        .collect::<Vec<_>>();
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

pub(super) fn angle_object_codes_from_payload(payload: &BasicPayload) -> HashSet<String> {
    payload
        .angles
        .iter()
        .map(|angle| angle.angle_code.clone())
        .collect()
}

pub(super) fn normalized_pair(left: &str, right: &str) -> (String, String) {
    if left <= right {
        (left.to_string(), right.to_string())
    } else {
        (right.to_string(), left.to_string())
    }
}

fn canonical_angle_is_valid(
    angles_by_code: &HashMap<&str, &crate::domain::BasicAngleFact>,
    angle_code: &str,
    opposite_angle_code: &str,
    axis: &str,
) -> bool {
    angles_by_code
        .get(angle_code)
        .is_some_and(|angle| angle.opposite_angle_code == opposite_angle_code && angle.axis == axis)
}
