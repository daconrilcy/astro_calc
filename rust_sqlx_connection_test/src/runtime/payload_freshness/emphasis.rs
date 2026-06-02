use crate::domain::BasicPayload;

const SIGN_HOUSE_EMPHASIS_MIN_SCORE: f64 = 0.35;
const OBJECT_EMPHASIS_MIN_SCORE: f64 = 0.5;
const MAX_DOMINANT_SIGNS: usize = 3;
const MAX_DOMINANT_HOUSES: usize = 3;
const MAX_DOMINANT_OBJECTS: usize = 5;

pub(super) fn has_current_chart_emphasis(payload: &BasicPayload) -> bool {
    !payload.chart_emphasis.dominant_signs.is_empty()
        && !payload.chart_emphasis.dominant_houses.is_empty()
        && !payload.chart_emphasis.dominant_objects.is_empty()
        && payload.chart_emphasis.dominant_signs.len() <= MAX_DOMINANT_SIGNS
        && payload.chart_emphasis.dominant_houses.len() <= MAX_DOMINANT_HOUSES
        && payload.chart_emphasis.dominant_objects.len() <= MAX_DOMINANT_OBJECTS
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
                    || entry.score >= SIGN_HOUSE_EMPHASIS_MIN_SCORE)
        })
        && payload.chart_emphasis.dominant_houses.iter().all(|entry| {
            (1..=12).contains(&entry.house_number)
                && !entry.theme_code.trim().is_empty()
                && valid_emphasis_score(entry.score)
                && valid_emphasis_reasons(&entry.reasons)
                && (payload.chart_emphasis.dominant_houses.len() == 1
                    || entry.score >= SIGN_HOUSE_EMPHASIS_MIN_SCORE)
        })
        && payload.chart_emphasis.dominant_objects.iter().all(|entry| {
            !entry.object_code.trim().is_empty()
                && valid_emphasis_score(entry.score)
                && valid_emphasis_reasons(&entry.reasons)
                && (payload.chart_emphasis.dominant_objects.len() == 1
                    || (entry.score >= OBJECT_EMPHASIS_MIN_SCORE
                        && has_non_placement_emphasis_reason(&entry.reasons)))
        })
}

fn valid_emphasis_score(score: f64) -> bool {
    score > 0.0 && score <= 1.0
}

fn valid_emphasis_reasons(reasons: &[String]) -> bool {
    !reasons.is_empty() && reasons.iter().all(|reason| !reason.trim().is_empty())
}

fn has_non_placement_emphasis_reason(reasons: &[String]) -> bool {
    reasons.iter().any(|reason| reason != "placement")
}
