use std::collections::HashMap;

use serde_json::json;

use crate::natal::catalog::BasicPayloadCatalog;
use crate::domain::{InterpretationSignalDraft, ObjectPositionFact};

use super::constants::SUPPRESSION_ACTIVE;
use super::context::{placement_context_object, placement_context_str, placement_context_value};
use super::positions::{
    angle_priority_base, house_modality_priority_delta, house_theme_code, object_source_weight,
    placement_context,
};
use super::tags::{dedupe_tags, house_tags, sign_tags};
use super::utils::round4;

pub(super) fn is_core_chart_object(object_code: &str) -> bool {
    matches!(object_code, "sun" | "moon" | "ascendant" | "mc")
}

pub(super) fn is_angle_position(position: &ObjectPositionFact) -> bool {
    placement_context_object(position, "angle_context").is_some()
}

pub(super) fn angle_signal(
    position: &ObjectPositionFact,
    angle_point_object_codes: &HashMap<String, String>,
    catalog: &BasicPayloadCatalog,
) -> InterpretationSignalDraft {
    let angle_context = angle_context(position, angle_point_object_codes);
    let semantic_tags = angle_semantic_tags(position);
    let associated_house = angle_associated_house(position).or(position.house_number);
    let theme_code = house_theme_code(position);
    let opposite_angle_object_code = opposite_angle_object_code(position, angle_point_object_codes);

    InterpretationSignalDraft {
        signal_key: format!("angle:{}:sign:{}", position.object_code, position.sign_code),
        signal_type_id: None,
        theme_code: Some(theme_code),
        title: format!("{} in {}", position.object_name, position.sign_name),
        summary: Some(format!(
            "{} falls in {}, giving the chart a concrete orientation through this angle.",
            position.object_name, position.sign_name
        )),
        priority_score: angle_priority(position),
        confidence_score: Some(0.95),
        suppression_state: SUPPRESSION_ACTIVE.to_string(),
        payload_json: Some(json!({
            "interpretive_hint": angle_interpretive_hint(position),
            "semantic_tags": semantic_tags,
            "source_weight": round4(object_source_weight(position)),
            "aggregation_group": format!("angle:{}:{}", position.object_code, position.sign_code),
            "angle_context": angle_context,
            "evidence": {
                "fact_type": "chart_angle",
                "angle_code": position.object_code,
                "angle_name": position.object_name,
                "angle_point_code": placement_context_str(position, "angle_context", "angle_point_code"),
                "short_label": placement_context_str(position, "angle_context", "short_label"),
                "axis": placement_context_str(position, "angle_context", "axis"),
                "opposite_angle_code": placement_context_str(position, "angle_context", "opposite_angle_code"),
                "opposite_angle_object_code": opposite_angle_object_code,
                "associated_house_number": associated_house,
                "chart_object_id": position.chart_object_id,
                "sign_id": position.sign_id,
                "sign_code": position.sign_code,
                "sign_name": position.sign_name,
                "house_id": position.house_id,
                "house_number": position.house_number,
                "house_name": position.house_name,
                "longitude_deg": position.longitude_deg,
                "placement_context": placement_context(position, catalog)
            }
        })),
    }
}

fn angle_priority(position: &ObjectPositionFact) -> f64 {
    round4((angle_priority_base(position) + house_modality_priority_delta(position)).min(100.0))
}

fn angle_context(
    position: &ObjectPositionFact,
    angle_point_object_codes: &HashMap<String, String>,
) -> serde_json::Value {
    json!({
        "angle_code": position.object_code,
        "angle_name": position.object_name,
        "angle_point_code": placement_context_str(position, "angle_context", "angle_point_code"),
        "short_label": placement_context_str(position, "angle_context", "short_label"),
        "full_name": placement_context_str(position, "angle_context", "full_name"),
        "axis": placement_context_str(position, "angle_context", "axis"),
        "opposite_angle_code": placement_context_str(position, "angle_context", "opposite_angle_code"),
        "opposite_angle_object_code": opposite_angle_object_code(position, angle_point_object_codes),
        "associated_house_number": angle_associated_house(position),
        "sign_code": position.sign_code,
        "sign_name": position.sign_name,
        "longitude_deg": position.longitude_deg
    })
}

fn opposite_angle_object_code(
    position: &ObjectPositionFact,
    angle_point_object_codes: &HashMap<String, String>,
) -> Option<String> {
    placement_context_str(position, "angle_context", "opposite_angle_code")
        .and_then(|code| angle_point_object_codes.get(code))
        .cloned()
}

fn angle_interpretive_hint(position: &ObjectPositionFact) -> String {
    match position.object_code.as_str() {
        "ascendant" => format!(
            "Use the Ascendant as the chart's immediate orientation: embodiment, instinctive style, and first impression through {} qualities.",
            position.sign_name
        ),
        "mc" => format!(
            "Use the MC as public direction and visibility, colored by {} qualities.",
            position.sign_name
        ),
        "descendant" => format!(
            "Use the Descendant as the relationship horizon and encounter style through {} qualities.",
            position.sign_name
        ),
        "ic" => format!(
            "Use the IC as private foundation, roots, and inner base through {} qualities.",
            position.sign_name
        ),
        _ => format!("Use this angle as a chart orientation marker in {}.", position.sign_name),
    }
}

fn angle_semantic_tags(position: &ObjectPositionFact) -> Vec<String> {
    let mut tags = vec![
        "angle".to_string(),
        position.object_code.clone(),
        position.sign_code.clone(),
    ];
    tags.extend(sign_tags(&position.sign_code));
    if let Some(house_number) = angle_associated_house(position).or(position.house_number) {
        tags.push(format!("house_{house_number}"));
        tags.push(house_theme_code(position));
        tags.extend(house_tags(house_number));
    }
    if let Some(element) = placement_context_str(position, "sign_context", "element") {
        tags.push(element.to_string());
    }
    if let Some(modality) = placement_context_str(position, "sign_context", "modality") {
        tags.push(modality.to_string());
    }
    if let Some(polarity) = placement_context_str(position, "sign_context", "polarity") {
        tags.push(polarity.to_string());
    }
    if let Some(axis) = placement_context_str(position, "angle_context", "axis") {
        tags.push(axis.to_string());
    }
    dedupe_tags(tags)
}

fn angle_associated_house(position: &ObjectPositionFact) -> Option<i32> {
    placement_context_value(position, "angle_context", "associated_house_number")
        .and_then(|value| value.as_i64())
        .and_then(|value| i32::try_from(value).ok())
}
