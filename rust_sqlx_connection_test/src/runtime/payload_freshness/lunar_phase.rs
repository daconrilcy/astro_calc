use std::collections::HashSet;

use crate::domain::{BasicLunarPhaseContext, BasicPayload};

pub(super) fn has_current_lunar_phase_context(payload: &BasicPayload) -> bool {
    let Some(context) = &payload.lunar_phase_context else {
        return false;
    };
    let Some(sun) = payload
        .positions
        .iter()
        .find(|position| position.object_code == "sun")
    else {
        return false;
    };
    let Some(moon) = payload
        .positions
        .iter()
        .find(|position| position.object_code == "moon")
    else {
        return false;
    };

    let signal_keys: HashSet<&str> = payload
        .signals
        .iter()
        .map(|signal| signal.signal_key.as_str())
        .collect();
    let reading_slots: HashSet<&str> = payload
        .reading_plan
        .iter()
        .map(|item| item.slot.as_str())
        .collect();
    let expected_angle = round4_degree(moon.longitude_deg - sun.longitude_deg);

    has_valid_phase_fields(context)
        && degree_matches(
            context.sun_longitude_deg,
            round4_degree(sun.longitude_deg),
            0.0001,
        )
        && degree_matches(
            context.moon_longitude_deg,
            round4_degree(moon.longitude_deg),
            0.0001,
        )
        && degree_matches(context.sun_moon_angle_deg, expected_angle, 0.01)
        && contains_angle(
            context.phase_angle_range_deg[0],
            context.phase_angle_range_deg[1],
            context.sun_moon_angle_deg,
        )
        && degree_matches(
            context.distance_to_exact_phase_deg,
            round4(circular_distance(
                context.sun_moon_angle_deg,
                context.exact_phase_anchor_deg,
            )),
            0.0001,
        )
        && degree_matches(
            context.phase_progress_ratio,
            phase_progress_ratio(
                context.sun_moon_angle_deg,
                context.phase_angle_range_deg[0],
                context.phase_angle_range_deg[1],
            ),
            0.0001,
        )
        && has_current_related_signal_keys(context, &signal_keys)
        && has_current_related_reading_slots(context, &reading_slots)
}

fn has_valid_phase_fields(context: &BasicLunarPhaseContext) -> bool {
    !context.phase_code.trim().is_empty()
        && !context.phase_label.trim().is_empty()
        && valid_cycle_family(context.cycle_family.as_str())
        && context.sun_object_code == "sun"
        && context.moon_object_code == "moon"
        && valid_degree(context.sun_longitude_deg)
        && valid_degree(context.moon_longitude_deg)
        && valid_degree(context.sun_moon_angle_deg)
        && context.phase_angle_range_deg.len() == 2
        && context
            .phase_angle_range_deg
            .iter()
            .all(|value| valid_degree(*value))
        && valid_degree(context.exact_phase_anchor_deg)
        && context.distance_to_exact_phase_deg.is_finite()
        && (0.0..=180.0).contains(&context.distance_to_exact_phase_deg)
        && context.phase_progress_ratio.is_finite()
        && (0.0..=1.0).contains(&context.phase_progress_ratio)
        && degree_matches(
            normalize_360(context.phase_angle_range_deg[1] - context.phase_angle_range_deg[0]),
            45.0,
            0.0001,
        )
        && contains_angle(
            context.phase_angle_range_deg[0],
            context.phase_angle_range_deg[1],
            context.exact_phase_anchor_deg,
        )
        && has_unique_non_empty_strings(&context.semantic_tags)
        && context.semantic_tags.iter().any(|tag| tag == "lunar_phase")
        && context
            .semantic_tags
            .iter()
            .any(|tag| tag == "sun_moon_cycle")
        && context
            .semantic_tags
            .iter()
            .any(|tag| tag == &context.phase_code)
        && context
            .semantic_tags
            .iter()
            .any(|tag| tag == &context.cycle_family)
        && !context.interpretive_hint.trim().is_empty()
}

fn has_current_related_signal_keys(
    context: &BasicLunarPhaseContext,
    signal_keys: &HashSet<&str>,
) -> bool {
    if !has_unique_non_empty_strings(&context.related_signal_keys)
        || context
            .related_signal_keys
            .iter()
            .any(|key| !signal_keys.contains(key.as_str()))
    {
        return false;
    }

    for required_key in ["object_position:sun", "object_position:moon"] {
        if signal_keys.contains(required_key)
            && !context
                .related_signal_keys
                .iter()
                .any(|key| key == required_key)
        {
            return false;
        }
    }

    true
}

fn has_current_related_reading_slots(
    context: &BasicLunarPhaseContext,
    reading_slots: &HashSet<&str>,
) -> bool {
    has_unique_non_empty_strings(&context.related_reading_slots)
        && context
            .related_reading_slots
            .iter()
            .all(|slot| reading_slots.contains(slot.as_str()))
        && (!reading_slots.contains("core_identity")
            || context
                .related_reading_slots
                .iter()
                .any(|slot| slot == "core_identity"))
}

fn has_unique_non_empty_strings(values: &[String]) -> bool {
    let mut seen = HashSet::new();
    values
        .iter()
        .all(|value| !value.trim().is_empty() && seen.insert(value.as_str()))
}

fn valid_cycle_family(value: &str) -> bool {
    matches!(value, "conjunction" | "waxing" | "opposition" | "waning")
}

fn valid_degree(value: f64) -> bool {
    value.is_finite() && (0.0..360.0).contains(&value)
}

fn contains_angle(range_start_deg: f64, range_end_deg: f64, angle: f64) -> bool {
    if range_start_deg <= range_end_deg {
        angle >= range_start_deg && angle < range_end_deg
    } else {
        angle >= range_start_deg || angle < range_end_deg
    }
}

fn phase_progress_ratio(angle: f64, range_start_deg: f64, range_end_deg: f64) -> f64 {
    round4(
        (normalize_360(angle - range_start_deg) / normalize_360(range_end_deg - range_start_deg))
            .clamp(0.0, 1.0),
    )
}

fn circular_distance(left: f64, right: f64) -> f64 {
    let delta = normalize_360(left - right);
    delta.min(360.0 - delta)
}

fn normalize_360(value: f64) -> f64 {
    value.rem_euclid(360.0)
}

fn degree_matches(left: f64, right: f64, tolerance: f64) -> bool {
    (left - right).abs() <= tolerance
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn round4_degree(value: f64) -> f64 {
    normalize_360(round4(normalize_360(value)))
}
