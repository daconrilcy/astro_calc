use std::collections::{HashMap, HashSet};

use crate::domain::{
    BasicAccidentalDignityCondition, BasicAccidentalDignityEvaluation, BasicObjectPosition,
    BasicPayload, BasicSignal,
};

use crate::runtime::references::ACCIDENTAL_CONDITION_CODES;

const SCORE_TOLERANCE: f64 = 0.001;
const ANGLE_ORB_TOLERANCE_DEG: f64 = 0.01;
const ANGLE_PROXIMITY_MAX_ORB_DEG: f64 = 10.0;

pub(super) fn has_current_accidental_dignities(payload: &BasicPayload) -> bool {
    payload.chart_context.payload_contract.contract_version == "natal_structured_v13"
        && has_valid_accidental_dignities_block(payload)
        && payload.positions.iter().all(has_current_position_accidental_context)
        && payload
            .signals
            .iter()
            .all(has_current_signal_accidental_context)
        && accidental_signal_context_matches_positions(payload)
        && accidental_context_matches_positions(payload)
        && accidental_conditions_match_position_facts(payload)
}

fn has_valid_accidental_dignities_block(payload: &BasicPayload) -> bool {
    let position_codes: HashSet<&str> = payload
        .positions
        .iter()
        .filter(|position| !is_angle_position(position))
        .map(|position| position.object_code.as_str())
        .collect();
    let evaluation_codes: HashSet<&str> = payload
        .accidental_dignities
        .iter()
        .map(|evaluation| evaluation.object_code.as_str())
        .collect();
    let signal_keys: HashSet<&str> = payload
        .signals
        .iter()
        .map(|signal| signal.signal_key.as_str())
        .collect();

    !payload.accidental_dignities.is_empty()
        && evaluation_codes.iter().all(|code| position_codes.contains(code))
        && !payload
            .accidental_dignities
            .iter()
            .any(|evaluation| is_angle_position_code(payload, &evaluation.object_code))
        && payload.accidental_dignities.iter().all(|evaluation| {
            has_valid_evaluation(evaluation, &signal_keys)
        })
}

fn has_valid_evaluation(
    evaluation: &BasicAccidentalDignityEvaluation,
    signal_keys: &HashSet<&str>,
) -> bool {
    !evaluation.object_code.trim().is_empty()
        && !evaluation.object_name.trim().is_empty()
        && (0.0..=1.0).contains(&evaluation.overall_score)
        && valid_overall_polarity(evaluation.overall_polarity.as_str())
        && overall_polarity_matches_score(evaluation.overall_score, &evaluation.overall_polarity)
        && !evaluation.expression_quality.trim().is_empty()
        && evaluation
            .related_signal_key
            .as_ref()
            .is_none_or(|key| !key.trim().is_empty() && signal_keys.contains(key.as_str()))
        && !evaluation.conditions.is_empty()
        && evaluation.conditions.iter().all(has_valid_condition)
        && has_unique_condition_codes(evaluation)
        && overall_score_matches_deltas(evaluation)
        && expression_quality_matches_polarity(
            evaluation.overall_polarity.as_str(),
            evaluation.expression_quality.as_str(),
        )
        && related_signal_key_matches_object(evaluation)
}

fn has_valid_condition(condition: &BasicAccidentalDignityCondition) -> bool {
    !condition.condition_code.trim().is_empty()
        && is_known_accidental_condition_code(condition.condition_code.as_str())
        && valid_condition_family(condition.condition_family.as_str())
        && valid_polarity(condition.polarity.as_str())
        && (0.0..=1.0).contains(&condition.strength_score)
        && (-1.0..=1.0).contains(&condition.score_delta)
        && condition.source.is_object()
        && !condition.interpretive_hint.trim().is_empty()
}

fn is_known_accidental_condition_code(code: &str) -> bool {
    ACCIDENTAL_CONDITION_CODES.contains(&code)
}

fn has_unique_condition_codes(evaluation: &BasicAccidentalDignityEvaluation) -> bool {
    let mut seen = HashSet::new();
    evaluation
        .conditions
        .iter()
        .all(|condition| seen.insert(condition.condition_code.as_str()))
}

fn related_signal_key_matches_object(evaluation: &BasicAccidentalDignityEvaluation) -> bool {
    evaluation.related_signal_key.as_ref().is_none_or(|key| {
        key == &format!("object_position:{}", evaluation.object_code)
    })
}

fn expression_quality_matches_polarity(polarity: &str, expression_quality: &str) -> bool {
    expression_quality == expression_quality_for_polarity(polarity)
}

fn expression_quality_for_polarity(polarity: &str) -> &'static str {
    match polarity {
        "fortified" => "strong_external_manifestation",
        "mixed_or_contextual" => "mixed_or_contextual_expression",
        "weakened" => "constrained_expression",
        "strongly_weakened" => "strongly_constrained_expression",
        _ => "",
    }
}

