use std::collections::HashMap;

use serde_json::json;

use crate::dignities::{
    dignity_is_signal_worthy, dignity_priority_delta_for_position, dignity_source_weight_delta,
    dignity_source_weight_delta_for_position, essential_dignities_for_position,
    essential_dignities_for_positions, EssentialDignityFact,
};
use crate::domain::{CalculatedChartFacts, InterpretationSignalDraft, ObjectPositionFact};

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
        let dignities = essential_dignities_for_position(position);
        let semantic_tags = position_semantic_tags(position);
        let source_weight = round4(
            object_source_weight(&position.object_code)
                + dignity_source_weight_delta_for_position(position),
        );
        let theme_code = position_theme_code(position);
        let aggregation_group = position_aggregation_group(position);
        let dignity_summary = dignity_summary_for_position(&dignities);
        let motion_summary = retrograde_summary(position);

        signals.push(InterpretationSignalDraft {
            signal_key: format!("object_position:{}", position.object_code),
            signal_type_id: None,
            theme_code: Some(theme_code.to_string()),
            title: format!(
                "{} in {}{}",
                position.object_name, position.sign_name, house_suffix
            ),
            summary: Some(format!(
                "{} is placed in {}{}, emphasizing this chart factor through a concrete, readable placement.{}{}",
                position.object_name,
                position.sign_name,
                summary_house,
                dignity_summary,
                motion_summary
            )),
            priority_score: position_priority(position),
            confidence_score: Some(0.95),
            suppression_state: "active".to_string(),
            payload_json: Some(json!({
                "interpretive_hint": position_interpretive_hint(position),
                "semantic_tags": semantic_tags,
                "source_weight": source_weight,
                "aggregation_group": aggregation_group,
                "writing_guidance": position_writing_guidance(position, &dignities),
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
                    "longitude_deg": position.longitude_deg,
                    "placement_context": placement_context(position),
                    "essential_dignities": dignity_evidence_array(&dignities)
                }
            })),
        });
    }

    add_dignity_signals(facts, &mut signals);

    for aspect in &facts.aspects {
        let strength_score = aspect.strength_score.unwrap_or(0.5);
        let suppression_state = if strength_score >= BASIC_ASPECT_MIN_STRENGTH {
            "active"
        } else {
            "suppressed"
        };
        let aspect_name = aspect.aspect_name.to_lowercase();
        let article = indefinite_article(&aspect_name);
        let aspect_context = aspect_context(aspect);

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
                "interpretive_hint": format!(
                    "{} and {} are connected by {} {}, so their functions should be read together with attention to the {} phase.",
                    aspect.source_object_name,
                    aspect.target_object_name,
                    article,
                    aspect_name,
                    aspect.phase_state
                ),
                "semantic_tags": aspect_semantic_tags(aspect, strength_score),
                "source_weight": round4(
                    object_source_weight(&aspect.source_object_code)
                        + object_source_weight(&aspect.target_object_code)
                ),
                "aggregation_group": format!("aspect:{}", aspect.aspect_code),
                "writing_guidance": aspect_writing_guidance(aspect),
                "aspect_context": aspect_context,
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
                    "aspect_family": aspect.aspect_family,
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

    add_position_cluster_signals(facts, &mut signals);

    signals.sort_by(|left, right| {
        right
            .priority_score
            .partial_cmp(&left.priority_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    suppress_over_basic_limit(&mut signals);
    for _ in 0..BASIC_MAX_ACTIVE_SIGNALS {
        if !apply_cluster_source_deduplication(&mut signals) {
            break;
        }
        fill_basic_active_limit(&mut signals);
    }
    preserve_strong_tension_aspect(&mut signals);
    signals
}

fn add_dignity_signals(facts: &CalculatedChartFacts, signals: &mut Vec<InterpretationSignalDraft>) {
    for dignity in essential_dignities_for_positions(&facts.positions)
        .into_iter()
        .filter(dignity_is_signal_worthy)
    {
        let theme_code = if dignity.polarity == "dignity" {
            "functional_strength"
        } else {
            "functional_challenge"
        };
        let title = dignity_title(&dignity);
        let summary = dignity_summary(&dignity);

        signals.push(InterpretationSignalDraft {
            signal_key: format!(
                "dignity:{}:{}:{}",
                dignity.object_code, dignity.dignity_type, dignity.sign_code
            ),
            signal_type_id: None,
            theme_code: Some(theme_code.to_string()),
            title,
            summary: Some(summary),
            priority_score: dignity_priority(&dignity),
            confidence_score: Some(0.95),
            suppression_state: "active".to_string(),
            payload_json: Some(json!({
                "interpretive_hint": dignity_interpretive_hint(&dignity),
                "semantic_tags": dignity_semantic_tags(&dignity),
                "source_weight": round4(
                    object_source_weight(&dignity.object_code)
                        + dignity_source_weight_delta(&dignity)
                ),
                "aggregation_group": format!("dignity:{}", dignity.object_code),
                "writing_guidance": dignity_writing_guidance(&dignity),
                "evidence": dignity_evidence(&dignity)
            })),
        });
    }
}

fn add_position_cluster_signals(
    facts: &CalculatedChartFacts,
    signals: &mut Vec<InterpretationSignalDraft>,
) {
    let mut sign_house_groups: HashMap<(String, i32), Vec<&ObjectPositionFact>> = HashMap::new();

    for position in &facts.positions {
        let Some(house_number) = position.house_number else {
            continue;
        };
        sign_house_groups
            .entry((position.sign_code.clone(), house_number))
            .or_default()
            .push(position);
    }

    let mut groups: Vec<_> = sign_house_groups
        .into_iter()
        .filter(|(_, positions)| positions.len() >= 3)
        .collect();
    groups.sort_by(|left, right| {
        right
            .1
            .len()
            .cmp(&left.1.len())
            .then_with(|| left.0.cmp(&right.0))
    });

    for ((sign_code, house_number), mut positions) in groups {
        positions.sort_by(|left, right| {
            position_priority(right)
                .partial_cmp(&position_priority(left))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.object_code.cmp(&right.object_code))
        });

        let sign_name = positions[0].sign_name.clone();
        let Some(house_name) = positions[0].house_name.clone() else {
            continue;
        };
        let source_signals: Vec<String> = positions
            .iter()
            .map(|position| format!("object_position:{}", position.object_code))
            .collect();
        let source_objects: Vec<String> = positions
            .iter()
            .map(|position| position.object_code.clone())
            .collect();
        let source_weight = round4(
            positions
                .iter()
                .map(|position| object_source_weight(&position.object_code))
                .sum(),
        );
        let priority_score =
            round4((90.0 + positions.len() as f64 * 1.5 + source_weight * 2.0).min(99.0));
        let aggregation_group = format!("{sign_code}_house_{house_number}_cluster");
        let semantic_tags = cluster_semantic_tags(&sign_code, house_number);

        signals.push(InterpretationSignalDraft {
            signal_key: format!("cluster:{sign_code}:house_{house_number}"),
            signal_type_id: None,
            theme_code: Some(house_theme_code(house_number).to_string()),
            title: format!("Strong concentration in {sign_name}, house {house_number}"),
            summary: Some(format!(
                "{} chart factors are concentrated in {sign_name} and the {house_name} house, giving extra interpretive weight to this area of the chart.",
                positions.len()
            )),
            priority_score,
            confidence_score: Some(0.9),
            suppression_state: "active".to_string(),
            payload_json: Some(json!({
                "interpretive_hint": format!(
                    "Read this as a repeated emphasis: {sign_name} qualities are focused through the themes of the {house_name} house."
                ),
                "semantic_tags": semantic_tags,
                "source_weight": source_weight,
                "aggregation_group": aggregation_group,
                "writing_guidance": "Use this cluster before individual placements and merge repeated wording from its source signals.",
                "evidence": {
                    "fact_type": "position_cluster",
                    "cluster_type": "sign_house",
                    "sign_code": sign_code,
                    "sign_name": sign_name,
                    "house_number": house_number,
                    "house_name": house_name,
                    "source_signals": source_signals,
                    "source_objects": source_objects
                }
            })),
        });
    }
}

fn apply_cluster_source_deduplication(signals: &mut [InterpretationSignalDraft]) -> bool {
    let mut source_to_cluster: HashMap<String, String> = HashMap::new();

    for signal in signals.iter() {
        if signal.suppression_state != "active" || !signal.signal_key.starts_with("cluster:") {
            continue;
        }

        let Some(source_signals) = signal
            .payload_json
            .as_ref()
            .and_then(|payload| payload.get("evidence"))
            .and_then(|evidence| evidence.get("source_signals"))
            .and_then(|value| value.as_array())
        else {
            continue;
        };

        for source_signal in source_signals {
            if let Some(source_signal) = source_signal.as_str() {
                source_to_cluster.insert(source_signal.to_string(), signal.signal_key.clone());
            }
        }
    }

    if source_to_cluster.is_empty() {
        return false;
    }

    let mut changed = false;
    for signal in signals.iter_mut() {
        let Some(cluster_key) = source_to_cluster.get(&signal.signal_key).cloned() else {
            continue;
        };

        let object_code = signal
            .payload_json
            .as_ref()
            .and_then(|payload| payload.get("evidence"))
            .and_then(|evidence| evidence.get("object_code"))
            .and_then(|value| value.as_str())
            .unwrap_or_default();

        if is_core_chart_object(object_code) {
            changed |= annotate_cluster_source(signal, &cluster_key, "kept");
        } else if signal.suppression_state != "merged" {
            signal.suppression_state = "merged".to_string();
            changed = true;
            changed |= annotate_cluster_source(signal, &cluster_key, "merged");
        }
    }

    changed
}

fn annotate_cluster_source(
    signal: &mut InterpretationSignalDraft,
    cluster_key: &str,
    editorial_state: &str,
) -> bool {
    let Some(payload) = signal
        .payload_json
        .as_mut()
        .and_then(|value| value.as_object_mut())
    else {
        return false;
    };

    let already_current = payload
        .get("editorial_state")
        .and_then(|state| state.get("state"))
        .and_then(|value| value.as_str())
        == Some(editorial_state)
        && payload
            .get("editorial_state")
            .and_then(|state| state.get("cluster_signal_key"))
            .and_then(|value| value.as_str())
            == Some(cluster_key);

    payload.insert(
        "editorial_state".to_string(),
        json!({
            "state": editorial_state,
            "reason": "source_signal_of_active_cluster",
            "cluster_signal_key": cluster_key
        }),
    );

    if editorial_state == "kept" {
        payload.insert(
            "writing_guidance".to_string(),
            json!("Keep this core placement, but draft it in relation to the active cluster to avoid repeating the same sign and house wording."),
        );
    } else {
        payload.insert(
            "writing_guidance".to_string(),
            json!("Do not draft this as a standalone Basic point; it is represented by the active cluster signal."),
        );
    }

    !already_current
}

fn is_core_chart_object(object_code: &str) -> bool {
    matches!(object_code, "sun" | "moon" | "ascendant" | "mc")
}

fn position_priority(position: &ObjectPositionFact) -> f64 {
    let base = match position.object_code.as_str() {
        "sun" | "moon" => 100.0,
        "mercury" | "venus" | "mars" => 85.0,
        "jupiter" | "saturn" => 75.0,
        _ => 60.0,
    };
    let dignity_delta = dignity_priority_delta_for_position(position);
    round4((base + house_modality_priority_delta(position) + dignity_delta).min(100.0))
}

fn house_modality_priority_delta(position: &ObjectPositionFact) -> f64 {
    match placement_context_value(position, "house_modality", "code")
        .and_then(|value| value.as_str())
    {
        Some("angular") => 2.0,
        Some("succedent") => 0.75,
        Some("cadent") => -0.75,
        _ => 0.0,
    }
}

fn object_source_weight(object_code: &str) -> f64 {
    match object_code {
        "sun" | "moon" => 1.0,
        "mercury" | "venus" | "mars" => 0.75,
        "jupiter" | "saturn" => 0.6,
        _ => 0.35,
    }
}

fn position_theme_code(position: &ObjectPositionFact) -> &'static str {
    position
        .house_number
        .map(house_theme_code)
        .unwrap_or("object_position")
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

fn position_aggregation_group(position: &ObjectPositionFact) -> String {
    match position.house_number {
        Some(house_number) => format!("{}:house_{}", position.sign_code, house_number),
        None => position.sign_code.clone(),
    }
}

fn position_interpretive_hint(position: &ObjectPositionFact) -> String {
    let base = match (position.house_name.as_deref(), position.house_number) {
        (Some(house_name), Some(_)) => format!(
            "{} expresses through {} qualities in the field of {}.",
            position.object_name, position.sign_name, house_name
        ),
        _ => format!(
            "{} expresses through {} qualities.",
            position.object_name, position.sign_name
        ),
    };

    let dignities = essential_dignities_for_position(position);
    if !dignities.is_empty() {
        format!(
            "{base} Its dignity context adds {}.{}",
            dignity_effect_phrase_for_position(&dignities),
            retrograde_hint(position)
        )
    } else {
        format!("{base}{}", retrograde_hint(position))
    }
}

fn position_semantic_tags(position: &ObjectPositionFact) -> Vec<String> {
    let mut tags = vec![
        "placement".to_string(),
        position.object_code.clone(),
        position.sign_code.clone(),
    ];
    tags.extend(sign_tags(&position.sign_code));
    if let Some(house_number) = position.house_number {
        tags.push(format!("house_{house_number}"));
        tags.push(house_theme_code(house_number).to_string());
        tags.extend(house_tags(house_number));
    }
    if let Some(element) = placement_context_str(position, "sign_context", "element") {
        tags.push(element.to_string());
    }
    if let Some(modality) = placement_context_str(position, "sign_context", "modality") {
        tags.push(modality.to_string());
    }
    if let Some(polarity) = placement_context_str(position, "sign_context", "polarity") {
        tags.push(polarity.to_string());
    }
    if let Some(house_modality) = placement_context_str(position, "house_modality", "code") {
        tags.push(house_modality.to_string());
    }
    if let Some(role) = placement_context_str(position, "object_context", "role") {
        tags.push(role.to_string());
    }
    if let Some(motion_state) = placement_context_str(position, "motion_context", "motion_state") {
        tags.push(motion_state.to_string());
    }
    for dignity in essential_dignities_for_position(position) {
        tags.extend(dignity_semantic_tags(&dignity));
    }
    dedupe_tags(tags)
}

fn placement_context(position: &ObjectPositionFact) -> serde_json::Value {
    json!({
        "sign_context": placement_context_object(position, "sign_context"),
        "house_modality": placement_context_object(position, "house_modality"),
        "object_context": placement_context_object(position, "object_context"),
        "motion_context": placement_context_object(position, "motion_context"),
        "dignity_context": dignity_evidence_array(&essential_dignities_for_position(position))
    })
}

fn position_writing_guidance(
    position: &ObjectPositionFact,
    dignities: &[EssentialDignityFact],
) -> String {
    match (!dignities.is_empty(), is_retrograde_position(position)) {
        (true, true) => format!(
            "Use this as a concise placement cue; include {} and retrograde motion as modifiers, not separate verdicts.",
            dignity_type_list(dignities)
        ),
        (true, false) => format!(
            "Use this as a concise placement cue and include {} as a modifier, not a separate verdict.",
            dignity_type_list(dignities)
        ),
        (false, true) => "Use this as a concise placement cue; treat retrograde motion as an inward, revising, or reflective modifier before drafting final text.".to_string(),
        (false, false) => "Use this as a concise placement cue; combine it with nearby cluster or aspect signals before drafting final text.".to_string(),
    }
}

fn retrograde_summary(position: &ObjectPositionFact) -> String {
    if is_retrograde_position(position) {
        " Its retrograde motion adds a reflective or revising layer to the placement.".to_string()
    } else {
        String::new()
    }
}

fn retrograde_hint(position: &ObjectPositionFact) -> String {
    if is_retrograde_position(position) {
        " Read the retrograde state as a modifier for pacing, review, and internal processing."
            .to_string()
    } else {
        String::new()
    }
}

fn is_retrograde_position(position: &ObjectPositionFact) -> bool {
    placement_context_str(position, "motion_context", "motion_state") == Some("retrograde")
}

fn dignity_summary_for_position(dignities: &[EssentialDignityFact]) -> String {
    if dignities.is_empty() {
        String::new()
    } else {
        format!(
            " Its dignity context adds {}.",
            dignity_effect_phrase_for_position(dignities)
        )
    }
}

fn dignity_effect_phrase_for_position(dignities: &[EssentialDignityFact]) -> String {
    let phrases = dignities
        .iter()
        .map(dignity_effect_phrase)
        .collect::<Vec<_>>();
    phrases.join(" and ")
}

fn dignity_type_list(dignities: &[EssentialDignityFact]) -> String {
    let dignity_types = dignities
        .iter()
        .map(|dignity| dignity.dignity_type.as_str())
        .collect::<Vec<_>>();

    match dignity_types.as_slice() {
        [] => "the dignity context".to_string(),
        [one] => format!("the {one} context"),
        [first, second] => format!("the {first} and {second} contexts"),
        _ => format!("the {} contexts", dignity_types.join(", ")),
    }
}

fn dignity_priority(dignity: &EssentialDignityFact) -> f64 {
    let base: f64 = match dignity.object_code.as_str() {
        "sun" | "moon" => 90.0,
        "mercury" | "venus" | "mars" => 86.0,
        "jupiter" | "saturn" => 82.0,
        _ => 72.0,
    };
    let type_delta: f64 = match dignity.dignity_type.as_str() {
        "domicile" => 6.0,
        "exaltation" => 4.0,
        "detriment" => 2.0,
        "fall" => 1.0,
        _ => 0.0,
    };
    round4((base + type_delta).min(95.0))
}

fn dignity_title(dignity: &EssentialDignityFact) -> String {
    if dignity.polarity == "dignity" {
        format!(
            "{} strongly placed in {}",
            dignity.object_name, dignity.sign_name
        )
    } else {
        format!(
            "{} under pressure in {}",
            dignity.object_name, dignity.sign_name
        )
    }
}

fn dignity_summary(dignity: &EssentialDignityFact) -> String {
    if dignity.polarity == "dignity" {
        format!(
            "{} is in {}, a sign where its function is reinforced by {}.",
            dignity.object_name, dignity.sign_name, dignity.dignity_type
        )
    } else {
        format!(
            "{} is in {}, a sign where its function needs more adjustment because of {}.",
            dignity.object_name, dignity.sign_name, dignity.dignity_type
        )
    }
}

fn dignity_interpretive_hint(dignity: &EssentialDignityFact) -> String {
    let article = indefinite_article(&dignity.dignity_type);
    format!(
        "Treat {} in {} as {} {} modifier for the existing placement signal.",
        dignity.object_name, dignity.sign_name, article, dignity.dignity_type
    )
}

fn dignity_writing_guidance(dignity: &EssentialDignityFact) -> String {
    if dignity.polarity == "dignity" {
        "Use this to strengthen the object's placement signal without overstating ease or outcome."
            .to_string()
    } else {
        "Use this as a contextual constraint on the object's placement signal without fatalistic wording."
            .to_string()
    }
}

fn dignity_effect_phrase(dignity: &EssentialDignityFact) -> &'static str {
    match dignity.dignity_type.as_str() {
        "domicile" => "functional strength, coherence, and self-command",
        "exaltation" => "heightened visibility and constructive emphasis",
        "detriment" => "a need for translation, adaptation, and deliberate handling",
        "fall" => "a more sensitive or constrained expression that needs care",
        _ => "additional interpretive context",
    }
}

fn dignity_semantic_tags(dignity: &EssentialDignityFact) -> Vec<String> {
    let mut tags = vec![
        "dignity".to_string(),
        dignity.object_code.clone(),
        dignity.sign_code.clone(),
        dignity.dignity_type.clone(),
    ];
    if dignity.polarity == "dignity" {
        tags.push("functional_strength".to_string());
    } else {
        tags.push("functional_challenge".to_string());
    }
    tags.extend(sign_tags(&dignity.sign_code));
    dedupe_tags(tags)
}

fn dignity_evidence(dignity: &EssentialDignityFact) -> serde_json::Value {
    json!({
        "fact_type": "essential_dignity",
        "chart_object_id": dignity.chart_object_id,
        "chart_object": dignity.object_code,
        "object_name": dignity.object_name,
        "sign_id": dignity.sign_id,
        "sign_code": dignity.sign_code,
        "sign_name": dignity.sign_name,
        "dignity_type": dignity.dignity_type,
        "dignity_label": dignity.dignity_label,
        "polarity": dignity.polarity,
        "strength_score": dignity.strength_score,
        "is_major": dignity.is_major
    })
}

fn dignity_evidence_array(dignities: &[EssentialDignityFact]) -> serde_json::Value {
    serde_json::Value::Array(dignities.iter().map(dignity_evidence).collect())
}

fn placement_context_object(position: &ObjectPositionFact, key: &str) -> Option<serde_json::Value> {
    position
        .facts_json
        .as_ref()
        .and_then(|facts| facts.get(key))
        .filter(|value| !value.is_null())
        .cloned()
}

fn placement_context_value<'a>(
    position: &'a ObjectPositionFact,
    context_key: &str,
    value_key: &str,
) -> Option<&'a serde_json::Value> {
    position
        .facts_json
        .as_ref()
        .and_then(|facts| facts.get(context_key))
        .and_then(|context| context.get(value_key))
        .filter(|value| !value.is_null())
}

