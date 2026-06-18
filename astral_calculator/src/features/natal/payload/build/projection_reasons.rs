//! Helpers de construction des reasons typées du payload natal.

use crate::domain::BasicProjectionReason;

pub(super) fn reason_object_in_sign(object_code: &str, sign_code: &str) -> BasicProjectionReason {
    BasicProjectionReason {
        reason_code: "object_in_sign".to_string(),
        object_code: Some(object_code.to_string()),
        sign_code: Some(sign_code.to_string()),
        ..BasicProjectionReason::default()
    }
}

pub(super) fn reason_object_in_house(
    object_code: &str,
    house_number: i32,
    theme_code: Option<&str>,
) -> BasicProjectionReason {
    BasicProjectionReason {
        reason_code: "object_in_house".to_string(),
        object_code: Some(object_code.to_string()),
        house_number: Some(house_number),
        theme_code: theme_code.map(ToString::to_string),
        ..BasicProjectionReason::default()
    }
}

pub(super) fn reason_simple(reason_code: &str) -> BasicProjectionReason {
    BasicProjectionReason {
        reason_code: reason_code.to_string(),
        ..BasicProjectionReason::default()
    }
}

pub(super) fn reason_essential_dignity(
    object_code: &str,
    dignity_type: &str,
) -> BasicProjectionReason {
    BasicProjectionReason {
        reason_code: "essential_dignity".to_string(),
        object_code: Some(object_code.to_string()),
        dignity_type: Some(dignity_type.to_string()),
        ..BasicProjectionReason::default()
    }
}

pub(super) fn reason_sign_emphasis(sign_code: &str) -> BasicProjectionReason {
    BasicProjectionReason {
        reason_code: "sign_emphasis".to_string(),
        sign_code: Some(sign_code.to_string()),
        ..BasicProjectionReason::default()
    }
}

pub(super) fn reason_luminary_in_house(
    object_code: &str,
    house_number: i32,
    theme_code: &str,
) -> BasicProjectionReason {
    BasicProjectionReason {
        reason_code: "luminary_in_house".to_string(),
        object_code: Some(object_code.to_string()),
        house_number: Some(house_number),
        theme_code: Some(theme_code.to_string()),
        ..BasicProjectionReason::default()
    }
}

pub(super) fn reason_angle_in_house(
    angle_code: &str,
    house_number: i32,
    theme_code: &str,
) -> BasicProjectionReason {
    BasicProjectionReason {
        reason_code: "angle_in_house".to_string(),
        angle_code: Some(angle_code.to_string()),
        house_number: Some(house_number),
        theme_code: Some(theme_code.to_string()),
        ..BasicProjectionReason::default()
    }
}

pub(super) fn reason_theme_emphasis(theme_code: &str) -> BasicProjectionReason {
    BasicProjectionReason {
        reason_code: "theme_emphasis".to_string(),
        theme_code: Some(theme_code.to_string()),
        ..BasicProjectionReason::default()
    }
}

pub(super) fn reason_active_signal(signal_key: &str) -> BasicProjectionReason {
    BasicProjectionReason {
        reason_code: "active_signal".to_string(),
        signal_key: Some(signal_key.to_string()),
        ..BasicProjectionReason::default()
    }
}

pub(super) fn reason_rulership_context(context_key: &str) -> BasicProjectionReason {
    BasicProjectionReason {
        reason_code: "rulership_context".to_string(),
        context_key: Some(context_key.to_string()),
        ..BasicProjectionReason::default()
    }
}

pub(super) fn reason_cross_axis_aspect(signal_key: &str) -> BasicProjectionReason {
    BasicProjectionReason {
        reason_code: "cross_axis_aspect".to_string(),
        signal_key: Some(signal_key.to_string()),
        ..BasicProjectionReason::default()
    }
}
