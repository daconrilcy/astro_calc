use serde_json::json;

use crate::domain::{CalculatedChartFacts, InterpretationSignalDraft};

const BASIC_MAX_ACTIVE_SIGNALS: usize = 12;
const BASIC_ASPECT_MIN_STRENGTH: f64 = 0.4;

pub fn aggregate_basic_signals(facts: &CalculatedChartFacts) -> Vec<InterpretationSignalDraft> {
    let mut signals = Vec::new();

    for position in &facts.positions {
        let house_suffix = position
            .house_number
            .map(|house_number| format!(", house {house_number}"))
            .unwrap_or_default();
        let summary_house = position
            .house_name
            .as_deref()
            .map(|house_name| format!(" and the {house_name} house"))
            .unwrap_or_default();

        signals.push(InterpretationSignalDraft {
            signal_key: format!("object_position:{}", position.object_code),
            signal_type_id: None,
            theme_code: Some("object_position".to_string()),
            title: format!(
                "{} in {}{}",
                position.object_name, position.sign_name, house_suffix
            ),
            summary: Some(format!(
                "{} is placed in {}{}, emphasizing this chart factor through a concrete, readable placement.",
                position.object_name, position.sign_name, summary_house
            )),
            priority_score: position_priority(&position.object_code),
            confidence_score: Some(0.95),
            suppression_state: "active".to_string(),
            payload_json: Some(json!({
                "evidence": {
                    "fact_type": "object_position",
                    "chart_object_id": position.chart_object_id,
                    "object_code": position.object_code,
                    "object_name": position.object_name,
                    "sign_id": position.sign_id,
                    "sign_code": position.sign_code,
                    "sign_name": position.sign_name,
                    "house_id": position.house_id,
                    "house_number": position.house_number,
                    "house_name": position.house_name,
                    "longitude_deg": position.longitude_deg
                }
            })),
        });
    }

    for aspect in &facts.aspects {
        let strength_score = aspect.strength_score.unwrap_or(0.5);
        let suppression_state = if strength_score >= BASIC_ASPECT_MIN_STRENGTH {
            "active"
        } else {
            "suppressed"
        };
        let aspect_name = aspect.aspect_name.to_lowercase();
        let article = indefinite_article(&aspect_name);

        signals.push(InterpretationSignalDraft {
            signal_key: format!(
                "aspect:{}:{}:{}",
                aspect.source_object_code, aspect.target_object_code, aspect.aspect_code
            ),
            signal_type_id: None,
            theme_code: Some("aspect".to_string()),
            title: format!(
                "{} {} {}",
                aspect.source_object_name, aspect_name, aspect.target_object_name
            ),
            summary: Some(format!(
                "{} and {} form {} {} with {:.2} degrees of orb; the phase is {}.",
                aspect.source_object_name,
                aspect.target_object_name,
                article,
                aspect_name,
                aspect.orb_deg,
                aspect.phase_state
            )),
            priority_score: strength_score * 80.0,
            confidence_score: Some(0.85),
            suppression_state: suppression_state.to_string(),
            payload_json: Some(json!({
                "evidence": {
                    "fact_type": "aspect",
                    "source_chart_object_id": aspect.source_chart_object_id,
                    "source_object_code": aspect.source_object_code,
                    "source_object_name": aspect.source_object_name,
                    "target_chart_object_id": aspect.target_chart_object_id,
                    "target_object_code": aspect.target_object_code,
                    "target_object_name": aspect.target_object_name,
                    "aspect_id": aspect.aspect_id,
                    "aspect_code": aspect.aspect_code,
                    "aspect_name": aspect.aspect_name,
                    "orb_deg": aspect.orb_deg,
                    "phase_state": aspect.phase_state,
                    "is_applying": aspect.is_applying,
                    "is_exact": aspect.is_exact,
                    "strength_score": aspect.strength_score,
                    "calculation_notes": aspect.calculation_notes_json
                }
            })),
        });
    }

    signals.sort_by(|left, right| {
        right
            .priority_score
            .partial_cmp(&left.priority_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    suppress_over_basic_limit(&mut signals);
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

fn suppress_over_basic_limit(signals: &mut [InterpretationSignalDraft]) {
    let mut active_count = 0;
    for signal in signals {
        if signal.suppression_state != "active" {
            continue;
        }

        active_count += 1;
        if active_count > BASIC_MAX_ACTIVE_SIGNALS {
            signal.suppression_state = "suppressed".to_string();
        }
    }
}

fn indefinite_article(phrase: &str) -> &'static str {
    match phrase
        .chars()
        .next()
        .map(|letter| letter.to_ascii_lowercase())
    {
        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
        _ => "a",
    }
}
