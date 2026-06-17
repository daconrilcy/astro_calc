use serde_json::json;

use crate::domain::{AspectFact, ObjectPositionFact};
use crate::facts::shortest_angular_distance;
use crate::infra::db::models::AspectDefinition;

pub fn canonical_aspect_orb_deg(aspect: &AspectDefinition) -> Option<f64> {
    let max = aspect.max_default_orb_deg;
    aspect
        .default_orb_deg
        .filter(|orb| orb.is_finite() && *orb > 0.0 && max.is_finite() && max > 0.0 && *orb <= max)
}

pub fn detect_aspects(
    positions: &[ObjectPositionFact],
    aspect_definitions: &[AspectDefinition],
) -> Vec<AspectFact> {
    let aspect_defs: Vec<&AspectDefinition> = aspect_definitions.iter().collect();
    let mut facts = Vec::new();

    for left_index in 0..positions.len() {
        for right_index in (left_index + 1)..positions.len() {
            let left = &positions[left_index];
            let right = &positions[right_index];
            let separation = shortest_angular_distance(left.longitude_deg, right.longitude_deg);

            for aspect in &aspect_defs {
                if is_structural_axis_aspect(left, right, aspect) {
                    continue;
                }

                let Some(orb_limit) = canonical_aspect_orb_deg(aspect) else {
                    continue;
                };
                let orb = (separation - aspect.angle).abs();
                if orb > orb_limit {
                    continue;
                }

                let (source, target) = canonical_pair(left, right);
                let is_applying = is_applying(left, right, aspect.angle, orb);
                facts.push(AspectFact {
                    source_chart_object_id: source.chart_object_id,
                    source_object_code: source.object_code.clone(),
                    source_object_name: source.object_name.clone(),
                    target_chart_object_id: target.chart_object_id,
                    target_object_code: target.object_code.clone(),
                    target_object_name: target.object_name.clone(),
                    aspect_id: aspect.id,
                    aspect_code: aspect.code.clone(),
                    aspect_name: aspect.name.clone(),
                    aspect_family: aspect.family.clone(),
                    orb_deg: round4(orb),
                    phase_state: phase_state(orb, is_applying).to_string(),
                    is_applying,
                    is_exact: orb <= 0.1,
                    strength_score: Some(round4((1.0 - (orb / orb_limit)).max(0.0))),
                    primary_valence: None,
                    intensity_modifier: None,
                    secondary_effect: None,
                    valence_family: None,
                    valence_is_tonal: None,
                    valence_is_intensity_modifier: None,
                    calculation_notes_json: Some(json!({
                        "aspect_code": aspect.code,
                        "aspect_name": aspect.name,
                        "exact_angle_deg": aspect.angle,
                        "separation_deg": round4(separation),
                        "orb_limit_deg": orb_limit
                    })),
                });
            }
        }
    }

    facts
}

fn canonical_pair<'a>(
    left: &'a ObjectPositionFact,
    right: &'a ObjectPositionFact,
) -> (&'a ObjectPositionFact, &'a ObjectPositionFact) {
    if left.chart_object_id <= right.chart_object_id {
        (left, right)
    } else {
        (right, left)
    }
}

fn is_structural_axis_aspect(
    left: &ObjectPositionFact,
    right: &ObjectPositionFact,
    aspect: &AspectDefinition,
) -> bool {
    if aspect.code != "opposition" {
        return false;
    }

    let Some(left_opposite_code) = angle_context_str(left, "opposite_angle_code") else {
        return false;
    };
    let Some(right_opposite_code) = angle_context_str(right, "opposite_angle_code") else {
        return false;
    };

    left_opposite_code == angle_identity_code(right)
        && right_opposite_code == angle_identity_code(left)
        && angle_context_str(left, "axis").is_some()
        && angle_context_str(left, "axis") == angle_context_str(right, "axis")
}

fn angle_identity_code(position: &ObjectPositionFact) -> &str {
    angle_context_str(position, "angle_point_code").unwrap_or(position.object_code.as_str())
}

fn angle_context_str<'a>(position: &'a ObjectPositionFact, key: &str) -> Option<&'a str> {
    position
        .facts_json
        .as_ref()
        .and_then(|facts| facts.get("angle_context"))
        .and_then(|context| context.get(key))
        .and_then(|value| value.as_str())
}

fn is_applying(
    left: &ObjectPositionFact,
    right: &ObjectPositionFact,
    aspect_angle: f64,
    current_orb: f64,
) -> bool {
    let Some(left_speed) = left.apparent_speed_deg_per_day else {
        return false;
    };
    let Some(right_speed) = right.apparent_speed_deg_per_day else {
        return false;
    };

    let next_left = left.longitude_deg + left_speed;
    let next_right = right.longitude_deg + right_speed;
    let next_separation = shortest_angular_distance(next_left, next_right);
    let next_orb = (next_separation - aspect_angle).abs();
    next_orb < current_orb
}

fn phase_state(orb: f64, is_applying: bool) -> &'static str {
    if orb <= 0.1 {
        "exact"
    } else if is_applying {
        "applying"
    } else {
        "separating"
    }
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}
