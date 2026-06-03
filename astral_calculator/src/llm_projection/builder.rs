use std::collections::{BTreeMap, HashMap, HashSet};

use crate::domain::{
    BasicAccidentalDignityEvaluation, BasicHouseAxisEmphasis, BasicObjectPosition, BasicPayload,
    HouseAxisReference,
};
use crate::llm_projection::axis_labels::house_axis_label;
use crate::llm_projection::dynamics::build_dynamics;
use crate::llm_projection::humanize::{
    accidental_overall_label, axis_balance_label, axis_importance, chart_sect_label,
    dignity_meaning, hemisphere_dominant_area, humanize_condition, humanize_motion_label,
    humanize_axis_summary, humanize_reason, importance_label,
    is_unremarkable_motion_condition, limit_keywords, push_unique, reading_slot_section,
    title_case_sign,
};
use crate::llm_projection::profiles::limits_envelope;
use crate::llm_projection::types::*;

pub struct LlmProjectionBuildContext<'a> {
    pub birth_location_label: &'a str,
    pub zodiac_label: &'a str,
    pub coordinate_label: &'a str,
    pub house_system_label: &'a str,
    pub house_axes: &'a [HouseAxisReference],
}

pub fn build_llm_projection_natal_v1(
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
    ctx: &LlmProjectionBuildContext<'_>,
) -> LlmProjectionNatalV1 {
    let limits = limits_envelope(profile);
    let object_names = object_name_map(payload);
    let dynamics = build_dynamics(payload, profile);
    let reading_order = build_reading_order(payload, profile, &dynamics);
    let keywords = build_keywords(payload, profile, &dynamics);

    LlmProjectionNatalV1 {
        contract_version: "llm_projection_natal_v1".to_string(),
        projection_level: profile.level_code.clone(),
        projection_limits: limits,
        chart: build_chart(payload, ctx),
        reading_order,
        core_identity: build_core_identity(payload, profile, &object_names),
        dominant_themes: build_dominant_themes(payload, profile, &object_names),
        placements: build_placements(payload, profile),
        angles: build_angles(payload, profile),
        strengths: build_strengths(payload, profile),
        relationship_network: build_relationship_network(payload, profile, &object_names),
        dynamics,
        house_axes: build_house_axes(payload, profile, ctx.house_axes),
        keywords,
    }
}

fn object_name_map(payload: &BasicPayload) -> HashMap<String, String> {
    payload
        .positions
        .iter()
        .map(|p| (p.object_code.clone(), p.object_name.clone()))
        .collect()
}

fn build_chart(payload: &BasicPayload, ctx: &LlmProjectionBuildContext<'_>) -> LlmChart {
    let sect = payload
        .chart_context
        .sect
        .chart_sect
        .as_deref()
        .map(chart_sect_label);
    let hemisphere = payload.chart_context.hemisphere_emphasis.interpretive_hint.as_ref().map(
        |hint| LlmHemisphereEmphasis {
            dominant_area: hemisphere_dominant_area(
                hint,
                payload.chart_context.hemisphere_emphasis.above_horizon_count,
                payload.chart_context.hemisphere_emphasis.below_horizon_count,
            ),
            summary: hint.clone(),
        },
    );

    LlmChart {
        chart_type: "Natal chart".to_string(),
        birth: LlmChartBirth {
            datetime_utc: payload.birth_datetime_utc.to_rfc3339(),
            location: ctx.birth_location_label.to_string(),
        },
        calculation: LlmChartCalculation {
            zodiac: ctx.zodiac_label.to_string(),
            coordinates: ctx.coordinate_label.to_string(),
            house_system: ctx.house_system_label.to_string(),
        },
        sect,
        hemisphere_emphasis: hemisphere,
    }
}

fn build_reading_order(
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
    dynamics: &LlmDynamics,
) -> Vec<LlmReadingOrderItem> {
    payload
        .reading_plan
        .iter()
        .map(|item| {
            let focus = match item.slot.as_str() {
                "dominant_cluster" => dominant_cluster_reading_focus(payload, profile),
                "main_tension_or_support" => main_dynamic_reading_focus(dynamics, payload, profile),
                _ => item
                    .primary_signal_keys
                    .iter()
                    .filter_map(|key| reading_focus_from_signal(payload, key))
                    .collect(),
            };
            let focus = limit_keywords(&focus, profile.max_keywords_per_item);
            LlmReadingOrderItem {
                section: reading_slot_section(&item.slot, &item.title),
                focus,
            }
        })
        .collect()
}

