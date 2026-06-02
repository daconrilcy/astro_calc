use serde_json::json;

use crate::domain::{AspectFact, ObjectPositionFact};
use crate::facts::normalize_degrees;
use crate::models::AspectDefinition;

const DEFAULT_MAJOR_ORB_DEG: f64 = 8.0;

pub fn detect_aspects(
    positions: &[ObjectPositionFact],
    aspect_definitions: &[AspectDefinition],
) -> Vec<AspectFact> {
    let major_aspects: Vec<&AspectDefinition> = aspect_definitions
        .iter()
        .filter(|aspect| aspect.angle <= 180.0)
        .collect();
    let mut facts = Vec::new();

    for left_index in 0..positions.len() {
        for right_index in (left_index + 1)..positions.len() {
            let left = &positions[left_index];
            let right = &positions[right_index];
            let separation = shortest_distance(left.longitude_deg, right.longitude_deg);

            for aspect in &major_aspects {
                let orb = (separation - aspect.angle).abs();
                if orb > DEFAULT_MAJOR_ORB_DEG {
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
                    aspect_family: "major".to_string(),
                    orb_deg: round4(orb),
                    phase_state: phase_state(orb, is_applying).to_string(),
                    is_applying,
                    is_exact: orb <= 0.1,
                    strength_score: Some(round4((1.0 - (orb / DEFAULT_MAJOR_ORB_DEG)).max(0.0))),
                    primary_valence: None,
                    intensity_modifier: None,
                    secondary_effect: None,
                    valence_family: None,
                    valence_is_tonal: None,
                    valence_is_intensity_modifier: None,
                    valence_writing_guidance: None,
                    calculation_notes_json: Some(json!({
                        "aspect_code": aspect.code,
                        "aspect_name": aspect.name,
                        "exact_angle_deg": aspect.angle,
                        "separation_deg": round4(separation),
                        "orb_limit_deg": DEFAULT_MAJOR_ORB_DEG
                    })),
                });
            }
        }
    }

    facts
}

fn shortest_distance(left: f64, right: f64) -> f64 {
    let diff = (normalize_degrees(left) - normalize_degrees(right)).abs();
    diff.min(360.0 - diff)
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
    let next_separation = shortest_distance(next_left, next_right);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn position(id: i32, longitude_deg: f64, speed: f64) -> ObjectPositionFact {
        ObjectPositionFact {
            chart_object_id: id,
            object_code: format!("object_{id}"),
            object_name: format!("Object {id}"),
            zodiacal_reference_system_id: 1,
            coordinate_reference_system_id: 1,
            sign_id: 1,
            sign_code: "aries".to_string(),
            sign_name: "Aries".to_string(),
            house_id: None,
            house_number: None,
            house_name: None,
            motion_state_id: None,
            horizon_position_id: None,
            longitude_deg,
            latitude_deg: None,
            apparent_speed_deg_per_day: Some(speed),
            altitude_deg: None,
            is_visible: None,
            facts_json: None,
        }
    }

    #[test]
    fn aspect_phase_uses_relative_speed() {
        let aspects = vec![AspectDefinition {
            id: 1,
            code: "conjunction".to_string(),
            name: "Conjunction".to_string(),
            angle: 0.0,
        }];

        let applying = detect_aspects(&[position(1, 0.0, 1.0), position(2, 2.0, 0.0)], &aspects);
        assert_eq!(applying[0].phase_state, "applying");
        assert!(applying[0].is_applying);

        let separating = detect_aspects(&[position(1, 0.0, -1.0), position(2, 2.0, 0.0)], &aspects);
        assert_eq!(separating[0].phase_state, "separating");
        assert!(!separating[0].is_applying);
    }
}
