use std::collections::HashSet;

use crate::domain::CalculationReferenceData;
use crate::models::ChartObject;

use super::RuntimeError;

pub fn validate_calculation_references(
    references: &CalculationReferenceData,
) -> Result<(), RuntimeError> {
    if references.signs.len() != 12 {
        return Err(RuntimeError::Ephemeris(format!(
            "expected 12 sign references, found {}",
            references.signs.len()
        )));
    }
    if references.houses.len() != 12 {
        return Err(RuntimeError::Ephemeris(format!(
            "expected 12 house references, found {}",
            references.houses.len()
        )));
    }
    if references.motion_states.is_empty() {
        return Err(RuntimeError::Ephemeris(
            "expected motion state references".to_string(),
        ));
    }
    if references.horizon_positions.len() != 3 {
        return Err(RuntimeError::Ephemeris(format!(
            "expected 3 horizon position references, found {}",
            references.horizon_positions.len()
        )));
    }
    if references.angle_points.len() != 4 {
        return Err(RuntimeError::Ephemeris(format!(
            "expected 4 angle point references, found {}",
            references.angle_points.len()
        )));
    }

    let mut sign_ids = HashSet::new();
    for sign in &references.signs {
        if !sign_ids.insert(sign.id) || sign.code.trim().is_empty() || sign.name.trim().is_empty() {
            return Err(RuntimeError::Ephemeris(
                "invalid sign references: duplicate IDs or empty labels".to_string(),
            ));
        }
    }

    let mut house_ids = HashSet::new();
    let mut house_numbers = HashSet::new();
    for house in &references.houses {
        if !house_ids.insert(house.id)
            || !house_numbers.insert(house.number)
            || !(1..=12).contains(&house.number)
            || house.name.trim().is_empty()
            || house.modality_code.is_none()
            || house.modality_priority_delta.is_none()
        {
            return Err(RuntimeError::Ephemeris(
                "invalid house references: duplicate IDs, invalid numbers, empty labels, or missing modality scoring".to_string(),
            ));
        }
    }

    let mut motion_state_ids = HashSet::new();
    for motion_state in &references.motion_states {
        if !motion_state_ids.insert(motion_state.id)
            || motion_state.code.trim().is_empty()
            || motion_state.label.trim().is_empty()
            || motion_state.motion_family.trim().is_empty()
        {
            return Err(RuntimeError::Ephemeris(
                "invalid motion state references: duplicate IDs or empty labels".to_string(),
            ));
        }
    }

    let mut horizon_position_ids = HashSet::new();
    let mut horizon_position_codes = HashSet::new();
    for horizon_position in &references.horizon_positions {
        if !horizon_position_ids.insert(horizon_position.id)
            || !horizon_position_codes.insert(horizon_position.code.as_str())
            || horizon_position.code.trim().is_empty()
            || horizon_position.label.trim().is_empty()
        {
            return Err(RuntimeError::Ephemeris(
                "invalid horizon position references: duplicate IDs or empty labels".to_string(),
            ));
        }
    }
    for expected_code in ["above_horizon", "below_horizon", "on_horizon"] {
        if !horizon_position_codes.contains(expected_code) {
            return Err(RuntimeError::Ephemeris(format!(
                "missing horizon position reference {expected_code}"
            )));
        }
    }

    let mut angle_ids = HashSet::new();
    let mut angle_object_ids = HashSet::new();
    for angle in &references.angle_points {
        if !angle_ids.insert(angle.id)
            || !angle_object_ids.insert(angle.chart_object_id)
            || angle.code.trim().is_empty()
            || angle.short_label.trim().is_empty()
            || angle.full_name.trim().is_empty()
            || angle.axis.trim().is_empty()
            || !(1..=12).contains(&angle.associated_house)
            || angle.chart_object_code.trim().is_empty()
            || angle.chart_object_name.trim().is_empty()
        {
            return Err(RuntimeError::Ephemeris(
                "invalid angle point references: duplicate IDs, invalid houses, or empty labels"
                    .to_string(),
            ));
        }
    }

    Ok(())
}

pub fn validate_chart_object_signal_profiles(
    chart_objects: &[ChartObject],
) -> Result<(), RuntimeError> {
    if chart_objects.is_empty() {
        return Err(RuntimeError::Ephemeris(
            "expected active chart object references".to_string(),
        ));
    }

    for object in chart_objects {
        let has_base_priority = object
            .position_priority_base
            .is_some_and(|value| (0.0..=100.0).contains(&value));
        let has_source_weight = object.source_weight.is_some_and(|value| value >= 0.0);
        let angle_requires_base = object.role_code.as_deref() == Some("angle");
        let has_angle_base = object
            .angle_priority_base
            .is_some_and(|value| (0.0..=100.0).contains(&value));

        if object.code.trim().is_empty()
            || !has_base_priority
            || !has_source_weight
            || (angle_requires_base && !has_angle_base)
        {
            return Err(RuntimeError::Ephemeris(format!(
                "invalid signal scoring profile for chart object {}",
                object.code
            )));
        }
    }

    Ok(())
}
