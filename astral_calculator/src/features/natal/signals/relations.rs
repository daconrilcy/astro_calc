//! Module astral_calculator\src\features\natal\signals\relations.rs du moteur astral_calculator.

use std::collections::{HashMap, HashSet};

use crate::domain::ObjectPositionFact;

use super::context::{placement_context_object, placement_context_str};

pub(super) fn angle_object_codes_from_positions(
    positions: &[ObjectPositionFact],
) -> HashSet<String> {
    positions
        .iter()
        .filter(|position| placement_context_object(position, "angle_context").is_some())
        .map(|position| position.object_code.clone())
        .collect()
}

pub(super) fn angle_point_object_codes_from_positions(
    positions: &[ObjectPositionFact],
) -> HashMap<String, String> {
    positions
        .iter()
        .filter_map(|position| {
            placement_context_str(position, "angle_context", "angle_point_code")
                .map(|code| (code.to_string(), position.object_code.clone()))
        })
        .collect()
}

pub(super) fn structural_axis_pairs_from_positions(
    positions: &[ObjectPositionFact],
) -> HashSet<(String, String)> {
    let angle_positions = positions
        .iter()
        .filter_map(|position| {
            placement_context_str(position, "angle_context", "axis")
                .map(|axis| (axis.to_string(), position.object_code.clone()))
        })
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

pub(super) fn normalized_pair(left: &str, right: &str) -> (String, String) {
    if left <= right {
        (left.to_string(), right.to_string())
    } else {
        (right.to_string(), left.to_string())
    }
}
