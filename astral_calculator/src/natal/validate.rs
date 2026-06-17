use std::collections::{HashMap, HashSet};

use crate::natal::catalog::accidental_polarity_bands_are_valid;
use crate::domain::{
    AccidentalConditionTrigger, AccidentalDignityConditionReference, AccidentalPolarityBand,
    AccidentalScoringParams, CalculationReferenceData, HouseAxisReference, HouseReference,
    LunarPhaseReference, ObjectSectAffinityReference,
};
use crate::infra::db::models::{AspectDefinition, ChartObject};

use crate::shared::error::RuntimeError;

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

pub fn validate_aspect_definitions(
    aspects: &[AspectDefinition],
    product_default_major_orb_deg: f64,
    expected_major_aspect_count: usize,
    max_default_orb_deg: f64,
) -> Result<(), RuntimeError> {
    if aspects.len() != expected_major_aspect_count {
        return Err(RuntimeError::Ephemeris(format!(
            "expected {expected_major_aspect_count} major aspect definitions from astral_aspect_families, found {}",
            aspects.len()
        )));
    }
    if !max_default_orb_deg.is_finite() || max_default_orb_deg <= 0.0 {
        return Err(RuntimeError::Ephemeris(format!(
            "invalid max_default_orb_deg for major aspect family: {max_default_orb_deg}"
        )));
    }
    if product_default_major_orb_deg <= 0.0 || product_default_major_orb_deg > max_default_orb_deg {
        return Err(RuntimeError::Ephemeris(
            "invalid product default_major_orb_deg (sanity check only; detection uses astral_aspects.default_orb_deg)".to_string(),
        ));
    }

    let mut seen_codes = HashSet::new();
    let mut seen_ids = HashSet::new();
    for aspect in aspects {
        if aspect.family != "major" {
            return Err(RuntimeError::Ephemeris(format!(
                "aspect_definitions must only load family = 'major', found {}",
                aspect.family
            )));
        }
        if !seen_ids.insert(aspect.id)
            || !seen_codes.insert(aspect.code.as_str())
            || aspect.code.trim().is_empty()
            || aspect.name.trim().is_empty()
        {
            return Err(RuntimeError::Ephemeris(format!(
                "invalid major aspect definition {}",
                aspect.code
            )));
        }

        if !major_aspect_angle_is_valid(aspect.angle) {
            return Err(RuntimeError::Ephemeris(format!(
                "invalid angle for major aspect {}",
                aspect.code
            )));
        }

        let Some(orb) = aspect.default_orb_deg else {
            return Err(RuntimeError::Ephemeris(format!(
                "missing default_orb_deg for major aspect {}",
                aspect.code
            )));
        };
        if (aspect.max_default_orb_deg - max_default_orb_deg).abs() > 0.0001 {
            return Err(RuntimeError::Ephemeris(format!(
                "inconsistent max_default_orb_deg for major aspect {}",
                aspect.code
            )));
        }
        if !orb.is_finite() || orb <= 0.0 || orb > max_default_orb_deg {
            return Err(RuntimeError::Ephemeris(format!(
                "invalid default_orb_deg for major aspect {}",
                aspect.code
            )));
        }
    }

    Ok(())
}

