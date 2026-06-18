//! Module astral_calculator\src\features\natal\payload\rules\angles.rs du moteur astral_calculator.

use std::collections::{HashMap, HashSet};

use crate::domain::{BasicAngleFact, BasicPayload, BasicSignal, ObjectPositionFact};
use crate::features::natal::payload::shared::aspect::{
    angle_object_codes, aspect_code, is_marked_structural_axis, object_pair_from_aspect_signal,
    structural_axis_pairs,
};
use crate::features::natal::payload::shared::text::has_text;

pub(crate) fn build_payload_angles<F>(
    positions: &[ObjectPositionFact],
    position_context: F,
) -> Vec<BasicAngleFact>
where
    F: Fn(&ObjectPositionFact, &str) -> Option<serde_json::Value>,
{
    let angle_object_codes: HashMap<String, String> = positions
        .iter()
        .filter_map(|position| {
            position_context(position, "angle_context")
                .and_then(|context| {
                    context
                        .get("angle_point_code")
                        .and_then(|value| value.as_str())
                        .map(ToString::to_string)
                })
                .map(|angle_point_code| (angle_point_code, position.object_code.clone()))
        })
        .collect();

    let mut angles: Vec<_> = positions
        .iter()
        .filter_map(|position| {
            let angle_context = position_context(position, "angle_context")?;
            let opposite_angle_code = angle_context
                .get("opposite_angle_code")
                .and_then(|value| value.as_str())
                .and_then(|code| angle_object_codes.get(code).map(String::as_str))
                .or_else(|| {
                    angle_context
                        .get("opposite_angle_code")
                        .and_then(|value| value.as_str())
                })
                .unwrap_or_default()
                .to_string();

            Some(BasicAngleFact {
                angle_code: position.object_code.clone(),
                angle_name: angle_context
                    .get("full_name")
                    .and_then(|value| value.as_str())
                    .unwrap_or(&position.object_name)
                    .to_string(),
                axis: angle_context
                    .get("axis")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string(),
                opposite_angle_code,
                longitude_deg: position.longitude_deg,
                sign_id: position.sign_id,
                sign_code: position.sign_code.clone(),
                sign_name: position.sign_name.clone(),
                house_id: position.house_id,
                house_number: angle_context
                    .get("associated_house_number")
                    .and_then(|value| value.as_i64())
                    .and_then(|value| i32::try_from(value).ok())
                    .or(position.house_number)
                    .unwrap_or_default(),
                house_name: position.house_name.clone(),
            })
        })
        .collect();
    angles.sort_by_key(|angle| {
        positions
            .iter()
            .find(|position| position.object_code == angle.angle_code)
            .and_then(|position| position_context(position, "angle_context"))
            .and_then(|context| {
                context
                    .get("chart_object_sort_order")
                    .and_then(|value| value.as_i64())
                    .and_then(|value| i32::try_from(value).ok())
            })
            .unwrap_or(i32::MAX)
    });
    angles
}

pub(crate) fn structural_axis_pairs_from_positions<F>(
    positions: &[ObjectPositionFact],
    position_context: F,
) -> HashSet<(String, String)>
where
    F: Fn(&ObjectPositionFact, &str) -> Option<serde_json::Value>,
{
    let angle_positions: Vec<_> = positions
        .iter()
        .filter_map(|position| {
            position_context(position, "angle_context")
                .and_then(|context| {
                    context
                        .get("axis")
                        .and_then(|value| value.as_str())
                        .map(ToString::to_string)
                })
                .map(|axis| (axis, position.object_code.clone()))
        })
        .collect();

    structural_axis_pairs(&angle_positions)
}

pub(crate) fn angle_object_codes_from_positions<F>(
    positions: &[ObjectPositionFact],
    position_context: F,
) -> HashSet<String>
where
    F: Fn(&ObjectPositionFact, &str) -> Option<serde_json::Value>,
{
    let angle_codes = positions
        .iter()
        .filter(|position| position_context(position, "angle_context").is_some())
        .map(|position| position.object_code.clone())
        .collect::<Vec<_>>();
    angle_object_codes(&angle_codes)
}

pub(crate) fn structural_axis_pairs_from_payload(
    payload: &BasicPayload,
) -> HashSet<(String, String)> {
    let angle_positions = payload
        .angles
        .iter()
        .map(|angle| (angle.axis.clone(), angle.angle_code.clone()))
        .collect::<Vec<_>>();
    structural_axis_pairs(&angle_positions)
}

pub(crate) fn angle_object_codes_from_payload(payload: &BasicPayload) -> HashSet<String> {
    let angle_codes = payload
        .angles
        .iter()
        .map(|angle| angle.angle_code.clone())
        .collect::<Vec<_>>();
    angle_object_codes(&angle_codes)
}

pub(crate) fn is_structural_axis_aspect_signal(
    signal: &BasicSignal,
    structural_axis_pairs: &HashSet<(String, String)>,
) -> bool {
    signal.signal_key.starts_with("aspect:")
        && (is_marked_structural_axis(signal)
            || (aspect_code(signal) == Some("opposition")
                && object_pair_from_aspect_signal(signal)
                    .is_some_and(|pair| structural_axis_pairs.contains(&pair))))
}

pub(crate) fn is_angle_to_angle_aspect_signal(
    signal: &BasicSignal,
    angle_object_codes: &HashSet<String>,
) -> bool {
    signal.signal_key.starts_with("aspect:")
        && object_pair_from_aspect_signal(signal).is_some_and(|(source, target)| {
            angle_object_codes.contains(&source) && angle_object_codes.contains(&target)
        })
}

pub(crate) fn has_current_angles(payload: &BasicPayload) -> bool {
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

pub(crate) fn has_current_angle_evidence(payload: &BasicPayload, signal: &BasicSignal) -> bool {
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

/// Fonction canonical_angle_is_valid.
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
