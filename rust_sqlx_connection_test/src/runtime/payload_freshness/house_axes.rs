use std::collections::{HashMap, HashSet};

use crate::domain::{BasicHouseAxisEmphasis, BasicPayload, BasicSignal};

pub(super) fn has_current_house_axis_emphasis(payload: &BasicPayload) -> bool {
    if payload.house_axis_emphasis.is_empty() || payload.house_axis_emphasis.len() > 3 {
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
        && axis.interpretive_hint == expected_interpretive_hint(axis)
        && has_current_cross_axis_aspect_context(axis, signals_by_key, position_house_by_object)
        && axis.house_scores.iter().enumerate().all(|(index, score)| {
            score.house_number == axis.houses[index]
                && score.theme_code == axis.theme_codes[index]
                && axis_score_is_valid(score.score)
                && !score.reasons.is_empty()
                && score.reasons.iter().all(|reason| !reason.trim().is_empty())
        })
}

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
        .reasons
        .iter()
        .any(|reason| reason == "cross_axis_aspect");
    let house_scores_have_reason = axis.house_scores.iter().all(|score| {
        score
            .reasons
            .iter()
            .any(|reason| reason == "cross_axis_aspect")
    });

    if has_bridge_aspect {
        axis_has_reason && house_scores_have_reason
    } else {
        !axis_has_reason
            && axis.house_scores.iter().all(|score| {
                !score
                    .reasons
                    .iter()
                    .any(|reason| reason == "cross_axis_aspect")
            })
    }
}

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

fn axis_score_is_valid(score: f64) -> bool {
    score.is_finite() && (0.0..=1.0).contains(&score)
}

fn has_unique_non_empty_strings(values: &[String]) -> bool {
    let mut seen = std::collections::HashSet::new();
    values
        .iter()
        .all(|value| !value.trim().is_empty() && seen.insert(value.as_str()))
}

fn push_unique(target: &mut Vec<String>, value: String) {
    if !target.iter().any(|existing| existing == &value) {
        target.push(value);
    }
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

fn axis_label(axis_code: &str) -> &'static str {
    match axis_code {
        "self_relationship" => "Self and Relationship",
        "resources_sharing" => "Resources and Sharing",
        "local_distant" => "Local and Distant",
        "private_public" => "Private and Public",
        "creation_collective" => "Creation and Collective",
        "control_surrender" => "Control and Surrender",
        _ => "",
    }
}
