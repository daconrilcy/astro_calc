//! Module astral_calculator\src\engine\projection\builder.rs du moteur astral_calculator.
//!
//! Point d'entree de la projection LLM natal et helpers partages entre les
//! sous-modules de construction.

use std::collections::{HashMap, HashSet};

mod chart;
mod house_axes;
mod identity;
mod keywords;
mod placements;
mod reading_order;
mod relationships;
mod strengths;
mod themes;

use super::dynamics::build_dynamics;
use super::humanize::{
    humanize_condition, humanize_motion_label, is_unremarkable_motion_condition, limit_keywords,
    push_unique, render_projection_reason, ProjectionTextCatalog,
};
use super::profiles::limits_envelope;
use super::types::*;
use crate::domain::{
    AccidentalDignityConditionReference, AnglePointReference, BasicObjectPosition, BasicPayload,
    BasicProjectionReason, EssentialDignityRuleReference, HouseAxisReference, HouseReference,
    MotionStateReference, ProjectionLabelDefinition, ProjectionReasonDefinition,
};
use crate::shared::error::RuntimeError;

use chart::build_chart;
use house_axes::build_house_axes;
use identity::build_core_identity;
use keywords::build_keywords;
use placements::{build_angles, build_placements};
use reading_order::build_reading_order;
use relationships::build_relationship_network;
use strengths::build_strengths;
use themes::build_dominant_themes;

/// Structure LlmProjectionBuildContext.
pub struct LlmProjectionBuildContext<'a> {
    pub birth_location_label: &'a str,
    pub zodiac_label: &'a str,
    pub coordinate_label: &'a str,
    pub house_system_label: &'a str,
    pub house_axes: &'a [HouseAxisReference],
    pub projection_reason_definitions: &'a [ProjectionReasonDefinition],
    pub projection_label_definitions: &'a [ProjectionLabelDefinition],
    pub house_references: &'a [HouseReference],
    pub angle_points: &'a [AnglePointReference],
    pub motion_states: &'a [MotionStateReference],
    pub accidental_condition_definitions: &'a [AccidentalDignityConditionReference],
    pub essential_dignity_rules: &'a [EssentialDignityRuleReference],
}

/// Fonction build_llm_projection_natal_v1.
pub fn build_llm_projection_natal_v1(
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
    ctx: &LlmProjectionBuildContext<'_>,
) -> Result<LlmProjectionNatalV1, RuntimeError> {
    let limits = limits_envelope(profile);
    let object_names = object_name_map(payload);
    let resolver = ProjectionTextCatalog::build(
        ctx.projection_reason_definitions,
        ctx.projection_label_definitions,
        ctx.house_references,
        ctx.angle_points,
        ctx.motion_states,
        ctx.accidental_condition_definitions,
        ctx.essential_dignity_rules,
    );
    let dynamics = build_dynamics(payload, profile, &resolver)?;
    let reading_order = build_reading_order(payload, profile, &dynamics, &resolver)?;
    let keywords = build_keywords(payload, profile, &dynamics, &resolver)?;

    Ok(LlmProjectionNatalV1 {
        contract_version: "llm_projection_natal_v1".to_string(),
        projection_level: profile.level_code.clone(),
        projection_limits: limits,
        chart: build_chart(payload, ctx, &resolver)?,
        reading_order,
        core_identity: build_core_identity(payload, profile, &object_names, &resolver)?,
        dominant_themes: build_dominant_themes(payload, profile, &object_names, &resolver)?,
        placements: build_placements(payload, profile, &resolver)?,
        angles: build_angles(payload, profile),
        strengths: build_strengths(payload, profile, &resolver)?,
        relationship_network: build_relationship_network(payload, profile, &object_names),
        dynamics,
        house_axes: build_house_axes(payload, profile, ctx.house_axes, &resolver)?,
        keywords,
    })
}

pub(super) fn object_name_map(payload: &BasicPayload) -> HashMap<String, String> {
    payload
        .positions
        .iter()
        .map(|p| (p.object_code.clone(), p.object_name.clone()))
        .collect()
}

