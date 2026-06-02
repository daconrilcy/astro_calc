use crate::dignities::{
    dignity_is_signal_worthy, essential_dignities_for_position, essential_dignities_for_positions,
    EssentialDignityFact,
};
use crate::domain::{
    BasicDignity, BasicDraftingPlanItem, BasicLlmHandoffContract, BasicObjectPosition,
    BasicPayload, BasicReadingPlanItem, BasicSignal, NatalChartInput, ObjectPositionFact,
};
use crate::models::InterpretationSignalRow;

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

    let dignities = build_payload_dignities(positions, &basic_signals);
    let reading_plan = build_reading_plan(&basic_signals);
    let drafting_plan = build_drafting_plan(&reading_plan, &basic_signals);

    BasicPayload {
        product_code: input.product_code().to_string(),
        chart_calculation_id,
        reference_version_id: input.reference_version_id,
        subject_label: input.subject_label.clone(),
        birth_datetime_utc: input.birth_datetime_utc,
        llm_handoff_contract: Some(basic_llm_handoff_contract()),
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
                sign_context: position_context(position, "sign_context"),
                house_modality: position_context(position, "house_modality"),
                object_context: position_context(position, "object_context"),
                motion_context: position_context(position, "motion_context"),
                dignity_context: position_dignity_context(position),
            })
            .collect(),
        dignities,
        signals: basic_signals,
        reading_plan,
        drafting_plan,
    }
}

fn position_dignity_context(position: &ObjectPositionFact) -> Option<serde_json::Value> {
    let dignities = essential_dignities_for_position(position);
    if dignities.is_empty() {
        None
    } else {
        Some(serde_json::Value::Array(
            dignities
                .into_iter()
                .map(|dignity| {
                    serde_json::json!({
                        "fact_type": "essential_dignity",
                        "dignity_type": dignity.dignity_type,
                        "dignity_label": dignity.dignity_label,
                        "polarity": dignity.polarity,
                        "strength_score": dignity.strength_score,
                    })
                })
                .collect(),
        ))
    }
}

pub fn basic_llm_handoff_contract() -> BasicLlmHandoffContract {
    BasicLlmHandoffContract {
        contract_version: "basic_natal_structured_v3".to_string(),
        payload_language_code: "en".to_string(),
        target_language_policy: "provided_by_llm_service".to_string(),
        audience_level: "beginner".to_string(),
        tone: "clear, warm, non fatalistic".to_string(),
        must_use: vec![
            "dignities".to_string(),
            "signals".to_string(),
            "reading_plan".to_string(),
            "drafting_plan".to_string(),
        ],
        must_not: vec![
            "invent facts not present in source signals".to_string(),
            "mention technical IDs".to_string(),
            "list placements mechanically".to_string(),
            "translate technical keys such as signal_key, theme_code, semantic_tags, slot, or aggregation_group".to_string(),
            "expose raw evidence unless explicitly requested".to_string(),
            "make deterministic or fatalistic predictions".to_string(),
        ],
        output_format: "structured_sections".to_string(),
    }
}

fn build_payload_dignities(
    positions: &[ObjectPositionFact],
    signals: &[BasicSignal],
) -> Vec<BasicDignity> {
    essential_dignities_for_positions(positions)
        .into_iter()
        .map(|dignity| {
            let signal_key = dignity_signal_key(&dignity);
            let signal_key = signals
                .iter()
                .any(|signal| signal.signal_key == signal_key)
                .then_some(signal_key);

            BasicDignity {
                object_code: dignity.object_code,
                object_name: dignity.object_name,
                sign_id: dignity.sign_id,
                sign_code: dignity.sign_code,
                sign_name: dignity.sign_name,
                dignity_type: dignity.dignity_type,
                dignity_label: dignity.dignity_label,
                polarity: dignity.polarity,
                strength_score: dignity.strength_score,
                signal_key,
            }
        })
        .collect()
}

