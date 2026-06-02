use crate::dignities::{
    dignity_is_signal_worthy, essential_dignities_for_position, essential_dignities_for_positions,
    EssentialDignityFact,
};
use crate::domain::{
    BasicAngleFact, BasicChartEmphasis, BasicDignity, BasicDominantHouse, BasicDominantObject,
    BasicDominantSign, BasicDraftingPlanItem, BasicEmphasisRefs, BasicLlmHandoffContract,
    BasicObjectPosition, BasicPayload, BasicReadingPlanItem, BasicSecondarySlotCandidate,
    BasicSignal, NatalChartInput, ObjectPositionFact,
};
use crate::models::InterpretationSignalRow;
use std::collections::{HashMap, HashSet};

pub fn build_basic_payload(
    chart_calculation_id: i32,
    input: &NatalChartInput,
    positions: &[ObjectPositionFact],
    signals: &[InterpretationSignalRow],
) -> BasicPayload {
    let structural_axis_pairs = structural_axis_pairs_from_positions(positions);
    let angle_object_codes = angle_object_codes_from_positions(positions);
    let mut basic_signals: Vec<BasicSignal> = signals
        .iter()
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
            aspect_context: payload_value(signal, "aspect_context"),
            evidence: payload_value(signal, "evidence"),
        })
        .collect();
    basic_signals.retain(|signal| {
        !is_structural_axis_signal_for_pairs(signal, &structural_axis_pairs)
            && !is_angle_to_angle_aspect_signal(signal, &angle_object_codes)
    });
    basic_signals.truncate(12);

    let angles = build_payload_angles(positions);
    let dignities = build_payload_dignities(positions, &basic_signals);
    let chart_emphasis = build_chart_emphasis(positions, &dignities, &basic_signals);
    let reading_plan = build_reading_plan(&basic_signals);
    let drafting_plan = build_drafting_plan(&reading_plan, &basic_signals, &chart_emphasis);

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
                house_context: position_context(position, "house_context"),
                house_modality: position_context(position, "house_modality"),
                object_context: position_context(position, "object_context"),
                motion_context: position_context(position, "motion_context"),
                dignity_context: position_dignity_context(position),
            })
            .collect(),
        angles,
        dignities,
        chart_emphasis,
        signals: basic_signals,
        reading_plan,
        drafting_plan,
    }
}

fn build_payload_angles(positions: &[ObjectPositionFact]) -> Vec<BasicAngleFact> {
    let angle_object_codes: HashMap<String, String> = positions
        .iter()
        .filter_map(|position| {
            position_context(position, "angle_context")
                .and_then(|context| {
                    context
                        .get("angle_point_code")
                        .and_then(|value| value.as_str())
                        .map(ToString::to_string)
                })
                .map(|angle_point_code| (angle_point_code, position.object_code.clone()))
        })
        .collect();

    let mut angles: Vec<_> = positions
        .iter()
        .filter_map(|position| {
            let angle_context = position_context(position, "angle_context")?;
            let opposite_angle_code = angle_context
                .get("opposite_angle_code")
                .and_then(|value| value.as_str())
                .and_then(|code| angle_object_codes.get(code).map(String::as_str))
                .or_else(|| {
                    angle_context
                        .get("opposite_angle_code")
                        .and_then(|value| value.as_str())
                })
                .unwrap_or_default()
                .to_string();

            Some(BasicAngleFact {
                angle_code: position.object_code.clone(),
                angle_name: angle_context
                    .get("full_name")
                    .and_then(|value| value.as_str())
                    .unwrap_or(&position.object_name)
                    .to_string(),
                axis: angle_context
                    .get("axis")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string(),
                opposite_angle_code,
                longitude_deg: position.longitude_deg,
                sign_id: position.sign_id,
                sign_code: position.sign_code.clone(),
                sign_name: position.sign_name.clone(),
                house_id: position.house_id,
                house_number: angle_context
                    .get("associated_house_number")
                    .and_then(|value| value.as_i64())
                    .and_then(|value| i32::try_from(value).ok())
                    .or(position.house_number)
                    .unwrap_or_default(),
                house_name: position.house_name.clone(),
            })
        })
        .collect();
    angles.sort_by_key(|angle| {
        positions
            .iter()
            .find(|position| position.object_code == angle.angle_code)
            .and_then(|position| position_context(position, "angle_context"))
            .and_then(|context| {
                context
                    .get("chart_object_sort_order")
                    .and_then(|value| value.as_i64())
                    .and_then(|value| i32::try_from(value).ok())
            })
            .unwrap_or(i32::MAX)
    });
    angles
}

