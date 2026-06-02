use crate::domain::{BasicObjectPosition, BasicSignal};

use super::json;

pub(super) fn has_current_position_context(position: &BasicObjectPosition) -> bool {
    let is_angle = position
        .object_context
        .as_ref()
        .and_then(|context| context.get("role"))
        .and_then(|value| value.as_str())
        == Some("angle")
        || position
            .object_context
            .as_ref()
            .and_then(|context| context.get("role_label"))
            .and_then(|value| value.as_str())
            == Some("Angle");

    !position.sign_code.is_empty()
        && !position.sign_name.is_empty()
        && position.dignity_context.is_array()
        && json::option_json_has_text(&position.sign_context, "element")
        && json::option_json_has_text(&position.sign_context, "modality")
        && json::option_json_has_text(&position.sign_context, "polarity")
        && json::option_json_has_text(&position.house_context, "theme_code")
        && json::option_json_has_text(&position.house_modality, "code")
        && json::option_json_has_text(&position.object_context, "role")
        && (is_angle || json::option_json_has_text(&position.motion_context, "motion_state"))
}

pub(super) fn has_current_placement_context(signal: &BasicSignal) -> bool {
    if !signal.signal_key.starts_with("object_position:") {
        return true;
    }

    let Some(evidence) = signal.evidence.as_ref() else {
        return false;
    };

    let Some(context) = evidence.get("placement_context") else {
        return false;
    };

    evidence
        .get("essential_dignities")
        .is_some_and(|value| value.is_array())
        && json::nested_json_has_text(context, "sign_context", "element")
        && json::nested_json_has_text(context, "sign_context", "modality")
        && json::nested_json_has_text(context, "sign_context", "polarity")
        && json::nested_json_has_text(context, "house_context", "theme_code")
        && json::nested_json_has_text(context, "house_modality", "code")
        && json::nested_json_has_text(context, "object_context", "role")
        && json::nested_json_has_text(context, "motion_context", "motion_state")
}
