use std::collections::{HashMap, HashSet};

use serde_json::{json, Value};

use crate::catalog::BasicPayloadCatalog;
use crate::domain::{
    AccidentalDignityConditionReference, BasicAccidentalDignityCondition,
    BasicAccidentalDignityContextSummary, BasicAccidentalDignityEvaluation, BasicChartEmphasis,
    BasicSignal, ObjectPositionFact, ObjectSectAffinityReference,
};

use super::chart_context;
use super::json::position_context;

pub(super) struct AccidentalDignityBuild {
    pub evaluations: Vec<BasicAccidentalDignityEvaluation>,
    pub context_by_object: HashMap<String, Vec<BasicAccidentalDignityContextSummary>>,
}

pub(super) fn build_accidental_dignities(
    positions: &[ObjectPositionFact],
    chart_sect: Option<&str>,
    condition_definitions: &[AccidentalDignityConditionReference],
    sect_affinities: &[ObjectSectAffinityReference],
    active_signal_keys: &HashSet<&str>,
    catalog: &BasicPayloadCatalog,
) -> AccidentalDignityBuild {
    let definitions: HashMap<&str, &AccidentalDignityConditionReference> = condition_definitions
        .iter()
        .map(|definition| (definition.condition_code.as_str(), definition))
        .collect();
    let sect_by_object: HashMap<&str, &ObjectSectAffinityReference> = sect_affinities
        .iter()
        .map(|affinity| (affinity.object_code.as_str(), affinity))
        .collect();
    let angle_longitudes = angle_longitudes_from_positions(positions);

    let mut evaluations = Vec::new();
    let mut context_by_object = HashMap::new();

    for position in positions.iter().filter(|position| !is_angle(position)) {
        let conditions = evaluate_mobile_object(
            position,
            chart_sect,
            &definitions,
            &sect_by_object,
            &angle_longitudes,
            catalog,
        );
        if conditions.is_empty() {
            continue;
        }

        let summaries: Vec<BasicAccidentalDignityContextSummary> = conditions
            .iter()
            .map(|condition| BasicAccidentalDignityContextSummary {
                condition_code: condition.condition_code.clone(),
                condition_family: condition.condition_family.clone(),
                polarity: condition.polarity.clone(),
                strength_score: condition.strength_score,
            })
            .collect();
        context_by_object.insert(position.object_code.clone(), summaries);

        let raw_score: f64 = conditions
            .iter()
            .map(|condition| condition.score_delta)
            .sum();
        let scoring = &catalog.accidental_scoring;
        let overall_score = round4(
            (scoring.overall_score_baseline + raw_score)
                .clamp(scoring.overall_score_min, scoring.overall_score_max),
        );
        let (overall_polarity, expression_quality) =
            catalog.overall_polarity_for_score(overall_score);
        let signal_key = format!("object_position:{}", position.object_code);
        let related_signal_key = active_signal_keys
            .contains(signal_key.as_str())
            .then_some(signal_key);

        evaluations.push(BasicAccidentalDignityEvaluation {
            object_code: position.object_code.clone(),
            object_name: position.object_name.clone(),
            overall_score,
            overall_polarity: overall_polarity.clone(),
            expression_quality,
            related_signal_key,
            conditions,
        });
    }

    evaluations.sort_by(|left, right| left.object_code.cmp(&right.object_code));

    AccidentalDignityBuild {
        evaluations,
        context_by_object,
    }
}

pub(super) fn apply_accidental_context_to_signals(
    signals: &mut [BasicSignal],
    context_by_object: &HashMap<String, Vec<BasicAccidentalDignityContextSummary>>,
) {
    for signal in signals.iter_mut() {
        if !signal.signal_key.starts_with("object_position:") {
            continue;
        }
        let Some(object_code) = signal.signal_key.strip_prefix("object_position:") else {
            continue;
        };
        let context = context_by_object
            .get(object_code)
            .cloned()
            .unwrap_or_default();
        let mut evidence = signal
            .evidence
            .take()
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        let placement_context = evidence
            .entry("placement_context".to_string())
            .or_insert_with(|| json!({}));
        if let Some(placement) = placement_context.as_object_mut() {
            placement.insert(
                "accidental_dignity_context".to_string(),
                serde_json::to_value(context).unwrap_or(Value::Array(vec![])),
            );
        }
        signal.evidence = Some(Value::Object(evidence));
    }
}

