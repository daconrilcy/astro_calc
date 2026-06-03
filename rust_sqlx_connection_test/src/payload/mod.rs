mod angles;
mod chart_context;
mod contract;
mod dignities;
mod drafting_plan;
mod emphasis;
mod json;
mod reading_plan;
mod rulership;
mod signal_filters;

use crate::domain::{
    BasicObjectPosition, BasicPayload, BasicSignal, NatalChartInput, ObjectPositionFact,
};
use crate::models::InterpretationSignalRow;
use angles::{
    angle_object_codes_from_positions, build_payload_angles, structural_axis_pairs_from_positions,
};
use chart_context::{build_chart_context, visibility_context};
use dignities::{build_payload_dignities, position_dignity_context};
use drafting_plan::build_drafting_plan;
use emphasis::build_chart_emphasis;
use json::{payload_f64, payload_string, payload_string_array, payload_value, position_context};
use reading_plan::build_reading_plan;
use rulership::build_rulership_context;
use signal_filters::{is_angle_to_angle_aspect_signal, is_structural_axis_signal_for_pairs};

pub use contract::basic_llm_handoff_contract;

pub fn build_basic_payload(
    chart_calculation_id: i32,
    input: &NatalChartInput,
    positions: &[ObjectPositionFact],
    signals: &[InterpretationSignalRow],
) -> BasicPayload {
    build_basic_payload_with_rulership(chart_calculation_id, input, positions, signals, &[])
}

pub fn build_basic_payload_with_rulership(
    chart_calculation_id: i32,
    input: &NatalChartInput,
    positions: &[ObjectPositionFact],
    signals: &[InterpretationSignalRow],
    domicile_rulers: &[crate::domain::DomicileRulerReference],
) -> BasicPayload {
    let structural_axis_pairs = structural_axis_pairs_from_positions(positions);
    let angle_object_codes = angle_object_codes_from_positions(positions);
    let mut basic_signals: Vec<BasicSignal> = signals
        .iter()
        .map(|signal| BasicSignal {
            signal_key: signal.signal_key.clone(),
            theme_code: signal.theme_code.clone(),
            title: signal.title.clone(),
            summary: signal.summary.clone(),
            priority_score: signal.priority_score,
            confidence_score: signal.confidence_score,
            interpretive_hint: payload_string(signal, "interpretive_hint"),
            semantic_tags: payload_string_array(signal, "semantic_tags"),
            source_weight: payload_f64(signal, "source_weight"),
            aggregation_group: payload_string(signal, "aggregation_group"),
            writing_guidance: payload_string(signal, "writing_guidance"),
            aspect_context: payload_value(signal, "aspect_context"),
            evidence: payload_value(signal, "evidence"),
        })
        .collect();
    basic_signals.retain(|signal| {
        !is_structural_axis_signal_for_pairs(signal, &structural_axis_pairs)
            && !is_angle_to_angle_aspect_signal(signal, &angle_object_codes)
    });
    basic_signals.truncate(12);

    let angles = build_payload_angles(positions);
    let dignities = build_payload_dignities(positions, &basic_signals);
    let chart_emphasis = build_chart_emphasis(positions, &dignities, &basic_signals);
    let rulership_context =
        build_rulership_context(positions, &chart_emphasis, domicile_rulers, &basic_signals);
    let chart_context = build_chart_context(input, positions);
    let reading_plan = build_reading_plan(&basic_signals);
    let drafting_plan = build_drafting_plan(&reading_plan, &basic_signals, &chart_emphasis);

    BasicPayload {
        product_code: input.product_code().to_string(),
        chart_calculation_id,
        reference_version_id: input.reference_version_id,
        subject_label: input.subject_label.clone(),
        birth_datetime_utc: input.birth_datetime_utc,
        llm_handoff_contract: Some(basic_llm_handoff_contract()),
        chart_context,
        positions: positions
            .iter()
            .map(|position| BasicObjectPosition {
                object_code: position.object_code.clone(),
                object_name: position.object_name.clone(),
                longitude_deg: position.longitude_deg,
                sign_id: position.sign_id,
                sign_code: position.sign_code.clone(),
                sign_name: position.sign_name.clone(),
                house_id: position.house_id,
                house_number: position.house_number,
                house_name: position.house_name.clone(),
                motion_state_id: position.motion_state_id,
                sign_context: position_context(position, "sign_context"),
                house_context: position_context(position, "house_context"),
                house_modality: position_context(position, "house_modality"),
                object_context: position_context(position, "object_context"),
                motion_context: position_context(position, "motion_context"),
                dignity_context: position_dignity_context(position),
                visibility_context: visibility_context(position),
            })
            .collect(),
        angles,
        dignities,
        chart_emphasis,
        rulership_context,
        signals: basic_signals,
        reading_plan,
        drafting_plan,
    }
}