fn has_current_position_accidental_context(position: &BasicObjectPosition) -> bool {
    position
        .accidental_dignity_context
        .iter()
        .all(|summary| {
            !summary.condition_code.trim().is_empty()
                && valid_condition_family(summary.condition_family.as_str())
                && valid_polarity(summary.polarity.as_str())
                && (0.0..=1.0).contains(&summary.strength_score)
        })
}

fn has_current_signal_accidental_context(signal: &BasicSignal) -> bool {
    if !signal.signal_key.starts_with("object_position:") {
        return true;
    }
    let Some(evidence) = signal.evidence.as_ref() else {
        return false;
    };
    let Some(context) = evidence.get("placement_context") else {
        return false;
    };
    context.get("accidental_dignity_context").is_some_and(|value| {
        value.is_array()
            && value
                .as_array()
                .is_some_and(|items| items.iter().all(|item| item.is_object()))
    })
}

pub(super) fn accidental_context_matches_positions(payload: &BasicPayload) -> bool {
    let evaluations: HashMap<&str, &BasicAccidentalDignityEvaluation> = payload
        .accidental_dignities
        .iter()
        .map(|evaluation| (evaluation.object_code.as_str(), evaluation))
        .collect();

    payload.positions.iter().all(|position| {
        if is_angle_position(position) {
            return position.accidental_dignity_context.is_empty();
        }
        let empty = Vec::new();
        let expected = evaluations
            .get(position.object_code.as_str())
            .map(|evaluation| &evaluation.conditions)
            .unwrap_or(&empty);
        summaries_match_conditions(&position.accidental_dignity_context, expected)
    })
}

pub(super) fn accidental_conditions_match_position_facts(payload: &BasicPayload) -> bool {
    let chart_sect = payload.chart_context.sect.chart_sect.as_deref();
    let angle_longitudes = angle_longitudes(payload);
    payload.accidental_dignities.iter().all(|evaluation| {
        let Some(position) = payload
            .positions
            .iter()
            .find(|position| position.object_code == evaluation.object_code)
        else {
            return false;
        };
        evaluation.conditions.iter().all(|condition| {
            condition_matches_position(
                condition,
                position,
                chart_sect,
                &angle_longitudes,
            )
        })
    })
}

fn condition_matches_position(
    condition: &BasicAccidentalDignityCondition,
    position: &BasicObjectPosition,
    chart_sect: Option<&str>,
    angle_longitudes: &HashMap<&str, f64>,
) -> bool {
    match condition.condition_code.as_str() {
        "angular_house" => house_modality_code(position) == Some("angular"),
        "succedent_house" => house_modality_code(position) == Some("succedent"),
        "cadent_house" => house_modality_code(position) == Some("cadent"),
        "retrograde_motion" => motion_state(position) == Some("retrograde"),
        "stationary_motion" => motion_state(position) == Some("stationary"),
        "above_horizon" => horizon_position(position) == Some("above_horizon"),
        "below_horizon" => horizon_position(position) == Some("below_horizon"),
        "on_horizon" => horizon_position(position) == Some("on_horizon"),
        "near_ascendant" => angle_proximity_matches(condition, position, "ascendant", angle_longitudes),
        "near_descendant" => {
            angle_proximity_matches(condition, position, "descendant", angle_longitudes)
        }
        "near_mc" => angle_proximity_matches(condition, position, "mc", angle_longitudes),
        "near_ic" => angle_proximity_matches(condition, position, "ic", angle_longitudes),
        "sect_affinity_match" => sect_matches(chart_sect, condition, true),
        "sect_affinity_mismatch" => sect_matches(chart_sect, condition, false),
        "sect_affinity_variable_unresolved" => condition
            .source
            .get("object_sect_affinity")
            .and_then(|value| value.as_str())
            == Some("variable"),
        _ => false,
    }
}

fn angle_proximity_matches(
    condition: &BasicAccidentalDignityCondition,
    position: &BasicObjectPosition,
    angle_code: &str,
    angle_longitudes: &HashMap<&str, f64>,
) -> bool {
    let Some(angle_longitude) = angle_longitudes.get(angle_code) else {
        return false;
    };
    let orb = zodiac_distance(position.longitude_deg, *angle_longitude);
    let source_orb = condition
        .source
        .get("orb_deg")
        .and_then(|value| value.as_f64());
    orb <= ANGLE_PROXIMITY_MAX_ORB_DEG + ANGLE_ORB_TOLERANCE_DEG
        && source_orb.is_some_and(|value| (value - orb).abs() <= ANGLE_ORB_TOLERANCE_DEG)
}

