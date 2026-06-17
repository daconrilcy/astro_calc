use crate::domain::{BasicObjectPosition, BasicSignal};
use crate::natal::payload::shared::text::{has_text, is_normalized_score};
use crate::natal::payload::rules::chart_context::{
    horizon_position_for_altitude, is_angle_role, is_horizon_position,
};

use super::json;

pub(super) fn has_current_position_context(position: &BasicObjectPosition) -> bool {
    let role = position
        .object_context
        .as_ref()
        .and_then(|context| context.get("role"))
        .and_then(|value| value.as_str());
    let role_label = position
        .object_context
        .as_ref()
        .and_then(|context| context.get("role_label"))
        .and_then(|value| value.as_str());
    let is_angle = is_angle_role(role, role_label)
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
        && has_current_accidental_dignity_context(position, is_angle)
}

fn has_current_accidental_dignity_context(position: &BasicObjectPosition, is_angle: bool) -> bool {
    if is_angle {
        return position.accidental_dignity_context.is_empty();
    }
    position.accidental_dignity_context.iter().all(|summary| {
        has_text(&summary.condition_code)
            && has_text(&summary.condition_family)
            && has_text(&summary.polarity)
            && is_normalized_score(summary.strength_score)
    })
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
        && has_current_mobile_visibility_context(context.get("visibility_context"))
        && context
            .get("accidental_dignity_context")
            .is_some_and(|value| value.is_array())
}

fn has_current_mobile_visibility_context(value: Option<&serde_json::Value>) -> bool {
    let Some(value) = value else {
        return false;
    };
    let horizon_position = value
        .get("horizon_position")
        .and_then(|value| value.as_str());
    let altitude_deg = value.get("altitude_deg").and_then(|value| value.as_f64());

    value.is_object()
        && value
            .get("horizon_position_id")
            .is_some_and(|value| value.as_i64().is_some_and(|id| id > 0))
        && horizon_position.is_some_and(is_horizon_position)
        && altitude_deg.is_some_and(f64::is_finite)
        && value.get("source").and_then(|value| value.as_str()) == Some("calculated_altitude")
        && json::has_bool_value(value.get("is_visible"))
        && altitude_deg
            .map(horizon_position_for_altitude)
            .is_some_and(|expected| horizon_position == Some(expected))
}