fn dominant_cluster_reading_focus(
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
) -> Vec<String> {
    let mut focus = Vec::new();
    if let Some(sign) = payload.chart_emphasis.dominant_signs.first() {
        push_unique(
            &mut focus,
            format!("{} emphasis", title_case_sign(&sign.sign_code)),
        );
    }
    if let Some(house) = payload.chart_emphasis.dominant_houses.first() {
        let theme = house_ref_from_payload(house.house_number, &house.theme_code, payload);
        push_unique(
            &mut focus,
            format!("House {} {} emphasis", theme.number, theme.theme.to_lowercase()),
        );
    }
    if let Some(object) = payload.chart_emphasis.dominant_objects.first() {
        let name = payload
            .positions
            .iter()
            .find(|p| p.object_code == object.object_code)
            .map(|p| p.object_name.clone())
            .unwrap_or_else(|| title_case_sign(&object.object_code));
        push_unique(&mut focus, format!("{name} strength"));
    }
    if focus.is_empty() {
        for key in payload
            .reading_plan
            .iter()
            .find(|item| item.slot == "dominant_cluster")
            .into_iter()
            .flat_map(|item| item.primary_signal_keys.iter())
        {
            if let Some(label) = reading_focus_from_signal(payload, key) {
                push_unique(&mut focus, label);
            }
            if focus.len() >= profile.max_keywords_per_item {
                break;
            }
        }
    }
    focus
}

fn main_dynamic_reading_focus(
    dynamics: &LlmDynamics,
    payload: &BasicPayload,
    _profile: &LlmProjectionProfile,
) -> Vec<String> {
    if let Some(aspect) = dynamics.major_aspects.first() {
        return vec![aspect.aspect.clone()];
    }
    payload
        .reading_plan
        .iter()
        .find(|item| item.slot == "main_tension_or_support")
        .into_iter()
        .flat_map(|item| item.primary_signal_keys.iter())
        .filter_map(|key| reading_focus_from_signal(payload, key))
        .collect()
}

fn reading_focus_from_signal(payload: &BasicPayload, signal_key: &str) -> Option<String> {
    if let Some(signal) = payload.signals.iter().find(|s| s.signal_key == signal_key) {
        return Some(signal.title.clone());
    }
    if signal_key.starts_with("object_position:") {
        let code = signal_key.trim_start_matches("object_position:");
        return payload
            .positions
            .iter()
            .find(|p| p.object_code == code)
            .map(|p| p.object_name.clone());
    }
    if let Some(rest) = signal_key.strip_prefix("angle:") {
        let parts: Vec<_> = rest.split(':').collect();
        if parts.len() >= 2 {
            let angle = title_case_sign(parts[0]);
            let sign = title_case_sign(parts[2]);
            return Some(format!("{angle} in {sign}"));
        }
    }
    if let Some(rest) = signal_key.strip_prefix("aspect:") {
        let parts: Vec<_> = rest.split(':').collect();
        if parts.len() >= 3 {
            let a = title_case_sign(parts[0]);
            let b = title_case_sign(parts[1]);
            let aspect = title_case_sign(parts[2]);
            return Some(format!("{a} {aspect} {b}"));
        }
    }
    if signal_key.starts_with("cluster:") {
        return Some("Dominant cluster".to_string());
    }
    if signal_key.starts_with("dignity:") {
        let parts: Vec<_> = signal_key.split(':').collect();
        if parts.len() >= 4 {
            return Some(format!(
                "{} {}",
                title_case_sign(parts[1]),
                title_case_sign(parts[2])
            ));
        }
    }
    None
}

