use crate::domain::{BasicObjectPosition, BasicPayload};

use super::json;

pub(super) fn has_current_chart_context(payload: &BasicPayload) -> bool {
    has_chart_context(payload)
        && payload.positions.iter().all(has_current_visibility_context)
        && has_consistent_sun_sect(payload)
        && has_consistent_hemisphere_counts(payload)
}

fn has_chart_context(payload: &BasicPayload) -> bool {
    let context = &payload.chart_context;

    context.chart_type == "natal"
        && context.zodiacal_reference_system_id > 0
        && context.coordinate_reference_system_id > 0
        && context.house_system_id > 0
        && context.reference_version_id > 0
        && context.reference_version_id == payload.reference_version_id
        && context.payload_contract.contract_version == "natal_structured_v11"
        && context.payload_contract.calculation_scope == "full_natal"
        && context.payload_contract.interpretation_scope == "structured_interpretation"
        && context.payload_contract.projection_depth == "rich"
        && context
            .calculation_reliability
            .birth_time_precision_required
        && context.calculation_reliability.house_system_sensitive
        && context
            .sect
            .chart_sect
            .as_deref()
            .is_some_and(|sect| matches!(sect, "day" | "night" | "all"))
        && context
            .sect
            .sun_horizon_position
            .as_deref()
            .is_some_and(is_horizon_position)
        && context
            .sect
            .source
            .as_deref()
            .is_some_and(|source| !source.trim().is_empty())
        && context.hemisphere_emphasis.count_scope == "mobile_chart_objects_only"
        && context.hemisphere_emphasis.above_horizon_count >= 0
        && context.hemisphere_emphasis.below_horizon_count >= 0
        && context.hemisphere_emphasis.on_horizon_count >= 0
}

fn has_current_visibility_context(position: &BasicObjectPosition) -> bool {
    let value = &position.visibility_context;
    let is_angle = is_angle(position);
    let horizon_position = value
        .get("horizon_position")
        .and_then(|value| value.as_str());
    let altitude_deg = value.get("altitude_deg").and_then(|value| value.as_f64());
    let source = value.get("source").and_then(|value| value.as_str());

    value.is_object()
        && horizon_position.is_some_and(is_horizon_position)
        && value
            .get("horizon_position_id")
            .is_some_and(|value| value.as_i64().is_some_and(|id| id > 0))
        && source.is_some_and(|source| !source.trim().is_empty())
        && if is_angle {
            has_consistent_angle_visibility(value, source)
        } else {
            json::has_bool_value(value.get("is_visible"))
                && has_consistent_calculated_altitude(horizon_position, altitude_deg, source)
        }
}

fn is_horizon_position(value: &str) -> bool {
    matches!(value, "above_horizon" | "below_horizon" | "on_horizon")
}

fn has_consistent_calculated_altitude(
    horizon_position: Option<&str>,
    altitude_deg: Option<f64>,
    source: Option<&str>,
) -> bool {
    let Some(altitude_deg) = altitude_deg.filter(|altitude| altitude.is_finite()) else {
        return false;
    };

    source == Some("calculated_altitude")
        && horizon_position == Some(horizon_position_for_altitude(altitude_deg))
}

fn has_consistent_angle_visibility(value: &serde_json::Value, source: Option<&str>) -> bool {
    source == Some("angle_context")
        && value
            .get("altitude_deg")
            .is_some_and(|value| value.is_null())
        && value.get("is_visible").is_some_and(|value| value.is_null())
}

fn horizon_position_for_altitude(altitude_deg: f64) -> &'static str {
    if altitude_deg > 0.0 {
        "above_horizon"
    } else if altitude_deg < 0.0 {
        "below_horizon"
    } else {
        "on_horizon"
    }
}

fn has_consistent_sun_sect(payload: &BasicPayload) -> bool {
    let Some(sun) = payload
        .positions
        .iter()
        .find(|position| position.object_code == "sun")
    else {
        return false;
    };

    let Some(sun_horizon_position) = horizon_position(sun) else {
        return false;
    };

    payload.chart_context.sect.sun_horizon_position.as_deref() == Some(sun_horizon_position)
        && payload.chart_context.sect.chart_sect.as_deref()
            == chart_sect_for_sun_horizon(sun_horizon_position)
        && payload.chart_context.sect.source.as_deref() == visibility_source(sun)
}

fn has_consistent_hemisphere_counts(payload: &BasicPayload) -> bool {
    let mut above_horizon_count = 0;
    let mut below_horizon_count = 0;
    let mut on_horizon_count = 0;

    for position in payload
        .positions
        .iter()
        .filter(|position| !is_angle(position))
    {
        match horizon_position(position) {
            Some("above_horizon") => above_horizon_count += 1,
            Some("below_horizon") => below_horizon_count += 1,
            Some("on_horizon") => on_horizon_count += 1,
            _ => return false,
        }
    }

    payload
        .chart_context
        .hemisphere_emphasis
        .above_horizon_count
        == above_horizon_count
        && payload
            .chart_context
            .hemisphere_emphasis
            .below_horizon_count
            == below_horizon_count
        && payload.chart_context.hemisphere_emphasis.on_horizon_count == on_horizon_count
}

fn horizon_position(position: &BasicObjectPosition) -> Option<&str> {
    position
        .visibility_context
        .get("horizon_position")
        .and_then(|value| value.as_str())
        .filter(|value| is_horizon_position(value))
}

fn visibility_source(position: &BasicObjectPosition) -> Option<&str> {
    position
        .visibility_context
        .get("source")
        .and_then(|value| value.as_str())
}

fn chart_sect_for_sun_horizon(horizon_position: &str) -> Option<&'static str> {
    match horizon_position {
        "above_horizon" => Some("day"),
        "below_horizon" => Some("night"),
        "on_horizon" => Some("all"),
        _ => None,
    }
}

fn is_angle(position: &BasicObjectPosition) -> bool {
    position
        .object_context
        .as_ref()
        .and_then(|context| context.get("role"))
        .and_then(|value| value.as_str())
        == Some("angle")
}
