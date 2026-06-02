use std::collections::HashMap;

use serde_json::json;

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
        let semantic_tags = position_semantic_tags(position);
        let source_weight = object_source_weight(&position.object_code);
        let theme_code = position_theme_code(position);
        let aggregation_group = position_aggregation_group(position);

        signals.push(InterpretationSignalDraft {
            signal_key: format!("object_position:{}", position.object_code),
            signal_type_id: None,
            theme_code: Some(theme_code.to_string()),
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
                "interpretive_hint": position_interpretive_hint(position),
                "semantic_tags": semantic_tags,
                "source_weight": source_weight,
                "aggregation_group": aggregation_group,
                "writing_guidance": "Use this as a concise placement cue; combine it with nearby cluster or aspect signals before drafting final text.",
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
                "interpretive_hint": format!(
                    "{} and {} are connected by {} {}, so their functions should be read together with attention to the {} phase.",
                    aspect.source_object_name,
                    aspect.target_object_name,
                    article,
                    aspect_name,
                    aspect.phase_state
                ),
                "semantic_tags": aspect_semantic_tags(&aspect.aspect_code, strength_score),
                "source_weight": round4(
                    object_source_weight(&aspect.source_object_code)
                        + object_source_weight(&aspect.target_object_code)
                ),
                "aggregation_group": format!("aspect:{}", aspect.aspect_code),
                "writing_guidance": "Use the aspect as a relationship between two chart factors, not as a standalone verdict.",
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
    signals
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
            position_priority(&right.object_code)
                .partial_cmp(&position_priority(&left.object_code))
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

fn position_priority(object_code: &str) -> f64 {
    match object_code {
        "sun" | "moon" => 100.0,
        "mercury" | "venus" | "mars" => 85.0,
        "jupiter" | "saturn" => 75.0,
        _ => 60.0,
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
    match (position.house_name.as_deref(), position.house_number) {
        (Some(house_name), Some(_)) => format!(
            "{} expresses through {} qualities in the field of {}.",
            position.object_name, position.sign_name, house_name
        ),
        _ => format!(
            "{} expresses through {} qualities.",
            position.object_name, position.sign_name
        ),
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
    dedupe_tags(tags)
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

fn aspect_semantic_tags(aspect_code: &str, strength_score: f64) -> Vec<String> {
    let mut tags = vec!["aspect".to_string(), aspect_code.to_string()];
    if strength_score >= 0.75 {
        tags.push("high_strength".to_string());
    } else if strength_score < BASIC_ASPECT_MIN_STRENGTH {
        tags.push("low_strength".to_string());
    }
    tags
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
            orb_deg: 1.0,
            phase_state: "applying".to_string(),
            is_applying: true,
            is_exact: false,
            strength_score: Some(strength_score),
            calculation_notes_json: None,
        }
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
        let pluto = signals
            .iter()
            .find(|signal| signal.signal_key == "object_position:pluto")
            .expect("expected Pluto signal");

        assert_eq!(active_count, BASIC_MAX_ACTIVE_SIGNALS);
        assert_eq!(weak_aspect.suppression_state, "suppressed");
        assert_eq!(pluto.suppression_state, "active");
    }

    #[test]
    fn indefinite_article_handles_opposition() {
        assert_eq!(indefinite_article("opposition"), "an");
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
                orb_deg: 0.7586,
                phase_state: "separating".to_string(),
                is_applying: false,
                is_exact: false,
                strength_score: Some(0.9052),
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
}
