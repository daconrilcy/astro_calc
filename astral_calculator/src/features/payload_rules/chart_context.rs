use serde_json::{json, Value};

use crate::catalog::BasicPayloadCatalog;
use crate::catalog::accidental_polarity_bands_are_valid;
use crate::domain::{
    BasicAccidentalScoringSnapshot, BasicCalculationReliability, BasicChartContext,
    BasicHemisphereEmphasis, BasicObjectPosition, BasicPayload, BasicPayloadContract,
    BasicProductScoringSnapshot, BasicSectContext, NatalChartInput, ObjectPositionFact,
};
use crate::features::payload_shared::contract::{
    CALCULATION_SCOPE_FULL_NATAL, CHART_TYPE_NATAL, CONTRACT_VERSION_V13,
    INTERPRETATION_SCOPE_STRUCTURED, PROJECTION_DEPTH_RICH,
};
use crate::features::payload_shared::text::{has_text, is_normalized_score};

const ABOVE_HORIZON: &str = "above_horizon";
const BELOW_HORIZON: &str = "below_horizon";
const ON_HORIZON: &str = "on_horizon";

pub(crate) fn build_chart_context(
    input: &NatalChartInput,
    positions: &[ObjectPositionFact],
    contract_version: &str,
    catalog: Option<&BasicPayloadCatalog>,
) -> BasicChartContext {
    let sun_position = positions.iter().find(|position| position.object_code == "sun");
    let sun_horizon_position = sun_position.and_then(horizon_position_code_for_fact);
    let chart_sect = sun_horizon_position
        .as_deref()
        .and_then(chart_sect_for_sun_horizon)
        .map(str::to_string);
    let sect_source = sun_position.map(visibility_source_for_fact);
    let hemisphere_emphasis = build_hemisphere_emphasis(positions);

    BasicChartContext {
        chart_type: CHART_TYPE_NATAL.to_string(),
        zodiacal_reference_system_id: input.zodiacal_reference_system_id,
        coordinate_reference_system_id: input.coordinate_reference_system_id,
        house_system_id: input.house_system_id,
        reference_version_id: input.reference_version_id,
        payload_contract: BasicPayloadContract {
            contract_version: contract_version.to_string(),
            calculation_scope: CALCULATION_SCOPE_FULL_NATAL.to_string(),
            interpretation_scope: INTERPRETATION_SCOPE_STRUCTURED.to_string(),
            projection_depth: PROJECTION_DEPTH_RICH.to_string(),
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
        accidental_scoring: catalog.map(|catalog| BasicAccidentalScoringSnapshot {
            overall_score_baseline: catalog.accidental_scoring.overall_score_baseline,
            overall_score_min: catalog.accidental_scoring.overall_score_min,
            overall_score_max: catalog.accidental_scoring.overall_score_max,
            angle_proximity_max_orb_deg: catalog.accidental_scoring.angle_proximity_max_orb_deg,
            polarity_bands: catalog.accidental_polarity_bands.clone(),
        }),
        product_scoring: catalog.map(|catalog| {
            let scoring = &catalog.product_scoring;
            BasicProductScoringSnapshot {
                sign_house_emphasis_min_score: scoring.sign_house_emphasis_min_score,
                object_emphasis_min_score: scoring.object_emphasis_min_score,
                max_dominant_signs: scoring.max_dominant_signs,
                max_dominant_houses: scoring.max_dominant_houses,
                max_dominant_objects: scoring.max_dominant_objects,
                max_active_signals: scoring.max_active_signals,
                aspect_min_strength: scoring.aspect_min_strength,
                max_house_axis_emphasis: scoring.max_house_axis_emphasis,
            }
        }),
    }
}

pub(crate) fn visibility_context(position: &ObjectPositionFact) -> Value {
    json!({
        "horizon_position_id": position.horizon_position_id,
        "horizon_position": horizon_position_code_for_fact(position),
        "altitude_deg": if is_angle_position_fact(position) { None } else { position.altitude_deg },
        "is_visible": visibility_flag_for_fact(position),
        "source": visibility_source_for_fact(position)
    })
}

pub(crate) fn has_current_chart_context(payload: &BasicPayload) -> bool {
    has_chart_context(payload)
        && payload.positions.iter().all(has_current_visibility_context)
        && has_consistent_sun_sect(payload)
        && has_consistent_hemisphere_counts(payload)
}

pub(crate) fn is_horizon_position(value: &str) -> bool {
    matches!(value, ABOVE_HORIZON | BELOW_HORIZON | ON_HORIZON)
}

pub(crate) fn horizon_position_for_altitude(altitude_deg: f64) -> &'static str {
    if altitude_deg > 0.0 {
        ABOVE_HORIZON
    } else if altitude_deg < 0.0 {
        BELOW_HORIZON
    } else {
        ON_HORIZON
    }
}