pub(super) fn accidental_signal_context_matches_positions(payload: &BasicPayload) -> bool {
    for signal in &payload.signals {
        if !signal.signal_key.starts_with("object_position:") {
            continue;
        }
        let Some(object_code) = signal.signal_key.strip_prefix("object_position:") else {
            return false;
        };
        let Some(position) = payload
            .positions
            .iter()
            .find(|position| position.object_code == object_code)
        else {
            return false;
        };
        let Some(context) = signal
            .evidence
            .as_ref()
            .and_then(|evidence| evidence.get("placement_context"))
        else {
            return false;
        };
        let Some(signal_context) = context.get("accidental_dignity_context") else {
            return false;
        };
        let Ok(signal_summaries) =
            serde_json::from_value::<Vec<crate::domain::BasicAccidentalDignityContextSummary>>(
                signal_context.clone(),
            )
        else {
            return false;
        };
        if signal_summaries != position.accidental_dignity_context {
            return false;
        }
    }
    true
}

fn sect_matches(
    chart_sect: Option<&str>,
    condition: &BasicAccidentalDignityCondition,
    expect_match: bool,
) -> bool {
    let Some(chart_sect) = chart_sect else {
        return false;
    };
    let Some(object_affinity) = condition
        .source
        .get("object_sect_affinity")
        .and_then(|value| value.as_str())
    else {
        return false;
    };
    if object_affinity == "variable" {
        return false;
    }
    (object_affinity == chart_sect) == expect_match
}

fn summaries_match_conditions(
    summaries: &[crate::domain::BasicAccidentalDignityContextSummary],
    conditions: &[BasicAccidentalDignityCondition],
) -> bool {
    if summaries.len() != conditions.len() {
        return false;
    }
    summaries.iter().zip(conditions.iter()).all(|(summary, condition)| {
        summary.condition_code == condition.condition_code
            && summary.condition_family == condition.condition_family
            && summary.polarity == condition.polarity
            && (summary.strength_score - condition.strength_score).abs() <= SCORE_TOLERANCE
    })
}

fn overall_score_matches_deltas(evaluation: &BasicAccidentalDignityEvaluation) -> bool {
    let raw_score: f64 = evaluation
        .conditions
        .iter()
        .map(|condition| condition.score_delta)
        .sum();
    let expected = (0.5 + raw_score).clamp(0.0, 1.0);
    (evaluation.overall_score - expected).abs() <= SCORE_TOLERANCE
}

fn overall_polarity_matches_score(score: f64, polarity: &str) -> bool {
    overall_polarity_for_score(score) == polarity
}

fn overall_polarity_for_score(score: f64) -> &'static str {
    if score >= 0.70 {
        "fortified"
    } else if score >= 0.45 {
        "mixed_or_contextual"
    } else if score >= 0.30 {
        "weakened"
    } else {
        "strongly_weakened"
    }
}

fn valid_overall_polarity(value: &str) -> bool {
    matches!(
        value,
        "fortified" | "mixed_or_contextual" | "weakened" | "strongly_weakened"
    )
}

fn valid_condition_family(value: &str) -> bool {
    matches!(
        value,
        "house_modality" | "angle_proximity" | "motion" | "horizon" | "sect"
    )
}

fn valid_polarity(value: &str) -> bool {
    matches!(value, "dignity" | "debility" | "contextual" | "intensifier")
}

fn is_angle_position(position: &BasicObjectPosition) -> bool {
    let role = position
        .object_context
        .as_ref()
        .and_then(|context| context.get("role"))
        .and_then(|value| value.as_str());
    let role_label = position
        .object_context
        .as_ref()
        .and_then(|context| context.get("role_label"))
        .and_then(|value| value.as_str());
    role == Some("angle") || role_label == Some("Angle")
}

fn is_angle_position_code(payload: &BasicPayload, object_code: &str) -> bool {
    payload
        .positions
        .iter()
        .find(|position| position.object_code == object_code)
        .is_some_and(is_angle_position)
}

fn house_modality_code(position: &BasicObjectPosition) -> Option<&str> {
    position
        .house_modality
        .as_ref()
        .and_then(|value| value.get("code"))
        .and_then(|value| value.as_str())
}

fn motion_state(position: &BasicObjectPosition) -> Option<&str> {
    position
        .motion_context
        .as_ref()
        .and_then(|value| value.get("motion_state"))
        .and_then(|value| value.as_str())
}

fn horizon_position(position: &BasicObjectPosition) -> Option<&str> {
    position
        .visibility_context
        .get("horizon_position")
        .and_then(|value| value.as_str())
}

fn angle_longitudes(payload: &BasicPayload) -> HashMap<&str, f64> {
    payload
        .positions
        .iter()
        .filter(|position| is_angle_position(position))
        .map(|position| (position.object_code.as_str(), position.longitude_deg))
        .collect()
}

fn zodiac_distance(left: f64, right: f64) -> f64 {
    let delta = (left - right).abs();
    delta.min(360.0 - delta)
}