fn placement_context_str<'a>(
    position: &'a ObjectPositionFact,
    context_key: &str,
    value_key: &str,
) -> Option<&'a str> {
    placement_context_value(position, context_key, value_key).and_then(|value| value.as_str())
}

fn cluster_semantic_tags(sign_code: &str, house_number: i32) -> Vec<String> {
    let mut tags = vec![
        "cluster".to_string(),
        sign_code.to_string(),
        format!("house_{house_number}"),
        house_theme_code(house_number).to_string(),
    ];
    tags.extend(sign_tags(sign_code));
    tags.extend(house_tags(house_number));
    dedupe_tags(tags)
}

fn aspect_semantic_tags(aspect: &crate::domain::AspectFact, strength_score: f64) -> Vec<String> {
    let mut tags = vec![
        "aspect".to_string(),
        aspect.aspect_code.clone(),
        aspect.aspect_family.clone(),
        aspect_dynamic_quality(aspect).to_string(),
    ];
    if let Some(primary_valence) = aspect.primary_valence.as_ref() {
        tags.push(primary_valence.clone());
    }
    if let Some(intensity_modifier) = aspect.intensity_modifier.as_ref() {
        tags.push(intensity_modifier.clone());
    }
    if let Some(secondary_effect) = aspect.secondary_effect.as_ref() {
        tags.push(secondary_effect.clone());
    }
    if strength_score >= 0.75 {
        tags.push("high_strength".to_string());
    } else if strength_score < BASIC_ASPECT_MIN_STRENGTH {
        tags.push("low_strength".to_string());
    }
    dedupe_tags(tags)
}