fn structural_axis_pairs_from_positions(
    positions: &[ObjectPositionFact],
) -> HashSet<(String, String)> {
    let angle_positions: Vec<_> = positions
        .iter()
        .filter_map(|position| {
            position_context(position, "angle_context")
                .and_then(|context| {
                    context
                        .get("axis")
                        .and_then(|value| value.as_str())
                        .map(ToString::to_string)
                })
                .map(|axis| (axis, position.object_code.clone()))
        })
        .collect();

    structural_axis_pairs(angle_positions)
}

fn angle_object_codes_from_positions(positions: &[ObjectPositionFact]) -> HashSet<String> {
    positions
        .iter()
        .filter(|position| position_context(position, "angle_context").is_some())
        .map(|position| position.object_code.clone())
        .collect()
}

fn structural_axis_pairs(angle_positions: Vec<(String, String)>) -> HashSet<(String, String)> {
    let mut pairs = HashSet::new();

    for left_index in 0..angle_positions.len() {
        for right_index in (left_index + 1)..angle_positions.len() {
            let (left_axis, left_code) = &angle_positions[left_index];
            let (right_axis, right_code) = &angle_positions[right_index];
            if left_axis == right_axis {
                pairs.insert(normalized_pair(left_code, right_code));
            }
        }
    }

    pairs
}

fn position_dignity_context(position: &ObjectPositionFact) -> serde_json::Value {
    let dignities = essential_dignities_for_position(position);
    serde_json::Value::Array(
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
    )
}

