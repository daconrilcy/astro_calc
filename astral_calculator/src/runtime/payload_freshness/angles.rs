use std::collections::{HashMap, HashSet};

use crate::domain::{BasicPayload, BasicSignal};
use crate::payload_shared::aspect::{angle_object_codes, structural_axis_pairs};
use crate::payload_shared::text::has_text;

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
            has_text(&angle.angle_code)
                && has_text(&angle.angle_name)
                && has_text(&angle.axis)
                && has_text(&angle.opposite_angle_code)
                && has_text(&angle.sign_code)
                && has_text(&angle.sign_name)
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
    structural_axis_pairs(&angle_positions)
}

pub(super) fn angle_object_codes_from_payload(payload: &BasicPayload) -> HashSet<String> {
    let angle_codes = payload
        .angles
        .iter()
        .map(|angle| angle.angle_code.clone())
        .collect::<Vec<_>>();
    angle_object_codes(&angle_codes)
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
