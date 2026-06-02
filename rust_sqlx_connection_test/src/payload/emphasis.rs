use crate::domain::{
    BasicChartEmphasis, BasicDignity, BasicDominantHouse, BasicDominantObject, BasicDominantSign,
    BasicSignal, ObjectPositionFact,
};
use std::collections::HashMap;

use super::json::position_context;
use super::signal_filters::{aspect_strength_score, is_structural_axis_signal};
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

pub(super) fn build_chart_emphasis(
    positions: &[ObjectPositionFact],
    dignities: &[BasicDignity],
    signals: &[BasicSignal],
) -> BasicChartEmphasis {
    let mut sign_scores: HashMap<String, EmphasisScore> = HashMap::new();
    let mut house_scores: HashMap<i32, EmphasisScore> = HashMap::new();
    let mut house_theme_codes: HashMap<i32, String> = HashMap::new();
    let mut object_scores: HashMap<String, EmphasisScore> = HashMap::new();
    let positions_by_object: HashMap<&str, &ObjectPositionFact> = positions
        .iter()
        .map(|position| (position.object_code.as_str(), position))
        .collect();

    for position in positions {
        let object_weight = object_source_weight(position);

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
            if let Some(theme_code) = house_theme_code(position) {
                house_theme_codes.entry(house_number).or_insert(theme_code);
            }
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
        &mut house_theme_codes,
        &mut object_scores,
    );
    add_sign_emphasis_to_objects(positions, &sign_scores, &mut object_scores);

    BasicChartEmphasis {
        dominant_signs: normalized_signs(sign_scores),
        dominant_houses: normalized_houses(house_scores, &house_theme_codes),
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
    house_theme_codes: &mut HashMap<i32, String>,
    object_scores: &mut HashMap<String, EmphasisScore>,
) {
    for signal in signals {
        if signal.signal_key.starts_with("cluster:") {
            add_cluster_emphasis(
                signal,
                sign_scores,
                house_scores,
                house_theme_codes,
                object_scores,
            );
        } else if signal.signal_key.starts_with("aspect:")
            && !is_structural_axis_signal(signal)
            && aspect_strength_score(signal) >= 0.75
        {
            add_aspect_object_emphasis(signal, object_scores);
        }
    }
}

fn add_cluster_emphasis(
    signal: &BasicSignal,
    sign_scores: &mut HashMap<String, EmphasisScore>,
    house_scores: &mut HashMap<i32, EmphasisScore>,
    house_theme_codes: &mut HashMap<i32, String>,
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
        if let Some(theme_code) = evidence
            .get("house_theme_code")
            .and_then(|value| value.as_str())
        {
            house_theme_codes
                .entry(house_number as i32)
                .or_insert_with(|| theme_code.to_string());
        }
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

fn normalized_houses(
    scores: HashMap<i32, EmphasisScore>,
    house_theme_codes: &HashMap<i32, String>,
) -> Vec<BasicDominantHouse> {
    let mut values: Vec<_> = scores
        .into_iter()
        .filter(|(_, entry)| entry.score > 0.0)
        .map(|(house_number, entry)| BasicDominantHouse {
            house_number,
            theme_code: house_theme_codes
                .get(&house_number)
                .cloned()
                .unwrap_or_else(|| "object_position".to_string()),
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

fn object_source_weight(position: &ObjectPositionFact) -> f64 {
    position_context(position, "object_context")
        .and_then(|context| {
            context
                .get("signal_scoring")
                .and_then(|scoring| scoring.get("source_weight"))
                .and_then(|value| value.as_f64())
        })
        .unwrap_or(0.0)
}

fn house_theme_code(position: &ObjectPositionFact) -> Option<String> {
    position_context(position, "house_context").and_then(|context| {
        context
            .get("theme_code")
            .and_then(|value| value.as_str())
            .map(ToString::to_string)
    })
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}