pub fn basic_llm_handoff_contract() -> BasicLlmHandoffContract {
    BasicLlmHandoffContract {
        contract_version: "basic_natal_structured_v8".to_string(),
        payload_language_code: "en".to_string(),
        target_language_policy: "provided_by_llm_service".to_string(),
        audience_level: "beginner".to_string(),
        tone: "clear, warm, non fatalistic".to_string(),
        must_use: vec![
            "chart_emphasis".to_string(),
            "dignities".to_string(),
            "angles".to_string(),
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
            "treat chart_emphasis as a standalone section instead of weighting context".to_string(),
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

#[derive(Default)]
struct EmphasisScore {
    score: f64,
    reasons: Vec<String>,
}

const SIGN_EMPHASIS_FULL_SCORE: f64 = 4.6;
const HOUSE_EMPHASIS_FULL_SCORE: f64 = 4.6;
const OBJECT_EMPHASIS_FULL_SCORE: f64 = 2.4;
const SIGN_HOUSE_EMPHASIS_MIN_SCORE: f64 = 0.35;
const OBJECT_EMPHASIS_MIN_SCORE: f64 = 0.5;

fn build_chart_emphasis(
    positions: &[ObjectPositionFact],
    dignities: &[BasicDignity],
    signals: &[BasicSignal],
) -> BasicChartEmphasis {
    let mut sign_scores: HashMap<String, EmphasisScore> = HashMap::new();
    let mut house_scores: HashMap<i32, EmphasisScore> = HashMap::new();
    let mut house_theme_codes: HashMap<i32, String> = HashMap::new();
    let mut object_scores: HashMap<String, EmphasisScore> = HashMap::new();
    let positions_by_object: HashMap<&str, &ObjectPositionFact> = positions
        .iter()
        .map(|position| (position.object_code.as_str(), position))
        .collect();

    for position in positions {
        let object_weight = object_source_weight(&position.object_code);

        add_score(
            sign_scores.entry(position.sign_code.clone()).or_default(),
            object_weight,
            format!("{}_in_sign", position.object_code),
        );
        add_score(
            object_scores
                .entry(position.object_code.clone())
                .or_default(),
            object_weight,
            "placement".to_string(),
        );

        if let Some(house_number) = position.house_number {
            if let Some(theme_code) = house_theme_code(position) {
                house_theme_codes.entry(house_number).or_insert(theme_code);
            }
            add_score(
                house_scores.entry(house_number).or_default(),
                object_weight,
                format!("{}_in_house", position.object_code),
            );
        }
    }

    add_multiple_object_reasons(positions, &mut sign_scores, &mut house_scores);
    add_dignity_emphasis(
        dignities,
        &positions_by_object,
        &mut sign_scores,
        &mut house_scores,
        &mut object_scores,
    );
    add_signal_emphasis(
        signals,
        &mut sign_scores,
        &mut house_scores,
        &mut house_theme_codes,
        &mut object_scores,
    );
    add_sign_emphasis_to_objects(positions, &sign_scores, &mut object_scores);

    BasicChartEmphasis {
        dominant_signs: normalized_signs(sign_scores),
        dominant_houses: normalized_houses(house_scores, &house_theme_codes),
        dominant_objects: normalized_objects(object_scores),
    }
}

fn add_multiple_object_reasons(
    positions: &[ObjectPositionFact],
    sign_scores: &mut HashMap<String, EmphasisScore>,
    house_scores: &mut HashMap<i32, EmphasisScore>,
) {
    let mut sign_counts: HashMap<&str, usize> = HashMap::new();
    let mut house_counts: HashMap<i32, usize> = HashMap::new();
    for position in positions {
        *sign_counts.entry(position.sign_code.as_str()).or_default() += 1;
        if let Some(house_number) = position.house_number {
            *house_counts.entry(house_number).or_default() += 1;
        }
    }

    for (sign_code, count) in sign_counts {
        if count >= 2 {
            add_reason(
                sign_scores.entry(sign_code.to_string()).or_default(),
                "multiple_objects",
            );
        }
    }
    for (house_number, count) in house_counts {
        if count >= 2 {
            add_reason(
                house_scores.entry(house_number).or_default(),
                "multiple_objects",
            );
        }
    }
}

fn add_dignity_emphasis(
    dignities: &[BasicDignity],
    positions_by_object: &HashMap<&str, &ObjectPositionFact>,
    sign_scores: &mut HashMap<String, EmphasisScore>,
    house_scores: &mut HashMap<i32, EmphasisScore>,
    object_scores: &mut HashMap<String, EmphasisScore>,
) {
    for dignity in dignities {
        let dignity_weight = dignity_emphasis_weight(dignity);
        add_score(
            sign_scores.entry(dignity.sign_code.clone()).or_default(),
            dignity_weight,
            format!("{}_{}", dignity.object_code, dignity.dignity_type),
        );
        add_score(
            object_scores
                .entry(dignity.object_code.clone())
                .or_default(),
            dignity_weight,
            dignity.dignity_type.clone(),
        );

        if let Some(position) = positions_by_object.get(dignity.object_code.as_str()) {
            if let Some(house_number) = position.house_number {
                add_score(
                    house_scores.entry(house_number).or_default(),
                    dignity_weight,
                    format!("{}_{}", dignity.object_code, dignity.dignity_type),
                );
            }
        }
    }
}

fn add_signal_emphasis(
    signals: &[BasicSignal],
    sign_scores: &mut HashMap<String, EmphasisScore>,
    house_scores: &mut HashMap<i32, EmphasisScore>,
    house_theme_codes: &mut HashMap<i32, String>,
    object_scores: &mut HashMap<String, EmphasisScore>,
) {
    for signal in signals {
        if signal.signal_key.starts_with("cluster:") {
            add_cluster_emphasis(
                signal,
                sign_scores,
                house_scores,
                house_theme_codes,
                object_scores,
            );
        } else if signal.signal_key.starts_with("aspect:")
            && !is_structural_axis_signal(signal)
            && aspect_strength_score(signal) >= 0.75
        {
            add_aspect_object_emphasis(signal, object_scores);
        }
    }
}

fn add_cluster_emphasis(
    signal: &BasicSignal,
    sign_scores: &mut HashMap<String, EmphasisScore>,
    house_scores: &mut HashMap<i32, EmphasisScore>,
    house_theme_codes: &mut HashMap<i32, String>,
    object_scores: &mut HashMap<String, EmphasisScore>,
) {
    let Some(evidence) = signal.evidence.as_ref() else {
        return;
    };
    let cluster_weight = (signal.priority_score / 100.0).clamp(0.0, 1.0);

    if let Some(sign_code) = evidence.get("sign_code").and_then(|value| value.as_str()) {
        add_score(
            sign_scores.entry(sign_code.to_string()).or_default(),
            cluster_weight,
            "sign_house_cluster".to_string(),
        );
    }
    if let Some(house_number) = evidence
        .get("house_number")
        .and_then(|value| value.as_i64())
    {
        if let Some(theme_code) = evidence
            .get("house_theme_code")
            .and_then(|value| value.as_str())
        {
            house_theme_codes
                .entry(house_number as i32)
                .or_insert_with(|| theme_code.to_string());
        }
        add_score(
            house_scores.entry(house_number as i32).or_default(),
            cluster_weight,
            "cluster".to_string(),
        );
    }
    if let Some(source_objects) = evidence
        .get("source_objects")
        .and_then(|value| value.as_array())
    {
        for object_code in source_objects.iter().filter_map(|value| value.as_str()) {
            add_score(
                object_scores.entry(object_code.to_string()).or_default(),
                0.35,
                "cluster_participant".to_string(),
            );
        }
    }
}

fn add_aspect_object_emphasis(
    signal: &BasicSignal,
    object_scores: &mut HashMap<String, EmphasisScore>,
) {
    let Some(evidence) = signal.evidence.as_ref() else {
        return;
    };
    let strength = aspect_strength_score(signal).clamp(0.0, 1.0);

    let mut object_codes: Vec<&str> = ["source_object_code", "target_object_code"]
        .into_iter()
        .filter_map(|key| evidence.get(key).and_then(|value| value.as_str()))
        .collect();
    if object_codes.is_empty() {
        let parts = signal.signal_key.split(':').collect::<Vec<_>>();
        if parts.len() >= 4 {
            object_codes.extend([parts[1], parts[2]]);
        }
    }

    for object_code in object_codes {
        add_score(
            object_scores.entry(object_code.to_string()).or_default(),
            strength * 0.35,
            "strong_aspect_participant".to_string(),
        );
    }
}

fn add_sign_emphasis_to_objects(
    positions: &[ObjectPositionFact],
    sign_scores: &HashMap<String, EmphasisScore>,
    object_scores: &mut HashMap<String, EmphasisScore>,
) {
    let Some(max_sign_score) = sign_scores
        .values()
        .map(|entry| entry.score)
        .max_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal))
    else {
        return;
    };
    if max_sign_score <= 0.0 {
        return;
    }

    for position in positions {
        let Some(sign_score) = sign_scores
            .get(&position.sign_code)
            .map(|entry| entry.score)
        else {
            continue;
        };
        let normalized_sign_score = normalized_emphasis_score(sign_score, SIGN_EMPHASIS_FULL_SCORE);
        if sign_score >= max_sign_score * 0.85
            && normalized_sign_score >= SIGN_HOUSE_EMPHASIS_MIN_SCORE
        {
            add_reason(
                object_scores
                    .entry(position.object_code.clone())
                    .or_default(),
                &format!("{}_emphasis", position.sign_code),
            );
        }
    }
}

fn normalized_signs(scores: HashMap<String, EmphasisScore>) -> Vec<BasicDominantSign> {
    let mut values: Vec<_> = scores
        .into_iter()
        .filter(|(_, entry)| entry.score > 0.0)
        .map(|(sign_code, entry)| BasicDominantSign {
            sign_code,
            score: normalized_emphasis_score(entry.score, SIGN_EMPHASIS_FULL_SCORE),
            reasons: entry.reasons,
        })
        .collect();
    values.sort_by(|left, right| {
        sort_emphasis(left.score, &left.sign_code, right.score, &right.sign_code)
    });
    retain_strong_or_top_signs(&mut values);
    values.truncate(3);
    values
}

fn normalized_houses(
    scores: HashMap<i32, EmphasisScore>,
    house_theme_codes: &HashMap<i32, String>,
) -> Vec<BasicDominantHouse> {
    let mut values: Vec<_> = scores
        .into_iter()
        .filter(|(_, entry)| entry.score > 0.0)
        .map(|(house_number, entry)| BasicDominantHouse {
            house_number,
            theme_code: house_theme_codes
                .get(&house_number)
                .cloned()
                .unwrap_or_else(|| "object_position".to_string()),
            score: normalized_emphasis_score(entry.score, HOUSE_EMPHASIS_FULL_SCORE),
            reasons: entry.reasons,
        })
        .collect();
    values.sort_by(|left, right| {
        sort_emphasis(
            left.score,
            &left.house_number,
            right.score,
            &right.house_number,
        )
    });
    retain_strong_or_top_houses(&mut values);
    values.truncate(3);
    values
}

fn normalized_objects(scores: HashMap<String, EmphasisScore>) -> Vec<BasicDominantObject> {
    let mut values: Vec<_> = scores
        .into_iter()
        .filter(|(_, entry)| entry.score > 0.0)
        .map(|(object_code, entry)| BasicDominantObject {
            object_code,
            score: normalized_emphasis_score(entry.score, OBJECT_EMPHASIS_FULL_SCORE),
            reasons: entry.reasons,
        })
        .collect();
    values.sort_by(|left, right| {
        sort_emphasis(
            left.score,
            &left.object_code,
            right.score,
            &right.object_code,
        )
    });
    retain_strong_or_top_objects(&mut values);
    values.truncate(5);
    values
}

fn retain_strong_or_top_signs(values: &mut Vec<BasicDominantSign>) {
    let top = values.first().cloned();
    values.retain(|entry| entry.score >= SIGN_HOUSE_EMPHASIS_MIN_SCORE);
    if values.is_empty() {
        if let Some(top) = top {
            values.push(top);
        }
    }
}

fn retain_strong_or_top_houses(values: &mut Vec<BasicDominantHouse>) {
    let top = values.first().cloned();
    values.retain(|entry| entry.score >= SIGN_HOUSE_EMPHASIS_MIN_SCORE);
    if values.is_empty() {
        if let Some(top) = top {
            values.push(top);
        }
    }
}

fn retain_strong_or_top_objects(values: &mut Vec<BasicDominantObject>) {
    let top = values.first().cloned();
    values.retain(|entry| {
        entry.score >= OBJECT_EMPHASIS_MIN_SCORE
            && entry
                .reasons
                .iter()
                .any(|reason| reason.as_str() != "placement")
    });
    if values.is_empty() {
        if let Some(top) = top {
            values.push(top);
        }
    }
}

fn normalized_emphasis_score(score: f64, full_score: f64) -> f64 {
    if full_score <= 0.0 {
        0.0
    } else {
        round4((score / full_score).clamp(0.0, 1.0))
    }
}

fn sort_emphasis<T: Ord>(
    left_score: f64,
    left_key: &T,
    right_score: f64,
    right_key: &T,
) -> std::cmp::Ordering {
    right_score
        .partial_cmp(&left_score)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| left_key.cmp(right_key))
}