fn build_core_identity(
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
    object_names: &HashMap<String, String>,
) -> LlmCoreIdentity {
    LlmCoreIdentity {
        sun: mobile_body(payload, "sun", profile),
        moon: mobile_body(payload, "moon", profile),
        ascendant: build_ascendant_core(payload, profile, object_names),
    }
}

fn chart_sect(payload: &BasicPayload) -> Option<&str> {
    payload.chart_context.sect.chart_sect.as_deref()
}

fn mobile_body(payload: &BasicPayload, code: &str, profile: &LlmProjectionProfile) -> Option<LlmCoreBody> {
    let position = payload.positions.iter().find(|p| p.object_code == code)?;
    Some(LlmCoreBody {
        placement: placement_from_position(position, profile.include_degrees),
        keywords: limited_keywords(position, profile.max_keywords_per_item),
        conditions: position_conditions(position, chart_sect(payload), profile),
        importance: "high".to_string(),
    })
}

fn build_ascendant_core(
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
    object_names: &HashMap<String, String>,
) -> Option<LlmAscendantBody> {
    let asc = payload
        .angles
        .iter()
        .find(|a| a.angle_code == "ascendant")?;
    let sign_keywords = payload
        .positions
        .iter()
        .find(|p| p.object_code == "ascendant")
        .map(|p| limited_keywords(p, profile.max_keywords_per_item))
        .unwrap_or_default();

    let ruler = if profile.include_rulership_details {
        payload.rulership_context.ascendant_ruler.as_ref().map(|r| {
        let mut rulers = LlmAscendantRulers {
            traditional: None,
            modern: None,
        };
        for source in &r.ruler_sources {
            if source.astral_system_code == "traditional" {
                rulers.traditional = Some(
                    object_names
                        .get(&source.object_code)
                        .cloned()
                        .unwrap_or_else(|| title_case_sign(&source.object_code)),
                );
            }
            if source.astral_system_code == "modern" {
                rulers.modern = Some(
                    object_names
                        .get(&source.object_code)
                        .cloned()
                        .unwrap_or_else(|| title_case_sign(&source.object_code)),
                );
            }
        }
        if rulers.traditional.is_none() {
            rulers.traditional = Some(
                object_names
                    .get(&r.ruler_object_code)
                    .cloned()
                    .unwrap_or_else(|| title_case_sign(&r.ruler_object_code)),
            );
        }
        rulers
        })
    } else {
        None
    };

    Some(LlmAscendantBody {
        sign: asc.sign_name.clone(),
        keywords: sign_keywords,
        ruler,
        importance: "high".to_string(),
    })
}

fn build_dominant_themes(
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
    object_names: &HashMap<String, String>,
) -> LlmDominantThemes {
    let signs = payload
        .chart_emphasis
        .dominant_signs
        .iter()
        .take(profile.max_dominant_signs)
        .map(|entry| {
            let sign = title_case_sign(&entry.sign_code);
            LlmDominantSign {
                name: sign.clone(),
                importance: importance_label(entry.score).to_string(),
                supporting_factors: dedupe_humanized_reasons(
                    &entry.reasons,
                    object_names,
                    profile.max_keywords_per_item,
                ),
                keywords: sign_keywords_from_positions(payload, &entry.sign_code, profile.max_keywords_per_item),
                score: profile.include_scores.then_some(entry.score),
            }
        })
        .collect();

    let houses = payload
        .chart_emphasis
        .dominant_houses
        .iter()
        .take(profile.max_dominant_houses)
        .map(|entry| {
            let house_ref = house_ref_from_payload(entry.house_number, &entry.theme_code, payload);
            LlmDominantHouse {
                number: house_ref.number,
                theme: house_ref.theme,
                importance: importance_label(entry.score).to_string(),
                supporting_factors: dedupe_humanized_reasons(
                    &entry.reasons,
                    object_names,
                    profile.max_keywords_per_item,
                ),
                score: profile.include_scores.then_some(entry.score),
            }
        })
        .collect();

    let objects = payload
        .chart_emphasis
        .dominant_objects
        .iter()
        .take(profile.max_dominant_objects)
        .map(|entry| LlmDominantObject {
            name: object_names
                .get(&entry.object_code)
                .cloned()
                .unwrap_or_else(|| title_case_sign(&entry.object_code)),
            importance: importance_label(entry.score).to_string(),
            supporting_factors: dedupe_humanized_reasons(
                &entry.reasons,
                object_names,
                profile.max_keywords_per_item,
            ),
            score: profile.include_scores.then_some(entry.score),
        })
        .collect();

    LlmDominantThemes {
        signs,
        houses,
        objects,
    }
}

