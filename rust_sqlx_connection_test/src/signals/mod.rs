use serde_json::json;

mod angles;
mod aspect_signals;
mod clusters;
mod constants;
mod context;
mod dignity;
mod dignity_helpers;
mod limits;
mod positions;
mod relations;
mod tags;
mod utils;

use angles::{angle_signal, is_angle_position};
use aspect_signals::{
    aspect_context, aspect_interpretive_hint, aspect_semantic_tags, aspect_writing_guidance,
};
use clusters::{add_position_cluster_signals, apply_cluster_source_deduplication};
pub use constants::BASIC_MAX_ACTIVE_SIGNALS;
use constants::{
    BASIC_ASPECT_MIN_STRENGTH, SUPPRESSION_ACTIVE, SUPPRESSION_SUPPRESSED, THEME_ASPECT,
};
use dignity::add_dignity_signals;
use dignity_helpers::dignity_evidence_array;
use limits::{
    fill_basic_active_limit, is_angle_to_angle_aspect, is_structural_axis_aspect,
    preserve_strong_non_structural_aspect, preserve_strong_tension_aspect,
    suppress_over_basic_limit,
};
use positions::{
    dignity_summary_for_position, object_source_weight, placement_context,
    position_aggregation_group, position_interpretive_hint, position_priority,
    position_semantic_tags, position_theme_code, position_writing_guidance, retrograde_summary,
};
use relations::{
    angle_object_codes_from_positions, angle_point_object_codes_from_positions,
    structural_axis_pairs_from_positions,
};
pub use utils::indefinite_article;
use utils::round4;

use crate::dignities::{
    dignity_source_weight_delta_for_position, essential_dignities_for_position,
};
use crate::domain::{CalculatedChartFacts, InterpretationSignalDraft};

