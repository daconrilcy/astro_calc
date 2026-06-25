//! Module astral_calculator\src\features\natal\payload\build\accidental_dignities.rs du moteur astral_calculator.

use std::collections::{HashMap, HashSet};

use serde_json::{json, Value};

use crate::domain::{
    AccidentalDignityConditionReference, BasicAccidentalDignityCondition,
    BasicAccidentalDignityContextSummary, BasicAccidentalDignityEvaluation, BasicChartEmphasis,
    BasicSignal, ObjectPositionFact, ObjectSectAffinityReference,
};
use crate::features::natal::catalog::BasicPayloadCatalog;
use crate::features::natal::payload::rules::chart_context::is_angle_role;

use super::json::position_context;
use super::projection_reasons::reason_simple;

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
    locale: &str,
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
            locale,
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
                .reason_details
                .iter()
                .any(|reason| reason.reason_code == "accidental_context")
        {
            dominant
                .reason_details
                .push(reason_simple("accidental_context"));
        }
    }
}

/// Fonction evaluate_mobile_object.
fn evaluate_mobile_object(
    position: &ObjectPositionFact,
    chart_sect: Option<&str>,
    definitions: &HashMap<&str, &AccidentalDignityConditionReference>,
    sect_by_object: &HashMap<&str, &ObjectSectAffinityReference>,
    angle_longitudes: &HashMap<&str, f64>,
    catalog: &BasicPayloadCatalog,
    locale: &str,
) -> Vec<BasicAccidentalDignityCondition> {
    let mut conditions = Vec::new();
    if let Some(condition) = house_modality_condition(position, definitions, catalog, locale) {
        conditions.push(condition);
    }
    conditions.extend(angle_proximity_conditions(
        position,
        definitions,
        angle_longitudes,
        catalog,
        locale,
    ));
    if let Some(condition) = motion_condition(position, definitions, catalog, locale) {
        conditions.push(condition);
    }
    if let Some(condition) = horizon_condition(position, definitions, catalog, locale) {
        conditions.push(condition);
    }
    if let Some(condition) = sect_condition(
        position,
        chart_sect,
        definitions,
        sect_by_object,
        catalog,
        locale,
    ) {
        conditions.push(condition);
    }
    conditions
}

/// Fonction house_modality_condition.
fn house_modality_condition(
    position: &ObjectPositionFact,
    definitions: &HashMap<&str, &AccidentalDignityConditionReference>,
    catalog: &BasicPayloadCatalog,
    locale: &str,
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
        house_modality_hint(&position.object_name, code, locale),
    ))
}

/// Fonction angle_proximity_conditions.
fn angle_proximity_conditions(
    position: &ObjectPositionFact,
    definitions: &HashMap<&str, &AccidentalDignityConditionReference>,
    angle_longitudes: &HashMap<&str, f64>,
    catalog: &BasicPayloadCatalog,
    locale: &str,
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
            angle_proximity_hint(&position.object_name, angle_code, locale),
        ));
    }
    conditions
}

/// Fonction motion_condition.
fn motion_condition(
    position: &ObjectPositionFact,
    definitions: &HashMap<&str, &AccidentalDignityConditionReference>,
    catalog: &BasicPayloadCatalog,
    locale: &str,
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
        motion_hint(&position.object_name, motion_state, locale),
    ))
}

/// Fonction horizon_condition.
fn horizon_condition(
    position: &ObjectPositionFact,
    definitions: &HashMap<&str, &AccidentalDignityConditionReference>,
    catalog: &BasicPayloadCatalog,
    locale: &str,
) -> Option<BasicAccidentalDignityCondition> {
    let visibility =
        crate::features::natal::payload::rules::chart_context::visibility_context(position);
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
        horizon_hint(&position.object_name, horizon_position, locale),
    ))
}

/// Fonction sect_condition.
fn sect_condition(
    position: &ObjectPositionFact,
    chart_sect: Option<&str>,
    definitions: &HashMap<&str, &AccidentalDignityConditionReference>,
    sect_by_object: &HashMap<&str, &ObjectSectAffinityReference>,
    catalog: &BasicPayloadCatalog,
    locale: &str,
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
            locale,
        ),
    ))
}

/// Fonction build_condition.
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