fn sign_keywords_from_positions(
    payload: &BasicPayload,
    sign_code: &str,
    limit: usize,
) -> Vec<String> {
    let mut out = Vec::new();
    for position in payload.positions.iter().filter(|p| p.sign_code == sign_code) {
        for kw in limited_keywords(position, limit) {
            push_unique(&mut out, kw);
            if out.len() >= limit {
                return out;
            }
        }
    }
    out
}

fn build_placements(payload: &BasicPayload, profile: &LlmProjectionProfile) -> LlmPlacementsGroup {
    let core_codes: HashSet<&str> = ["sun", "moon"].into_iter().collect();
    let angle_codes: HashSet<&str> = ["ascendant", "descendant", "mc", "ic"].into_iter().collect();
    let reading_codes: HashSet<String> = payload
        .reading_plan
        .iter()
        .flat_map(|item| item.primary_signal_keys.clone())
        .filter_map(|key| key.strip_prefix("object_position:").map(str::to_string))
        .collect();

    let primary = Vec::new();
    let mut supporting = Vec::new();
    let mut background = Vec::new();

    let mut mobiles: Vec<&BasicObjectPosition> = payload
        .positions
        .iter()
        .filter(|p| !angle_codes.contains(p.object_code.as_str()))
        .collect();
    mobiles.sort_by(|a, b| object_priority(a).cmp(&object_priority(b)).reverse());

    for position in mobiles {
        let mut item = placement_from_position(position, profile.include_degrees);
        item.house = position
            .house_number
            .map(|n| house_ref_from_payload(n, house_theme_code(position), payload));
        item.keywords = limited_keywords(position, profile.max_keywords_per_item);
        item.conditions = position_conditions(position, chart_sect(payload), profile);
        item.importance = Some(importance_for_object(&position.object_code, &reading_codes).to_string());

        if core_codes.contains(position.object_code.as_str()) {
            continue;
        } else if supporting.len() < profile.max_supporting_placements {
            supporting.push(item);
        } else if background.len() < profile.max_background_placements {
            background.push(item);
        }
    }

    LlmPlacementsGroup {
        primary,
        supporting: supporting
            .into_iter()
            .take(profile.max_supporting_placements)
            .collect(),
        background: background
            .into_iter()
            .take(profile.max_background_placements)
            .collect(),
    }
}

fn object_priority(position: &BasicObjectPosition) -> i32 {
    match position.object_code.as_str() {
        "sun" | "moon" => 100,
        "mercury" | "venus" | "mars" => 80,
        "jupiter" | "saturn" => 70,
        _ => 50,
    }
}

fn importance_for_object(code: &str, reading_codes: &HashSet<String>) -> &'static str {
    if reading_codes.contains(code) || matches!(code, "sun" | "moon") {
        "high"
    } else {
        "moderate"
    }
}

fn build_angles(payload: &BasicPayload, profile: &LlmProjectionProfile) -> LlmAngles {
    let mut angles = LlmAngles::default();
    for angle in &payload.angles {
        let keywords = payload
            .positions
            .iter()
            .find(|p| p.object_code == angle.angle_code)
            .map(|p| limited_keywords(p, profile.max_keywords_per_item))
            .unwrap_or_default();
        let entry = LlmAngleEntry {
            sign: angle.sign_name.clone(),
            house: angle.house_number,
            keywords,
        };
        match angle.angle_code.as_str() {
            "ascendant" => angles.ascendant = Some(entry),
            "mc" => angles.midheaven = Some(entry),
            "descendant" => angles.descendant = Some(entry),
            "ic" => angles.imum_coeli = Some(entry),
            _ => {}
        }
    }
    angles
}

