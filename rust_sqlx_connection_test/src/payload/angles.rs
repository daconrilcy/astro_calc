use crate::domain::{BasicAngleFact, ObjectPositionFact};
use std::collections::{HashMap, HashSet};

use super::json::position_context;
use super::signal_filters::normalized_pair;
pub(super) fn build_payload_angles(positions: &[ObjectPositionFact]) -> Vec<BasicAngleFact> {
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

pub(super) fn structural_axis_pairs_from_positions(
    positions: &[ObjectPositionFact],
) -> HashSet<(String, String)> {
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

    structural_axis_pairs(angle_positions)
}

pub(super) fn angle_object_codes_from_positions(
    positions: &[ObjectPositionFact],
) -> HashSet<String> {
    positions
        .iter()
        .filter(|position| position_context(position, "angle_context").is_some())
        .map(|position| position.object_code.clone())
        .collect()
}

fn structural_axis_pairs(angle_positions: Vec<(String, String)>) -> HashSet<(String, String)> {
    let mut pairs = HashSet::new();

    for left_index in 0..angle_positions.len() {
        for right_index in (left_index + 1)..angle_positions.len() {
            let (left_axis, left_code) = &angle_positions[left_index];
            let (right_axis, right_code) = &angle_positions[right_index];
            if left_axis == right_axis {
                pairs.insert(normalized_pair(left_code, right_code));
            }
        }
    }

    pairs
}
