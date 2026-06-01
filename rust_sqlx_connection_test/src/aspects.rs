use serde_json::json;

use crate::domain::{AspectDefinition, AspectFact, ObjectPositionFact};
use crate::facts::normalize_degrees;

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

                let (source_id, target_id) =
                    canonical_pair(left.chart_object_id, right.chart_object_id);
                facts.push(AspectFact {
                    source_chart_object_id: source_id,
                    target_chart_object_id: target_id,
                    aspect_id: aspect.id,
                    orb_deg: round4(orb),
                    phase_state: phase_state(orb).to_string(),
                    is_applying: false,
                    is_exact: orb <= 0.1,
                    strength_score: Some(round4((1.0 - (orb / DEFAULT_MAJOR_ORB_DEG)).max(0.0))),
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

fn canonical_pair(left: i32, right: i32) -> (i32, i32) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

fn phase_state(orb: f64) -> &'static str {
    if orb <= 0.1 {
        "exact"
    } else {
        "applying"
    }
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}