fn dignity_signal_key(dignity: &EssentialDignityFact) -> String {
    if dignity_is_signal_worthy(dignity) {
        format!(
            "dignity:{}:{}:{}",
            dignity.object_code, dignity.dignity_type, dignity.sign_code
        )
    } else {
        String::new()
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
        source_signal_keys.extend(cluster_source_dignity_keys(signals, cluster));
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
        main_dynamic_aspect_keys(signals),
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

fn main_dynamic_aspect_keys(signals: &[BasicSignal]) -> Vec<String> {
    let mut keys: Vec<String> = signals
        .iter()
        .filter(|signal| signal.signal_key.starts_with("aspect:"))
        .take(3)
        .map(|signal| signal.signal_key.clone())
        .collect();

    if keys.iter().any(|key| {
        signals
            .iter()
            .find(|signal| signal.signal_key == *key)
            .is_some_and(is_strong_tension_aspect)
    }) {
        return keys;
    }

    if let Some(tension_key) = signals
        .iter()
        .filter(|signal| signal.signal_key.starts_with("aspect:"))
        .find(|signal| is_strong_tension_aspect(signal))
        .map(|signal| signal.signal_key.clone())
    {
        if keys.len() >= 3 {
            keys.pop();
        }
        keys.push(tension_key);
        dedupe_strings(&mut keys);
    }

    keys
}

fn is_strong_tension_aspect(signal: &BasicSignal) -> bool {
    let aspect_code = signal
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.get("aspect_code"))
        .and_then(|value| value.as_str());
    let strength_score = signal
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.get("strength_score"))
        .and_then(|value| value.as_f64())
        .unwrap_or(signal.priority_score / 80.0);

    matches!(aspect_code, Some("square" | "opposition")) && strength_score >= 0.75
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
        keys.extend(dignity_signal_keys_for_object(signals, object_code));
        if keys.len() >= limit {
            break;
        }
    }

    keys.truncate(limit);
    dedupe_strings(&mut keys);
    keys
}

fn cluster_source_dignity_keys(signals: &[BasicSignal], cluster: &BasicSignal) -> Vec<String> {
    cluster
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.get("source_objects"))
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str())
        .flat_map(|object_code| dignity_signal_keys_for_object(signals, object_code))
        .collect()
}

fn dignity_signal_keys_for_object(signals: &[BasicSignal], object_code: &str) -> Vec<String> {
    let prefix = format!("dignity:{object_code}:");
    signals
        .iter()
        .filter(|signal| signal.signal_key.starts_with(&prefix))
        .map(|signal| signal.signal_key.clone())
        .collect()
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

fn build_drafting_plan(
    reading_plan: &[BasicReadingPlanItem],
    signals: &[BasicSignal],
) -> Vec<BasicDraftingPlanItem> {
    reading_plan
        .iter()
        .map(|item| {
            let source_signals = signals_for_keys(signals, &item.source_signal_keys);
            BasicDraftingPlanItem {
                slot: item.slot.clone(),
                section_title: section_title(item, &source_signals),
                source_signal_keys: item.source_signal_keys.clone(),
                writing_objective: writing_objective(item, &source_signals),
                max_words: max_words_for_slot(&item.slot),
                avoid: avoid_rules_for_slot(&item.slot),
            }
        })
        .collect()
}

fn signals_for_keys<'a>(signals: &'a [BasicSignal], keys: &[String]) -> Vec<&'a BasicSignal> {
    keys.iter()
        .filter_map(|key| signals.iter().find(|signal| signal.signal_key == *key))
        .collect()
}

fn section_title(item: &BasicReadingPlanItem, signals: &[&BasicSignal]) -> String {
    match item.slot.as_str() {
        "core_identity" => "Core chart markers".to_string(),
        "dominant_cluster" => cluster_section_title(signals),
        "main_tension_or_support" => "Main dynamics".to_string(),
        "expression_style" => "Expression and action style".to_string(),
        "background_factors" => "Background factors".to_string(),
        _ => item.title.clone(),
    }
}

fn cluster_section_title(signals: &[&BasicSignal]) -> String {
    let Some(cluster) = signals
        .iter()
        .copied()
        .find(|signal| signal.signal_key.starts_with("cluster:"))
    else {
        return "A structuring dominant theme".to_string();
    };

    let sign_name = cluster
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.get("sign_name"))
        .and_then(|value| value.as_str())
        .unwrap_or("the sign");
    let house_name = cluster
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.get("house_name"))
        .and_then(|value| value.as_str())
        .unwrap_or("the house");

    format!("A {sign_name} dominant theme around {house_name}")
}

