mod accidental_dignities;
mod angles;
mod aspects;
mod chart_context;
mod dignities;
mod emphasis;
mod house_axes;
mod json;
mod lunar_phase;
mod placements;
mod plan;
mod rulership;
mod text;

use crate::domain::{BasicPayload, BasicSignal, DomicileRulerReference};

pub fn is_current_basic_payload(payload: &BasicPayload) -> bool {
    let structural_axis_pairs = angles::structural_axis_pairs_from_payload(payload);
    let angle_object_codes = angles::angle_object_codes_from_payload(payload);

    let max_active_signals = payload
        .chart_context
        .product_scoring
        .as_ref()
        .map(|scoring| scoring.max_active_signals)
        .unwrap_or(0);

    !payload.signals.is_empty()
        && max_active_signals > 0
        && payload.signals.len() <= max_active_signals
        && chart_context::has_current_chart_context(payload)
        && angles::has_current_angles(payload)
        && dignities::has_current_dignities(payload)
        && emphasis::has_current_chart_emphasis(payload)
        && rulership::has_current_rulership_context(&payload.rulership_context)
        && house_axes::has_current_house_axis_emphasis(payload)
        && plan::has_current_reading_plan(payload)
        && lunar_phase::has_current_lunar_phase_context(payload)
        && accidental_dignities::has_current_accidental_dignities(payload)
        && payload
            .positions
            .iter()
            .all(placements::has_current_position_context)
        && payload.signals.iter().all(|signal| {
            signal_is_current(payload, signal, &structural_axis_pairs, &angle_object_codes)
        })
}

pub fn has_current_rulership_references(
    payload: &BasicPayload,
    domicile_rulers: &[DomicileRulerReference],
) -> bool {
    rulership::matches_domicile_ruler_references(&payload.rulership_context, domicile_rulers)
}

fn signal_is_current(
    payload: &BasicPayload,
    signal: &BasicSignal,
    structural_axis_pairs: &std::collections::HashSet<(String, String)>,
    angle_object_codes: &std::collections::HashSet<String>,
) -> bool {
    signal.evidence.is_some()
        && text::has_text(&signal.theme_code)
        && text::has_text(&signal.interpretive_hint)
        && !signal.semantic_tags.is_empty()
        && signal
            .semantic_tags
            .iter()
            .all(|tag| !tag.trim().is_empty())
        && text::has_text(&signal.aggregation_group)
        && text::has_current_aspect_hint(&signal.interpretive_hint)
        && placements::has_current_placement_context(signal)
        && angles::has_current_angle_evidence(payload, signal)
        && aspects::has_current_aspect_context(signal)
        && !aspects::is_structural_axis_aspect_signal(signal, structural_axis_pairs)
        && !aspects::is_angle_to_angle_aspect_signal(signal, angle_object_codes)
}
