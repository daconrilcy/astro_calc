use crate::domain::{
    BasicObjectPosition, BasicPayload, BasicReadingPlanItem, BasicSignal, InterpretationSignalRow,
    NatalChartInput, ObjectPositionFact,
};

pub fn build_basic_payload(
    chart_calculation_id: i32,
    input: &NatalChartInput,
    positions: &[ObjectPositionFact],
    signals: &[InterpretationSignalRow],
) -> BasicPayload {
    let basic_signals: Vec<BasicSignal> = signals
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
        .collect();

    let reading_plan = build_reading_plan(&basic_signals);

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
        signals: basic_signals,
        reading_plan,
    }
}

fn build_reading_plan(signals: &[BasicSignal]) -> Vec<BasicReadingPlanItem> {
    let mut plan = Vec::new();

    push_plan_item(
        &mut plan,
        "core_identity",
        "Core identity markers",
        signal_keys_for_objects(signals, &["sun", "moon", "ascendant", "mc"], 4),
    );

    if let Some(cluster) = signals
        .iter()
        .find(|signal| signal.signal_key.starts_with("cluster:"))
    {
        let mut source_signal_keys = vec![cluster.signal_key.clone()];
        source_signal_keys.extend(
            cluster
                .evidence
                .as_ref()
                .and_then(|evidence| evidence.get("source_signals"))
                .and_then(|value| value.as_array())
                .into_iter()
                .flatten()
                .filter_map(|value| value.as_str())
                .filter(|source_key| {
                    signals
                        .iter()
                        .any(|signal| signal.signal_key == *source_key)
                })
                .map(ToString::to_string),
        );
        dedupe_strings(&mut source_signal_keys);

        push_plan_item(
            &mut plan,
            "dominant_cluster",
            "Dominant repeated theme",
            source_signal_keys,
        );
    }

    push_plan_item(
        &mut plan,
        "main_tension_or_support",
        "Main dynamic aspect",
        signals
            .iter()
            .filter(|signal| signal.signal_key.starts_with("aspect:"))
            .take(3)
            .map(|signal| signal.signal_key.clone())
            .collect(),
    );

    push_plan_item(
        &mut plan,
        "expression_style",
        "Expression style",
        signal_keys_for_objects(signals, &["mercury", "venus", "mars"], 3),
    );

    push_plan_item(
        &mut plan,
        "background_factors",
        "Background factors",
        signal_keys_for_objects(
            signals,
            &["jupiter", "saturn", "uranus", "neptune", "pluto"],
            3,
        ),
    );

    plan
}

fn push_plan_item(
    plan: &mut Vec<BasicReadingPlanItem>,
    slot: &str,
    title: &str,
    source_signal_keys: Vec<String>,
) {
    if source_signal_keys.is_empty() {
        return;
    }

    plan.push(BasicReadingPlanItem {
        slot: slot.to_string(),
        title: title.to_string(),
        source_signal_keys,
    });
}

fn signal_keys_for_objects(
    signals: &[BasicSignal],
    object_codes: &[&str],
    limit: usize,
) -> Vec<String> {
    let mut keys = Vec::new();

    for object_code in object_codes {
        let signal_key = format!("object_position:{object_code}");
        if signals.iter().any(|signal| signal.signal_key == signal_key) {
            keys.push(signal_key);
        }
        if keys.len() >= limit {
            break;
        }
    }

    keys
}

fn dedupe_strings(values: &mut Vec<String>) {
    let mut deduped = Vec::new();
    for value in values.drain(..) {
        if !deduped.contains(&value) {
            deduped.push(value);
        }
    }
    *values = deduped;
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
        assert_eq!(payload.reading_plan.len(), 1);
        assert_eq!(payload.reading_plan[0].slot, "core_identity");
        assert_eq!(
            payload.reading_plan[0].source_signal_keys,
            vec!["object_position:sun"]
        );
    }

    #[test]
    fn basic_payload_builds_reading_plan_with_cluster_sources() {
        let signals = vec![
            InterpretationSignalRow {
                id: 1,
                signal_key: "cluster:capricorn:house_2".to_string(),
                theme_code: Some("resources".to_string()),
                title: "Strong concentration in Capricorn, house 2".to_string(),
                summary: Some("summary".to_string()),
                priority_score: 99.0,
                confidence_score: Some(0.9),
                payload_json: Some(json!({
                    "interpretive_hint": "hint",
                    "semantic_tags": ["cluster", "capricorn", "house_2"],
                    "source_weight": 2.0,
                    "aggregation_group": "capricorn_house_2_cluster",
                    "writing_guidance": "guidance",
                    "evidence": {
                        "fact_type": "position_cluster",
                        "source_signals": [
                            "object_position:sun",
                            "object_position:saturn"
                        ]
                    }
                })),
            },
            InterpretationSignalRow {
                id: 2,
                signal_key: "object_position:sun".to_string(),
                theme_code: Some("resources".to_string()),
                title: "Sun in Capricorn, house 2".to_string(),
                summary: Some("summary".to_string()),
                priority_score: 100.0,
                confidence_score: Some(0.95),
                payload_json: Some(json!({
                    "interpretive_hint": "hint",
                    "semantic_tags": ["placement", "sun"],
                    "source_weight": 1.0,
                    "aggregation_group": "capricorn:house_2",
                    "writing_guidance": "guidance",
                    "evidence": {"fact_type": "object_position", "object_code": "sun"}
                })),
            },
            InterpretationSignalRow {
                id: 3,
                signal_key: "aspect:sun:neptune:conjunction".to_string(),
                theme_code: Some("aspect".to_string()),
                title: "Sun conjunction Neptune".to_string(),
                summary: Some("summary".to_string()),
                priority_score: 78.0,
                confidence_score: Some(0.85),
                payload_json: Some(json!({
                    "interpretive_hint": "hint",
                    "semantic_tags": ["aspect", "conjunction"],
                    "source_weight": 1.35,
                    "aggregation_group": "aspect:conjunction",
                    "writing_guidance": "guidance",
                    "evidence": {"fact_type": "aspect"}
                })),
            },
        ];

        let payload = build_basic_payload(42, &input(), &[position()], &signals);
        let cluster_plan = payload
            .reading_plan
            .iter()
            .find(|item| item.slot == "dominant_cluster")
            .expect("expected dominant cluster plan item");

        assert_eq!(
            cluster_plan.source_signal_keys,
            vec!["cluster:capricorn:house_2", "object_position:sun"]
        );
        assert!(payload
            .reading_plan
            .iter()
            .any(|item| item.slot == "main_tension_or_support"));
    }
}