fn writing_objective(item: &BasicReadingPlanItem, signals: &[&BasicSignal]) -> String {
    match item.slot.as_str() {
        "core_identity" => {
            "Explain the central identity markers, emotional needs, and overall chart orientation in plain language.".to_string()
        }
        "dominant_cluster" => dominant_cluster_objective(signals),
        "main_tension_or_support" => {
            "Explain the main relationships between chart factors, distinguishing supportive and challenging dynamics without turning aspects into verdicts.".to_string()
        }
        "expression_style" => {
            "Show how the person thinks, communicates, desires, chooses, and acts day to day without listing each placement separately.".to_string()
        }
        "background_factors" => {
            "Place the more collective or less central factors in the background with brief, proportionate wording.".to_string()
        }
        _ => format!(
            "Draft a short section from the {} slot while staying strictly grounded in the source signals.",
            item.slot
        ),
    }
}

fn dominant_cluster_objective(signals: &[&BasicSignal]) -> String {
    let Some(cluster) = signals
        .iter()
        .copied()
        .find(|signal| signal.signal_key.starts_with("cluster:"))
    else {
        return "Explain the chart's dominant theme in plain language without repeating each placement one by one.".to_string();
    };

    let sign_name = cluster
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.get("sign_name"))
        .and_then(|value| value.as_str())
        .unwrap_or("the sign");
    let house_name = cluster
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.get("house_name"))
        .and_then(|value| value.as_str())
        .unwrap_or("the house");
    let sign_code = sign_name.to_lowercase();
    let house_code = house_name.to_lowercase();
    let themes = cluster
        .semantic_tags
        .iter()
        .filter(|tag| {
            !matches!(
                tag.as_str(),
                "cluster" | "placement" | "aspect" | "high_strength" | "medium_strength"
            ) && !tag.starts_with("house_")
                && tag.as_str() != sign_code
                && tag.as_str() != house_code
        })
        .take(4)
        .cloned()
        .collect::<Vec<_>>();

    let theme_text = if themes.is_empty() {
        "the cluster's recurring themes".to_string()
    } else {
        themes.join(", ")
    };

    format!(
        "Explain in plain language that the chart emphasizes {sign_name}, {house_name}, and {theme_text}, grouping the related placements instead of enumerating them."
    )
}

fn max_words_for_slot(slot: &str) -> u16 {
    match slot {
        "dominant_cluster" => 120,
        "core_identity" | "main_tension_or_support" | "expression_style" => 110,
        "background_factors" => 80,
        _ => 100,
    }
}