fn aspect_context(aspect: &crate::domain::AspectFact) -> serde_json::Value {
    json!({
        "aspect_family": aspect.aspect_family,
        "primary_valence": aspect.primary_valence,
        "intensity_modifier": aspect.intensity_modifier,
        "secondary_effect": aspect.secondary_effect,
        "dynamic_quality": aspect_dynamic_quality(aspect),
        "phase_state": aspect.phase_state,
        "valence_family": aspect.valence_family,
        "is_tonal_valence": aspect.valence_is_tonal,
        "is_intensity_modifier": aspect.valence_is_intensity_modifier,
        "writing_guidance": aspect.valence_writing_guidance
            .as_deref()
            .unwrap_or_else(|| aspect_default_writing_guidance(aspect))
    })
}

fn aspect_dynamic_quality(aspect: &crate::domain::AspectFact) -> &'static str {
    match aspect.primary_valence.as_deref() {
        Some(
            "supportive" | "harmonious" | "creative" | "refined_creative" | "creative_ordering",
        ) => "flow",
        Some("dynamic_challenging" | "polarizing" | "minor_friction" | "indirect_tension") => {
            "tension"
        }
        Some("adjustment" | "subtle_adjustment") => "adjustment",
        Some("symbolic_fated") => "symbolic",
        Some("spiritual_integration") => "integration",
        Some(_) => "contextual",
        None if aspect.intensity_modifier.is_some() => "intensification",
        None => "contextual",
    }
}