fn add_score(entry: &mut EmphasisScore, score: f64, reason: String) {
    entry.score += score;
    add_reason(entry, &reason);
}

fn add_reason(entry: &mut EmphasisScore, reason: &str) {
    if !entry.reasons.iter().any(|existing| existing == reason) {
        entry.reasons.push(reason.to_string());
    }
}

fn dignity_emphasis_weight(dignity: &BasicDignity) -> f64 {
    match dignity.dignity_type.as_str() {
        "domicile" => 0.65,
        "exaltation" => 0.55,
        "detriment" => 0.45,
        "fall" => 0.35,
        _ => 0.25,
    }
}

fn build_reading_plan(signals: &[BasicSignal]) -> Vec<BasicReadingPlanItem> {
    let mut plan = Vec::new();

    push_plan_item(
        &mut plan,
        "core_identity",
        "Core identity markers",
        signal_keys_for_objects(signals, &["sun", "moon", "ascendant"], 3),
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
            &["mc", "jupiter", "saturn", "uranus", "neptune", "pluto"],
            3,
        ),
    );

    finalize_reading_plan(&mut plan);
    plan
}

fn main_dynamic_aspect_keys(signals: &[BasicSignal]) -> Vec<String> {
    let mut keys: Vec<String> = signals
        .iter()
        .filter(|signal| is_interpretive_aspect_signal(signal))
        .take(3)
        .map(|signal| signal.signal_key.clone())
        .collect();

    let has_tension = keys.iter().any(|key| {
        signals
            .iter()
            .find(|signal| signal.signal_key == *key)
            .is_some_and(is_interpretive_tension_aspect)
    });
    if !has_tension {
        if let Some(tension_key) = signals
            .iter()
            .filter(|signal| is_interpretive_aspect_signal(signal))
            .find(|signal| is_interpretive_tension_aspect(signal))
            .map(|signal| signal.signal_key.clone())
        {
            push_balanced_aspect_key(
                &mut keys,
                tension_key,
                signals,
                is_interpretive_support_aspect,
            );
        }
    }

    let has_support = keys.iter().any(|key| {
        signals
            .iter()
            .find(|signal| signal.signal_key == *key)
            .is_some_and(is_interpretive_support_aspect)
    });
    if !has_support {
        if let Some(support_key) = signals
            .iter()
            .filter(|signal| is_interpretive_aspect_signal(signal))
            .find(|signal| is_interpretive_support_aspect(signal))
            .map(|signal| signal.signal_key.clone())
        {
            push_balanced_aspect_key(
                &mut keys,
                support_key,
                signals,
                is_interpretive_tension_aspect,
            );
        }
    }

    dedupe_strings(&mut keys);
    keys.truncate(3);
    keys
}

