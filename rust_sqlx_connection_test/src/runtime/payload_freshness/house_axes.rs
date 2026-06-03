use std::collections::HashSet;

use crate::domain::{BasicHouseAxisEmphasis, BasicPayload};

pub(super) fn has_current_house_axis_emphasis(payload: &BasicPayload) -> bool {
    if payload.house_axis_emphasis.is_empty() || payload.house_axis_emphasis.len() > 3 {
        return false;
    }

    let signal_keys: HashSet<&str> = payload
        .signals
        .iter()
        .map(|signal| signal.signal_key.as_str())
        .collect();

    let mut previous_score = f64::INFINITY;
    let mut seen_axes = HashSet::new();
    for axis in &payload.house_axis_emphasis {
        if axis.axis_score > previous_score {
            return false;
        }
        previous_score = axis.axis_score;

        if !seen_axes.insert(axis.axis_code.as_str())
            || !has_current_axis(axis)
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

fn has_current_axis(axis: &BasicHouseAxisEmphasis) -> bool {
    let Some(reference) = canonical_axis(axis.axis_code.as_str()) else {
        return false;
    };

    if axis.houses.len() != 2
        || axis.theme_codes.len() != 2
        || axis.house_scores.len() != 2
        || axis.houses != reference.houses
        || axis.theme_codes != reference.theme_codes
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
        && !axis.reasons.is_empty()
        && axis.reasons.iter().all(|reason| !reason.trim().is_empty())
        && has_unique_non_empty_strings(&axis.source_signal_keys)
        && has_unique_non_empty_strings(&axis.source_context_keys)
        && axis
            .source_signal_keys
            .iter()
            .chain(axis.source_context_keys.iter())
            .all(|key| !key.trim().is_empty())
        && !axis.interpretive_hint.trim().is_empty()
        && axis.house_scores.iter().enumerate().all(|(index, score)| {
            score.house_number == axis.houses[index]
                && score.theme_code == axis.theme_codes[index]
                && axis_score_is_valid(score.score)
                && !score.reasons.is_empty()
                && score.reasons.iter().all(|reason| !reason.trim().is_empty())
        })
}

fn axis_score_is_valid(score: f64) -> bool {
    score.is_finite() && (0.0..=1.0).contains(&score)
}

fn has_unique_non_empty_strings(values: &[String]) -> bool {
    let mut seen = std::collections::HashSet::new();
    values
        .iter()
        .all(|value| !value.trim().is_empty() && seen.insert(value.as_str()))
}

struct CanonicalAxis {
    houses: Vec<i32>,
    theme_codes: Vec<String>,
}

fn canonical_axis(axis_code: &str) -> Option<CanonicalAxis> {
    let (houses, theme_codes) = match axis_code {
        "self_relationship" => ([1, 7], ["identity", "relationships"]),
        "resources_sharing" => ([2, 8], ["resources", "shared_resources"]),
        "local_distant" => ([3, 9], ["communication", "beliefs"]),
        "private_public" => ([4, 10], ["roots", "career"]),
        "creation_collective" => ([5, 11], ["creativity", "community"]),
        "control_surrender" => ([6, 12], ["work_health", "inner_world"]),
        _ => return None,
    };

    Some(CanonicalAxis {
        houses: houses.to_vec(),
        theme_codes: theme_codes.into_iter().map(ToString::to_string).collect(),
    })
}

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

fn score_matches(left: f64, right: f64) -> bool {
    (left - right).abs() <= 0.0001
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}