fn aspect_writing_guidance(aspect: &crate::domain::AspectFact) -> String {
    let base = aspect
        .valence_writing_guidance
        .as_deref()
        .unwrap_or_else(|| aspect_default_writing_guidance(aspect));

    match aspect.intensity_modifier.as_deref() {
        Some(modifier) if aspect.primary_valence.is_none() => format!(
            "{base} Treat {modifier} as an intensity modifier, not as a supportive or challenging valence by itself."
        ),
        Some(modifier) => format!(
            "{base} Use {modifier} only as an intensity modifier layered onto the primary valence."
        ),
        None => base.to_string(),
    }
}

fn aspect_default_writing_guidance(aspect: &crate::domain::AspectFact) -> &'static str {
    match aspect_dynamic_quality(aspect) {
        "flow" => {
            "Describe ease or cooperation between the two chart factors without presenting it as an automatic benefit."
        }
        "tension" => {
            "Describe the tension between the two chart factors without making it unstable or negative by default."
        }
        "adjustment" => {
            "Describe this as an adjustment between the two chart factors, with practical recalibration rather than blame."
        }
        "intensification" => {
            "Describe this as intensified contact between the two chart factors, and use the planets involved to qualify the tone."
        }
        _ => "Use the aspect as a relationship between two chart factors, not as a standalone verdict.",
    }
}

