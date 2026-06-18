//! Module astral_calculator\src\features\natal\payload\validate\mod.rs du moteur astral_calculator.

mod accidental_dignities;
/// Module angles.
mod angles;
/// Module aspects.
mod aspects;
/// Module chart_context.
mod chart_context;
/// Module dignities.
mod dignities;
/// Module emphasis.
mod emphasis;
/// Module house_axes.
mod house_axes;
/// Module json.
mod json;
/// Module lunar_phase.
mod lunar_phase;
/// Module placements.
mod placements;
/// Module plan.
mod plan;
/// Module rulership.
mod rulership;
/// Module text.
mod text;

use crate::domain::{
    BasicPayload, BasicSignal, DomicileRulerReference, ProjectionReasonDefinition,
};

/// Fonction is_current_basic_payload.
pub fn is_current_basic_payload(
    payload: &BasicPayload,
    projection_reason_definitions: &[ProjectionReasonDefinition],
) -> bool {
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
        && emphasis::has_current_chart_emphasis(payload, projection_reason_definitions)
        && rulership::has_current_rulership_context(&payload.rulership_context)
        && house_axes::has_current_house_axis_emphasis(payload, projection_reason_definitions)
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

/// Fonction has_current_rulership_references.
pub fn has_current_rulership_references(
    payload: &BasicPayload,
    domicile_rulers: &[DomicileRulerReference],
) -> bool {
    rulership::matches_domicile_ruler_references(&payload.rulership_context, domicile_rulers)
}

/// Fonction signal_is_current.
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
