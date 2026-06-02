use serde_json::json;

use crate::dignities::{
    dignity_priority_delta_for_position, essential_dignities_for_position, EssentialDignityFact,
};
use crate::domain::ObjectPositionFact;

use super::constants::THEME_OBJECT_POSITION;
use super::context::{placement_context_object, placement_context_str, placement_context_value};
use super::dignity_helpers::{
    dignity_effect_phrase, dignity_evidence_array, dignity_semantic_tags,
};
use super::tags::{dedupe_tags, house_tags, sign_tags};
use super::utils::round4;

pub(super) fn position_priority(position: &ObjectPositionFact) -> f64 {
    let base = object_signal_scoring_number(position, "position_priority_base").unwrap_or(0.0);
    let dignity_delta = dignity_priority_delta_for_position(position);
    round4((base + house_modality_priority_delta(position) + dignity_delta).min(100.0))
}

pub(super) fn house_modality_priority_delta(position: &ObjectPositionFact) -> f64 {
    placement_context_value(position, "house_modality", "priority_delta")
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0)
}

pub(super) fn angle_priority_base(position: &ObjectPositionFact) -> f64 {
    object_signal_scoring_number(position, "angle_priority_base")
        .or_else(|| object_signal_scoring_number(position, "position_priority_base"))
        .unwrap_or(0.0)
}

pub(super) fn object_source_weight(position: &ObjectPositionFact) -> f64 {
    object_signal_scoring_number(position, "source_weight").unwrap_or(0.0)
}

fn object_signal_scoring_number(position: &ObjectPositionFact, key: &str) -> Option<f64> {
    placement_context_value(position, "object_context", "signal_scoring")
        .and_then(|value| value.get(key))
        .and_then(|value| value.as_f64())
}

pub(super) fn position_theme_code(position: &ObjectPositionFact) -> String {
    house_theme_code(position)
}

pub(super) fn house_theme_code(position: &ObjectPositionFact) -> String {
    placement_context_str(position, "house_context", "theme_code")
        .or_else(|| placement_context_str(position, "angle_context", "house_theme_code"))
        .unwrap_or(THEME_OBJECT_POSITION)
        .to_string()
}

pub(super) fn position_aggregation_group(position: &ObjectPositionFact) -> String {
    match position.house_number {
        Some(house_number) => format!("{}:house_{}", position.sign_code, house_number),
        None => position.sign_code.clone(),
    }
}

pub(super) fn position_interpretive_hint(position: &ObjectPositionFact) -> String {
    let base = match (position.house_name.as_deref(), position.house_number) {
        (Some(house_name), Some(_)) => format!(
            "{} expresses through {} qualities in the field of {}.",
            position.object_name, position.sign_name, house_name
        ),
        _ => format!(
            "{} expresses through {} qualities.",
            position.object_name, position.sign_name
        ),
    };

    let dignities = essential_dignities_for_position(position);
    if !dignities.is_empty() {
        format!(
            "{base} Its dignity context adds {}.{}",
            dignity_effect_phrase_for_position(&dignities),
            retrograde_hint(position)
        )
    } else {
        format!("{base}{}", retrograde_hint(position))
    }
}

pub(super) fn position_semantic_tags(position: &ObjectPositionFact) -> Vec<String> {
    let mut tags = vec![
        "placement".to_string(),
        position.object_code.clone(),
        position.sign_code.clone(),
    ];
    tags.extend(sign_tags(&position.sign_code));
    if let Some(house_number) = position.house_number {
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
    if let Some(house_modality) = placement_context_str(position, "house_modality", "code") {
        tags.push(house_modality.to_string());
    }
    if let Some(role) = placement_context_str(position, "object_context", "role") {
        tags.push(role.to_string());
    }
    if let Some(motion_state) = placement_context_str(position, "motion_context", "motion_state") {
        tags.push(motion_state.to_string());
    }
    for dignity in essential_dignities_for_position(position) {
        tags.extend(dignity_semantic_tags(&dignity));
    }
    dedupe_tags(tags)
}

pub(super) fn placement_context(position: &ObjectPositionFact) -> serde_json::Value {
    json!({
        "sign_context": placement_context_object(position, "sign_context"),
        "house_context": placement_context_object(position, "house_context"),
        "house_modality": placement_context_object(position, "house_modality"),
        "object_context": placement_context_object(position, "object_context"),
        "motion_context": placement_context_object(position, "motion_context"),
        "dignity_context": dignity_evidence_array(&essential_dignities_for_position(position))
    })
}

pub(super) fn position_writing_guidance(
    position: &ObjectPositionFact,
    dignities: &[EssentialDignityFact],
) -> String {
    match (!dignities.is_empty(), is_retrograde_position(position)) {
        (true, true) => format!(
            "Use this as a concise placement cue; include {} and retrograde motion as modifiers, not separate verdicts.",
            dignity_type_list(dignities)
        ),
        (true, false) => format!(
            "Use this as a concise placement cue and include {} as a modifier, not a separate verdict.",
            dignity_type_list(dignities)
        ),
        (false, true) => "Use this as a concise placement cue; treat retrograde motion as an inward, revising, or reflective modifier before drafting final text.".to_string(),
        (false, false) => "Use this as a concise placement cue; combine it with nearby cluster or aspect signals before drafting final text.".to_string(),
    }
}

pub(super) fn retrograde_summary(position: &ObjectPositionFact) -> String {
    if is_retrograde_position(position) {
        " Its retrograde motion adds a reflective or revising layer to the placement.".to_string()
    } else {
        String::new()
    }
}

fn retrograde_hint(position: &ObjectPositionFact) -> String {
    if is_retrograde_position(position) {
        " Read the retrograde state as a modifier for pacing, review, and internal processing."
            .to_string()
    } else {
        String::new()
    }
}

fn is_retrograde_position(position: &ObjectPositionFact) -> bool {
    placement_context_str(position, "motion_context", "motion_state") == Some("retrograde")
}

pub(super) fn dignity_summary_for_position(dignities: &[EssentialDignityFact]) -> String {
    if dignities.is_empty() {
        String::new()
    } else {
        format!(
            " Its dignity context adds {}.",
            dignity_effect_phrase_for_position(dignities)
        )
    }
}

fn dignity_effect_phrase_for_position(dignities: &[EssentialDignityFact]) -> String {
    let phrases = dignities
        .iter()
        .map(dignity_effect_phrase)
        .collect::<Vec<_>>();
    phrases.join(" and ")
}

fn dignity_type_list(dignities: &[EssentialDignityFact]) -> String {
    let dignity_types = dignities
        .iter()
        .map(|dignity| dignity.dignity_type.as_str())
        .collect::<Vec<_>>();

    match dignity_types.as_slice() {
        [] => "the dignity context".to_string(),
        [one] => format!("the {one} context"),
        [first, second] => format!("the {first} and {second} contexts"),
        _ => format!("the {} contexts", dignity_types.join(", ")),
    }
}