fn avoid_rules_for_slot(slot: &str) -> Vec<String> {
    let mut rules = vec![
        "use technical IDs".to_string(),
        "make fatalistic predictions".to_string(),
        "add information that is absent from the source signals".to_string(),
    ];

    match slot {
        "dominant_cluster" => {
            rules.insert(0, "repeat each placement one by one".to_string());
        }
        "main_tension_or_support" => {
            rules.insert(0, "present an aspect as an isolated verdict".to_string());
        }
        "background_factors" => {
            rules.insert(0, "give too much weight to background factors".to_string());
        }
        _ => {}
    }

    rules
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

fn position_context(position: &ObjectPositionFact, key: &str) -> Option<serde_json::Value> {
    position
        .facts_json
        .as_ref()
        .and_then(|facts| facts.get(key))
        .filter(|value| !value.is_null())
        .cloned()
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
            facts_json: Some(json!({
                "sign_context": {
                    "element": "air",
                    "modality": "mutable",
                    "polarity": "yang",
                    "keywords": ["communication"]
                },
                "house_modality": {
                    "code": "cadent",
                    "accidental_strength": "weak_or_background",
                    "interpretation_weight": "lower_for_external_manifestation"
                },
                "object_context": {
                    "role": "luminary",
                    "nature": ["luminary"],
                    "is_luminary": true
                },
                "motion_context": {
                    "motion_state": "direct",
                    "label": "Direct",
                    "motion_family": "forward"
                }
            })),
        }
    }

    fn saturn_capricorn_position() -> ObjectPositionFact {
        ObjectPositionFact {
            chart_object_id: 7,
            object_code: "saturn".to_string(),
            object_name: "Saturn".to_string(),
            zodiacal_reference_system_id: 1,
            coordinate_reference_system_id: 1,
            sign_id: 10,
            sign_code: "capricorn".to_string(),
            sign_name: "Capricorn".to_string(),
            house_id: Some(2),
            house_number: Some(2),
            house_name: Some("Resources".to_string()),
            motion_state_id: Some(1),
            horizon_position_id: None,
            longitude_deg: 276.0,
            latitude_deg: None,
            apparent_speed_deg_per_day: Some(0.05),
            altitude_deg: None,
            is_visible: None,
            facts_json: Some(json!({
                "sign_context": {
                    "element": "earth",
                    "modality": "cardinal",
                    "polarity": "yin"
                },
                "house_modality": {
                    "code": "succedent"
                },
                "object_context": {
                    "role": "planet"
                },
                "motion_context": {
                    "motion_state": "direct"
                }
            })),
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
        assert_eq!(payload.drafting_plan.len(), 1);
        assert_eq!(payload.drafting_plan[0].slot, "core_identity");
        assert_eq!(
            payload.drafting_plan[0].source_signal_keys,
            payload.reading_plan[0].source_signal_keys
        );
        assert_eq!(payload.drafting_plan[0].max_words, 110);
        assert_eq!(
            payload
                .llm_handoff_contract
                .as_ref()
                .expect("llm handoff contract")
                .contract_version,
            "basic_natal_structured_v3"
        );
        let contract = payload
            .llm_handoff_contract
            .as_ref()
            .expect("llm handoff contract");
        assert!(contract.must_use.contains(&"dignities".to_string()));
        assert_eq!(contract.payload_language_code, "en");
        assert_eq!(contract.target_language_policy, "provided_by_llm_service");
        assert!(contract.must_use.contains(&"signals".to_string()));
        assert_eq!(
            payload.positions[0]
                .sign_context
                .as_ref()
                .and_then(|context| context.get("element"))
                .and_then(|value| value.as_str()),
            Some("air")
        );
        assert_eq!(
            payload.positions[0]
                .motion_context
                .as_ref()
                .and_then(|context| context.get("motion_state"))
                .and_then(|value| value.as_str()),
            Some("direct")
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
                    "semantic_tags": ["cluster", "capricorn", "house_2", "resources", "structure", "responsibility"],
                    "source_weight": 2.0,
                    "aggregation_group": "capricorn_house_2_cluster",
                    "writing_guidance": "guidance",
                    "evidence": {
                        "fact_type": "position_cluster",
                        "sign_name": "Capricorn",
                        "house_name": "Resources",
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

        let cluster_drafting = payload
            .drafting_plan
            .iter()
            .find(|item| item.slot == "dominant_cluster")
            .expect("expected dominant cluster drafting item");
        assert_eq!(
            cluster_drafting.source_signal_keys,
            vec!["cluster:capricorn:house_2", "object_position:sun"]
        );
        assert_eq!(
            cluster_drafting.section_title,
            "A Capricorn dominant theme around Resources"
        );
        assert!(cluster_drafting
            .avoid
            .contains(&"repeat each placement one by one".to_string()));
    }

    #[test]
    fn basic_payload_exposes_structured_dignities() {
        let signal = InterpretationSignalRow {
            id: 1,
            signal_key: "dignity:saturn:domicile:capricorn".to_string(),
            theme_code: Some("functional_strength".to_string()),
            title: "Saturn strongly placed in Capricorn".to_string(),
            summary: Some("summary".to_string()),
            priority_score: 88.0,
            confidence_score: Some(0.95),
            payload_json: Some(json!({
                "interpretive_hint": "hint",
                "semantic_tags": ["dignity", "saturn", "capricorn", "domicile"],
                "source_weight": 0.75,
                "aggregation_group": "dignity:saturn",
                "writing_guidance": "guidance",
                "evidence": {
                    "fact_type": "essential_dignity",
                    "chart_object": "saturn",
                    "sign_code": "capricorn",
                    "dignity_type": "domicile"
                }
            })),
        };

        let position = saturn_capricorn_position();
        let payload = build_basic_payload(42, &input(), &[position], &[signal]);

        assert_eq!(payload.dignities.len(), 1);
        assert_eq!(payload.dignities[0].object_code, "saturn");
        assert_eq!(payload.dignities[0].dignity_type, "domicile");
        assert_eq!(
            payload.dignities[0].signal_key.as_deref(),
            Some("dignity:saturn:domicile:capricorn")
        );
        assert_eq!(
            payload.positions[0]
                .dignity_context
                .as_ref()
                .and_then(|context| context.as_array())
                .and_then(|context| context.first())
                .and_then(|context| context.get("dignity_type"))
                .and_then(|value| value.as_str()),
            Some("domicile")
        );
    }

    #[test]
    fn reading_plan_uses_active_dignity_signals() {
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
                        "sign_name": "Capricorn",
                        "house_name": "Resources",
                        "source_signals": ["object_position:sun", "object_position:saturn"],
                        "source_objects": ["sun", "saturn"]
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
            dignity_signal_row(3, "dignity:saturn:domicile:capricorn", "saturn"),
            InterpretationSignalRow {
                id: 4,
                signal_key: "object_position:jupiter".to_string(),
                theme_code: Some("shared_resources".to_string()),
                title: "Jupiter in Cancer, house 8".to_string(),
                summary: Some("summary".to_string()),
                priority_score: 81.75,
                confidence_score: Some(0.95),
                payload_json: Some(json!({
                    "interpretive_hint": "hint",
                    "semantic_tags": ["placement", "jupiter"],
                    "source_weight": 0.75,
                    "aggregation_group": "cancer:house_8",
                    "writing_guidance": "guidance",
                    "evidence": {"fact_type": "object_position", "object_code": "jupiter"}
                })),
            },
            dignity_signal_row(5, "dignity:jupiter:exaltation:cancer", "jupiter"),
        ];

        let payload = build_basic_payload(42, &input(), &[position()], &signals);
        let cluster_plan = payload
            .reading_plan
            .iter()
            .find(|item| item.slot == "dominant_cluster")
            .expect("expected cluster plan");
        let background_plan = payload
            .reading_plan
            .iter()
            .find(|item| item.slot == "background_factors")
            .expect("expected background plan");

        assert!(cluster_plan
            .source_signal_keys
            .contains(&"dignity:saturn:domicile:capricorn".to_string()));
        assert!(background_plan
            .source_signal_keys
            .contains(&"dignity:jupiter:exaltation:cancer".to_string()));
    }

    #[test]
    fn main_dynamic_aspects_include_strong_tension_when_available() {
        let signals = vec![
            aspect_signal(1, "aspect:moon:neptune:sextile", "sextile", 0.95),
            aspect_signal(2, "aspect:sun:moon:sextile", "sextile", 0.93),
            aspect_signal(3, "aspect:sun:neptune:conjunction", "conjunction", 0.9),
            aspect_signal(4, "aspect:moon:mars:square", "square", 0.88),
        ];

        let payload = build_basic_payload(42, &input(), &[position()], &signals);
        let aspect_plan = payload
            .reading_plan
            .iter()
            .find(|item| item.slot == "main_tension_or_support")
            .expect("expected aspect plan");

        assert_eq!(aspect_plan.source_signal_keys.len(), 3);
        assert!(aspect_plan
            .source_signal_keys
            .contains(&"aspect:moon:mars:square".to_string()));
    }

    fn aspect_signal(
        id: i32,
        signal_key: &str,
        aspect_code: &str,
        strength_score: f64,
    ) -> InterpretationSignalRow {
        InterpretationSignalRow {
            id,
            signal_key: signal_key.to_string(),
            theme_code: Some("aspect".to_string()),
            title: format!("Aspect {aspect_code}"),
            summary: Some(format!(
                "Two chart factors form a {aspect_code} with a controlled summary."
            )),
            priority_score: strength_score * 80.0,
            confidence_score: Some(0.85),
            payload_json: Some(json!({
                "interpretive_hint": "hint",
                "semantic_tags": ["aspect", aspect_code],
                "source_weight": 1.0,
                "aggregation_group": format!("aspect:{aspect_code}"),
                "writing_guidance": "guidance",
                "evidence": {
                    "fact_type": "aspect",
                    "aspect_code": aspect_code,
                    "aspect_name": aspect_code,
                    "strength_score": strength_score
                }
            })),
        }
    }

    fn dignity_signal_row(id: i32, signal_key: &str, object_code: &str) -> InterpretationSignalRow {
        InterpretationSignalRow {
            id,
            signal_key: signal_key.to_string(),
            theme_code: Some("functional_strength".to_string()),
            title: format!("{object_code} dignity"),
            summary: Some("summary".to_string()),
            priority_score: 86.0,
            confidence_score: Some(0.95),
            payload_json: Some(json!({
                "interpretive_hint": "hint",
                "semantic_tags": ["dignity", object_code],
                "source_weight": 0.75,
                "aggregation_group": format!("dignity:{object_code}"),
                "writing_guidance": "guidance",
                "evidence": {
                    "fact_type": "essential_dignity",
                    "chart_object": object_code
                }
            })),
        }
    }
}
