use std::collections::HashSet;

use crate::domain::{CalculationReferenceData, HouseAxisReference, LunarPhaseReference};
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

pub fn validate_house_axis_references(
    references: &[HouseAxisReference],
) -> Result<(), RuntimeError> {
    if references.len() != 6 {
        return Err(RuntimeError::Ephemeris(format!(
            "expected 6 house axis references, found {}",
            references.len()
        )));
    }

    let mut seen_axis_codes = HashSet::new();
    let mut seen_house_pairs = HashSet::new();
    for reference in references {
        let Some((expected_houses, expected_themes)) =
            canonical_house_axis(reference.axis_code.as_str())
        else {
            return Err(RuntimeError::Ephemeris(format!(
                "unknown house axis reference {}",
                reference.axis_code
            )));
        };

        let house_pair = (reference.house_a_number, reference.house_b_number);
        if !seen_axis_codes.insert(reference.axis_code.as_str())
            || !seen_house_pairs.insert(house_pair)
            || house_pair != expected_houses
            || (
                reference.theme_a_code.as_str(),
                reference.theme_b_code.as_str(),
            ) != expected_themes
            || reference.label.trim().is_empty()
            || reference.description.trim().is_empty()
        {
            return Err(RuntimeError::Ephemeris(format!(
                "invalid house axis reference {}",
                reference.axis_code
            )));
        }
    }

    Ok(())
}

pub fn validate_lunar_phase_references(
    references: &[LunarPhaseReference],
) -> Result<(), RuntimeError> {
    if references.len() != 8 {
        return Err(RuntimeError::Ephemeris(format!(
            "expected 8 lunar phase references, found {}",
            references.len()
        )));
    }

    let mut seen_phase_codes = HashSet::new();
    for reference in references {
        if !seen_phase_codes.insert(reference.phase_code.as_str())
            || reference.phase_code.trim().is_empty()
            || reference.label.trim().is_empty()
            || reference.description.trim().is_empty()
            || !valid_cycle_family(reference.cycle_family.as_str())
            || !valid_degree(reference.range_start_deg)
            || !valid_degree(reference.range_end_deg)
            || !valid_degree(reference.exact_anchor_deg)
            || !degree_matches(phase_width(reference), 45.0)
            || !contains_angle(
                reference.range_start_deg,
                reference.range_end_deg,
                reference.exact_anchor_deg,
            )
        {
            return Err(RuntimeError::Ephemeris(format!(
                "invalid lunar phase reference {}",
                reference.phase_code
            )));
        }
    }

    if !lunar_phase_ranges_cover_cycle(references) {
        return Err(RuntimeError::Ephemeris(
            "lunar phase references do not cover a continuous 360 degree cycle".to_string(),
        ));
    }

    Ok(())
}

type HouseAxisPair = (i32, i32);
type HouseAxisThemes = (&'static str, &'static str);

fn canonical_house_axis(axis_code: &str) -> Option<(HouseAxisPair, HouseAxisThemes)> {
    match axis_code {
        "self_relationship" => Some(((1, 7), ("identity", "relationships"))),
        "resources_sharing" => Some(((2, 8), ("resources", "shared_resources"))),
        "local_distant" => Some(((3, 9), ("communication", "beliefs"))),
        "private_public" => Some(((4, 10), ("roots", "career"))),
        "creation_collective" => Some(((5, 11), ("creativity", "community"))),
        "control_surrender" => Some(((6, 12), ("work_health", "inner_world"))),
        _ => None,
    }
}

fn degree_matches(left: f64, right: f64) -> bool {
    (left - right).abs() <= 0.0001
}

fn valid_cycle_family(value: &str) -> bool {
    matches!(value, "conjunction" | "waxing" | "opposition" | "waning")
}

fn valid_degree(value: f64) -> bool {
    value.is_finite() && (0.0..360.0).contains(&value)
}

fn phase_width(reference: &LunarPhaseReference) -> f64 {
    normalize_360(reference.range_end_deg - reference.range_start_deg)
}

fn contains_angle(range_start_deg: f64, range_end_deg: f64, angle: f64) -> bool {
    if range_start_deg <= range_end_deg {
        angle >= range_start_deg && angle < range_end_deg
    } else {
        angle >= range_start_deg || angle < range_end_deg
    }
}

fn normalize_360(value: f64) -> f64 {
    value.rem_euclid(360.0)
}

fn lunar_phase_ranges_cover_cycle(references: &[LunarPhaseReference]) -> bool {
    let mut intervals = references
        .iter()
        .map(|reference| {
            let start = reference.range_start_deg;
            let mut end = reference.range_end_deg;
            if end <= start {
                end += 360.0;
            }
            (start, end)
        })
        .collect::<Vec<_>>();
    intervals.sort_by(|left, right| {
        left.0
            .partial_cmp(&right.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    intervals
        .windows(2)
        .all(|window| degree_matches(window[0].1, window[1].0))
        && intervals
            .first()
            .zip(intervals.last())
            .is_some_and(|(first, last)| degree_matches(last.1, first.0 + 360.0))
}