fn push_balanced_aspect_key(
    keys: &mut Vec<String>,
    new_key: String,
    signals: &[BasicSignal],
    preserve: fn(&BasicSignal) -> bool,
) {
    if keys.contains(&new_key) {
        return;
    }

    if keys.len() < 3 {
        keys.push(new_key);
        return;
    }

    let preserved_count = keys
        .iter()
        .filter_map(|key| signals.iter().find(|signal| signal.signal_key == **key))
        .filter(|signal| preserve(signal))
        .count();

    let replacement_index = keys
        .iter()
        .enumerate()
        .rev()
        .find(|(_, key)| {
            signals
                .iter()
                .find(|signal| signal.signal_key == **key)
                .is_none_or(|signal| !preserve(signal) || preserved_count > 1)
        })
        .map(|(index, _)| index)
        .unwrap_or(keys.len() - 1);

    keys[replacement_index] = new_key;
}

fn is_interpretive_tension_aspect(signal: &BasicSignal) -> bool {
    if !is_interpretive_aspect_signal(signal) {
        return false;
    }

    let dynamic_quality = aspect_context_str(signal, "dynamic_quality");
    let primary_valence = aspect_context_str(signal, "primary_valence");
    let strength_score = aspect_strength_score(signal);

    matches!(dynamic_quality, Some("tension"))
        || matches!(
            primary_valence,
            Some("dynamic_challenging" | "polarizing" | "minor_friction" | "indirect_tension")
        ) && strength_score >= 0.75
}

