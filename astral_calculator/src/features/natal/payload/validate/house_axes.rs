//! Module astral_calculator\src\features\natal\payload\validate\house_axes.rs du moteur astral_calculator.

use std::collections::{HashMap, HashSet};

use crate::domain::{BasicHouseAxisEmphasis, BasicPayload, BasicSignal};
use crate::features::natal::payload::rules::house_axes::{axis_label, canonical_axis};
use crate::features::natal::payload::shared::text::{
    has_text, has_unique_non_empty_strings, is_normalized_score, push_unique,
};

use super::emphasis::{product_scoring_snapshot, valid_projection_reasons};

pub(super) fn has_current_house_axis_emphasis(payload: &BasicPayload) -> bool {
    let Some(scoring) = product_scoring_snapshot(payload) else {
        return false;
    };
    if payload.house_axis_emphasis.is_empty()
        || payload.house_axis_emphasis.len() > scoring.max_house_axis_emphasis
    {
        return false;
    }

    let signal_keys: HashSet<&str> = payload
        .signals
        .iter()
        .map(|signal| signal.signal_key.as_str())
        .collect();
    let signals_by_key: HashMap<&str, &BasicSignal> = payload
        .signals
        .iter()
        .map(|signal| (signal.signal_key.as_str(), signal))
        .collect();
    let position_house_by_object: HashMap<&str, i32> = payload
        .positions
        .iter()
        .filter_map(|position| {
            position
                .house_number
                .map(|house| (position.object_code.as_str(), house))
        })
        .collect();

    let mut previous_score = f64::INFINITY;
    let mut seen_axes = HashSet::new();
    for axis in &payload.house_axis_emphasis {
        if axis.axis_score > previous_score {
            return false;
        }
        previous_score = axis.axis_score;

        if !seen_axes.insert(axis.axis_code.as_str())
            || !has_current_axis(axis, &signals_by_key, &position_house_by_object)
            || axis
                .source_signal_keys
                .iter()
                .any(|key| !signal_keys.contains(key.as_str()))
        {
            return false;
        }
    }

    true
}

/// Fonction has_current_axis.
fn has_current_axis(
    axis: &BasicHouseAxisEmphasis,
    signals_by_key: &HashMap<&str, &BasicSignal>,
    position_house_by_object: &HashMap<&str, i32>,
) -> bool {
    let Some(reference) = canonical_axis(axis.axis_code.as_str()) else {
        return false;
    };

    if axis.houses.len() != 2
        || axis.theme_codes.len() != 2
        || axis.house_scores.len() != 2
        || axis.houses != reference.houses.to_vec()
        || axis.theme_codes
            != reference
                .theme_codes
                .into_iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
    {
        return false;
    }

    let first_score = axis.house_scores[0].score;
    let second_score = axis.house_scores[1].score;
    let expected_axis_score = round4(
        (first_score.max(second_score) + 0.35 * first_score.min(second_score)).clamp(0.0, 1.0),
    );
    let expected_primary_house = if first_score >= second_score {
        axis.houses[0]
    } else {
        axis.houses[1]
    };
    let expected_secondary_house = if expected_primary_house == axis.houses[0] {
        axis.houses[1]
    } else {
        axis.houses[0]
    };
    let expected_polarity_balance = polarity_balance(first_score, second_score);

    axis_score_is_valid(axis.axis_score)
        && score_matches(axis.axis_score, expected_axis_score)
        && axis.primary_house == expected_primary_house
        && axis.secondary_house == expected_secondary_house
        && expected_polarity_balance != "weak_axis"
        && axis.polarity_balance == expected_polarity_balance
        && valid_projection_reasons(&axis.reason_details)
        && has_unique_non_empty_strings(&axis.source_signal_keys)
        && has_unique_non_empty_strings(&axis.source_context_keys)
        && axis
            .source_signal_keys
            .iter()
            .chain(axis.source_context_keys.iter())
            .all(|key| has_text(key))
        && axis.interpretive_hint == expected_interpretive_hint(axis)
        && has_current_cross_axis_aspect_context(axis, signals_by_key, position_house_by_object)
        && axis.house_scores.iter().enumerate().all(|(index, score)| {
            score.house_number == axis.houses[index]
                && score.theme_code == axis.theme_codes[index]
                && axis_score_is_valid(score.score)
                && valid_projection_reasons(&score.reason_details)
        })
}