fn build_strengths(payload: &BasicPayload, profile: &LlmProjectionProfile) -> LlmStrengths {
    let mut dignity_rows: Vec<_> = payload.dignities.iter().collect();
    dignity_rows.sort_by(|a, b| {
        b.strength_score
            .partial_cmp(&a.strength_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let essential_dignities = dignity_rows
        .into_iter()
        .take(payload.dignities.len().max(1))
        .map(|d| LlmEssentialDignity {
            object: d.object_name.clone(),
            dignity: d.dignity_label.clone(),
            sign: d.sign_name.clone(),
            meaning: dignity_meaning(&d.dignity_type).to_string(),
            strength_score: profile.include_scores.then_some(d.strength_score),
        })
        .collect();

    let accidental_conditions = if profile.include_accidental_conditions {
        payload
            .accidental_dignities
            .iter()
            .map(|entry| accidental_to_llm(entry, payload, profile))
            .collect()
    } else {
        Vec::new()
    };

    LlmStrengths {
        essential_dignities,
        accidental_conditions,
    }
}

fn accidental_to_llm(
    entry: &BasicAccidentalDignityEvaluation,
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
) -> LlmAccidentalCondition {
    LlmAccidentalCondition {
        object: entry.object_name.clone(),
        overall: accidental_overall_label(&entry.expression_quality, &entry.overall_polarity),
        conditions: {
            let mut out = Vec::new();
            for condition in entry.conditions.iter() {
                push_unique(
                    &mut out,
                    humanize_condition(&condition.condition_code, chart_sect(payload)),
                );
                if out.len() >= profile.max_accidental_conditions_per_object {
                    break;
                }
            }
            out
        },
        overall_score: profile.include_scores.then_some(entry.overall_score),
    }
}

fn build_relationship_network(
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
    object_names: &HashMap<String, String>,
) -> LlmRelationshipNetwork {
    if !profile.include_rulership_details {
        return LlmRelationshipNetwork::default();
    }

    let ascendant_ruler = payload.rulership_context.ascendant_ruler.as_ref().map(|r| {
        let mut traditional = None;
        let mut modern = None;
        for source in &r.ruler_sources {
            if source.astral_system_code == "traditional" {
                traditional = Some(
                    object_names
                        .get(&source.object_code)
                        .cloned()
                        .unwrap_or_else(|| title_case_sign(&source.object_code)),
                );
            }
            if source.astral_system_code == "modern" {
                modern = Some(
                    object_names
                        .get(&source.object_code)
                        .cloned()
                        .unwrap_or_else(|| title_case_sign(&source.object_code)),
                );
            }
        }
        if traditional.is_none() {
            traditional = Some(
                object_names
                    .get(&r.ruler_object_code)
                    .cloned()
                    .unwrap_or_else(|| title_case_sign(&r.ruler_object_code)),
            );
        }
        LlmAscendantRulerNetwork {
            ascendant_sign: title_case_sign(&r.sign_code),
            traditional_ruler: traditional,
            modern_ruler: modern,
            main_ruler_placement: ruler_placement_text(payload, &r.ruler_object_code),
        }
    });

    let midheaven_ruler = payload.rulership_context.mc_ruler.as_ref().map(|r| LlmMcRulerNetwork {
        midheaven_sign: title_case_sign(&r.sign_code),
        ruler: object_names
            .get(&r.ruler_object_code)
            .cloned()
            .unwrap_or_else(|| title_case_sign(&r.ruler_object_code)),
        ruler_placement: ruler_placement_text(payload, &r.ruler_object_code),
    });

    let final_dispositors = payload
        .rulership_context
        .final_dispositors
        .iter()
        .map(|d| LlmFinalDispositor {
            object: object_names
                .get(&d.object_code)
                .cloned()
                .unwrap_or_else(|| title_case_sign(&d.object_code)),
            source_objects: d
                .source_objects
                .iter()
                .map(|code| {
                    object_names
                        .get(code)
                        .cloned()
                        .unwrap_or_else(|| title_case_sign(code))
                })
                .collect(),
        })
        .collect();

    let mutual_receptions = payload
        .rulership_context
        .mutual_receptions
        .iter()
        .map(|m| {
            let objects: Vec<String> = m
                .object_codes
                .iter()
                .map(|code| {
                    object_names
                        .get(code)
                        .cloned()
                        .unwrap_or_else(|| title_case_sign(code))
                })
                .collect();
            let source_objects = m
                .source_objects
                .iter()
                .map(|code| {
                    object_names
                        .get(code)
                        .cloned()
                        .unwrap_or_else(|| title_case_sign(code))
                })
                .collect();
            LlmMutualReception {
                objects,
                source_objects,
            }
        })
        .collect();

    LlmRelationshipNetwork {
        ascendant_ruler,
        midheaven_ruler,
        final_dispositors,
        mutual_receptions,
    }
}

fn ruler_placement_text(payload: &BasicPayload, ruler_code: &str) -> String {
    let Some(position) = payload.positions.iter().find(|p| p.object_code == ruler_code) else {
        return title_case_sign(ruler_code);
    };
    format!(
        "{} in {}, house {}",
        position.object_name,
        position.sign_name,
        position.house_number.unwrap_or_default()
    )
}

fn build_house_axes(
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
    axis_refs: &[HouseAxisReference],
) -> Vec<LlmHouseAxis> {
    let object_names = object_name_map(payload);
    payload
        .house_axis_emphasis
        .iter()
        .take(profile.max_house_axes)
        .map(|axis| house_axis_to_llm(axis, axis_refs, payload, profile, &object_names))
        .collect()
}

fn house_axis_to_llm(
    axis: &BasicHouseAxisEmphasis,
    axis_refs: &[HouseAxisReference],
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
    object_names: &HashMap<String, String>,
) -> LlmHouseAxis {
    let axis_title = house_axis_label(&axis.axis_code, axis_refs);
    let houses: Vec<LlmHouseRef> = axis
        .house_scores
        .iter()
        .map(|score| house_ref_from_payload(score.house_number, &score.theme_code, payload))
        .collect();

    let supporting_factors =
        dedupe_humanized_reasons(&axis.reasons, object_names, profile.max_keywords_per_item);

    let theme_in_parens: Vec<(String, String)> = axis
        .house_scores
        .iter()
        .map(|score| {
            let label = house_ref_from_payload(score.house_number, &score.theme_code, payload).theme;
            (score.theme_code.clone(), label)
        })
        .collect();
    let summary = humanize_axis_summary(&axis.interpretive_hint, &theme_in_parens);

    LlmHouseAxis {
        axis: axis_title,
        houses,
        balance: axis_balance_label(&axis.polarity_balance, axis.primary_house, axis.secondary_house),
        importance: axis_importance(axis.axis_score).to_string(),
        summary,
        supporting_factors,
    }
}

fn build_keywords(
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
    dynamics: &LlmDynamics,
) -> LlmKeywords {
    let mut main = Vec::new();
    let mut by_area: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let allow_technical = profile.level_code == "expert";

    for position in &payload.positions {
        if angle_codes().contains(position.object_code.as_str()) {
            continue;
        }
        let area = readable_area_key(position);
        let kws: Vec<String> = limited_keywords(position, profile.max_keywords_per_item)
            .into_iter()
            .filter(|kw| allow_technical || !is_placement_technical_keyword(kw))
            .collect();
        for kw in &kws {
            if main.len() < profile.max_keywords_per_item * 2 {
                push_unique(&mut main, kw.clone());
            }
        }
        let area_entry = by_area.entry(area).or_default();
        for kw in kws {
            if area_entry.len() < profile.max_keywords_per_item {
                push_unique(area_entry, kw);
            }
        }
    }

    if let Some(sign) = payload.chart_emphasis.dominant_signs.first() {
        push_unique(&mut main, title_case_sign(&sign.sign_code));
    }
    if let Some(house) = payload.chart_emphasis.dominant_houses.first() {
        let theme = house_ref_from_payload(house.house_number, &house.theme_code, payload);
        push_unique(&mut main, theme.theme.to_lowercase());
    }

    let mut dynamics_kws = Vec::new();
    for aspect in &dynamics.major_aspects {
        push_unique(&mut dynamics_kws, aspect.aspect.clone());
        push_unique(&mut dynamics_kws, aspect.quality.to_lowercase());
        for kw in &aspect.keywords {
            push_unique(&mut dynamics_kws, kw.clone());
        }
    }
    if !dynamics_kws.is_empty() {
        by_area.insert(
            "dynamics".to_string(),
            limit_keywords(&dynamics_kws, profile.max_keywords_per_item),
        );
    }

    LlmKeywords {
        main: limit_keywords(&main, profile.max_keywords_per_item * 2),
        by_area,
    }
}

fn readable_area_key(position: &BasicObjectPosition) -> String {
    position
        .house_context
        .as_ref()
        .and_then(|ctx| ctx.get("theme_code"))
        .and_then(|v| v.as_str())
        .map(|code| match code {
            "identity" => "identity",
            "resources" => "resources",
            "communication" => "communication",
            "home" | "roots" => "roots",
            "partnership" => "partnership",
            other => other,
        })
        .unwrap_or("general")
        .to_string()
}

fn is_placement_technical_keyword(kw: &str) -> bool {
    let lower = kw.to_ascii_lowercase();
    lower.contains("cadent")
        || lower.contains("succedent")
        || lower.contains("sect")
        || lower == "angular"
}

fn angle_codes() -> HashSet<&'static str> {
    ["ascendant", "descendant", "mc", "ic"].into_iter().collect()
}

fn placement_from_position(position: &BasicObjectPosition, include_degrees: bool) -> LlmPlacement {
    LlmPlacement {
        object: position.object_name.clone(),
        sign: position.sign_name.clone(),
        house: position
            .house_number
            .map(|n| LlmHouseRef {
                number: n,
                theme: position.house_name.clone().unwrap_or_else(|| title_case_sign(house_theme_code(position))),
            }),
        motion: motion_label(position),
        keywords: Vec::new(),
        conditions: Vec::new(),
        importance: None,
        longitude_deg: include_degrees.then_some(position.longitude_deg),
    }
}

fn house_ref_from_payload(house_number: i32, theme_code: &str, payload: &BasicPayload) -> LlmHouseRef {
    let theme = payload
        .positions
        .iter()
        .find(|pos| pos.house_number == Some(house_number))
        .and_then(|p| p.house_name.clone())
        .unwrap_or_else(|| title_case_sign(theme_code));
    LlmHouseRef {
        number: house_number,
        theme,
    }
}

fn house_theme_code(position: &BasicObjectPosition) -> &str {
    position
        .house_context
        .as_ref()
        .and_then(|ctx| ctx.get("theme_code"))
        .and_then(|v| v.as_str())
        .unwrap_or("general")
}

fn limited_keywords(position: &BasicObjectPosition, limit: usize) -> Vec<String> {
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

fn position_conditions(
    position: &BasicObjectPosition,
    chart_sect: Option<&str>,
    profile: &LlmProjectionProfile,
) -> Vec<String> {
    let motion = motion_label(position);
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
        let label = if horizon.contains('_') {
            humanize_condition(horizon, chart_sect)
        } else {
            title_case_sign(horizon)
        };
        if !is_unremarkable_motion_condition(&label, motion.as_deref()) {
            push_unique(&mut out, label);
        }
    }
    for summary in &position.accidental_dignity_context {
        let label = humanize_condition(&summary.condition_code, chart_sect);
        if !is_unremarkable_motion_condition(&label, motion.as_deref()) {
            push_unique(&mut out, label);
        }
        if out.len() >= profile.max_accidental_conditions_per_object {
            break;
        }
    }
    out
}

fn dedupe_humanized_reasons(
    reasons: &[String],
    object_names: &HashMap<String, String>,
    limit: usize,
) -> Vec<String> {
    let mut out = Vec::new();
    for reason in reasons {
        let human = humanize_reason(reason, object_names);
        push_unique(&mut out, human);
        if out.len() >= limit {
            break;
        }
    }
    out
}

fn motion_label(position: &BasicObjectPosition) -> Option<String> {
    position
        .motion_context
        .as_ref()
        .and_then(|ctx| ctx.get("label"))
        .and_then(|v| v.as_str())
        .map(humanize_motion_label)
}
