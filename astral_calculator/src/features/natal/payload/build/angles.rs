//! Module astral_calculator\src\features\natal\payload\build\angles.rs du moteur astral_calculator.

use crate::domain::{BasicAngleFact, ObjectPositionFact};
use std::collections::HashSet;

use super::json::position_context;
pub(super) fn build_payload_angles(positions: &[ObjectPositionFact]) -> Vec<BasicAngleFact> {
    crate::features::natal::payload::rules::angles::build_payload_angles(
        positions,
        position_context,
    )
}

pub(super) fn structural_axis_pairs_from_positions(
    positions: &[ObjectPositionFact],
) -> HashSet<(String, String)> {
    crate::features::natal::payload::rules::angles::structural_axis_pairs_from_positions(
        positions,
        position_context,
    )
}

pub(super) fn angle_object_codes_from_positions(
    positions: &[ObjectPositionFact],
) -> HashSet<String> {
    crate::features::natal::payload::rules::angles::angle_object_codes_from_positions(
        positions,
        position_context,
    )
}