fn major_aspect_angle_is_valid(angle: f64) -> bool {
    angle.is_finite() && (0.0..=180.0).contains(&angle)
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
    houses: &[HouseReference],
) -> Result<(), RuntimeError> {
    if references.len() != 6 {
        return Err(RuntimeError::Ephemeris(format!(
            "expected 6 house axis references, found {}",
            references.len()
        )));
    }

    let theme_by_house: HashMap<i32, &str> = houses
        .iter()
        .map(|house| (house.number, house.theme_code.as_str()))
        .collect();
    if theme_by_house.len() != 12 {
        return Err(RuntimeError::Ephemeris(format!(
            "expected 12 house theme references, found {}",
            theme_by_house.len()
        )));
    }

    let mut seen_axis_codes = HashSet::new();
    let mut seen_house_pairs = HashSet::new();
    for reference in references {
        let house_pair = (reference.house_a_number, reference.house_b_number);
        let theme_a = theme_by_house.get(&reference.house_a_number).copied();
        let theme_b = theme_by_house.get(&reference.house_b_number).copied();
        if !seen_axis_codes.insert(reference.axis_code.as_str())
            || !seen_house_pairs.insert(house_pair)
            || reference.house_a_number >= reference.house_b_number
            || reference.house_b_number - reference.house_a_number != 6
            || !(1..=12).contains(&reference.house_a_number)
            || !(1..=12).contains(&reference.house_b_number)
            || reference.theme_a_code.trim().is_empty()
            || reference.theme_b_code.trim().is_empty()
            || reference.label.trim().is_empty()
            || reference.description.trim().is_empty()
            || theme_a != Some(reference.theme_a_code.as_str())
            || theme_b != Some(reference.theme_b_code.as_str())
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

pub fn validate_accidental_dignity_condition_references(
    references: &[AccidentalDignityConditionReference],
    triggers: &[AccidentalConditionTrigger],
) -> Result<(), RuntimeError> {
    if references.is_empty() {
        return Err(RuntimeError::Ephemeris(
            "expected accidental dignity condition references".to_string(),
        ));
    }

    let mut seen_codes = HashSet::new();
    let mut families = HashSet::new();
    let mut polarities = HashSet::new();
    for reference in references {
        if !seen_codes.insert(reference.condition_code.as_str())
            || reference.condition_code.trim().is_empty()
            || reference.label.trim().is_empty()
            || reference.description.trim().is_empty()
            || reference.condition_family.trim().is_empty()
            || reference.polarity.trim().is_empty()
            || !(0.0..=1.0).contains(&reference.strength_score)
            || !(-1.0..=1.0).contains(&reference.score_delta)
        {
            return Err(RuntimeError::Ephemeris(format!(
                "invalid accidental dignity condition reference {}",
                reference.condition_code
            )));
        }
        families.insert(reference.condition_family.as_str());
        polarities.insert(reference.polarity.as_str());
    }

    for trigger in triggers {
        if trigger.condition_code.trim().is_empty()
            || trigger.trigger_family.trim().is_empty()
            || !seen_codes.contains(trigger.condition_code.as_str())
        {
            return Err(RuntimeError::Ephemeris(format!(
                "invalid accidental condition trigger for {}",
                trigger.condition_code
            )));
        }
    }

    let _ = (families, polarities);
    Ok(())
}

pub fn validate_accidental_condition_triggers(
    triggers: &[AccidentalConditionTrigger],
) -> Result<(), RuntimeError> {
    if triggers.is_empty() {
        return Err(RuntimeError::Ephemeris(
            "expected accidental condition triggers".to_string(),
        ));
    }

    let mut seen = HashSet::new();
    for trigger in triggers {
        let key = (
            trigger.trigger_family.as_str(),
            trigger.source_code.as_deref(),
            trigger.angle_object_code.as_deref(),
            trigger.condition_code.as_str(),
        );
        if !seen.insert(key) || trigger.trigger_family.trim().is_empty() {
            return Err(RuntimeError::Ephemeris(
                "invalid accidental condition trigger row".to_string(),
            ));
        }
    }

    Ok(())
}

pub fn validate_accidental_scoring_params(
    params: &AccidentalScoringParams,
) -> Result<(), RuntimeError> {
    if params.code.trim().is_empty()
        || params.overall_score_min > params.overall_score_baseline
        || params.overall_score_baseline > params.overall_score_max
        || params.angle_proximity_max_orb_deg <= 0.0
    {
        return Err(RuntimeError::Ephemeris(
            "invalid accidental scoring params".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_accidental_polarity_bands(
    bands: &[AccidentalPolarityBand],
) -> Result<(), RuntimeError> {
    if !accidental_polarity_bands_are_valid(bands) {
        return Err(RuntimeError::Ephemeris(
            "invalid accidental overall polarity bands".to_string(),
        ));
    }

    Ok(())
}

pub fn validate_object_sect_affinity_references(
    references: &[ObjectSectAffinityReference],
) -> Result<(), RuntimeError> {
    if references.is_empty() {
        return Err(RuntimeError::Ephemeris(
            "expected object sect affinity references".to_string(),
        ));
    }

    let mut seen_objects = HashSet::new();
    let mut affinity_codes = HashSet::new();
    for reference in references {
        if !seen_objects.insert(reference.object_code.as_str())
            || reference.object_code.trim().is_empty()
            || reference.description.trim().is_empty()
            || reference.sect_affinity_code.trim().is_empty()
            || reference.is_variable != (reference.sect_affinity_code == "variable")
        {
            return Err(RuntimeError::Ephemeris(format!(
                "invalid object sect affinity reference {}",
                reference.object_code
            )));
        }
        affinity_codes.insert(reference.sect_affinity_code.as_str());
    }

    let _ = affinity_codes;
    Ok(())
}