pub(super) fn apply_accidental_context_to_emphasis(
    chart_emphasis: &mut BasicChartEmphasis,
    evaluations: &[BasicAccidentalDignityEvaluation],
) {
    let objects_with_accidental: HashSet<&str> = evaluations
        .iter()
        .filter(|evaluation| !evaluation.conditions.is_empty())
        .map(|evaluation| evaluation.object_code.as_str())
        .collect();
    for dominant in chart_emphasis.dominant_objects.iter_mut() {
        if objects_with_accidental.contains(dominant.object_code.as_str())
            && !dominant
                .reasons
                .iter()
                .any(|reason| reason == "accidental_context")
        {
            dominant.reasons.push("accidental_context".to_string());
        }
    }
}

fn evaluate_mobile_object(
    position: &ObjectPositionFact,
    chart_sect: Option<&str>,
    definitions: &HashMap<&str, &AccidentalDignityConditionReference>,
    sect_by_object: &HashMap<&str, &ObjectSectAffinityReference>,
    angle_longitudes: &HashMap<&str, f64>,
    catalog: &BasicPayloadCatalog,
) -> Vec<BasicAccidentalDignityCondition> {
    let mut conditions = Vec::new();
    if let Some(condition) = house_modality_condition(position, definitions, catalog) {
        conditions.push(condition);
    }
    conditions.extend(angle_proximity_conditions(
        position,
        definitions,
        angle_longitudes,
        catalog,
    ));
    if let Some(condition) = motion_condition(position, definitions, catalog) {
        conditions.push(condition);
    }
    if let Some(condition) = horizon_condition(position, definitions, catalog) {
        conditions.push(condition);
    }
    if let Some(condition) =
        sect_condition(position, chart_sect, definitions, sect_by_object, catalog)
    {
        conditions.push(condition);
    }
    conditions
}