/// Fonction has_current_cross_axis_aspect_context.
fn has_current_cross_axis_aspect_context(
    axis: &BasicHouseAxisEmphasis,
    signals_by_key: &HashMap<&str, &BasicSignal>,
    position_house_by_object: &HashMap<&str, i32>,
) -> bool {
    let has_bridge_aspect = axis.source_signal_keys.iter().any(|signal_key| {
        let Some(signal) = signals_by_key.get(signal_key.as_str()) else {
            return false;
        };
        signal.signal_key.starts_with("aspect:")
            && aspect_bridges_axis(signal, axis, position_house_by_object)
    });
    let axis_has_reason = axis
        .reason_details
        .iter()
        .any(|reason| reason.reason_code == "cross_axis_aspect");
    let house_scores_have_reason = axis.house_scores.iter().all(|score| {
        score
            .reason_details
            .iter()
            .any(|reason| reason.reason_code == "cross_axis_aspect")
    });

    if has_bridge_aspect {
        axis_has_reason && house_scores_have_reason
    } else {
        !axis_has_reason
            && axis.house_scores.iter().all(|score| {
                !score
                    .reason_details
                    .iter()
                    .any(|reason| reason.reason_code == "cross_axis_aspect")
            })
    }
}

/// Fonction aspect_bridges_axis.
fn aspect_bridges_axis(
    signal: &BasicSignal,
    axis: &BasicHouseAxisEmphasis,
    position_house_by_object: &HashMap<&str, i32>,
) -> bool {
    let object_houses = signal_object_codes(signal)
        .iter()
        .filter_map(|object_code| position_house_by_object.get(object_code.as_str()).copied())
        .collect::<HashSet<_>>();

    object_houses.contains(&axis.houses[0]) && object_houses.contains(&axis.houses[1])
}

/// Fonction signal_object_codes.
fn signal_object_codes(signal: &BasicSignal) -> Vec<String> {
    let mut object_codes = Vec::new();
    if let Some(evidence) = &signal.evidence {
        for key in ["source_object_code", "target_object_code"] {
            if let Some(object_code) = evidence.get(key).and_then(|value| value.as_str()) {
                push_unique(&mut object_codes, object_code.to_string());
            }
        }
    }

    let parts: Vec<&str> = signal.signal_key.split(':').collect();
    if signal.signal_key.starts_with("aspect:") && parts.len() >= 4 {
        push_unique(&mut object_codes, parts[1].to_string());
        push_unique(&mut object_codes, parts[2].to_string());
    }

    object_codes
}

/// Fonction axis_score_is_valid.
fn axis_score_is_valid(score: f64) -> bool {
    is_normalized_score(score)
}

/// Fonction polarity_balance.
fn polarity_balance(first_score: f64, second_score: f64) -> String {
    if first_score >= second_score + 0.2 {
        "primary_house_dominant".to_string()
    } else if second_score >= first_score + 0.2 {
        "secondary_house_dominant".to_string()
    } else if first_score >= 0.35 && second_score >= 0.35 {
        "balanced_axis".to_string()
    } else {
        "weak_axis".to_string()
    }
}

/// Fonction score_matches.
fn score_matches(left: f64, right: f64) -> bool {
    (left - right).abs() <= 0.0001
}

/// Fonction round4.
fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

/// Fonction expected_interpretive_hint.
fn expected_interpretive_hint(axis: &BasicHouseAxisEmphasis) -> String {
    match axis.polarity_balance.as_str() {
        "primary_house_dominant" => format!(
            "{} is activated mainly through house {} ({}), with house {} ({}) present as a secondary counterpoint.",
            axis_label(axis.axis_code.as_str()),
            axis.houses[0],
            axis.theme_codes[0],
            axis.houses[1],
            axis.theme_codes[1]
        ),
        "secondary_house_dominant" => format!(
            "{} is activated mainly through house {} ({}), with house {} ({}) present as a secondary counterpoint.",
            axis_label(axis.axis_code.as_str()),
            axis.houses[1],
            axis.theme_codes[1],
            axis.houses[0],
            axis.theme_codes[0]
        ),
        "balanced_axis" => format!(
            "{} is activated with both house {} ({}) and house {} ({}) strongly active.",
            axis_label(axis.axis_code.as_str()),
            axis.houses[0],
            axis.theme_codes[0],
            axis.houses[1],
            axis.theme_codes[1]
        ),
        _ => String::new(),
    }
}
