use serde_json::{json, Value};

use crate::domain::{
    BasicCalculationReliability, BasicChartContext, BasicHemisphereEmphasis, BasicPayloadContract,
    BasicSectContext, NatalChartInput, ObjectPositionFact,
};

const ABOVE_HORIZON: &str = "above_horizon";
const BELOW_HORIZON: &str = "below_horizon";
const ON_HORIZON: &str = "on_horizon";

pub(super) fn build_chart_context(
    input: &NatalChartInput,
    positions: &[ObjectPositionFact],
) -> BasicChartContext {
    let sun_position = positions
        .iter()
        .find(|position| position.object_code == "sun");
    let sun_horizon_position = sun_position.and_then(horizon_position_code);
    let chart_sect = sun_horizon_position
        .as_deref()
        .and_then(chart_sect_from_sun_horizon);
    let sect_source = sun_position.map(visibility_source);
    let hemisphere_emphasis = build_hemisphere_emphasis(positions);

    BasicChartContext {
        chart_type: "natal".to_string(),
        zodiacal_reference_system_id: input.zodiacal_reference_system_id,
        coordinate_reference_system_id: input.coordinate_reference_system_id,
        house_system_id: input.house_system_id,
        reference_version_id: input.reference_version_id,
        payload_contract: BasicPayloadContract {
            contract_version: "natal_structured_v10".to_string(),
            calculation_scope: "full_natal".to_string(),
            interpretation_scope: "structured_interpretation".to_string(),
            projection_depth: "rich".to_string(),
        },
        calculation_reliability: BasicCalculationReliability {
            birth_time_precision_required: true,
            house_system_sensitive: true,
        },
        sect: BasicSectContext {
            chart_sect,
            sun_horizon_position,
            source: sect_source,
        },
        hemisphere_emphasis,
    }
}

pub(super) fn visibility_context(position: &ObjectPositionFact) -> Value {
    json!({
        "horizon_position_id": position.horizon_position_id,
        "horizon_position": horizon_position_code(position),
        "altitude_deg": if is_angle(position) { None } else { position.altitude_deg },
        "is_visible": visibility_flag(position),
        "source": visibility_source(position)
    })
}

fn build_hemisphere_emphasis(positions: &[ObjectPositionFact]) -> BasicHemisphereEmphasis {
    let mut above_horizon_count = 0;
    let mut below_horizon_count = 0;
    let mut on_horizon_count = 0;

    for position in positions.iter().filter(|position| !is_angle(position)) {
        match horizon_position_code(position).as_deref() {
            Some(ABOVE_HORIZON) => above_horizon_count += 1,
            Some(BELOW_HORIZON) => below_horizon_count += 1,
            Some(ON_HORIZON) => on_horizon_count += 1,
            _ => {}
        }
    }

    BasicHemisphereEmphasis {
        count_scope: "mobile_chart_objects_only".to_string(),
        above_horizon_count,
        below_horizon_count,
        on_horizon_count,
        interpretive_hint: hemisphere_hint(above_horizon_count, below_horizon_count),
    }
}

fn horizon_position_code(position: &ObjectPositionFact) -> Option<String> {
    if !is_angle(position) {
        if let Some(altitude) = position.altitude_deg {
            return Some(horizon_position_code_for_altitude(altitude).to_string());
        }
    }

    if let Some(code) = position
        .facts_json
        .as_ref()
        .and_then(|facts| facts.get("visibility_context"))
        .and_then(|context| context.get("horizon_position"))
        .and_then(|value| value.as_str())
    {
        return Some(code.to_string());
    }

    if let Some(code) = angle_horizon_position(position) {
        return Some(code.to_string());
    }

    position.house_number.and_then(|house_number| {
        if (7..=12).contains(&house_number) {
            Some(ABOVE_HORIZON.to_string())
        } else if (1..=6).contains(&house_number) {
            Some(BELOW_HORIZON.to_string())
        } else {
            None
        }
    })
}

fn visibility_flag(position: &ObjectPositionFact) -> Option<bool> {
    if is_angle(position) {
        return None;
    }

    position
        .is_visible
        .or_else(|| match horizon_position_code(position).as_deref() {
            Some(ABOVE_HORIZON) | Some(ON_HORIZON) => Some(true),
            Some(BELOW_HORIZON) => Some(false),
            _ => None,
        })
}

fn visibility_source(position: &ObjectPositionFact) -> String {
    if is_angle(position) {
        return "angle_context".to_string();
    }

    if !is_angle(position) && position.altitude_deg.is_some() {
        return "calculated_altitude".to_string();
    }

    if let Some(source) = position
        .facts_json
        .as_ref()
        .and_then(|facts| facts.get("visibility_context"))
        .and_then(|context| context.get("source"))
        .and_then(|value| value.as_str())
        .filter(|source| !source.trim().is_empty())
    {
        return source.to_string();
    }

    if position.is_visible.is_some() {
        "calculated_altitude".to_string()
    } else if is_angle(position) {
        "angle_context".to_string()
    } else {
        "house_hemisphere_projection".to_string()
    }
}

fn horizon_position_code_for_altitude(altitude: f64) -> &'static str {
    if altitude > 0.0 {
        ABOVE_HORIZON
    } else if altitude < 0.0 {
        BELOW_HORIZON
    } else {
        ON_HORIZON
    }
}

fn chart_sect_from_sun_horizon(horizon_position: &str) -> Option<String> {
    match horizon_position {
        ABOVE_HORIZON => Some("day".to_string()),
        BELOW_HORIZON => Some("night".to_string()),
        ON_HORIZON => Some("all".to_string()),
        _ => None,
    }
}

fn hemisphere_hint(above_horizon_count: i32, below_horizon_count: i32) -> Option<String> {
    if above_horizon_count > below_horizon_count {
        Some("The chart has a stronger visible or outward emphasis.".to_string())
    } else if below_horizon_count > above_horizon_count {
        Some("The chart has a stronger private or interior emphasis.".to_string())
    } else if above_horizon_count > 0 {
        Some("The chart balances visible and private emphases.".to_string())
    } else {
        None
    }
}

fn angle_horizon_position(position: &ObjectPositionFact) -> Option<&'static str> {
    match position
        .facts_json
        .as_ref()
        .and_then(|facts| facts.get("angle_context"))
        .and_then(|context| context.get("angle_point_code"))
        .and_then(|value| value.as_str())
    {
        Some("asc") | Some("dsc") => Some(ON_HORIZON),
        Some("mc") => Some(ABOVE_HORIZON),
        Some("ic") => Some(BELOW_HORIZON),
        _ => None,
    }
}

fn is_angle(position: &ObjectPositionFact) -> bool {
    position
        .facts_json
        .as_ref()
        .and_then(|facts| facts.get("object_context"))
        .and_then(|context| context.get("role"))
        .and_then(|value| value.as_str())
        == Some("angle")
        || position
            .facts_json
            .as_ref()
            .and_then(|facts| facts.get("angle_context"))
            .is_some()
}
