use crate::domain::{
    BasicObjectPosition, BasicPayload, BasicSignal, InterpretationSignalRow, NatalChartInput,
    ObjectPositionFact,
};

pub fn build_basic_payload(
    chart_calculation_id: i32,
    input: &NatalChartInput,
    positions: &[ObjectPositionFact],
    signals: &[InterpretationSignalRow],
) -> BasicPayload {
    BasicPayload {
        product_code: input.product_code().to_string(),
        chart_calculation_id,
        reference_version_id: input.reference_version_id,
        subject_label: input.subject_label.clone(),
        birth_datetime_utc: input.birth_datetime_utc,
        positions: positions
            .iter()
            .map(|position| BasicObjectPosition {
                object_code: position.object_code.clone(),
                object_name: position.object_name.clone(),
                longitude_deg: position.longitude_deg,
                sign_id: position.sign_id,
                sign_code: position.sign_code.clone(),
                sign_name: position.sign_name.clone(),
                house_id: position.house_id,
                house_number: position.house_number,
                house_name: position.house_name.clone(),
                motion_state_id: position.motion_state_id,
            })
            .collect(),
        signals: signals
            .iter()
            .take(12)
            .map(|signal| BasicSignal {
                signal_key: signal.signal_key.clone(),
                theme_code: signal.theme_code.clone(),
                title: signal.title.clone(),
                summary: signal.summary.clone(),
                priority_score: signal.priority_score,
                confidence_score: signal.confidence_score,
                interpretive_hint: payload_string(signal, "interpretive_hint"),
                semantic_tags: payload_string_array(signal, "semantic_tags"),
                source_weight: payload_f64(signal, "source_weight"),
                aggregation_group: payload_string(signal, "aggregation_group"),
                writing_guidance: payload_string(signal, "writing_guidance"),
                evidence: payload_value(signal, "evidence"),
            })
            .collect(),
    }
}

fn payload_value(signal: &InterpretationSignalRow, key: &str) -> Option<serde_json::Value> {
    signal
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get(key).cloned())
}

fn payload_string(signal: &InterpretationSignalRow, key: &str) -> Option<String> {
    signal
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get(key))
        .and_then(|value| value.as_str())
        .map(ToString::to_string)
}

fn payload_f64(signal: &InterpretationSignalRow, key: &str) -> Option<f64> {
    signal
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get(key))
        .and_then(|value| value.as_f64())
}

fn payload_string_array(signal: &InterpretationSignalRow, key: &str) -> Vec<String> {
    signal
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get(key))
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use serde_json::json;

    use super::*;

    fn input() -> NatalChartInput {
        NatalChartInput {
            subject_label: None,
            birth_datetime_utc: Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap(),
            latitude_deg: 48.8566,
            longitude_deg: 2.3522,
            altitude_m: None,
            reference_version_id: 1,
            calculation_profile_id: None,
            zodiacal_reference_system_id: 1,
            coordinate_reference_system_id: 1,
            house_system_id: 1,
            product_code: Some("basic".to_string()),
            language_id: None,
        }
    }

    fn position() -> ObjectPositionFact {
        ObjectPositionFact {
            chart_object_id: 1,
            object_code: "sun".to_string(),
            object_name: "Sun".to_string(),
            zodiacal_reference_system_id: 1,
            coordinate_reference_system_id: 1,
            sign_id: 3,
            sign_code: "gemini".to_string(),
            sign_name: "Gemini".to_string(),
            house_id: Some(9),
            house_number: Some(9),
            house_name: Some("Beliefs".to_string()),
            motion_state_id: Some(1),
            horizon_position_id: None,
            longitude_deg: 84.0,
            latitude_deg: None,
            apparent_speed_deg_per_day: Some(1.0),
            altitude_deg: None,
            is_visible: None,
            facts_json: None,
        }
    }

    #[test]
    fn basic_payload_exposes_semantic_signal_fields() {
        let signal = InterpretationSignalRow {
            id: 1,
            signal_key: "object_position:sun".to_string(),
            theme_code: Some("beliefs".to_string()),
            title: "Sun in Gemini, house 9".to_string(),
            summary: Some("summary".to_string()),
            priority_score: 100.0,
            confidence_score: Some(0.95),
            payload_json: Some(json!({
                "interpretive_hint": "hint",
                "semantic_tags": ["placement", "gemini", "beliefs"],
                "source_weight": 1.0,
                "aggregation_group": "gemini:house_9",
                "writing_guidance": "guidance",
                "evidence": {"fact_type": "object_position"}
            })),
        };

        let payload = build_basic_payload(42, &input(), &[position()], &[signal]);
        let basic_signal = &payload.signals[0];

        assert_eq!(basic_signal.theme_code.as_deref(), Some("beliefs"));
        assert_eq!(basic_signal.interpretive_hint.as_deref(), Some("hint"));
        assert_eq!(
            basic_signal.semantic_tags,
            vec!["placement", "gemini", "beliefs"]
        );
        assert_eq!(basic_signal.source_weight, Some(1.0));
        assert_eq!(
            basic_signal.aggregation_group.as_deref(),
            Some("gemini:house_9")
        );
        assert_eq!(basic_signal.writing_guidance.as_deref(), Some("guidance"));
        assert_eq!(
            basic_signal
                .evidence
                .as_ref()
                .and_then(|value| value.get("fact_type"))
                .and_then(|value| value.as_str()),
            Some("object_position")
        );
    }
}