fn is_interpretive_support_aspect(signal: &BasicSignal) -> bool {
    if !is_interpretive_aspect_signal(signal) {
        return false;
    }

    matches!(aspect_context_str(signal, "dynamic_quality"), Some("flow"))
        || matches!(
            aspect_context_str(signal, "primary_valence"),
            Some("supportive" | "harmonious")
        )
}

fn is_interpretive_aspect_signal(signal: &BasicSignal) -> bool {
    signal.signal_key.starts_with("aspect:") && !is_structural_axis_signal(signal)
}

fn is_structural_axis_signal(signal: &BasicSignal) -> bool {
    signal
        .aspect_context
        .as_ref()
        .and_then(|context| context.get("is_structural_axis"))
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
        || signal
            .evidence
            .as_ref()
            .and_then(|evidence| evidence.get("is_structural_axis"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
}

fn is_structural_axis_signal_for_pairs(
    signal: &BasicSignal,
    structural_axis_pairs: &HashSet<(String, String)>,
) -> bool {
    if !signal.signal_key.starts_with("aspect:") {
        return false;
    }
    if is_structural_axis_signal(signal) {
        return true;
    }
    if aspect_code(signal) != Some("opposition") {
        return false;
    }

    object_pair_from_aspect_signal(signal).is_some_and(|pair| structural_axis_pairs.contains(&pair))
}

fn is_angle_to_angle_aspect_signal(
    signal: &BasicSignal,
    angle_object_codes: &HashSet<String>,
) -> bool {
    if !signal.signal_key.starts_with("aspect:") {
        return false;
    }

    object_pair_from_aspect_signal(signal).is_some_and(|(source, target)| {
        angle_object_codes.contains(&source) && angle_object_codes.contains(&target)
    })
}

fn aspect_code(signal: &BasicSignal) -> Option<&str> {
    signal
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.get("aspect_code"))
        .and_then(|value| value.as_str())
        .or_else(|| signal.signal_key.split(':').nth(3))
}

fn object_pair_from_aspect_signal(signal: &BasicSignal) -> Option<(String, String)> {
    let evidence_pair = signal.evidence.as_ref().and_then(|evidence| {
        let source = evidence
            .get("source_object_code")
            .and_then(|value| value.as_str())?;
        let target = evidence
            .get("target_object_code")
            .and_then(|value| value.as_str())?;
        Some(normalized_pair(source, target))
    });
    if evidence_pair.is_some() {
        return evidence_pair;
    }

    let parts = signal.signal_key.split(':').collect::<Vec<_>>();
    if parts.len() >= 4 {
        Some(normalized_pair(parts[1], parts[2]))
    } else {
        None
    }
}

fn normalized_pair(left: &str, right: &str) -> (String, String) {
    if left <= right {
        (left.to_string(), right.to_string())
    } else {
        (right.to_string(), left.to_string())
    }
}

fn aspect_context_str<'a>(signal: &'a BasicSignal, key: &str) -> Option<&'a str> {
    signal
        .aspect_context
        .as_ref()
        .and_then(|context| context.get(key))
        .and_then(|value| value.as_str())
}

fn aspect_strength_score(signal: &BasicSignal) -> f64 {
    signal
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.get("strength_score"))
        .and_then(|value| value.as_f64())
        .unwrap_or(signal.priority_score / 80.0)
}

fn object_source_weight(object_code: &str) -> f64 {
    match object_code {
        "sun" | "moon" | "ascendant" => 1.0,
        "mc" => 0.8,
        "mercury" | "venus" | "mars" => 0.75,
        "jupiter" | "saturn" => 0.6,
        "descendant" | "ic" => 0.4,
        _ => 0.35,
    }
}

fn house_theme_code(position: &ObjectPositionFact) -> Option<String> {
    position_context(position, "house_context").and_then(|context| {
        context
            .get("theme_code")
            .and_then(|value| value.as_str())
            .map(ToString::to_string)
    })
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
        primary_signal_keys: source_signal_keys.clone(),
        source_signal_keys,
        secondary_slot_candidates: Vec::new(),
    });
}

