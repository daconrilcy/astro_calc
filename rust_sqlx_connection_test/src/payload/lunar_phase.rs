use std::collections::HashSet;

use crate::domain::{
    BasicLunarPhaseContext, BasicReadingPlanItem, BasicSignal, LunarPhaseReference,
    ObjectPositionFact,
};

pub(super) fn build_lunar_phase_context(
    references: &[LunarPhaseReference],
    positions: &[ObjectPositionFact],
    signals: &[BasicSignal],
    reading_plan: &[BasicReadingPlanItem],
) -> Option<BasicLunarPhaseContext> {
    let sun = positions
        .iter()
        .find(|position| position.object_code == "sun")?;
    let moon = positions
        .iter()
        .find(|position| position.object_code == "moon")?;
    let angle = round4_degree(moon.longitude_deg - sun.longitude_deg);
    let reference = references
        .iter()
        .find(|reference| contains_angle(reference, angle))?;

    let signal_keys: HashSet<&str> = signals
        .iter()
        .map(|signal| signal.signal_key.as_str())
        .collect();
    let mut related_signal_keys = Vec::new();
    push_if_active(
        &mut related_signal_keys,
        &signal_keys,
        "object_position:sun",
    );
    push_if_active(
        &mut related_signal_keys,
        &signal_keys,
        "object_position:moon",
    );

    let related_reading_slots = if reading_plan.iter().any(|item| item.slot == "core_identity") {
        vec!["core_identity".to_string()]
    } else {
        Vec::new()
    };

    Some(BasicLunarPhaseContext {
        phase_code: reference.phase_code.clone(),
        phase_label: reference.label.clone(),
        cycle_family: reference.cycle_family.clone(),
        sun_object_code: "sun".to_string(),
        moon_object_code: "moon".to_string(),
        sun_longitude_deg: round4_degree(sun.longitude_deg),
        moon_longitude_deg: round4_degree(moon.longitude_deg),
        sun_moon_angle_deg: angle,
        phase_angle_range_deg: vec![
            round4_degree(reference.range_start_deg),
            round4_degree(reference.range_end_deg),
        ],
        exact_phase_anchor_deg: round4_degree(reference.exact_anchor_deg),
        distance_to_exact_phase_deg: round4(circular_distance(angle, reference.exact_anchor_deg)),
        phase_progress_ratio: phase_progress_ratio(angle, reference),
        is_major_lunar_phase: reference.is_major_lunar_phase,
        related_signal_keys,
        related_reading_slots,
        semantic_tags: vec![
            "lunar_phase".to_string(),
            "sun_moon_cycle".to_string(),
            reference.cycle_family.clone(),
            reference.phase_code.clone(),
        ],
        interpretive_hint: interpretive_hint(reference),
    })
}

fn contains_angle(reference: &LunarPhaseReference, angle: f64) -> bool {
    if reference.range_start_deg <= reference.range_end_deg {
        angle >= reference.range_start_deg && angle < reference.range_end_deg
    } else {
        angle >= reference.range_start_deg || angle < reference.range_end_deg
    }
}

fn phase_progress_ratio(angle: f64, reference: &LunarPhaseReference) -> f64 {
    let phase_width = normalize_360(reference.range_end_deg - reference.range_start_deg);
    if phase_width == 0.0 {
        return 0.0;
    }
    round4((normalize_360(angle - reference.range_start_deg) / phase_width).clamp(0.0, 1.0))
}

fn circular_distance(left: f64, right: f64) -> f64 {
    let delta = normalize_360(left - right);
    delta.min(360.0 - delta)
}

fn normalize_360(value: f64) -> f64 {
    value.rem_euclid(360.0)
}

fn push_if_active(target: &mut Vec<String>, signal_keys: &HashSet<&str>, signal_key: &str) {
    if signal_keys.contains(signal_key) {
        target.push(signal_key.to_string());
    }
}

fn interpretive_hint(reference: &LunarPhaseReference) -> String {
    format!(
        "The Sun-Moon cycle is in a {} phase, indicating a structured {} relationship between solar identity and lunar needs.",
        reference.label.to_ascii_lowercase(),
        reference.cycle_family
    )
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn round4_degree(value: f64) -> f64 {
    normalize_360(round4(normalize_360(value)))
}
