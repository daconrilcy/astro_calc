mod accidental_dignities;
mod angles;
mod chart_context;
mod dignities;
mod emphasis;
mod house_axes;
mod json;
mod lunar_phase;
mod reading_plan;
mod rulership;
mod signal_filters;

use std::collections::{HashMap, HashSet};

use crate::catalog::BasicPayloadCatalog;
use crate::domain::{
    AccidentalDignityConditionReference, BasicObjectPosition, BasicPayload, BasicSignal,
    HouseAxisReference, LunarPhaseReference, NatalChartInput, ObjectPositionFact,
    ObjectSectAffinityReference,
};
use crate::models::InterpretationSignalRow;
use angles::{
    angle_object_codes_from_positions, build_payload_angles, structural_axis_pairs_from_positions,
};
use chart_context::{build_chart_context, visibility_context};
use dignities::{build_payload_dignities, position_dignity_context};
use emphasis::build_chart_emphasis;
use json::{
    payload_aspect_context, payload_f64, payload_string, payload_string_array, payload_value,
    position_context,
};
use reading_plan::build_reading_plan;
use rulership::build_rulership_context;
use signal_filters::{is_angle_to_angle_aspect_signal, is_structural_axis_signal_for_pairs};

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
    build_basic_payload_with_references(
        chart_calculation_id,
        input,
        positions,
        signals,
        domicile_rulers,
        &[],
    )
}

pub fn build_basic_payload_with_references(
    chart_calculation_id: i32,
    input: &NatalChartInput,
    positions: &[ObjectPositionFact],
    signals: &[InterpretationSignalRow],
    domicile_rulers: &[crate::domain::DomicileRulerReference],
    house_axes: &[HouseAxisReference],
) -> BasicPayload {
    build_basic_payload_with_all_references(
        chart_calculation_id,
        input,
        positions,
        signals,
        domicile_rulers,
        house_axes,
        &[],
    )
}

pub fn build_basic_payload_with_all_references(
    chart_calculation_id: i32,
    input: &NatalChartInput,
    positions: &[ObjectPositionFact],
    signals: &[InterpretationSignalRow],
    domicile_rulers: &[crate::domain::DomicileRulerReference],
    house_axes: &[HouseAxisReference],
    lunar_phases: &[LunarPhaseReference],
) -> BasicPayload {
    build_basic_payload_with_accidental_references(
        chart_calculation_id,
        input,
        positions,
        signals,
        domicile_rulers,
        house_axes,
        lunar_phases,
        &[],
        &[],
        &crate::catalog::test_catalog(),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_basic_payload_with_accidental_references(
    chart_calculation_id: i32,
    input: &NatalChartInput,
    positions: &[ObjectPositionFact],
    signals: &[InterpretationSignalRow],
    domicile_rulers: &[crate::domain::DomicileRulerReference],
    house_axes: &[HouseAxisReference],
    lunar_phases: &[LunarPhaseReference],
    accidental_conditions: &[AccidentalDignityConditionReference],
    sect_affinities: &[ObjectSectAffinityReference],
    catalog: &BasicPayloadCatalog,
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
            aspect_context: payload_aspect_context(signal),
            evidence: payload_value(signal, "evidence"),
        })
        .collect();
    basic_signals.retain(|signal| {
        !is_structural_axis_signal_for_pairs(signal, &structural_axis_pairs)
            && !is_angle_to_angle_aspect_signal(signal, &angle_object_codes)
    });
    basic_signals.truncate(catalog.product_scoring.max_active_signals);

    let angles = build_payload_angles(positions);
    let dignities = build_payload_dignities(positions, &basic_signals, catalog);
    let chart_emphasis = build_chart_emphasis(positions, &dignities, &basic_signals, catalog);
    let rulership_context =
        build_rulership_context(positions, &chart_emphasis, domicile_rulers, &basic_signals);
    let house_axis_emphasis = house_axes::build_house_axis_emphasis(
        house_axes,
        positions,
        &angles,
        &dignities,
        &chart_emphasis,
        &rulership_context,
        &basic_signals,
        catalog,
    );
    let reading_plan = build_reading_plan(&basic_signals);
    let lunar_phase_context = lunar_phase::build_lunar_phase_context(
        lunar_phases,
        positions,
        &basic_signals,
        &reading_plan,
    );
    let has_accidental_references = !accidental_conditions.is_empty()
        && !sect_affinities.is_empty()
        && lunar_phase_context.is_some();
    let contract_version = if has_accidental_references {
        "natal_structured_v13"
    } else if lunar_phase_context.is_some() {
        "natal_structured_v12"
    } else {
        "natal_structured_v11"
    };
    let chart_context = build_chart_context(
        input,
        positions,
        contract_version,
        has_accidental_references.then_some(catalog),
    );
    let chart_sect = chart_context.sect.chart_sect.as_deref();
    let active_signal_keys: HashSet<&str> = basic_signals
        .iter()
        .map(|signal| signal.signal_key.as_str())
        .collect();
    let accidental_build = if has_accidental_references {
        accidental_dignities::build_accidental_dignities(
            positions,
            chart_sect,
            accidental_conditions,
            sect_affinities,
            &active_signal_keys,
            catalog,
        )
    } else {
        accidental_dignities::AccidentalDignityBuild {
            evaluations: Vec::new(),
            context_by_object: HashMap::new(),
        }
    };
    if has_accidental_references {
        accidental_dignities::apply_accidental_context_to_signals(
            &mut basic_signals,
            &accidental_build.context_by_object,
        );
    }

    let mut chart_emphasis = chart_emphasis;
    if has_accidental_references {
        accidental_dignities::apply_accidental_context_to_emphasis(
            &mut chart_emphasis,
            &accidental_build.evaluations,
        );
    }

    BasicPayload {
        product_code: input.product_code().to_string(),
        chart_calculation_id,
        reference_version_id: input.reference_version_id,
        subject_label: input.subject_label.clone(),
        birth_datetime_utc: input.birth_datetime_utc,
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
                dignity_context: position_dignity_context(position, catalog),
                visibility_context: visibility_context(position),
                accidental_dignity_context: accidental_build
                    .context_by_object
                    .get(&position.object_code)
                    .cloned()
                    .unwrap_or_default(),
            })
            .collect(),
        angles,
        dignities,
        chart_emphasis,
        rulership_context,
        house_axis_emphasis,
        lunar_phase_context,
        accidental_dignities: accidental_build.evaluations,
        signals: basic_signals,
        reading_plan,
    }
}
