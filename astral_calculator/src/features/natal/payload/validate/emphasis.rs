//! Module astral_calculator\src\features\natal\payload\validate\emphasis.rs du moteur astral_calculator.

use std::collections::HashSet;

use crate::domain::{
    BasicPayload, BasicProductScoringSnapshot, BasicProjectionReason, ProjectionReasonDefinition,
};

pub(super) fn has_current_chart_emphasis(
    payload: &BasicPayload,
    projection_reason_definitions: &[ProjectionReasonDefinition],
) -> bool {
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
                && valid_projection_reasons(&entry.reason_details, projection_reason_definitions)
                && (payload.chart_emphasis.dominant_signs.len() == 1
                    || entry.score >= scoring.sign_house_emphasis_min_score)
        })
        && payload.chart_emphasis.dominant_houses.iter().all(|entry| {
            (1..=12).contains(&entry.house_number)
                && !entry.theme_code.trim().is_empty()
                && valid_emphasis_score(entry.score)
                && valid_projection_reasons(&entry.reason_details, projection_reason_definitions)
                && (payload.chart_emphasis.dominant_houses.len() == 1
                    || entry.score >= scoring.sign_house_emphasis_min_score)
        })
        && payload.chart_emphasis.dominant_objects.iter().all(|entry| {
            !entry.object_code.trim().is_empty()
                && valid_emphasis_score(entry.score)
                && valid_projection_reasons(&entry.reason_details, projection_reason_definitions)
                && (payload.chart_emphasis.dominant_objects.len() == 1
                    || (entry.score >= scoring.object_emphasis_min_score
                        && has_non_placement_emphasis_reason(&entry.reason_details)))
        })
}

pub(super) fn product_scoring_snapshot(
    payload: &BasicPayload,
) -> Option<&BasicProductScoringSnapshot> {
    payload.chart_context.product_scoring.as_ref()
}

pub(super) fn valid_projection_reasons(
    reasons: &[BasicProjectionReason],
    projection_reason_definitions: &[ProjectionReasonDefinition],
) -> bool {
    let mut seen_reasons = HashSet::new();

    !reasons.is_empty()
        && reasons.iter().all(|reason| {
            if !seen_reasons.insert(projection_reason_fingerprint(reason)) {
                return false;
            }

            let Some(definition) = projection_reason_definitions
                .iter()
                .find(|definition| definition.reason_code == reason.reason_code)
            else {
                return false;
            };
            valid_projection_reason(reason, definition)
        })
}

fn valid_projection_reason(
    reason: &BasicProjectionReason,
    definition: &ProjectionReasonDefinition,
) -> bool {
    if !definition.is_active || reason.reason_code.trim().is_empty() {
        return false;
    }

    field_presence_ok(&reason.object_code, definition.requires_object)
        && field_presence_ok(&reason.dignity_type, definition.requires_dignity_type)
        && field_presence_ok(&reason.sign_code, definition.requires_sign_code)
        && int_field_presence_ok(
            reason.house_number,
            definition.requires_house_number,
            |value| (1..=12).contains(&value),
        )
        && field_presence_ok(&reason.theme_code, definition.requires_theme_code)
        && field_presence_ok(&reason.angle_code, definition.requires_angle_code)
        && field_presence_ok(&reason.signal_key, definition.requires_signal_key)
        && field_presence_ok(&reason.context_key, definition.requires_context_key)
}

fn field_presence_ok(value: &Option<String>, required: bool) -> bool {
    if required {
        value.as_ref().is_some_and(|value| !value.trim().is_empty())
    } else {
        value.as_ref().is_none_or(|value| !value.trim().is_empty())
    }
}

fn int_field_presence_ok<F>(value: Option<i32>, required: bool, predicate: F) -> bool
where
    F: FnOnce(i32) -> bool,
{
    if required {
        value.is_some_and(predicate)
    } else {
        value.is_none_or(predicate)
    }
}

fn valid_emphasis_score(score: f64) -> bool {
    score.is_finite() && score > 0.0
}

fn has_non_placement_emphasis_reason(reasons: &[BasicProjectionReason]) -> bool {
    reasons
        .iter()
        .any(|reason| reason.reason_code != "placement")
}

fn projection_reason_fingerprint(
    reason: &BasicProjectionReason,
) -> (
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i32>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    (
        reason.reason_code.clone(),
        reason.object_code.clone(),
        reason.dignity_type.clone(),
        reason.sign_code.clone(),
        reason.house_number,
        reason.theme_code.clone(),
        reason.angle_code.clone(),
        reason.signal_key.clone(),
        reason.context_key.clone(),
    )
}
