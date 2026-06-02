use crate::dignities::{
    dignity_is_signal_worthy, essential_dignities_for_position, essential_dignities_for_positions,
    EssentialDignityFact,
};
use crate::domain::{
    BasicChartEmphasis, BasicDignity, BasicDominantHouse, BasicDominantObject, BasicDominantSign,
    BasicDraftingPlanItem, BasicEmphasisRefs, BasicLlmHandoffContract, BasicObjectPosition,
    BasicPayload, BasicReadingPlanItem, BasicSecondarySlotCandidate, BasicSignal, NatalChartInput,
    ObjectPositionFact,
};
use crate::models::InterpretationSignalRow;
use std::collections::HashMap;

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
            aspect_context: payload_value(signal, "aspect_context"),
            evidence: payload_value(signal, "evidence"),
        })
        .collect();

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
                house_modality: position_context(position, "house_modality"),
                object_context: position_context(position, "object_context"),
                motion_context: position_context(position, "motion_context"),
                dignity_context: position_dignity_context(position),
            })
            .collect(),
        dignities,
        chart_emphasis,
        signals: basic_signals,
        reading_plan,
        drafting_plan,
    }
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
        contract_version: "basic_natal_structured_v6".to_string(),
        payload_language_code: "en".to_string(),
        target_language_policy: "provided_by_llm_service".to_string(),
        audience_level: "beginner".to_string(),
        tone: "clear, warm, non fatalistic".to_string(),
        must_use: vec![
            "chart_emphasis".to_string(),
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
        &mut object_scores,
    );
    add_sign_emphasis_to_objects(positions, &sign_scores, &mut object_scores);

    BasicChartEmphasis {
        dominant_signs: normalized_signs(sign_scores),
        dominant_houses: normalized_houses(house_scores),
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
    object_scores: &mut HashMap<String, EmphasisScore>,
) {
    for signal in signals {
        if signal.signal_key.starts_with("cluster:") {
            add_cluster_emphasis(signal, sign_scores, house_scores, object_scores);
        } else if signal.signal_key.starts_with("aspect:") && aspect_strength_score(signal) >= 0.75
        {
            add_aspect_object_emphasis(signal, object_scores);
        }
    }
}