fn finalize_reading_plan(plan: &mut Vec<BasicReadingPlanItem>) {
    let mut primary_slots: Vec<(String, String)> = Vec::new();

    for item in plan.iter_mut() {
        let mut primary_signal_keys = Vec::new();
        let mut secondary_slot_candidates = Vec::new();

        for signal_key in item.source_signal_keys.drain(..) {
            if let Some((_, primary_slot)) = primary_slots
                .iter()
                .find(|(assigned_key, _)| assigned_key == &signal_key)
            {
                secondary_slot_candidates.push(BasicSecondarySlotCandidate {
                    signal_key,
                    primary_slot: primary_slot.clone(),
                    candidate_slot: item.slot.clone(),
                });
            } else {
                primary_slots.push((signal_key.clone(), item.slot.clone()));
                primary_signal_keys.push(signal_key);
            }
        }

        item.source_signal_keys = primary_signal_keys.clone();
        item.primary_signal_keys = primary_signal_keys;
        item.secondary_slot_candidates = secondary_slot_candidates;
    }

    plan.retain(|item| !item.source_signal_keys.is_empty());
}

fn signal_keys_for_objects(
    signals: &[BasicSignal],
    object_codes: &[&str],
    limit: usize,
) -> Vec<String> {
    let mut keys = Vec::new();
    let mut selected_objects = 0;

    for object_code in object_codes {
        if let Some(signal_key) = position_signal_key_for_object(signals, object_code) {
            keys.push(signal_key);
            selected_objects += 1;
        }
        keys.extend(dignity_signal_keys_for_object(signals, object_code));
        if selected_objects >= limit {
            break;
        }
    }

    dedupe_strings(&mut keys);
    keys
}

fn position_signal_key_for_object(signals: &[BasicSignal], object_code: &str) -> Option<String> {
    let object_position_key = format!("object_position:{object_code}");
    if signals
        .iter()
        .any(|signal| signal.signal_key == object_position_key)
    {
        return Some(object_position_key);
    }

    let angle_prefix = format!("angle:{object_code}:sign:");
    signals
        .iter()
        .find(|signal| signal.signal_key.starts_with(&angle_prefix))
        .map(|signal| signal.signal_key.clone())
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
    chart_emphasis: &BasicChartEmphasis,
) -> Vec<BasicDraftingPlanItem> {
    let has_dominant_cluster = reading_plan
        .iter()
        .any(|item| item.slot == "dominant_cluster");

    reading_plan
        .iter()
        .map(|item| {
            let source_signals = signals_for_keys(signals, &item.source_signal_keys);
            BasicDraftingPlanItem {
                slot: item.slot.clone(),
                section_title: section_title(item, &source_signals),
                source_signal_keys: item.source_signal_keys.clone(),
                primary_signal_keys: item.primary_signal_keys.clone(),
                secondary_slot_candidates: item.secondary_slot_candidates.clone(),
                emphasis_refs: emphasis_refs_for_slot(
                    item,
                    &source_signals,
                    chart_emphasis,
                    has_dominant_cluster,
                ),
                writing_objective: writing_objective(item, &source_signals, has_dominant_cluster),
                max_words: max_words_for_slot(&item.slot),
                avoid: avoid_rules_for_slot(&item.slot),
            }
        })
        .collect()
}

fn emphasis_refs_for_slot(
    item: &BasicReadingPlanItem,
    signals: &[&BasicSignal],
    chart_emphasis: &BasicChartEmphasis,
    has_dominant_cluster: bool,
) -> BasicEmphasisRefs {
    let should_attach =
        item.slot == "dominant_cluster" || (item.slot == "core_identity" && !has_dominant_cluster);
    if !should_attach {
        return BasicEmphasisRefs::default();
    }

    let (dominant_signs, dominant_houses) = if item.slot == "dominant_cluster" {
        let cluster_signs = cluster_sign_refs(signals);
        let cluster_houses = cluster_house_refs(signals);
        (
            filtered_or_all_sign_refs(chart_emphasis, &cluster_signs),
            filtered_or_all_house_refs(chart_emphasis, &cluster_houses),
        )
    } else {
        (
            chart_emphasis
                .dominant_signs
                .iter()
                .map(|entry| entry.sign_code.clone())
                .collect(),
            chart_emphasis
                .dominant_houses
                .iter()
                .map(|entry| entry.house_number)
                .collect(),
        )
    };

    let slot_objects = emphasis_object_scope(item);
    let dominant_objects = if slot_objects.is_empty() {
        chart_emphasis
            .dominant_objects
            .iter()
            .map(|entry| entry.object_code.clone())
            .collect()
    } else {
        chart_emphasis
            .dominant_objects
            .iter()
            .filter(|entry| slot_objects.contains(&entry.object_code))
            .map(|entry| entry.object_code.clone())
            .collect()
    };

    BasicEmphasisRefs {
        dominant_signs,
        dominant_houses,
        dominant_objects,
    }
}

