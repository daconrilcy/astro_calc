mod angles;
mod aspects;
mod chart_context;
mod contract;
mod dignities;
mod emphasis;
mod json;
mod placements;
mod plan;
mod text;

use crate::domain::{BasicPayload, BasicSignal};

pub fn is_current_basic_payload(payload: &BasicPayload) -> bool {
    let structural_axis_pairs = angles::structural_axis_pairs_from_payload(payload);
    let angle_object_codes = angles::angle_object_codes_from_payload(payload);

    !payload.signals.is_empty()
        && payload.signals.len() <= 12
        && contract::has_current_llm_handoff_contract(payload)
        && chart_context::has_current_chart_context(payload)
        && angles::has_current_angles(payload)
        && dignities::has_current_dignities(payload)
        && emphasis::has_current_chart_emphasis(payload)
        && plan::has_current_reading_plan(payload)
        && plan::has_current_drafting_plan(payload)
        && payload
            .positions
            .iter()
            .all(placements::has_current_position_context)
        && payload.signals.iter().all(|signal| {
            signal_is_current(payload, signal, &structural_axis_pairs, &angle_object_codes)
        })
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
        && text::has_text(&signal.writing_guidance)
        && text::has_current_aspect_hint(&signal.interpretive_hint)
        && placements::has_current_placement_context(signal)
        && angles::has_current_angle_evidence(payload, signal)
        && aspects::has_current_aspect_context(signal)
        && !aspects::is_structural_axis_aspect_signal(signal, structural_axis_pairs)
        && !aspects::is_angle_to_angle_aspect_signal(signal, angle_object_codes)
}