fn house_modality_condition(
    position: &ObjectPositionFact,
    definitions: &HashMap<&str, &AccidentalDignityConditionReference>,
    catalog: &BasicPayloadCatalog,
) -> Option<BasicAccidentalDignityCondition> {
    let modality = position_context(position, "house_modality")?;
    let code = modality.get("code")?.as_str()?;
    let condition_code = catalog.condition_code_for_house_modality(code)?;
    let definition = definitions.get(condition_code)?;
    let theme_code = position_context(position, "house_context")
        .and_then(|context| {
            context
                .get("theme_code")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "object_position".to_string());
    Some(build_condition(
        definition,
        json!({
            "house_number": position.house_number,
            "house_modality": code,
            "theme_code": theme_code
        }),
        house_modality_hint(&position.object_name, code),
    ))
}

fn angle_proximity_conditions(
    position: &ObjectPositionFact,
    definitions: &HashMap<&str, &AccidentalDignityConditionReference>,
    angle_longitudes: &HashMap<&str, f64>,
    catalog: &BasicPayloadCatalog,
) -> Vec<BasicAccidentalDignityCondition> {
    let mut conditions = Vec::new();
    let max_orb = catalog.accidental_scoring.angle_proximity_max_orb_deg;
    for trigger in catalog.angle_proximity_triggers() {
        let Some(angle_code) = trigger.angle_object_code.as_deref() else {
            continue;
        };
        let Some(angle_longitude) = angle_longitudes.get(angle_code) else {
            continue;
        };
        let orb = zodiac_distance(position.longitude_deg, *angle_longitude);
        if orb > max_orb {
            continue;
        }
        let Some(definition) = definitions.get(trigger.condition_code.as_str()) else {
            continue;
        };
        conditions.push(build_condition(
            definition,
            json!({
                "angle_code": angle_code,
                "angle_longitude_deg": round4_degree(*angle_longitude),
                "object_longitude_deg": round4_degree(position.longitude_deg),
                "orb_deg": round4(orb)
            }),
            angle_proximity_hint(&position.object_name, angle_code),
        ));
    }
    conditions
}

fn motion_condition(
    position: &ObjectPositionFact,
    definitions: &HashMap<&str, &AccidentalDignityConditionReference>,
    catalog: &BasicPayloadCatalog,
) -> Option<BasicAccidentalDignityCondition> {
    let motion_context = position_context(position, "motion_context")?;
    let motion_state = motion_context
        .get("motion_state")
        .and_then(|value| value.as_str())?;
    let condition_code = catalog.condition_code_for_motion_state(motion_state)?;
    let definition = definitions.get(condition_code)?;
    Some(build_condition(
        definition,
        json!({
            "motion_state": motion_state,
            "motion_state_id": position.motion_state_id
        }),
        motion_hint(&position.object_name, motion_state),
    ))
}

fn horizon_condition(
    position: &ObjectPositionFact,
    definitions: &HashMap<&str, &AccidentalDignityConditionReference>,
    catalog: &BasicPayloadCatalog,
) -> Option<BasicAccidentalDignityCondition> {
    let visibility = chart_context::visibility_context(position);
    let horizon_position = visibility
        .get("horizon_position")
        .and_then(|value| value.as_str())?;
    let condition_code = catalog.condition_code_for_horizon_position(horizon_position)?;
    let definition = definitions.get(condition_code)?;
    let mut source = json!({
        "horizon_position": horizon_position
    });
    if let Some(altitude) = visibility
        .get("altitude_deg")
        .and_then(|value| value.as_f64())
    {
        source["altitude_deg"] = json!(round4(altitude));
    }
    Some(build_condition(
        definition,
        source,
        horizon_hint(&position.object_name, horizon_position),
    ))
}

fn sect_condition(
    position: &ObjectPositionFact,
    chart_sect: Option<&str>,
    definitions: &HashMap<&str, &AccidentalDignityConditionReference>,
    sect_by_object: &HashMap<&str, &ObjectSectAffinityReference>,
    catalog: &BasicPayloadCatalog,
) -> Option<BasicAccidentalDignityCondition> {
    let affinity = sect_by_object.get(position.object_code.as_str())?;
    let chart_sect = chart_sect?;
    let condition_code = catalog.sect_condition_code(chart_sect, affinity)?;
    let definition = definitions.get(condition_code)?;
    Some(build_condition(
        definition,
        json!({
            "chart_sect": chart_sect,
            "object_sect_affinity": affinity.sect_affinity_code
        }),
        sect_hint(
            &position.object_name,
            chart_sect,
            &affinity.sect_affinity_code,
            affinity.is_variable,
        ),
    ))
}

fn build_condition(
    definition: &AccidentalDignityConditionReference,
    source: Value,
    interpretive_hint: String,
) -> BasicAccidentalDignityCondition {
    BasicAccidentalDignityCondition {
        condition_code: definition.condition_code.clone(),
        condition_family: definition.condition_family.clone(),
        polarity: definition.polarity.clone(),
        strength_score: definition.strength_score,
        score_delta: definition.score_delta,
        source,
        interpretive_hint,
    }
}

fn angle_longitudes_from_positions(positions: &[ObjectPositionFact]) -> HashMap<&str, f64> {
    let mut map = HashMap::new();
    for position in positions.iter().filter(|position| is_angle(position)) {
        map.insert(position.object_code.as_str(), position.longitude_deg);
    }
    map
}

fn is_angle(position: &ObjectPositionFact) -> bool {
    let Some(context) = position_context(position, "object_context") else {
        return false;
    };
    let role = context
        .get("role")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let role_label = context
        .get("role_label")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    role == "angle" || role_label == "Angle"
}

fn zodiac_distance(left: f64, right: f64) -> f64 {
    let delta = (left - right).abs();
    delta.min(360.0 - delta)
}

fn house_modality_hint(object_name: &str, modality: &str) -> String {
    match modality {
        "angular" => format!(
            "{object_name} is placed in an angular house, increasing its concrete expression in the chart."
        ),
        "succedent" => format!(
            "{object_name} is placed in a succedent house, giving it a stabilizing but less immediate expression context."
        ),
        "cadent" => format!(
            "{object_name} is placed in a cadent house, giving this factor a more indirect or contextual expression."
        ),
        _ => format!("{object_name} house modality shapes its accidental expression context."),
    }
}

fn angle_proximity_hint(object_name: &str, angle_code: &str) -> String {
    let angle_label = match angle_code {
        "ascendant" => "Ascendant",
        "descendant" => "Descendant",
        "mc" => "Midheaven",
        "ic" => "IC",
        _ => angle_code,
    };
    format!(
        "{object_name} is close to the {angle_label}, making its symbolism more prominent in the chart structure."
    )
}

fn motion_hint(object_name: &str, motion_state: &str) -> String {
    match motion_state {
        "retrograde" => format!(
            "{object_name} is retrograde, adding internalization or reconsideration to its expression context."
        ),
        "stationary" => format!(
            "{object_name} is stationary, intensifying its expression at this point in the chart."
        ),
        _ => format!("{object_name} motion state shapes its accidental expression context."),
    }
}

fn horizon_hint(object_name: &str, horizon_position: &str) -> String {
    match horizon_position {
        "above_horizon" => format!(
            "{object_name} is above the horizon, giving this factor a more outward or visible expression context."
        ),
        "below_horizon" => format!(
            "{object_name} is below the horizon, giving this factor a more private or interior expression context."
        ),
        "on_horizon" => format!(
            "{object_name} is on the horizon, giving this factor a highly visible expression context."
        ),
        _ => format!("{object_name} horizon position shapes its accidental expression context."),
    }
}

fn sect_hint(
    object_name: &str,
    chart_sect: &str,
    object_affinity: &str,
    is_variable: bool,
) -> String {
    if is_variable {
        return format!(
            "{object_name} has a variable sect affinity that is not fully resolved in this MVP."
        );
    }
    if object_affinity == chart_sect {
        format!(
            "{object_name} matches the {} sect of the chart.",
            sect_label(chart_sect)
        )
    } else {
        format!(
            "{object_name} contrasts with the {} sect of the chart.",
            sect_label(chart_sect)
        )
    }
}

fn sect_label(sect: &str) -> &'static str {
    match sect {
        "day" => "diurnal",
        "night" => "nocturnal",
        _ => "chart",
    }
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn round4_degree(value: f64) -> f64 {
    round4(value.rem_euclid(360.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{overall_polarity_for_score_with_bands, test_catalog};
    use crate::domain::{
        AccidentalDignityConditionReference, ObjectPositionFact, ObjectSectAffinityReference,
    };
    use serde_json::json;

    fn mobile_position(
        object_code: &str,
        longitude: f64,
        house_modality: &str,
    ) -> ObjectPositionFact {
        ObjectPositionFact {
            chart_object_id: 1,
            object_code: object_code.to_string(),
            object_name: object_code.to_string(),
            zodiacal_reference_system_id: 1,
            coordinate_reference_system_id: 1,
            sign_id: 1,
            sign_code: "aries".to_string(),
            sign_name: "Aries".to_string(),
            house_id: Some(1),
            house_number: Some(1),
            house_name: Some("Self".to_string()),
            motion_state_id: Some(1),
            horizon_position_id: Some(1),
            longitude_deg: longitude,
            latitude_deg: Some(0.0),
            apparent_speed_deg_per_day: Some(1.0),
            altitude_deg: Some(5.0),
            is_visible: Some(true),
            facts_json: Some(json!({
                "house_modality": { "code": house_modality },
                "house_context": { "theme_code": "identity" },
                "object_context": { "role": "personal_planet" },
                "motion_context": { "motion_state": "direct" }
            })),
        }
    }

    fn angle_position(object_code: &str, longitude: f64) -> ObjectPositionFact {
        ObjectPositionFact {
            chart_object_id: 99,
            object_code: object_code.to_string(),
            object_name: object_code.to_string(),
            zodiacal_reference_system_id: 1,
            coordinate_reference_system_id: 1,
            sign_id: 1,
            sign_code: "aries".to_string(),
            sign_name: "Aries".to_string(),
            house_id: Some(1),
            house_number: Some(1),
            house_name: Some("Self".to_string()),
            motion_state_id: None,
            horizon_position_id: None,
            longitude_deg: longitude,
            latitude_deg: None,
            apparent_speed_deg_per_day: None,
            altitude_deg: None,
            is_visible: None,
            facts_json: Some(json!({
                "object_context": { "role": "angle", "role_label": "Angle" }
            })),
        }
    }

    fn definitions() -> Vec<AccidentalDignityConditionReference> {
        vec![
            AccidentalDignityConditionReference {
                condition_code: "angular_house".to_string(),
                condition_family: "house_modality".to_string(),
                label: "Angular house".to_string(),
                polarity: "dignity".to_string(),
                strength_score: 0.75,
                score_delta: 0.25,
                description: "angular".to_string(),
            },
            AccidentalDignityConditionReference {
                condition_code: "sect_affinity_match".to_string(),
                condition_family: "sect".to_string(),
                label: "Sect affinity match".to_string(),
                polarity: "dignity".to_string(),
                strength_score: 0.45,
                score_delta: 0.08,
                description: "sect match".to_string(),
            },
        ]
    }

    #[test]
    fn angles_are_excluded_from_accidental_evaluations() {
        let positions = vec![
            angle_position("ascendant", 0.0),
            mobile_position("mars", 10.0, "angular"),
        ];
        let build = build_accidental_dignities(
            &positions,
            Some("night"),
            &definitions(),
            &[],
            &HashSet::new(),
            &test_catalog(),
        );
        assert_eq!(build.evaluations.len(), 1);
        assert_eq!(build.evaluations[0].object_code, "mars");
    }

    #[test]
    fn near_ascendant_triggers_within_ten_degree_orb() {
        let positions = vec![
            angle_position("ascendant", 0.0),
            mobile_position("pluto", 3.75, "angular"),
        ];
        let near_asc = AccidentalDignityConditionReference {
            condition_code: "near_ascendant".to_string(),
            condition_family: "angle_proximity".to_string(),
            label: "Near Ascendant".to_string(),
            polarity: "dignity".to_string(),
            strength_score: 0.82,
            score_delta: 0.22,
            description: "near asc".to_string(),
        };
        let build = build_accidental_dignities(
            &positions,
            None,
            &[definitions()[0].clone(), near_asc],
            &[],
            &HashSet::new(),
            &test_catalog(),
        );
        assert!(build.evaluations[0]
            .conditions
            .iter()
            .any(|condition| condition.condition_code == "near_ascendant"));
    }

    #[test]
    fn overall_polarity_uses_point_three_as_weakened_floor() {
        let bands = test_catalog().accidental_polarity_bands;
        assert_eq!(
            overall_polarity_for_score_with_bands(0.30, &bands).0,
            "weakened"
        );
        assert_eq!(
            overall_polarity_for_score_with_bands(0.299, &bands).0,
            "strongly_weakened"
        );
        assert_eq!(
            overall_polarity_for_score_with_bands(0.28, &bands).0,
            "strongly_weakened"
        );
        assert_eq!(
            overall_polarity_for_score_with_bands(0.45, &bands).0,
            "mixed_or_contextual"
        );
        assert_eq!(
            overall_polarity_for_score_with_bands(0.70, &bands).0,
            "fortified"
        );
    }

    #[test]
    fn overall_score_clamps_after_delta_sum() {
        let positions = vec![mobile_position("pluto", 10.0, "angular")];
        let build = build_accidental_dignities(
            &positions,
            Some("night"),
            &definitions(),
            &[],
            &HashSet::new(),
            &test_catalog(),
        );
        assert!((build.evaluations[0].overall_score - 0.75).abs() <= 0.0001);
        assert_eq!(build.evaluations[0].overall_polarity, "fortified");
    }

    #[test]
    fn sect_affinity_match_requires_chart_sect() {
        let positions = vec![mobile_position("mars", 10.0, "angular")];
        let sects = vec![ObjectSectAffinityReference {
            object_code: "mars".to_string(),
            sect_affinity_code: "night".to_string(),
            is_variable: false,
            description: "mars night".to_string(),
        }];
        let build = build_accidental_dignities(
            &positions,
            Some("night"),
            &definitions(),
            &sects,
            &HashSet::new(),
            &test_catalog(),
        );
        assert!(build.evaluations[0]
            .conditions
            .iter()
            .any(|condition| condition.condition_code == "sect_affinity_match"));
    }
}
