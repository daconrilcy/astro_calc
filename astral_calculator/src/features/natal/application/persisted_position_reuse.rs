use std::collections::HashSet;

use crate::domain::{CalculationReferenceData, ObjectPositionFact};

pub(super) fn has_reusable_persisted_positions(
    positions: &[ObjectPositionFact],
    references: &CalculationReferenceData,
) -> bool {
    let position_object_ids: HashSet<i32> = positions
        .iter()
        .map(|position| position.chart_object_id)
        .collect();

    references
        .angle_points
        .iter()
        .all(|angle| position_object_ids.contains(&angle.chart_object_id))
        && positions.iter().all(|position| {
            has_reusable_horizon_context(position)
                && (is_angle_position(position)
                    || position
                        .altitude_deg
                        .is_some_and(|altitude| altitude.is_finite()))
        })
}

fn has_reusable_horizon_context(position: &ObjectPositionFact) -> bool {
    position
        .horizon_position_id
        .is_some_and(|horizon_position_id| horizon_position_id > 0)
        && position.is_visible.is_some()
}

fn is_angle_position(position: &ObjectPositionFact) -> bool {
    let context = position.context();
    context
        .as_ref()
        .and_then(|context| context.object_context.as_ref())
        .and_then(|object_context| object_context.role.as_deref())
        == Some("angle")
        || context.and_then(|context| context.angle_context).is_some()
}
