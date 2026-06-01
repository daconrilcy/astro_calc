use serde_json::json;

use crate::domain::{CalculatedChartFacts, InterpretationSignalDraft};

pub fn aggregate_basic_signals(facts: &CalculatedChartFacts) -> Vec<InterpretationSignalDraft> {
    let mut signals = Vec::new();

    for position in &facts.positions {
        signals.push(InterpretationSignalDraft {
            signal_key: format!("object_position:{}", position.object_code),
            signal_type_id: None,
            theme_code: Some("object_position".to_string()),
            title: format!("{} position", position.object_name),
            summary: Some(format!(
                "{} is in sign {}{}.",
                position.object_name,
                position.sign_id,
                position
                    .house_id
                    .map(|house_id| format!(" and house {house_id}"))
                    .unwrap_or_default()
            )),
            priority_score: position_priority(&position.object_code),
            confidence_score: Some(0.95),
            suppression_state: "active".to_string(),
            payload_json: Some(json!({
                "chart_object_id": position.chart_object_id,
                "object_code": position.object_code,
                "sign_id": position.sign_id,
                "house_id": position.house_id,
                "longitude_deg": position.longitude_deg
            })),
        });
    }

    for aspect in &facts.aspects {
        signals.push(InterpretationSignalDraft {
            signal_key: format!(
                "aspect:{}:{}:{}",
                aspect.source_chart_object_id, aspect.target_chart_object_id, aspect.aspect_id
            ),
            signal_type_id: None,
            theme_code: Some("aspect".to_string()),
            title: "Active aspect".to_string(),
            summary: Some(format!(
                "Objects {} and {} form aspect {} with {:.2} degrees of orb.",
                aspect.source_chart_object_id,
                aspect.target_chart_object_id,
                aspect.aspect_id,
                aspect.orb_deg
            )),
            priority_score: aspect.strength_score.unwrap_or(0.5) * 80.0,
            confidence_score: Some(0.85),
            suppression_state: "active".to_string(),
            payload_json: aspect.calculation_notes_json.clone(),
        });
    }

    signals.sort_by(|left, right| {
        right
            .priority_score
            .partial_cmp(&left.priority_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    signals
}

fn position_priority(object_code: &str) -> f64 {
    match object_code {
        "sun" | "moon" => 100.0,
        "mercury" | "venus" | "mars" => 85.0,
        "jupiter" | "saturn" => 75.0,
        _ => 60.0,
    }
}