pub(super) fn chart_sect(payload: &BasicPayload) -> Option<&str> {
    payload.chart_context.sect.chart_sect.as_deref()
}

pub(super) fn angle_codes() -> HashSet<&'static str> {
    ["ascendant", "descendant", "mc", "ic"]
        .into_iter()
        .collect()
}

pub(super) fn house_ref_from_payload(
    house_number: i32,
    theme_code: &str,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<LlmHouseRef, RuntimeError> {
    let theme = resolver.house_label(house_number, theme_code)?;
    Ok(LlmHouseRef {
        number: house_number,
        theme,
    })
}

pub(super) fn house_theme_code(position: &BasicObjectPosition) -> &str {
    position
        .house_context
        .as_ref()
        .and_then(|ctx| ctx.get("theme_code"))
        .and_then(|v| v.as_str())
        .unwrap_or("general")
}

pub(super) fn limited_keywords(position: &BasicObjectPosition, limit: usize) -> Vec<String> {
    let raw: Vec<String> = position
        .sign_context
        .as_ref()
        .and_then(|ctx| ctx.get("keywords"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    limit_keywords(&raw, limit)
}

pub(super) fn motion_label(
    position: &BasicObjectPosition,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<Option<String>, RuntimeError> {
    let Some(context) = position.motion_context.as_ref() else {
        return Ok(None);
    };
    let value = context
        .get("motion_state")
        .and_then(|v| v.as_str())
        .or_else(|| context.get("label").and_then(|v| v.as_str()));
    value
        .map(|label| humanize_motion_label(label, resolver))
        .transpose()
}

pub(super) fn placement_from_position(
    position: &BasicObjectPosition,
    include_degrees: bool,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<LlmPlacement, RuntimeError> {
    Ok(LlmPlacement {
        object: position.object_name.clone(),
        sign: position.sign_name.clone(),
        house: position
            .house_number
            .map(|n| house_ref_from_payload(n, house_theme_code(position), resolver))
            .transpose()?,
        motion: motion_label(position, resolver)?,
        keywords: Vec::new(),
        conditions: Vec::new(),
        importance: None,
        longitude_deg: include_degrees.then_some(position.longitude_deg),
    })
}

pub(super) fn position_conditions(
    position: &BasicObjectPosition,
    chart_sect: Option<&str>,
    profile: &LlmProjectionProfile,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<Vec<String>, RuntimeError> {
    let motion = motion_label(position, resolver)?;
    let mut out = Vec::new();
    if let Some(modality) = position.house_modality.as_ref() {
        if let Some(label) = modality.get("label").and_then(|v| v.as_str()) {
            push_unique(&mut out, format!("{label} house"));
        }
    }
    if let Some(horizon) = position
        .visibility_context
        .get("horizon_position")
        .and_then(|v| v.as_str())
    {
        let label = humanize_condition(horizon, chart_sect, resolver)?;
        if !is_unremarkable_motion_condition(&label, motion.as_deref()) {
            push_unique(&mut out, label);
        }
    }
    for summary in &position.accidental_dignity_context {
        let label = humanize_condition(&summary.condition_code, chart_sect, resolver)?;
        if !is_unremarkable_motion_condition(&label, motion.as_deref()) {
            push_unique(&mut out, label);
        }
        if out.len() >= profile.max_accidental_conditions_per_object {
            break;
        }
    }
    Ok(out)
}

pub(super) fn dedupe_rendered_reasons(
    reasons: &[BasicProjectionReason],
    resolver: &ProjectionTextCatalog<'_>,
    object_names: &HashMap<String, String>,
    limit: usize,
) -> Result<Vec<String>, RuntimeError> {
    let mut out = Vec::new();
    for reason in reasons {
        let human = render_projection_reason(reason, resolver, object_names)?;
        push_unique(&mut out, human);
        if out.len() >= limit {
            break;
        }
    }
    Ok(out)
}
