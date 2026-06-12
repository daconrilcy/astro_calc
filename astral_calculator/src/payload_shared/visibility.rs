pub(crate) fn is_horizon_position(value: &str) -> bool {
    matches!(value, "above_horizon" | "below_horizon" | "on_horizon")
}

pub(crate) fn horizon_position_for_altitude(altitude_deg: f64) -> &'static str {
    if altitude_deg > 0.0 {
        "above_horizon"
    } else if altitude_deg < 0.0 {
        "below_horizon"
    } else {
        "on_horizon"
    }
}

pub(crate) fn chart_sect_for_sun_horizon(horizon_position: &str) -> Option<&'static str> {
    match horizon_position {
        "above_horizon" => Some("day"),
        "below_horizon" => Some("night"),
        "on_horizon" => Some("all"),
        _ => None,
    }
}

pub(crate) fn is_angle_role(role: Option<&str>, role_label: Option<&str>) -> bool {
    role == Some("angle") || role_label == Some("Angle")
}
