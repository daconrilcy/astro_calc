use std::collections::HashMap;

use serde_json::json;

use crate::dignities::{
    dignity_is_signal_worthy, dignity_priority_delta_for_position, dignity_source_weight_delta,
    dignity_source_weight_delta_for_position, essential_dignities_for_position,
    essential_dignities_for_positions, EssentialDignityFact,
};
use crate::domain::{CalculatedChartFacts, InterpretationSignalDraft, ObjectPositionFact};

pub const BASIC_MAX_ACTIVE_SIGNALS: usize = 12;
const BASIC_ASPECT_MIN_STRENGTH: f64 = 0.4;

pub fn aggregate_basic_signals(facts: &CalculatedChartFacts) -> Vec<InterpretationSignalDraft> {
    let mut signals = Vec::new();

    for position in &facts.positions {
        if is_angle_position(position) {
            signals.push(angle_signal(position));
            continue;
        }

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
        if is_structural_axis_aspect(aspect) {
            continue;
        }

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
                "interpretive_hint": aspect_interpretive_hint(aspect, &aspect_name),
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
        let house_theme_code = house_theme_code(positions[0]);
        let semantic_tags = cluster_semantic_tags(&sign_code, house_number, &house_theme_code);

        signals.push(InterpretationSignalDraft {
            signal_key: format!("cluster:{sign_code}:house_{house_number}"),
            signal_type_id: None,
            theme_code: Some(house_theme_code.clone()),
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
                    "house_theme_code": house_theme_code,
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

fn is_angle_position(position: &ObjectPositionFact) -> bool {
    placement_context_value(position, "angle_context", "angle_point_id").is_some()
}

fn angle_signal(position: &ObjectPositionFact) -> InterpretationSignalDraft {
    let angle_context = angle_context(position);
    let semantic_tags = angle_semantic_tags(position);
    let associated_house = angle_associated_house(position).or(position.house_number);
    let theme_code = house_theme_code(position);

    InterpretationSignalDraft {
        signal_key: format!("angle:{}:sign:{}", position.object_code, position.sign_code),
        signal_type_id: None,
        theme_code: Some(theme_code),
        title: format!("{} in {}", position.object_name, position.sign_name),
        summary: Some(format!(
            "{} falls in {}, giving the chart a concrete orientation through this angle.",
            position.object_name, position.sign_name
        )),
        priority_score: angle_priority(position),
        confidence_score: Some(0.95),
        suppression_state: "active".to_string(),
        payload_json: Some(json!({
            "interpretive_hint": angle_interpretive_hint(position),
            "semantic_tags": semantic_tags,
            "source_weight": round4(object_source_weight(&position.object_code)),
            "aggregation_group": format!("angle:{}:{}", position.object_code, position.sign_code),
            "writing_guidance": angle_writing_guidance(position),
            "angle_context": angle_context,
            "evidence": {
                "fact_type": "chart_angle",
                "angle_code": position.object_code,
                "angle_name": position.object_name,
                "angle_point_code": placement_context_str(position, "angle_context", "angle_point_code"),
                "short_label": placement_context_str(position, "angle_context", "short_label"),
                "axis": placement_context_str(position, "angle_context", "axis"),
                "opposite_angle_code": placement_context_str(position, "angle_context", "opposite_angle_code"),
                "associated_house_number": associated_house,
                "chart_object_id": position.chart_object_id,
                "sign_id": position.sign_id,
                "sign_code": position.sign_code,
                "sign_name": position.sign_name,
                "house_id": position.house_id,
                "house_number": position.house_number,
                "house_name": position.house_name,
                "longitude_deg": position.longitude_deg,
                "placement_context": placement_context(position)
            }
        })),
    }
}

fn angle_priority(position: &ObjectPositionFact) -> f64 {
    let base = match position.object_code.as_str() {
        "ascendant" => 99.0,
        "mc" => 82.0,
        "descendant" => 68.0,
        "ic" => 66.0,
        _ => 60.0,
    };
    round4((base + house_modality_priority_delta(position)).min(100.0))
}

fn angle_context(position: &ObjectPositionFact) -> serde_json::Value {
    json!({
        "angle_code": position.object_code,
        "angle_name": position.object_name,
        "angle_point_code": placement_context_str(position, "angle_context", "angle_point_code"),
        "short_label": placement_context_str(position, "angle_context", "short_label"),
        "full_name": placement_context_str(position, "angle_context", "full_name"),
        "axis": placement_context_str(position, "angle_context", "axis"),
        "opposite_angle_code": placement_context_str(position, "angle_context", "opposite_angle_code"),
        "associated_house_number": angle_associated_house(position),
        "sign_code": position.sign_code,
        "sign_name": position.sign_name,
        "longitude_deg": position.longitude_deg
    })
}

fn angle_interpretive_hint(position: &ObjectPositionFact) -> String {
    match position.object_code.as_str() {
        "ascendant" => format!(
            "Use the Ascendant as the chart's immediate orientation: embodiment, instinctive style, and first impression through {} qualities.",
            position.sign_name
        ),
        "mc" => format!(
            "Use the MC as public direction and visibility, colored by {} qualities.",
            position.sign_name
        ),
        "descendant" => format!(
            "Use the Descendant as the relationship horizon and encounter style through {} qualities.",
            position.sign_name
        ),
        "ic" => format!(
            "Use the IC as private foundation, roots, and inner base through {} qualities.",
            position.sign_name
        ),
        _ => format!("Use this angle as a chart orientation marker in {}.", position.sign_name),
    }
}

fn angle_writing_guidance(position: &ObjectPositionFact) -> String {
    match position.object_code.as_str() {
        "ascendant" => "Integrate this with Sun and Moon as a core identity marker, not as a physical description only.".to_string(),
        "mc" => "Use this proportionately as public direction or visibility context; keep it secondary to Sun, Moon, and Ascendant in Basic.".to_string(),
        "descendant" => "Use this as relationship orientation only when it supports a larger Basic theme.".to_string(),
        "ic" => "Use this as roots and private-foundation context only when it supports a larger Basic theme.".to_string(),
        _ => "Use this angle as concise orientation context.".to_string(),
    }
}

fn angle_semantic_tags(position: &ObjectPositionFact) -> Vec<String> {
    let mut tags = vec![
        "angle".to_string(),
        position.object_code.clone(),
        position.sign_code.clone(),
    ];
    tags.extend(sign_tags(&position.sign_code));
    if let Some(house_number) = angle_associated_house(position).or(position.house_number) {
        tags.push(format!("house_{house_number}"));
        tags.push(house_theme_code(position));
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
    if let Some(axis) = placement_context_str(position, "angle_context", "axis") {
        tags.push(axis.to_string());
    }
    dedupe_tags(tags)
}

fn angle_associated_house(position: &ObjectPositionFact) -> Option<i32> {
    placement_context_value(position, "angle_context", "associated_house_number")
        .and_then(|value| value.as_i64())
        .and_then(|value| i32::try_from(value).ok())
}

fn position_priority(position: &ObjectPositionFact) -> f64 {
    let base = match position.object_code.as_str() {
        "ascendant" => 99.0,
        "sun" | "moon" => 100.0,
        "mc" => 82.0,
        "descendant" | "ic" => 68.0,
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
        "sun" | "moon" | "ascendant" => 1.0,
        "mc" => 0.8,
        "mercury" | "venus" | "mars" => 0.75,
        "jupiter" | "saturn" => 0.6,
        "descendant" | "ic" => 0.4,
        _ => 0.35,
    }
}

fn position_theme_code(position: &ObjectPositionFact) -> String {
    house_theme_code(position)
}

fn house_theme_code(position: &ObjectPositionFact) -> String {
    placement_context_str(position, "house_context", "theme_code")
        .or_else(|| placement_context_str(position, "angle_context", "house_theme_code"))
        .unwrap_or("object_position")
        .to_string()
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
        tags.push(house_theme_code(position));
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
        "house_context": placement_context_object(position, "house_context"),
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

fn cluster_semantic_tags(
    sign_code: &str,
    house_number: i32,
    house_theme_code: &str,
) -> Vec<String> {
    let mut tags = vec![
        "cluster".to_string(),
        sign_code.to_string(),
        format!("house_{house_number}"),
        house_theme_code.to_string(),
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

fn aspect_interpretive_hint(aspect: &crate::domain::AspectFact, aspect_name: &str) -> String {
    format!(
        "Read this {aspect_name} as {} between {} and {}, with attention to the {} phase.",
        aspect_hint_quality_phrase(aspect),
        aspect.source_object_name,
        aspect.target_object_name,
        aspect.phase_state
    )
}

fn aspect_hint_quality_phrase(aspect: &crate::domain::AspectFact) -> String {
    let base = match aspect.primary_valence.as_deref() {
        Some("supportive") => "a supportive flow",
        Some("harmonious") => "a natural flow",
        Some("creative" | "refined_creative" | "creative_ordering") => "a creative opening",
        Some("dynamic_challenging") => "an active tension",
        Some("polarizing") => "a polarity to balance",
        Some("minor_friction") => "manageable friction",
        Some("indirect_tension") => "indirect tension",
        Some("adjustment") => "an adjustment",
        Some("subtle_adjustment") => "a subtle adjustment",
        Some("symbolic_fated") => "a symbolic emphasis",
        Some("spiritual_integration") => "an integrating connection",
        Some(_) => "a contextual relationship",
        None => return intensity_only_aspect_hint_phrase(aspect).to_string(),
    };

    match aspect.intensity_modifier.as_deref() {
        Some("amplifying") => format!("{base} with extra emphasis"),
        Some("obsessive_focus") => format!("{base} with intensified focus"),
        Some(_) => format!("{base} with extra intensity"),
        None => base.to_string(),
    }
}

fn intensity_only_aspect_hint_phrase(aspect: &crate::domain::AspectFact) -> &'static str {
    match aspect.intensity_modifier.as_deref() {
        Some("amplifying") => "an amplifying contact",
        Some("obsessive_focus") => "an intensified focus",
        Some(_) => "an intensified contact",
        None => dynamic_quality_aspect_hint_phrase(aspect),
    }
}

fn dynamic_quality_aspect_hint_phrase(aspect: &crate::domain::AspectFact) -> &'static str {
    match aspect_dynamic_quality(aspect) {
        "flow" => "a flow",
        "tension" => "a tension",
        "adjustment" => "an adjustment",
        "intensification" => "an intensified contact",
        _ => "a relationship",
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
    if is_structural_axis_signal(signal) {
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
    !is_weak_aspect_signal(signal) && !is_structural_axis_signal(signal)
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

fn is_structural_axis_aspect(aspect: &crate::domain::AspectFact) -> bool {
    aspect
        .calculation_notes_json
        .as_ref()
        .and_then(|notes| notes.get("is_structural_axis"))
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn is_structural_axis_signal(signal: &InterpretationSignalDraft) -> bool {
    signal
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get("aspect_context"))
        .and_then(|context| context.get("is_structural_axis"))
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
        || signal
            .payload_json
            .as_ref()
            .and_then(|payload| payload.get("evidence"))
            .and_then(|evidence| evidence.get("is_structural_axis"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
}

pub fn indefinite_article(phrase: &str) -> &'static str {
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