/// Fonction angle_longitudes_from_positions.
fn angle_longitudes_from_positions(positions: &[ObjectPositionFact]) -> HashMap<&str, f64> {
    let mut map = HashMap::new();
    for position in positions.iter().filter(|position| is_angle(position)) {
        map.insert(position.object_code.as_str(), position.longitude_deg);
    }
    map
}

/// Fonction is_angle.
fn is_angle(position: &ObjectPositionFact) -> bool {
    let role = position_context(position, "object_context").and_then(|context| {
        context
            .get("role")
            .and_then(|value| value.as_str())
            .map(str::to_string)
    });
    let role_label = position_context(position, "object_context").and_then(|context| {
        context
            .get("role_label")
            .and_then(|value| value.as_str())
            .map(str::to_string)
    });
    is_angle_role(role.as_deref(), role_label.as_deref())
}

/// Fonction zodiac_distance.
fn zodiac_distance(left: f64, right: f64) -> f64 {
    let delta = (left - right).abs();
    delta.min(360.0 - delta)
}

/// Fonction house_modality_hint.
fn house_modality_hint(object_name: &str, modality: &str, locale: &str) -> String {
    match modality {
        "angular" => localized_hint(
            object_name,
            locale,
            "is placed in an angular house, increasing its concrete expression in the chart.",
            "est placé dans une maison angulaire, ce qui renforce son expression concrète dans le thème.",
            "está situado en una casa angular, intensificando su expresión concreta en la carta.",
            "befindet sich in einem Winkelhaus, was seinen konkreten Ausdruck im Horoskop verstärkt.",
        ),
        "succedent" => localized_hint(
            object_name,
            locale,
            "is placed in a succedent house, giving it a stabilizing but less immediate expression context.",
            "est placé dans une maison succédente, ce qui lui donne un contexte d'expression stabilisant mais moins immédiat.",
            "está situado en una casa sucedente, dándole un contexto de expresión estabilizador pero menos inmediato.",
            "befindet sich in einem sukzedenten Haus und erhält dadurch einen stabilisierenden, aber weniger unmittelbaren Ausdruckskontext.",
        ),
        "cadent" => localized_hint(
            object_name,
            locale,
            "is placed in a cadent house, giving this factor a more indirect or contextual expression.",
            "est placé dans une maison cadente, ce qui donne à ce facteur une expression plus indirecte ou contextuelle.",
            "está situado en una casa cadente, dando a este factor una expresión más indirecta o contextual.",
            "befindet sich in einem fallenden Haus und erhält dadurch einen indirekteren oder kontextuellen Ausdruck.",
        ),
        _ => format!("{object_name} house modality shapes its accidental expression context."),
    }
}

/// Fonction angle_proximity_hint.
fn angle_proximity_hint(object_name: &str, angle_code: &str, locale: &str) -> String {
    let angle_label = match angle_code {
        "ascendant" => "Ascendant",
        "descendant" => "Descendant",
        "mc" => "Midheaven",
        "ic" => "IC",
        _ => angle_code,
    };
    localized_hint(
        object_name,
        locale,
        &format!(
            "is close to the {angle_label}, making its symbolism more prominent in the chart structure."
        ),
        &format!(
            "est proche de {angle_label}, rendant son symbolisme plus saillant dans la structure du thème."
        ),
        &format!(
            "está cerca del {angle_label}, haciendo que su simbolismo sea más prominente en la estructura de la carta."
        ),
        &format!(
            "ist nahe dem {angle_label} und macht seine Symbolik in der Struktur des Horoskops präsenter."
        ),
    )
}

/// Fonction motion_hint.
fn motion_hint(object_name: &str, motion_state: &str, locale: &str) -> String {
    match motion_state {
        "retrograde" => localized_hint(
            object_name,
            locale,
            "is retrograde, adding internalization or reconsideration to its expression context.",
            "est rétrograde, ajoutant intériorisation ou révision à son contexte d'expression.",
            "está retrógrado, añadiendo interiorización o reconsideración a su contexto de expresión.",
            "ist rückläufig und bringt Innenschau oder Überprüfung in seinen Ausdruckskontext ein.",
        ),
        "stationary" => localized_hint(
            object_name,
            locale,
            "is stationary, intensifying its expression at this point in the chart.",
            "est stationnaire, ce qui intensifie son expression à ce point du thème.",
            "está estacionario, intensificando su expresión en este punto de la carta.",
            "ist stationär und verstärkt seinen Ausdruck an diesem Punkt des Horoskops.",
        ),
        _ => format!("{object_name} motion state shapes its accidental expression context."),
    }
}