fn add_cluster_emphasis(
    signal: &BasicSignal,
    sign_scores: &mut HashMap<String, EmphasisScore>,
    house_scores: &mut HashMap<i32, EmphasisScore>,
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

fn normalized_houses(scores: HashMap<i32, EmphasisScore>) -> Vec<BasicDominantHouse> {
    let mut values: Vec<_> = scores
        .into_iter()
        .filter(|(_, entry)| entry.score > 0.0)
        .map(|(house_number, entry)| BasicDominantHouse {
            house_number,
            theme_code: house_theme_code(house_number).to_string(),
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

    finalize_reading_plan(&mut plan);
    plan
}

fn main_dynamic_aspect_keys(signals: &[BasicSignal]) -> Vec<String> {
    let mut keys: Vec<String> = signals
        .iter()
        .filter(|signal| signal.signal_key.starts_with("aspect:"))
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
            .filter(|signal| signal.signal_key.starts_with("aspect:"))
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
            .filter(|signal| signal.signal_key.starts_with("aspect:"))
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
    matches!(aspect_context_str(signal, "dynamic_quality"), Some("flow"))
        || matches!(
            aspect_context_str(signal, "primary_valence"),
            Some("supportive" | "harmonious")
        )
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
        "sun" | "moon" => 1.0,
        "mercury" | "venus" | "mars" => 0.75,
        "jupiter" | "saturn" => 0.6,
        _ => 0.35,
    }
}

fn house_theme_code(house_number: i32) -> &'static str {
    match house_number {
        1 => "identity",
        2 => "resources",
        3 => "communication",
        4 => "roots",
        5 => "creativity",
        6 => "work_health",
        7 => "relationships",
        8 => "shared_resources",
        9 => "beliefs",
        10 => "career",
        11 => "community",
        12 => "inner_world",
        _ => "object_position",
    }
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

fn finalize_reading_plan(plan: &mut [BasicReadingPlanItem]) {
    let mut primary_slots: Vec<(String, String)> = Vec::new();

    for item in plan {
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
}

fn signal_keys_for_objects(
    signals: &[BasicSignal],
    object_codes: &[&str],
    limit: usize,
) -> Vec<String> {
    let mut keys = Vec::new();
    let mut selected_objects = 0;

    for object_code in object_codes {
        let signal_key = format!("object_position:{object_code}");
        if signals.iter().any(|signal| signal.signal_key == signal_key) {
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

    fn capricorn_house_2_position(
        chart_object_id: i32,
        object_code: &str,
        object_name: &str,
    ) -> ObjectPositionFact {
        ObjectPositionFact {
            chart_object_id,
            object_code: object_code.to_string(),
            object_name: object_name.to_string(),
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
            longitude_deg: 270.0 + chart_object_id as f64,
            latitude_deg: None,
            apparent_speed_deg_per_day: Some(1.0),
            altitude_deg: None,
            is_visible: None,
            facts_json: Some(json!({
                "sign_context": {
                    "element": "earth",
                    "modality": "cardinal",
                    "polarity": "yin"
                },
                "house_modality": {"code": "succedent"},
                "object_context": {"role": "planet"},
                "motion_context": {"motion_state": "direct"}
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
            payload.drafting_plan[0].emphasis_refs.dominant_signs,
            vec!["gemini"]
        );
        assert_eq!(
            payload.drafting_plan[0].emphasis_refs.dominant_houses,
            vec![9]
        );
        assert_eq!(
            payload.drafting_plan[0].emphasis_refs.dominant_objects,
            vec!["sun"]
        );
        assert_eq!(
            payload
                .llm_handoff_contract
                .as_ref()
                .expect("llm handoff contract")
                .contract_version,
            "basic_natal_structured_v6"
        );
        let contract = payload
            .llm_handoff_contract
            .as_ref()
            .expect("llm handoff contract");
        assert!(contract.must_use.contains(&"chart_emphasis".to_string()));
        assert!(contract.must_not.contains(
            &"treat chart_emphasis as a standalone section instead of weighting context"
                .to_string()
        ));
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
        assert_eq!(
            payload.positions[0]
                .dignity_context
                .as_array()
                .map(Vec::len),
            Some(0)
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

        let payload = build_basic_payload(
            42,
            &input(),
            &[capricorn_house_2_position(1, "sun", "Sun")],
            &signals,
        );
        let cluster_plan = payload
            .reading_plan
            .iter()
            .find(|item| item.slot == "dominant_cluster")
            .expect("expected dominant cluster plan item");

        assert_eq!(
            cluster_plan.source_signal_keys,
            vec!["cluster:capricorn:house_2"]
        );
        assert_eq!(
            cluster_plan.primary_signal_keys,
            vec!["cluster:capricorn:house_2"]
        );
        assert!(cluster_plan
            .secondary_slot_candidates
            .iter()
            .any(|candidate| {
                candidate.signal_key == "object_position:sun"
                    && candidate.primary_slot == "core_identity"
                    && candidate.candidate_slot == "dominant_cluster"
            }));
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
            cluster_plan.source_signal_keys
        );
        assert_eq!(
            cluster_drafting.primary_signal_keys,
            cluster_plan.primary_signal_keys
        );
        assert_eq!(
            cluster_drafting.secondary_slot_candidates,
            cluster_plan.secondary_slot_candidates
        );
        assert_eq!(
            cluster_drafting.section_title,
            "A Capricorn dominant theme around Resources"
        );
        assert_eq!(
            cluster_drafting.emphasis_refs.dominant_signs,
            vec!["capricorn"]
        );
        assert_eq!(cluster_drafting.emphasis_refs.dominant_houses, vec![2]);
        assert!(cluster_drafting
            .emphasis_refs
            .dominant_objects
            .contains(&"sun".to_string()));
        let core_drafting = payload
            .drafting_plan
            .iter()
            .find(|item| item.slot == "core_identity")
            .expect("expected core identity drafting item");
        assert!(core_drafting.emphasis_refs.dominant_signs.is_empty());
        assert!(core_drafting.emphasis_refs.dominant_houses.is_empty());
        assert!(core_drafting.emphasis_refs.dominant_objects.is_empty());
        assert!(cluster_drafting
            .avoid
            .contains(&"repeat each placement one by one".to_string()));
    }

    #[test]
    fn basic_payload_exposes_chart_emphasis_summary() {
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
                    "semantic_tags": ["cluster", "capricorn", "house_2", "resources"],
                    "source_weight": 2.35,
                    "aggregation_group": "capricorn_house_2_cluster",
                    "writing_guidance": "guidance",
                    "evidence": {
                        "fact_type": "position_cluster",
                        "cluster_type": "sign_house",
                        "sign_code": "capricorn",
                        "sign_name": "Capricorn",
                        "house_number": 2,
                        "house_name": "Resources",
                        "source_signals": [
                            "object_position:sun",
                            "object_position:saturn",
                            "object_position:mars"
                        ],
                        "source_objects": ["sun", "saturn", "mars"]
                    }
                })),
            },
            placement_signal_row(2, "object_position:sun", "sun"),
            placement_signal_row(3, "object_position:saturn", "saturn"),
            dignity_signal_row(4, "dignity:saturn:domicile:capricorn", "saturn"),
            aspect_signal(5, "aspect:sun:saturn:trine", "trine", 0.82),
        ];
        let positions = vec![
            capricorn_house_2_position(1, "sun", "Sun"),
            capricorn_house_2_position(7, "saturn", "Saturn"),
            capricorn_house_2_position(5, "mars", "Mars"),
        ];

        let payload = build_basic_payload(42, &input(), &positions, &signals);

        let dominant_sign = payload
            .chart_emphasis
            .dominant_signs
            .first()
            .expect("expected dominant sign");
        assert_eq!(dominant_sign.sign_code, "capricorn");
        assert!(dominant_sign.score >= 0.85);
        assert!(dominant_sign.score < 1.0);
        assert!(dominant_sign.reasons.contains(&"sun_in_sign".to_string()));
        assert!(dominant_sign
            .reasons
            .contains(&"saturn_domicile".to_string()));
        assert!(dominant_sign
            .reasons
            .contains(&"sign_house_cluster".to_string()));
        assert!(dominant_sign
            .reasons
            .contains(&"multiple_objects".to_string()));

        let dominant_house = payload
            .chart_emphasis
            .dominant_houses
            .first()
            .expect("expected dominant house");
        assert_eq!(dominant_house.house_number, 2);
        assert_eq!(dominant_house.theme_code, "resources");
        assert!(dominant_house.reasons.contains(&"sun_in_house".to_string()));
        assert!(dominant_house.reasons.contains(&"cluster".to_string()));

        let saturn = payload
            .chart_emphasis
            .dominant_objects
            .iter()
            .find(|entry| entry.object_code == "saturn")
            .expect("expected saturn emphasis");
        assert!(saturn.score > 0.0);
        assert!(saturn.reasons.contains(&"domicile".to_string()));
        assert!(saturn.reasons.contains(&"cluster_participant".to_string()));
        assert!(saturn.reasons.contains(&"capricorn_emphasis".to_string()));
        assert!(saturn
            .reasons
            .contains(&"strong_aspect_participant".to_string()));
    }

    #[test]
    fn chart_emphasis_omits_placement_only_objects_when_stronger_evidence_exists() {
        let signals = vec![
            placement_signal_row(1, "object_position:sun", "sun"),
            placement_signal_row(2, "object_position:moon", "moon"),
            placement_signal_row(3, "object_position:mercury", "mercury"),
            dignity_signal_row(4, "dignity:mercury:domicile:gemini", "mercury"),
        ];
        let positions = vec![
            position(),
            ObjectPositionFact {
                chart_object_id: 2,
                object_code: "moon".to_string(),
                object_name: "Moon".to_string(),
                zodiacal_reference_system_id: 1,
                coordinate_reference_system_id: 1,
                sign_id: 7,
                sign_code: "libra".to_string(),
                sign_name: "Libra".to_string(),
                house_id: Some(1),
                house_number: Some(1),
                house_name: Some("Self".to_string()),
                motion_state_id: Some(1),
                horizon_position_id: None,
                longitude_deg: 180.0,
                latitude_deg: None,
                apparent_speed_deg_per_day: Some(12.0),
                altitude_deg: None,
                is_visible: None,
                facts_json: Some(json!({
                    "sign_context": {"element": "air", "modality": "cardinal", "polarity": "yang"},
                    "house_modality": {"code": "angular"},
                    "object_context": {"role": "luminary"},
                    "motion_context": {"motion_state": "direct"}
                })),
            },
            ObjectPositionFact {
                chart_object_id: 3,
                object_code: "mercury".to_string(),
                object_name: "Mercury".to_string(),
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
                longitude_deg: 70.0,
                latitude_deg: None,
                apparent_speed_deg_per_day: Some(1.0),
                altitude_deg: None,
                is_visible: None,
                facts_json: Some(json!({
                    "sign_context": {"element": "air", "modality": "mutable", "polarity": "yang"},
                    "house_modality": {"code": "cadent"},
                    "object_context": {"role": "planet"},
                    "motion_context": {"motion_state": "direct"}
                })),
            },
        ];

        let payload = build_basic_payload(42, &input(), &positions, &signals);

        assert!(payload
            .chart_emphasis
            .dominant_objects
            .iter()
            .any(|entry| entry.object_code == "mercury"));
        assert!(!payload
            .chart_emphasis
            .dominant_objects
            .iter()
            .any(|entry| entry.object_code == "moon"
                && entry.reasons == vec!["placement".to_string()]));
    }

    #[test]
    fn drafting_emphasis_refs_scope_objects_to_the_receiving_slot() {
        let signals = vec![
            InterpretationSignalRow {
                id: 1,
                signal_key: "cluster:gemini:house_9".to_string(),
                theme_code: Some("beliefs".to_string()),
                title: "Strong concentration in Gemini, house 9".to_string(),
                summary: Some("summary".to_string()),
                priority_score: 99.0,
                confidence_score: Some(0.9),
                payload_json: Some(json!({
                    "interpretive_hint": "hint",
                    "semantic_tags": ["cluster", "gemini", "house_9"],
                    "source_weight": 2.35,
                    "aggregation_group": "gemini_house_9_cluster",
                    "writing_guidance": "guidance",
                    "evidence": {
                        "fact_type": "position_cluster",
                        "cluster_type": "sign_house",
                        "sign_code": "gemini",
                        "sign_name": "Gemini",
                        "house_number": 9,
                        "house_name": "Beliefs",
                        "source_signals": [
                            "object_position:sun",
                            "object_position:mercury",
                            "object_position:jupiter"
                        ],
                        "source_objects": ["sun", "mercury", "jupiter"]
                    }
                })),
            },
            placement_signal_row(2, "object_position:sun", "sun"),
            placement_signal_row(3, "object_position:mercury", "mercury"),
            placement_signal_row(4, "object_position:jupiter", "jupiter"),
            placement_signal_row(5, "object_position:mars", "mars"),
            dignity_signal_row(6, "dignity:mercury:domicile:gemini", "mercury"),
            dignity_signal_row(7, "dignity:mars:detriment:taurus", "mars"),
        ];
        let positions = vec![
            ObjectPositionFact {
                object_code: "sun".to_string(),
                object_name: "Sun".to_string(),
                ..position()
            },
            ObjectPositionFact {
                chart_object_id: 3,
                object_code: "mercury".to_string(),
                object_name: "Mercury".to_string(),
                longitude_deg: 70.0,
                ..position()
            },
            ObjectPositionFact {
                chart_object_id: 6,
                object_code: "jupiter".to_string(),
                object_name: "Jupiter".to_string(),
                longitude_deg: 80.0,
                ..position()
            },
            ObjectPositionFact {
                chart_object_id: 5,
                object_code: "mars".to_string(),
                object_name: "Mars".to_string(),
                sign_id: 2,
                sign_code: "taurus".to_string(),
                sign_name: "Taurus".to_string(),
                house_id: Some(8),
                house_number: Some(8),
                house_name: Some("Transformation".to_string()),
                longitude_deg: 45.0,
                ..position()
            },
        ];

        let payload = build_basic_payload(42, &input(), &positions, &signals);
        let cluster_drafting = payload
            .drafting_plan
            .iter()
            .find(|item| item.slot == "dominant_cluster")
            .expect("expected dominant cluster drafting item");

        assert!(payload
            .chart_emphasis
            .dominant_objects
            .iter()
            .any(|entry| entry.object_code == "mars"));
        assert!(!cluster_drafting
            .emphasis_refs
            .dominant_objects
            .contains(&"mars".to_string()));
    }

    #[test]
    fn chart_emphasis_scores_do_not_overstate_weak_distributions() {
        let signals = vec![placement_signal_row(1, "object_position:sun", "sun")];
        let payload = build_basic_payload(42, &input(), &[position()], &signals);

        let dominant_sign = payload
            .chart_emphasis
            .dominant_signs
            .first()
            .expect("expected fallback dominant sign");
        let dominant_house = payload
            .chart_emphasis
            .dominant_houses
            .first()
            .expect("expected fallback dominant house");
        let dominant_object = payload
            .chart_emphasis
            .dominant_objects
            .first()
            .expect("expected fallback dominant object");

        assert_eq!(dominant_sign.sign_code, "gemini");
        assert_eq!(dominant_house.house_number, 9);
        assert_eq!(dominant_object.object_code, "sun");
        assert!(dominant_sign.score < 0.35);
        assert!(dominant_house.score < 0.35);
        assert!(dominant_object.score < 0.5);
        assert_eq!(dominant_object.reasons, vec!["placement"]);
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
                .as_array()
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
        assert!(!background_plan
            .source_signal_keys
            .contains(&"dignity:saturn:domicile:capricorn".to_string()));
        assert!(background_plan
            .secondary_slot_candidates
            .iter()
            .any(|candidate| {
                candidate.signal_key == "dignity:saturn:domicile:capricorn"
                    && candidate.primary_slot == "dominant_cluster"
                    && candidate.candidate_slot == "background_factors"
            }));
        assert!(background_plan
            .source_signal_keys
            .contains(&"dignity:jupiter:exaltation:cancer".to_string()));

        let background_drafting = payload
            .drafting_plan
            .iter()
            .find(|item| item.slot == "background_factors")
            .expect("expected background drafting plan");
        assert_eq!(
            background_drafting.secondary_slot_candidates,
            background_plan.secondary_slot_candidates
        );
    }

    #[test]
    fn reading_plan_object_limits_do_not_count_dignity_sources() {
        let signals = vec![
            placement_signal_row(1, "object_position:mercury", "mercury"),
            dignity_signal_row(2, "dignity:mercury:domicile:virgo", "mercury"),
            dignity_signal_row(3, "dignity:mercury:exaltation:virgo", "mercury"),
            placement_signal_row(4, "object_position:venus", "venus"),
            placement_signal_row(5, "object_position:mars", "mars"),
        ];

        let payload = build_basic_payload(42, &input(), &[position()], &signals);
        let expression_plan = payload
            .reading_plan
            .iter()
            .find(|item| item.slot == "expression_style")
            .expect("expected expression style plan");

        assert!(expression_plan
            .source_signal_keys
            .contains(&"object_position:mercury".to_string()));
        assert!(expression_plan
            .source_signal_keys
            .contains(&"dignity:mercury:domicile:virgo".to_string()));
        assert!(expression_plan
            .source_signal_keys
            .contains(&"dignity:mercury:exaltation:virgo".to_string()));
        assert!(expression_plan
            .source_signal_keys
            .contains(&"object_position:venus".to_string()));
        assert!(expression_plan
            .source_signal_keys
            .contains(&"object_position:mars".to_string()));
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

    #[test]
    fn main_dynamic_aspects_balance_support_and_tension_by_valence() {
        let signals = vec![
            aspect_signal(1, "aspect:sun:neptune:conjunction", "conjunction", 0.99),
            aspect_signal(2, "aspect:moon:pluto:conjunction", "conjunction", 0.98),
            aspect_signal(3, "aspect:mars:saturn:conjunction", "conjunction", 0.97),
            aspect_signal(4, "aspect:moon:mars:square", "square", 0.86),
            aspect_signal(5, "aspect:venus:jupiter:sextile", "sextile", 0.84),
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
        assert!(aspect_plan
            .source_signal_keys
            .contains(&"aspect:venus:jupiter:sextile".to_string()));
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
                "aspect_context": {
                    "aspect_family": "major",
                    "primary_valence": primary_valence_for_test(aspect_code),
                    "intensity_modifier": intensity_modifier_for_test(aspect_code),
                    "secondary_effect": null,
                    "dynamic_quality": dynamic_quality_for_test(aspect_code),
                    "phase_state": "applying",
                    "writing_guidance": "guidance"
                },
                "evidence": {
                    "fact_type": "aspect",
                    "aspect_code": aspect_code,
                    "aspect_name": aspect_code,
                    "strength_score": strength_score
                }
            })),
        }
    }

    fn primary_valence_for_test(aspect_code: &str) -> Option<&'static str> {
        match aspect_code {
            "sextile" => Some("supportive"),
            "square" => Some("dynamic_challenging"),
            "trine" => Some("harmonious"),
            "opposition" => Some("polarizing"),
            _ => None,
        }
    }

    fn intensity_modifier_for_test(aspect_code: &str) -> Option<&'static str> {
        match aspect_code {
            "conjunction" => Some("amplifying"),
            _ => None,
        }
    }

    fn dynamic_quality_for_test(aspect_code: &str) -> &'static str {
        match aspect_code {
            "sextile" | "trine" => "flow",
            "square" | "opposition" => "tension",
            "conjunction" => "intensification",
            _ => "contextual",
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

    fn placement_signal_row(
        id: i32,
        signal_key: &str,
        object_code: &str,
    ) -> InterpretationSignalRow {
        InterpretationSignalRow {
            id,
            signal_key: signal_key.to_string(),
            theme_code: Some("object_position".to_string()),
            title: format!("{object_code} placement"),
            summary: Some("summary".to_string()),
            priority_score: 85.0,
            confidence_score: Some(0.95),
            payload_json: Some(json!({
                "interpretive_hint": "hint",
                "semantic_tags": ["placement", object_code],
                "source_weight": 0.75,
                "aggregation_group": object_code,
                "writing_guidance": "guidance",
                "evidence": {
                    "fact_type": "object_position",
                    "object_code": object_code
                }
            })),
        }
    }
}