fn sign_tags(sign_code: &str) -> Vec<String> {
    match sign_code {
        "aries" => vec!["initiative", "assertion"],
        "taurus" => vec!["stability", "embodiment"],
        "gemini" => vec!["learning", "adaptability"],
        "cancer" => vec!["protection", "belonging"],
        "leo" => vec!["expression", "confidence"],
        "virgo" => vec!["analysis", "service"],
        "libra" => vec!["balance", "relationship"],
        "scorpio" => vec!["intensity", "transformation"],
        "sagittarius" => vec!["meaning", "exploration"],
        "capricorn" => vec!["structure", "responsibility"],
        "aquarius" => vec!["systems", "independence"],
        "pisces" => vec!["imagination", "sensitivity"],
        _ => Vec::new(),
    }
    .into_iter()
    .map(ToString::to_string)
    .collect()
}

fn house_tags(house_number: i32) -> Vec<String> {
    match house_number {
        1 => vec!["self_expression", "temperament"],
        2 => vec!["security", "value"],
        3 => vec!["learning", "local_environment"],
        4 => vec!["home", "family"],
        5 => vec!["pleasure", "creation"],
        6 => vec!["routine", "maintenance"],
        7 => vec!["partnership", "contracts"],
        8 => vec!["intimacy", "transformation"],
        9 => vec!["philosophy", "travel"],
        10 => vec!["vocation", "reputation"],
        11 => vec!["groups", "future_plans"],
        12 => vec!["retreat", "unconscious"],
        _ => Vec::new(),
    }
    .into_iter()
    .map(ToString::to_string)
    .collect()
}