/// Fonction horizon_hint.
fn horizon_hint(object_name: &str, horizon_position: &str, locale: &str) -> String {
    match horizon_position {
        "above_horizon" => localized_hint(
            object_name,
            locale,
            "is above the horizon, giving this factor a more outward or visible expression context.",
            "est au-dessus de l'horizon, ce qui donne à ce facteur un contexte d'expression plus extérieur ou visible.",
            "está por encima del horizonte, otorgando a este factor un contexto de expresión más externo o visible.",
            "steht über dem Horizont und gibt diesem Faktor einen eher nach außen gerichteten oder sichtbaren Ausdruckskontext.",
        ),
        "below_horizon" => localized_hint(
            object_name,
            locale,
            "is below the horizon, giving this factor a more private or interior expression context.",
            "est sous l'horizon, ce qui donne à ce facteur un contexte d'expression plus privé ou intérieur.",
            "está por debajo del horizonte, otorgando a este factor un contexto de expresión más privado o interior.",
            "steht unter dem Horizont und gibt diesem Faktor einen privateren oder inneren Ausdruckskontext.",
        ),
        "on_horizon" => localized_hint(
            object_name,
            locale,
            "is on the horizon, giving this factor a highly visible expression context.",
            "est sur l'horizon, ce qui donne à ce facteur un contexte d'expression très visible.",
            "está en el horizonte, otorgando a este factor un contexto de expresión altamente visible.",
            "steht am Horizont und gibt diesem Faktor einen sehr sichtbaren Ausdruckskontext.",
        ),
        _ => format!("{object_name} horizon position shapes its accidental expression context."),
    }
}

/// Fonction sect_hint.
fn sect_hint(
    object_name: &str,
    chart_sect: &str,
    object_affinity: &str,
    is_variable: bool,
    locale: &str,
) -> String {
    if is_variable {
        return localized_hint(
            object_name,
            locale,
            "has a variable sect affinity that is not fully resolved in this MVP.",
            "a une affinité de secte variable qui n'est pas entièrement résolue dans ce MVP.",
            "tiene una afinidad de secta variable que no está completamente resuelta en este MVP.",
            "hat eine variable Sektzugehörigkeit, die in diesem MVP noch nicht vollständig aufgelöst ist.",
        );
    }
    if object_affinity == chart_sect {
        localized_hint(
            object_name,
            locale,
            &format!("matches the {} sect of the chart.", sect_label(chart_sect)),
            &format!("correspond à la secte {} du thème.", sect_label(chart_sect)),
            &format!(
                "coincide con la secta {} de la carta.",
                sect_label(chart_sect)
            ),
            &format!(
                "entspricht der {}-Sekte des Horoskops.",
                sect_label(chart_sect)
            ),
        )
    } else {
        localized_hint(
            object_name,
            locale,
            &format!(
                "contrasts with the {} sect of the chart.",
                sect_label(chart_sect)
            ),
            &format!(
                "contraste avec la secte {} du thème.",
                sect_label(chart_sect)
            ),
            &format!(
                "contrasta con la secta {} de la carta.",
                sect_label(chart_sect)
            ),
            &format!(
                "steht im Kontrast zur {}-Sekte des Horoskops.",
                sect_label(chart_sect)
            ),
        )
    }
}

/// Fonction sect_label.
fn sect_label(sect: &str) -> &'static str {
    match sect {
        "day" => "diurnal",
        "night" => "nocturnal",
        _ => "chart",
    }
}

/// Fonction round4.
fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

/// Fonction round4_degree.
fn round4_degree(value: f64) -> f64 {
    round4(value.rem_euclid(360.0))
}

fn localized_hint(
    object_name: &str,
    locale: &str,
    en_tail: &str,
    fr_tail: &str,
    es_tail: &str,
    de_tail: &str,
) -> String {
    match locale {
        "fr" => format!("{object_name} {fr_tail}"),
        "es" => format!("{object_name} {es_tail}"),
        "de" => format!("{object_name} {de_tail}"),
        _ => format!("{object_name} {en_tail}"),
    }
}
