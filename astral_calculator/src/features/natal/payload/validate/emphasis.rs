//! Module astral_calculator\src\features\natal\payload\validate\emphasis.rs du moteur astral_calculator.

use crate::domain::{BasicPayload, BasicProductScoringSnapshot};

pub(super) fn has_current_chart_emphasis(payload: &BasicPayload) -> bool {
    let Some(scoring) = payload.chart_context.product_scoring.as_ref() else {
        return false;
    };

    !payload.chart_emphasis.dominant_signs.is_empty()
        && !payload.chart_emphasis.dominant_houses.is_empty()
        && !payload.chart_emphasis.dominant_objects.is_empty()
        && payload.chart_emphasis.dominant_signs.len() <= scoring.max_dominant_signs
        && payload.chart_emphasis.dominant_houses.len() <= scoring.max_dominant_houses
        && payload.chart_emphasis.dominant_objects.len() <= scoring.max_dominant_objects
        && payload
            .chart_emphasis
            .dominant_signs
            .windows(2)
            .all(|window| window[0].score >= window[1].score)
        && payload
            .chart_emphasis
            .dominant_houses
            .windows(2)
            .all(|window| window[0].score >= window[1].score)
        && payload
            .chart_emphasis
            .dominant_objects
            .windows(2)
            .all(|window| window[0].score >= window[1].score)
        && payload.chart_emphasis.dominant_signs.iter().all(|entry| {
            !entry.sign_code.trim().is_empty()
                && valid_emphasis_score(entry.score)
                && valid_emphasis_reasons(&entry.reasons)
                && (payload.chart_emphasis.dominant_signs.len() == 1
                    || entry.score >= scoring.sign_house_emphasis_min_score)
        })
        && payload.chart_emphasis.dominant_houses.iter().all(|entry| {
            (1..=12).contains(&entry.house_number)
                && !entry.theme_code.trim().is_empty()
                && valid_emphasis_score(entry.score)
                && valid_emphasis_reasons(&entry.reasons)
                && (payload.chart_emphasis.dominant_houses.len() == 1
                    || entry.score >= scoring.sign_house_emphasis_min_score)
        })
        && payload.chart_emphasis.dominant_objects.iter().all(|entry| {
            !entry.object_code.trim().is_empty()
                && valid_emphasis_score(entry.score)
                && valid_emphasis_reasons(&entry.reasons)
                && (payload.chart_emphasis.dominant_objects.len() == 1
                    || (entry.score >= scoring.object_emphasis_min_score
                        && has_non_placement_emphasis_reason(&entry.reasons)))
        })
}

pub(super) fn product_scoring_snapshot(
    payload: &BasicPayload,
) -> Option<&BasicProductScoringSnapshot> {
    payload.chart_context.product_scoring.as_ref()
}

/// Fonction valid_emphasis_score.
fn valid_emphasis_score(score: f64) -> bool {
    score.is_finite() && score > 0.0
}

/// Fonction valid_emphasis_reasons.
fn valid_emphasis_reasons(reasons: &[String]) -> bool {
    !reasons.is_empty() && reasons.iter().all(|reason| !reason.trim().is_empty())
}

/// Fonction has_non_placement_emphasis_reason.
fn has_non_placement_emphasis_reason(reasons: &[String]) -> bool {
    reasons.iter().any(|reason| reason != "placement")
}