pub fn aggregate_basic_signals(facts: &CalculatedChartFacts) -> Vec<InterpretationSignalDraft> {
    let mut signals = Vec::new();
    let structural_axis_pairs = structural_axis_pairs_from_positions(&facts.positions);
    let angle_object_codes = angle_object_codes_from_positions(&facts.positions);
    let angle_point_object_codes = angle_point_object_codes_from_positions(&facts.positions);

    for position in &facts.positions {
        if is_angle_position(position) {
            signals.push(angle_signal(position, &angle_point_object_codes));
            continue;
        }

        let house_suffix = position
            .house_number
            .map(|house_number| format!(", house {house_number}"))
            .unwrap_or_default();
        let summary_house = position
            .house_name
            .as_deref()
            .map(|house_name| format!(" and the {house_name} house"))
            .unwrap_or_default();
        let dignities = essential_dignities_for_position(position);
        let semantic_tags = position_semantic_tags(position);
        let source_weight = round4(
            object_source_weight(&position.object_code)
                + dignity_source_weight_delta_for_position(position),
        );
        let theme_code = position_theme_code(position);
        let aggregation_group = position_aggregation_group(position);
        let dignity_summary = dignity_summary_for_position(&dignities);
        let motion_summary = retrograde_summary(position);

        signals.push(InterpretationSignalDraft {
            signal_key: format!("object_position:{}", position.object_code),
            signal_type_id: None,
            theme_code: Some(theme_code.to_string()),
            title: format!(
                "{} in {}{}",
                position.object_name, position.sign_name, house_suffix
            ),
            summary: Some(format!(
                "{} is placed in {}{}, emphasizing this chart factor through a concrete, readable placement.{}{}",
                position.object_name,
                position.sign_name,
                summary_house,
                dignity_summary,
                motion_summary
            )),
            priority_score: position_priority(position),
            confidence_score: Some(0.95),
            suppression_state: SUPPRESSION_ACTIVE.to_string(),
            payload_json: Some(json!({
                "interpretive_hint": position_interpretive_hint(position),
                "semantic_tags": semantic_tags,
                "source_weight": source_weight,
                "aggregation_group": aggregation_group,
                "writing_guidance": position_writing_guidance(position, &dignities),
                "evidence": {
                    "fact_type": "object_position",
                    "chart_object_id": position.chart_object_id,
                    "object_code": position.object_code,
                    "object_name": position.object_name,
                    "sign_id": position.sign_id,
                    "sign_code": position.sign_code,
                    "sign_name": position.sign_name,
                    "house_id": position.house_id,
                    "house_number": position.house_number,
                    "house_name": position.house_name,
                    "longitude_deg": position.longitude_deg,
                    "placement_context": placement_context(position),
                    "essential_dignities": dignity_evidence_array(&dignities)
                }
            })),
        });
    }

    add_dignity_signals(facts, &mut signals);

    for aspect in &facts.aspects {
        if is_structural_axis_aspect(aspect, &structural_axis_pairs) {
            continue;
        }

        let strength_score = aspect.strength_score.unwrap_or(0.5);
        let suppression_state = if strength_score >= BASIC_ASPECT_MIN_STRENGTH
            && !is_angle_to_angle_aspect(aspect, &angle_object_codes)
        {
            SUPPRESSION_ACTIVE
        } else {
            SUPPRESSION_SUPPRESSED
        };
        let aspect_name = aspect.aspect_name.to_lowercase();
        let article = indefinite_article(&aspect_name);
        let aspect_context = aspect_context(aspect);

        signals.push(InterpretationSignalDraft {
            signal_key: format!(
                "aspect:{}:{}:{}",
                aspect.source_object_code, aspect.target_object_code, aspect.aspect_code
            ),
            signal_type_id: None,
            theme_code: Some(THEME_ASPECT.to_string()),
            title: format!(
                "{} {} {}",
                aspect.source_object_name, aspect_name, aspect.target_object_name
            ),
            summary: Some(format!(
                "{} and {} form {} {} with {:.2} degrees of orb; the phase is {}.",
                aspect.source_object_name,
                aspect.target_object_name,
                article,
                aspect_name,
                aspect.orb_deg,
                aspect.phase_state
            )),
            priority_score: strength_score * 80.0,
            confidence_score: Some(0.85),
            suppression_state: suppression_state.to_string(),
            payload_json: Some(json!({
                "interpretive_hint": aspect_interpretive_hint(aspect, &aspect_name),
                "semantic_tags": aspect_semantic_tags(aspect, strength_score),
                "source_weight": round4(
                    object_source_weight(&aspect.source_object_code)
                        + object_source_weight(&aspect.target_object_code)
                ),
                "aggregation_group": format!("aspect:{}", aspect.aspect_code),
                "writing_guidance": aspect_writing_guidance(aspect),
                "aspect_context": aspect_context,
                "evidence": {
                    "fact_type": "aspect",
                    "source_chart_object_id": aspect.source_chart_object_id,
                    "source_object_code": aspect.source_object_code,
                    "source_object_name": aspect.source_object_name,
                    "target_chart_object_id": aspect.target_chart_object_id,
                    "target_object_code": aspect.target_object_code,
                    "target_object_name": aspect.target_object_name,
                    "aspect_id": aspect.aspect_id,
                    "aspect_code": aspect.aspect_code,
                    "aspect_name": aspect.aspect_name,
                    "aspect_family": aspect.aspect_family,
                    "orb_deg": aspect.orb_deg,
                    "phase_state": aspect.phase_state,
                    "is_applying": aspect.is_applying,
                    "is_exact": aspect.is_exact,
                    "strength_score": aspect.strength_score,
                    "calculation_notes": aspect.calculation_notes_json
                }
            })),
        });
    }

    add_position_cluster_signals(facts, &mut signals);

    signals.sort_by(|left, right| {
        right
            .priority_score
            .partial_cmp(&left.priority_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    suppress_over_basic_limit(&mut signals);
    for _ in 0..BASIC_MAX_ACTIVE_SIGNALS {
        if !apply_cluster_source_deduplication(&mut signals) {
            break;
        }
        fill_basic_active_limit(&mut signals, &angle_object_codes);
    }
    preserve_strong_tension_aspect(&mut signals, &angle_object_codes);
    preserve_strong_non_structural_aspect(&mut signals, &angle_object_codes);
    signals
}