pub(crate) fn chart_sect_for_sun_horizon(horizon_position: &str) -> Option<&'static str> {
    match horizon_position {
        ABOVE_HORIZON => Some("day"),
        BELOW_HORIZON => Some("night"),
        ON_HORIZON => Some("all"),
        _ => None,
    }
}

pub(crate) fn is_angle_role(role: Option<&str>, role_label: Option<&str>) -> bool {
    role == Some("angle") || role_label == Some("Angle")
}

pub(crate) fn is_angle_position_fact(position: &ObjectPositionFact) -> bool {
    let role = position
        .facts_json
        .as_ref()
        .and_then(|facts| facts.get("object_context"))
        .and_then(|context| context.get("role"))
        .and_then(|value| value.as_str());
    let role_label = position
        .facts_json
        .as_ref()
        .and_then(|facts| facts.get("object_context"))
        .and_then(|context| context.get("role_label"))
        .and_then(|value| value.as_str());
    is_angle_role(role, role_label)
        || position
            .facts_json
            .as_ref()
            .and_then(|facts| facts.get("angle_context"))
            .is_some()
}

pub(crate) fn is_angle_position(position: &BasicObjectPosition) -> bool {
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
    is_angle_role(role, role_label)
}

pub(crate) fn horizon_position_code_for_fact(position: &ObjectPositionFact) -> Option<String> {
    if !is_angle_position_fact(position) {
        if let Some(altitude) = position.altitude_deg {
            return Some(horizon_position_for_altitude(altitude).to_string());
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

pub(crate) fn visibility_source_for_fact(position: &ObjectPositionFact) -> String {
    if is_angle_position_fact(position) {
        return "angle_context".to_string();
    }

    if position.altitude_deg.is_some() {
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
    } else {
        "house_hemisphere_projection".to_string()
    }
}

fn visibility_flag_for_fact(position: &ObjectPositionFact) -> Option<bool> {
    if is_angle_position_fact(position) {
        return None;
    }

    position
        .is_visible
        .or_else(|| match horizon_position_code_for_fact(position).as_deref() {
            Some(ABOVE_HORIZON) | Some(ON_HORIZON) => Some(true),
            Some(BELOW_HORIZON) => Some(false),
            _ => None,
        })
}

fn build_hemisphere_emphasis(positions: &[ObjectPositionFact]) -> BasicHemisphereEmphasis {
    let mut above_horizon_count = 0;
    let mut below_horizon_count = 0;
    let mut on_horizon_count = 0;

    for position in positions.iter().filter(|position| !is_angle_position_fact(position)) {
        match horizon_position_code_for_fact(position).as_deref() {
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

fn has_chart_context(payload: &BasicPayload) -> bool {
    let context = &payload.chart_context;

    context.chart_type == CHART_TYPE_NATAL
        && context.zodiacal_reference_system_id > 0
        && context.coordinate_reference_system_id > 0
        && context.house_system_id > 0
        && context.reference_version_id > 0
        && context.reference_version_id == payload.reference_version_id
        && context.payload_contract.contract_version == CONTRACT_VERSION_V13
        && context.payload_contract.calculation_scope == CALCULATION_SCOPE_FULL_NATAL
        && context.payload_contract.interpretation_scope == INTERPRETATION_SCOPE_STRUCTURED
        && context.payload_contract.projection_depth == PROJECTION_DEPTH_RICH
        && context.calculation_reliability.birth_time_precision_required
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
        && context.sect.source.as_deref().is_some_and(has_text)
        && context.hemisphere_emphasis.count_scope == "mobile_chart_objects_only"
        && context.hemisphere_emphasis.above_horizon_count >= 0
        && context.hemisphere_emphasis.below_horizon_count >= 0
        && context.hemisphere_emphasis.on_horizon_count >= 0
        && has_valid_v13_scoring_snapshots(context)
}

fn has_valid_v13_scoring_snapshots(context: &BasicChartContext) -> bool {
    let Some(accidental) = context.accidental_scoring.as_ref() else {
        return false;
    };
    let Some(product) = context.product_scoring.as_ref() else {
        return false;
    };

    accidental.overall_score_min <= accidental.overall_score_baseline
        && accidental.overall_score_baseline <= accidental.overall_score_max
        && accidental.overall_score_min >= 0.0
        && accidental.overall_score_max <= 1.0
        && accidental.angle_proximity_max_orb_deg > 0.0
        && accidental_polarity_bands_are_valid(&accidental.polarity_bands)
        && product.sign_house_emphasis_min_score >= 0.0
        && product.object_emphasis_min_score >= 0.0
        && is_normalized_score(product.aspect_min_strength)
        && product.max_dominant_signs > 0
        && product.max_dominant_houses > 0
        && product.max_dominant_objects > 0
        && product.max_active_signals > 0
        && product.max_house_axis_emphasis > 0
}

fn has_current_visibility_context(position: &BasicObjectPosition) -> bool {
    let value = &position.visibility_context;
    let is_angle = is_angle_position(position);
    let horizon_position = value.get("horizon_position").and_then(|value| value.as_str());
    let altitude_deg = value.get("altitude_deg").and_then(|value| value.as_f64());
    let source = value.get("source").and_then(|value| value.as_str());

    value.is_object()
        && horizon_position.is_some_and(is_horizon_position)
        && value
            .get("horizon_position_id")
            .is_some_and(|value| value.as_i64().is_some_and(|id| id > 0))
        && source.is_some_and(|source| !source.trim().is_empty())
        && if is_angle {
            source == Some("angle_context")
                && value.get("altitude_deg").is_some_and(|value| value.is_null())
                && value.get("is_visible").is_some_and(|value| value.is_null())
        } else {
            let Some(altitude_deg) = altitude_deg.filter(|altitude| altitude.is_finite()) else {
                return false;
            };

            value.get("is_visible").is_some_and(|value| value.is_boolean())
                && source == Some("calculated_altitude")
                && horizon_position == Some(horizon_position_for_altitude(altitude_deg))
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

    let Some(sun_horizon_position) = sun
        .visibility_context
        .get("horizon_position")
        .and_then(|value| value.as_str())
        .filter(|value| is_horizon_position(value))
    else {
        return false;
    };

    payload.chart_context.sect.sun_horizon_position.as_deref() == Some(sun_horizon_position)
        && payload.chart_context.sect.chart_sect.as_deref()
            == chart_sect_for_sun_horizon(sun_horizon_position)
        && payload.chart_context.sect.source.as_deref()
            == sun.visibility_context.get("source").and_then(|value| value.as_str())
}

fn has_consistent_hemisphere_counts(payload: &BasicPayload) -> bool {
    let mut above_horizon_count = 0;
    let mut below_horizon_count = 0;
    let mut on_horizon_count = 0;

    for position in payload
        .positions
        .iter()
        .filter(|position| !is_angle_position(position))
    {
        match position
            .visibility_context
            .get("horizon_position")
            .and_then(|value| value.as_str())
        {
            Some(ABOVE_HORIZON) => above_horizon_count += 1,
            Some(BELOW_HORIZON) => below_horizon_count += 1,
            Some(ON_HORIZON) => on_horizon_count += 1,
            _ => return false,
        }
    }

    payload.chart_context.hemisphere_emphasis.above_horizon_count == above_horizon_count
        && payload.chart_context.hemisphere_emphasis.below_horizon_count == below_horizon_count
        && payload.chart_context.hemisphere_emphasis.on_horizon_count == on_horizon_count
}