fn cluster_sign_refs(signals: &[&BasicSignal]) -> Vec<String> {
    signals
        .iter()
        .filter(|signal| signal.signal_key.starts_with("cluster:"))
        .filter_map(|signal| {
            signal
                .evidence
                .as_ref()
                .and_then(|evidence| evidence.get("sign_code"))
                .and_then(|value| value.as_str())
                .map(ToString::to_string)
        })
        .collect()
}

fn cluster_house_refs(signals: &[&BasicSignal]) -> Vec<i32> {
    signals
        .iter()
        .filter(|signal| signal.signal_key.starts_with("cluster:"))
        .filter_map(|signal| {
            signal
                .evidence
                .as_ref()
                .and_then(|evidence| evidence.get("house_number"))
                .and_then(|value| value.as_i64())
                .and_then(|value| i32::try_from(value).ok())
        })
        .collect()
}

fn filtered_or_all_sign_refs(
    chart_emphasis: &BasicChartEmphasis,
    allowed_signs: &[String],
) -> Vec<String> {
    let refs = chart_emphasis
        .dominant_signs
        .iter()
        .filter(|entry| allowed_signs.contains(&entry.sign_code))
        .map(|entry| entry.sign_code.clone())
        .collect::<Vec<_>>();
    if refs.is_empty() {
        chart_emphasis
            .dominant_signs
            .iter()
            .map(|entry| entry.sign_code.clone())
            .collect()
    } else {
        refs
    }
}

fn filtered_or_all_house_refs(
    chart_emphasis: &BasicChartEmphasis,
    allowed_houses: &[i32],
) -> Vec<i32> {
    let refs = chart_emphasis
        .dominant_houses
        .iter()
        .filter(|entry| allowed_houses.contains(&entry.house_number))
        .map(|entry| entry.house_number)
        .collect::<Vec<_>>();
    if refs.is_empty() {
        chart_emphasis
            .dominant_houses
            .iter()
            .map(|entry| entry.house_number)
            .collect()
    } else {
        refs
    }
}

fn emphasis_object_scope(item: &BasicReadingPlanItem) -> Vec<String> {
    let mut object_codes = Vec::new();
    for signal_key in item.source_signal_keys.iter().chain(
        item.secondary_slot_candidates
            .iter()
            .map(|candidate| &candidate.signal_key),
    ) {
        if let Some(object_code) = object_code_from_signal_key(signal_key) {
            if !object_codes.contains(&object_code) {
                object_codes.push(object_code);
            }
        }
    }
    object_codes
}

fn object_code_from_signal_key(signal_key: &str) -> Option<String> {
    if let Some(object_code) = signal_key.strip_prefix("object_position:") {
        return Some(object_code.to_string());
    }
    if let Some(tail) = signal_key.strip_prefix("angle:") {
        return tail
            .split(':')
            .next()
            .filter(|object_code| !object_code.is_empty())
            .map(ToString::to_string);
    }
    signal_key
        .strip_prefix("dignity:")
        .and_then(|tail| tail.split(':').next())
        .filter(|object_code| !object_code.is_empty())
        .map(ToString::to_string)
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

fn writing_objective(
    item: &BasicReadingPlanItem,
    signals: &[&BasicSignal],
    has_dominant_cluster: bool,
) -> String {
    match item.slot.as_str() {
        "core_identity" if has_dominant_cluster => {
            "Explain the central identity markers, emotional needs, and overall chart orientation in plain language, letting chart_emphasis only adjust relative weight without creating another section.".to_string()
        }
        "core_identity" => {
            "Explain the central identity markers, emotional needs, and overall chart orientation in plain language, using emphasis_refs as weighting context rather than as a separate topic.".to_string()
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

    let grouping_context = cluster_grouping_context(signals);

    format!(
        "Explain in plain language that the chart emphasizes {sign_name}, {house_name}, and {theme_text}, grouping the {grouping_context} instead of enumerating placements. Use emphasis_refs only as weighting context, not as a separate section."
    )
}

fn cluster_grouping_context(signals: &[&BasicSignal]) -> String {
    let dignity_objects = signals
        .iter()
        .filter(|signal| signal.signal_key.starts_with("dignity:"))
        .filter_map(|signal| {
            signal
                .evidence
                .as_ref()
                .and_then(|evidence| evidence.get("chart_object"))
                .and_then(|value| value.as_str())
                .map(capitalize_ascii)
        })
        .collect::<Vec<_>>();

    match dignity_objects.as_slice() {
        [] => "cluster evidence".to_string(),
        [object] => format!("cluster evidence and {object} dignity context"),
        _ => "cluster evidence and dignity contexts".to_string(),
    }
}

fn capitalize_ascii(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    format!(
        "{}{}",
        first.to_ascii_uppercase(),
        chars.as_str().to_ascii_lowercase()
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
        "turn chart_emphasis into a standalone section".to_string(),
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

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}