fn dedupe_tags(tags: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::new();
    for tag in tags {
        if !deduped.contains(&tag) {
            deduped.push(tag);
        }
    }
    deduped
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

fn preserve_strong_tension_aspect(signals: &mut [InterpretationSignalDraft]) {
    if signals
        .iter()
        .any(|signal| signal.suppression_state == "active" && is_strong_tension_signal(signal))
    {
        return;
    }

    let Some(tension_index) = signals
        .iter()
        .enumerate()
        .filter(|(_, signal)| is_strong_tension_signal(signal))
        .max_by(|(_, left), (_, right)| {
            left.priority_score
                .partial_cmp(&right.priority_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(index, _)| index)
    else {
        return;
    };

    let Some(replacement_index) = signals
        .iter()
        .enumerate()
        .rev()
        .find(|(_, signal)| {
            signal.suppression_state == "active" && !is_basic_required_signal(signal)
        })
        .map(|(index, _)| index)
    else {
        return;
    };

    signals[replacement_index].suppression_state = "suppressed".to_string();
    signals[tension_index].suppression_state = "active".to_string();
}

fn is_strong_tension_signal(signal: &InterpretationSignalDraft) -> bool {
    if !signal.signal_key.starts_with("aspect:") {
        return false;
    }

    let Some(evidence) = signal
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get("evidence"))
    else {
        return false;
    };

    let aspect_code = evidence.get("aspect_code").and_then(|value| value.as_str());
    let strength_score = evidence
        .get("strength_score")
        .and_then(|value| value.as_f64())
        .unwrap_or(signal.priority_score / 80.0);

    matches!(aspect_code, Some("square" | "opposition")) && strength_score >= 0.75
}

fn is_basic_required_signal(signal: &InterpretationSignalDraft) -> bool {
    if signal.signal_key.starts_with("cluster:") {
        return true;
    }

    let Some(object_code) = signal
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get("evidence"))
        .and_then(|evidence| evidence.get("object_code"))
        .and_then(|value| value.as_str())
    else {
        return false;
    };

    matches!(
        object_code,
        "sun" | "moon" | "ascendant" | "mc" | "mercury" | "venus" | "mars"
    )
}

fn fill_basic_active_limit(signals: &mut [InterpretationSignalDraft]) {
    let mut active_count = signals
        .iter()
        .filter(|signal| signal.suppression_state == "active")
        .count();

    if active_count >= BASIC_MAX_ACTIVE_SIGNALS {
        return;
    }

    for signal in signals {
        if active_count >= BASIC_MAX_ACTIVE_SIGNALS {
            break;
        }

        if signal.suppression_state == "suppressed" && is_basic_fill_eligible(signal) {
            signal.suppression_state = "active".to_string();
            active_count += 1;
        }
    }
}

fn is_basic_fill_eligible(signal: &InterpretationSignalDraft) -> bool {
    !is_weak_aspect_signal(signal)
}

fn is_weak_aspect_signal(signal: &InterpretationSignalDraft) -> bool {
    if !signal.signal_key.starts_with("aspect:") {
        return false;
    }

    signal
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get("evidence"))
        .and_then(|evidence| evidence.get("strength_score"))
        .and_then(|value| value.as_f64())
        .is_some_and(|strength_score| strength_score < BASIC_ASPECT_MIN_STRENGTH)
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

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::AspectFact;

    fn capricorn_house_2_position(
        id: i32,
        object_code: &str,
        object_name: &str,
    ) -> ObjectPositionFact {
        ObjectPositionFact {
            chart_object_id: id,
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
            motion_state_id: None,
            horizon_position_id: None,
            longitude_deg: 270.0 + f64::from(id),
            latitude_deg: None,
            apparent_speed_deg_per_day: Some(1.0),
            altitude_deg: None,
            is_visible: None,
            facts_json: None,
        }
    }

    fn position(
        id: i32,
        object_code: &str,
        object_name: &str,
        sign_code: &str,
        sign_name: &str,
        house_number: i32,
    ) -> ObjectPositionFact {
        ObjectPositionFact {
            chart_object_id: id,
            object_code: object_code.to_string(),
            object_name: object_name.to_string(),
            zodiacal_reference_system_id: 1,
            coordinate_reference_system_id: 1,
            sign_id: id,
            sign_code: sign_code.to_string(),
            sign_name: sign_name.to_string(),
            house_id: Some(house_number),
            house_number: Some(house_number),
            house_name: Some(format!("House {house_number}")),
            motion_state_id: None,
            horizon_position_id: None,
            longitude_deg: f64::from(id) * 30.0,
            latitude_deg: None,
            apparent_speed_deg_per_day: Some(1.0),
            altitude_deg: None,
            is_visible: None,
            facts_json: None,
        }
    }

    fn enriched_position() -> ObjectPositionFact {
        let mut position = position(1, "sun", "Sun", "gemini", "Gemini", 9);
        position.facts_json = Some(json!({
            "sign_context": {
                "element": "air",
                "modality": "mutable",
                "polarity": "yang",
                "keywords": ["communication", "curiosity"]
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
        }));
        position
    }

    fn retrograde_mercury_position() -> ObjectPositionFact {
        let mut position = position(3, "mercury", "Mercury", "capricorn", "Capricorn", 3);
        position.facts_json = Some(json!({
            "sign_context": {
                "element": "earth",
                "modality": "cardinal",
                "polarity": "yin"
            },
            "house_modality": {
                "code": "cadent"
            },
            "object_context": {
                "role": "planet"
            },
            "motion_context": {
                "motion_state": "retrograde",
                "label": "Retrograde",
                "motion_family": "reverse"
            }
        }));
        position
    }

    fn aspect(
        source_code: &str,
        source_name: &str,
        target_code: &str,
        target_name: &str,
        aspect_code: &str,
        aspect_name: &str,
        strength_score: f64,
    ) -> AspectFact {
        AspectFact {
            source_chart_object_id: 1,
            source_object_code: source_code.to_string(),
            source_object_name: source_name.to_string(),
            target_chart_object_id: 2,
            target_object_code: target_code.to_string(),
            target_object_name: target_name.to_string(),
            aspect_id: 1,
            aspect_code: aspect_code.to_string(),
            aspect_name: aspect_name.to_string(),
            aspect_family: "major".to_string(),
            orb_deg: 1.0,
            phase_state: "applying".to_string(),
            is_applying: true,
            is_exact: false,
            strength_score: Some(strength_score),
            primary_valence: primary_valence_for_test(aspect_code).map(ToString::to_string),
            intensity_modifier: intensity_modifier_for_test(aspect_code).map(ToString::to_string),
            secondary_effect: None,
            valence_family: primary_valence_for_test(aspect_code)
                .map(|_| "tonal".to_string())
                .or_else(|| {
                    intensity_modifier_for_test(aspect_code).map(|_| "intensity".to_string())
                }),
            valence_is_tonal: primary_valence_for_test(aspect_code)
                .map(|_| true)
                .or_else(|| intensity_modifier_for_test(aspect_code).map(|_| false)),
            valence_is_intensity_modifier: primary_valence_for_test(aspect_code)
                .map(|_| false)
                .or_else(|| intensity_modifier_for_test(aspect_code).map(|_| true)),
            valence_writing_guidance: valence_guidance_for_test(aspect_code)
                .map(ToString::to_string),
            calculation_notes_json: None,
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

    fn valence_guidance_for_test(aspect_code: &str) -> Option<&'static str> {
        match aspect_code {
            "sextile" => Some("Present as a resource, support or facilitation that the native can mobilize."),
            "square" => Some("Present as active tension or constructive challenge, not as a purely negative outcome."),
            "trine" => Some("Present as ease, compatibility or natural cooperation without implying that no effort is ever needed."),
            "opposition" => Some("Present as a polarity to balance, integrate or negotiate, especially across axes or oppositions."),
            _ => None,
        }
    }

    #[test]
    fn major_dignities_create_dedicated_signals_and_enrich_placements() {
        let facts = CalculatedChartFacts {
            positions: vec![
                position(6, "jupiter", "Jupiter", "cancer", "Cancer", 8),
                position(7, "saturn", "Saturn", "capricorn", "Capricorn", 2),
            ],
            house_cusps: Vec::new(),
            aspects: Vec::new(),
        };

        let signals = aggregate_basic_signals(&facts);
        let saturn_dignity = signals
            .iter()
            .find(|signal| signal.signal_key == "dignity:saturn:domicile:capricorn")
            .expect("expected Saturn domicile dignity signal");
        let jupiter_dignity = signals
            .iter()
            .find(|signal| signal.signal_key == "dignity:jupiter:exaltation:cancer")
            .expect("expected Jupiter exaltation dignity signal");
        let saturn_placement = signals
            .iter()
            .find(|signal| signal.signal_key == "object_position:saturn")
            .expect("expected Saturn placement signal");

        assert_eq!(
            saturn_dignity.theme_code.as_deref(),
            Some("functional_strength")
        );
        assert_eq!(saturn_dignity.priority_score, 88.0);
        assert_eq!(jupiter_dignity.priority_score, 86.0);
        assert_eq!(
            saturn_dignity
                .payload_json
                .as_ref()
                .and_then(|payload| payload.get("evidence"))
                .and_then(|evidence| evidence.get("dignity_type"))
                .and_then(|value| value.as_str()),
            Some("domicile")
        );
        assert_eq!(
            saturn_placement
                .payload_json
                .as_ref()
                .and_then(|payload| payload.get("evidence"))
                .and_then(|evidence| evidence.get("essential_dignities"))
                .and_then(|dignities| dignities.as_array())
                .and_then(|dignities| dignities.first())
                .and_then(|dignity| dignity.get("dignity_type"))
                .and_then(|value| value.as_str()),
            Some("domicile")
        );
        assert_eq!(
            jupiter_dignity
                .payload_json
                .as_ref()
                .and_then(|payload| payload.get("interpretive_hint"))
                .and_then(|value| value.as_str()),
            Some("Treat Jupiter in Cancer as an exaltation modifier for the existing placement signal.")
        );
    }

    #[test]
    fn double_dignity_positions_create_all_signals_and_evidence() {
        let facts = CalculatedChartFacts {
            positions: vec![position(3, "mercury", "Mercury", "virgo", "Virgo", 6)],
            house_cusps: Vec::new(),
            aspects: Vec::new(),
        };

        let signals = aggregate_basic_signals(&facts);
        let placement = signals
            .iter()
            .find(|signal| signal.signal_key == "object_position:mercury")
            .expect("expected Mercury placement");

        assert!(signals
            .iter()
            .any(|signal| signal.signal_key == "dignity:mercury:domicile:virgo"));
        assert!(signals
            .iter()
            .any(|signal| signal.signal_key == "dignity:mercury:exaltation:virgo"));
        assert_eq!(
            placement
                .payload_json
                .as_ref()
                .and_then(|payload| payload.get("evidence"))
                .and_then(|evidence| evidence.get("essential_dignities"))
                .and_then(|value| value.as_array())
                .map(Vec::len),
            Some(2)
        );
        assert_eq!(placement.priority_score, 94.0);
    }

    #[test]
    fn basic_signals_include_semantic_position_cluster() {
        let facts = CalculatedChartFacts {
            positions: vec![
                capricorn_house_2_position(1, "sun", "Sun"),
                capricorn_house_2_position(6, "saturn", "Saturn"),
                capricorn_house_2_position(8, "uranus", "Uranus"),
            ],
            house_cusps: Vec::new(),
            aspects: Vec::new(),
        };

        let signals = aggregate_basic_signals(&facts);
        let cluster = signals
            .iter()
            .find(|signal| signal.signal_key == "cluster:capricorn:house_2")
            .expect("expected a Capricorn house 2 cluster");

        assert_eq!(cluster.theme_code.as_deref(), Some("resources"));
        assert_eq!(cluster.suppression_state, "active");
        let payload = cluster.payload_json.as_ref().expect("cluster payload");
        assert_eq!(
            payload
                .get("aggregation_group")
                .and_then(|value| value.as_str()),
            Some("capricorn_house_2_cluster")
        );
        assert!(payload
            .get("semantic_tags")
            .and_then(|value| value.as_array())
            .expect("semantic tags")
            .iter()
            .any(|value| value.as_str() == Some("responsibility")));
        assert_eq!(
            payload
                .get("evidence")
                .and_then(|value| value.get("source_signals"))
                .and_then(|value| value.as_array())
                .map(Vec::len),
            Some(3)
        );
    }

    #[test]
    fn placement_signal_includes_contextual_evidence_and_tags() {
        let facts = CalculatedChartFacts {
            positions: vec![enriched_position()],
            house_cusps: Vec::new(),
            aspects: Vec::new(),
        };

        let signals = aggregate_basic_signals(&facts);
        let signal = signals
            .iter()
            .find(|signal| signal.signal_key == "object_position:sun")
            .expect("expected Sun signal");
        let payload = signal.payload_json.as_ref().expect("signal payload");
        let tags = payload
            .get("semantic_tags")
            .and_then(|value| value.as_array())
            .expect("semantic tags");

        assert!(tags.iter().any(|tag| tag.as_str() == Some("air")));
        assert!(tags.iter().any(|tag| tag.as_str() == Some("mutable")));
        assert!(tags.iter().any(|tag| tag.as_str() == Some("cadent")));
        assert_eq!(
            payload
                .get("evidence")
                .and_then(|evidence| evidence.get("placement_context"))
                .and_then(|context| context.get("motion_context"))
                .and_then(|motion| motion.get("motion_state"))
                .and_then(|value| value.as_str()),
            Some("direct")
        );
    }

    #[test]
    fn retrograde_placements_get_specific_writing_context() {
        let facts = CalculatedChartFacts {
            positions: vec![retrograde_mercury_position()],
            house_cusps: Vec::new(),
            aspects: Vec::new(),
        };

        let signals = aggregate_basic_signals(&facts);
        let signal = signals
            .iter()
            .find(|signal| signal.signal_key == "object_position:mercury")
            .expect("expected Mercury signal");
        let payload = signal.payload_json.as_ref().expect("signal payload");

        assert!(signal
            .summary
            .as_deref()
            .expect("summary")
            .contains("retrograde motion"));
        assert!(payload
            .get("interpretive_hint")
            .and_then(|value| value.as_str())
            .expect("hint")
            .contains("internal processing"));
        assert!(payload
            .get("writing_guidance")
            .and_then(|value| value.as_str())
            .expect("guidance")
            .contains("retrograde motion"));
    }

    #[test]
    fn basic_cluster_merges_secondary_source_signals() {
        let facts = CalculatedChartFacts {
            positions: vec![
                capricorn_house_2_position(1, "sun", "Sun"),
                capricorn_house_2_position(6, "saturn", "Saturn"),
                capricorn_house_2_position(8, "uranus", "Uranus"),
            ],
            house_cusps: Vec::new(),
            aspects: Vec::new(),
        };

        let signals = aggregate_basic_signals(&facts);
        let sun = signals
            .iter()
            .find(|signal| signal.signal_key == "object_position:sun")
            .expect("expected Sun signal");
        let saturn = signals
            .iter()
            .find(|signal| signal.signal_key == "object_position:saturn")
            .expect("expected Saturn signal");

        assert_eq!(sun.suppression_state, "active");
        assert_eq!(saturn.suppression_state, "merged");
        assert_eq!(
            saturn
                .payload_json
                .as_ref()
                .and_then(|payload| payload.get("editorial_state"))
                .and_then(|state| state.get("cluster_signal_key"))
                .and_then(|value| value.as_str()),
            Some("cluster:capricorn:house_2")
        );
    }

    #[test]
    fn basic_cluster_dedup_refills_without_reactivating_weak_aspects() {
        let facts = CalculatedChartFacts {
            positions: vec![
                capricorn_house_2_position(1, "sun", "Sun"),
                position(2, "moon", "Moon", "cancer", "Cancer", 4),
                position(3, "mercury", "Mercury", "gemini", "Gemini", 3),
                position(4, "venus", "Venus", "taurus", "Taurus", 5),
                position(5, "mars", "Mars", "aries", "Aries", 1),
                position(6, "jupiter", "Jupiter", "sagittarius", "Sagittarius", 9),
                capricorn_house_2_position(7, "saturn", "Saturn"),
                capricorn_house_2_position(8, "uranus", "Uranus"),
                capricorn_house_2_position(9, "neptune", "Neptune"),
                position(10, "pluto", "Pluto", "scorpio", "Scorpio", 8),
                position(11, "north_node", "North Node", "aquarius", "Aquarius", 11),
            ],
            house_cusps: Vec::new(),
            aspects: vec![
                aspect("sun", "Sun", "moon", "Moon", "trine", "Trine", 0.99),
                aspect(
                    "mercury", "Mercury", "venus", "Venus", "sextile", "Sextile", 0.98,
                ),
                aspect(
                    "mars", "Mars", "jupiter", "Jupiter", "square", "Square", 0.97,
                ),
                aspect(
                    "saturn",
                    "Saturn",
                    "pluto",
                    "Pluto",
                    "opposition",
                    "Opposition",
                    0.2,
                ),
            ],
        };

        let signals = aggregate_basic_signals(&facts);
        let active_count = signals
            .iter()
            .filter(|signal| signal.suppression_state == "active")
            .count();
        let weak_aspect = signals
            .iter()
            .find(|signal| signal.signal_key == "aspect:saturn:pluto:opposition")
            .expect("expected weak aspect signal");
        let jupiter_dignity = signals
            .iter()
            .find(|signal| signal.signal_key == "dignity:jupiter:domicile:sagittarius")
            .expect("expected Jupiter dignity signal");

        assert_eq!(active_count, BASIC_MAX_ACTIVE_SIGNALS);
        assert_eq!(weak_aspect.suppression_state, "suppressed");
        assert_eq!(jupiter_dignity.suppression_state, "active");
    }

    #[test]
    fn basic_filter_preserves_one_strong_tension_aspect() {
        let facts = CalculatedChartFacts {
            positions: vec![
                capricorn_house_2_position(1, "sun", "Sun"),
                position(2, "moon", "Moon", "cancer", "Cancer", 4),
                position(3, "mercury", "Mercury", "gemini", "Gemini", 3),
                position(4, "venus", "Venus", "taurus", "Taurus", 5),
                position(5, "mars", "Mars", "aries", "Aries", 1),
                position(6, "jupiter", "Jupiter", "sagittarius", "Sagittarius", 9),
                capricorn_house_2_position(7, "saturn", "Saturn"),
                capricorn_house_2_position(8, "uranus", "Uranus"),
                capricorn_house_2_position(9, "neptune", "Neptune"),
                position(10, "pluto", "Pluto", "scorpio", "Scorpio", 8),
                position(11, "north_node", "North Node", "aquarius", "Aquarius", 11),
            ],
            house_cusps: Vec::new(),
            aspects: vec![
                aspect("sun", "Sun", "moon", "Moon", "trine", "Trine", 0.99),
                aspect(
                    "mercury", "Mercury", "venus", "Venus", "sextile", "Sextile", 0.98,
                ),
                aspect(
                    "sun",
                    "Sun",
                    "neptune",
                    "Neptune",
                    "conjunction",
                    "Conjunction",
                    0.97,
                ),
                aspect(
                    "moon", "Moon", "neptune", "Neptune", "sextile", "Sextile", 0.96,
                ),
                aspect(
                    "jupiter",
                    "Jupiter",
                    "uranus",
                    "Uranus",
                    "opposition",
                    "Opposition",
                    0.88,
                ),
            ],
        };

        let signals = aggregate_basic_signals(&facts);
        let active_count = signals
            .iter()
            .filter(|signal| signal.suppression_state == "active")
            .count();
        let tension = signals
            .iter()
            .find(|signal| signal.signal_key == "aspect:jupiter:uranus:opposition")
            .expect("expected strong opposition");

        assert_eq!(active_count, BASIC_MAX_ACTIVE_SIGNALS);
        assert_eq!(tension.suppression_state, "active");
    }

    #[test]
    fn indefinite_article_handles_opposition() {
        assert_eq!(indefinite_article("opposition"), "an");
        assert_eq!(indefinite_article("exaltation"), "an");
        assert_eq!(indefinite_article("square"), "a");
    }

    #[test]
    fn aspect_hint_uses_indefinite_article() {
        let facts = CalculatedChartFacts {
            positions: Vec::new(),
            house_cusps: Vec::new(),
            aspects: vec![AspectFact {
                source_chart_object_id: 6,
                source_object_code: "jupiter".to_string(),
                source_object_name: "Jupiter".to_string(),
                target_chart_object_id: 8,
                target_object_code: "uranus".to_string(),
                target_object_name: "Uranus".to_string(),
                aspect_id: 5,
                aspect_code: "opposition".to_string(),
                aspect_name: "Opposition".to_string(),
                aspect_family: "major".to_string(),
                orb_deg: 0.7586,
                phase_state: "separating".to_string(),
                is_applying: false,
                is_exact: false,
                strength_score: Some(0.9052),
                primary_valence: Some("polarizing".to_string()),
                intensity_modifier: None,
                secondary_effect: None,
                valence_family: Some("tonal".to_string()),
                valence_is_tonal: Some(true),
                valence_is_intensity_modifier: Some(false),
                valence_writing_guidance: Some(
                    "Present as a polarity to balance, integrate or negotiate, especially across axes or oppositions."
                        .to_string(),
                ),
                calculation_notes_json: None,
            }],
        };

        let signals = aggregate_basic_signals(&facts);
        let payload = signals[0].payload_json.as_ref().expect("aspect payload");

        assert_eq!(
            signals[0].summary.as_deref(),
            Some("Jupiter and Uranus form an opposition with 0.76 degrees of orb; the phase is separating.")
        );
        assert_eq!(
            payload
                .get("interpretive_hint")
                .and_then(|value| value.as_str()),
            Some("Jupiter and Uranus are connected by an opposition, so their functions should be read together with attention to the separating phase.")
        );
    }

    #[test]
    fn aspect_signals_include_interpretive_context_and_valence_tags() {
        let facts = CalculatedChartFacts {
            positions: Vec::new(),
            house_cusps: Vec::new(),
            aspects: vec![
                aspect(
                    "venus", "Venus", "jupiter", "Jupiter", "sextile", "Sextile", 0.9,
                ),
                aspect("moon", "Moon", "mars", "Mars", "square", "Square", 0.88),
                aspect(
                    "sun",
                    "Sun",
                    "neptune",
                    "Neptune",
                    "conjunction",
                    "Conjunction",
                    0.86,
                ),
            ],
        };

        let signals = aggregate_basic_signals(&facts);
        let sextile = aspect_payload(&signals, "aspect:venus:jupiter:sextile");
        let square = aspect_payload(&signals, "aspect:moon:mars:square");
        let conjunction = aspect_payload(&signals, "aspect:sun:neptune:conjunction");

        assert_eq!(
            sextile
                .get("aspect_context")
                .and_then(|context| context.get("primary_valence"))
                .and_then(|value| value.as_str()),
            Some("supportive")
        );
        assert!(sextile
            .get("semantic_tags")
            .and_then(|value| value.as_array())
            .expect("semantic tags")
            .iter()
            .any(|tag| tag.as_str() == Some("flow")));
        assert_eq!(
            square
                .get("aspect_context")
                .and_then(|context| context.get("primary_valence"))
                .and_then(|value| value.as_str()),
            Some("dynamic_challenging")
        );
        assert!(square
            .get("semantic_tags")
            .and_then(|value| value.as_array())
            .expect("semantic tags")
            .iter()
            .any(|tag| tag.as_str() == Some("tension")));
        assert_eq!(
            conjunction
                .get("aspect_context")
                .and_then(|context| context.get("primary_valence")),
            Some(&serde_json::Value::Null)
        );
        assert_eq!(
            conjunction
                .get("aspect_context")
                .and_then(|context| context.get("intensity_modifier"))
                .and_then(|value| value.as_str()),
            Some("amplifying")
        );
        assert_eq!(
            conjunction
                .get("aspect_context")
                .and_then(|context| context.get("valence_family"))
                .and_then(|value| value.as_str()),
            Some("intensity")
        );
        assert_eq!(
            conjunction
                .get("aspect_context")
                .and_then(|context| context.get("is_intensity_modifier"))
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        assert!(conjunction
            .get("writing_guidance")
            .and_then(|value| value.as_str())
            .expect("writing guidance")
            .contains("not as a supportive or challenging valence"));
    }

    fn aspect_payload<'a>(
        signals: &'a [InterpretationSignalDraft],
        signal_key: &str,
    ) -> &'a serde_json::Value {
        signals
            .iter()
            .find(|signal| signal.signal_key == signal_key)
            .and_then(|signal| signal.payload_json.as_ref())
            .expect("aspect payload")
    }
}
