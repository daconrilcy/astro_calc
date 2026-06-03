use std::collections::{BTreeMap, HashMap, HashSet};

use crate::domain::{
    BasicAccidentalDignityEvaluation, BasicHouseAxisEmphasis, BasicObjectPosition,
    BasicPayload, BasicSignal, HouseAxisReference,
};
use crate::llm_projection::axis_labels::house_axis_label;
use crate::llm_projection::humanize::{
    accidental_overall_label, axis_balance_label, axis_importance, chart_sect_label,
    dignity_effect, hemisphere_dominant_area, humanize_condition_code, humanize_reason,
    push_unique, reading_slot_section, strength_label, title_case_sign,
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

    LlmProjectionNatalV1 {
        contract_version: "llm_projection_natal_v1".to_string(),
        projection_level: profile.level_code.clone(),
        projection_limits: limits,
        chart: build_chart(payload, ctx),
        reading_order: build_reading_order(payload, profile),
        core_identity: build_core_identity(payload, profile),
        dominant_themes: build_dominant_themes(payload, profile, &object_names),
        placements: build_placements(payload, profile, &object_names),
        angles: build_angles(payload, profile),
        strengths: build_strengths(payload, profile),
        relationship_network: build_relationship_network(payload, profile, &object_names),
        dynamics: build_dynamics(payload, profile),
        house_axes: build_house_axes(payload, profile, ctx.house_axes),
        keywords: build_keywords(payload, profile),
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

fn build_reading_order(payload: &BasicPayload, profile: &LlmProjectionProfile) -> Vec<LlmReadingOrderItem> {
    payload
        .reading_plan
        .iter()
        .map(|item| {
            let focus = item
                .primary_signal_keys
                .iter()
                .filter_map(|key| reading_focus_from_signal(payload, key))
                .take(profile.max_keywords_per_item)
                .collect();
            LlmReadingOrderItem {
                section: reading_slot_section(&item.slot, &item.title),
                focus,
            }
        })
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

fn build_core_identity(payload: &BasicPayload, profile: &LlmProjectionProfile) -> LlmCoreIdentity {
    LlmCoreIdentity {
        sun: mobile_body(payload, "sun", profile),
        moon: mobile_body(payload, "moon", profile),
        ascendant: build_ascendant_core(payload, profile),
    }
}

fn chart_sect<'a>(payload: &'a BasicPayload) -> Option<&'a str> {
    payload.chart_context.sect.chart_sect.as_deref()
}

fn mobile_body(payload: &BasicPayload, code: &str, profile: &LlmProjectionProfile) -> Option<LlmCoreBody> {
    let position = payload.positions.iter().find(|p| p.object_code == code)?;
    Some(LlmCoreBody {
        placement: placement_from_position(position, profile.include_degrees),
        keywords: limited_keywords(position, profile.max_keywords_per_item),
        conditions: position_conditions(position, chart_sect(payload)),
        importance: "high".to_string(),
    })
}

fn build_ascendant_core(payload: &BasicPayload, profile: &LlmProjectionProfile) -> Option<LlmAscendantBody> {
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
                rulers.traditional = Some(title_case_sign(&source.object_code));
            }
            if source.astral_system_code == "modern" {
                rulers.modern = Some(title_case_sign(&source.object_code));
            }
        }
        if rulers.traditional.is_none() {
            rulers.traditional = Some(title_case_sign(&r.ruler_object_code));
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
                sign: sign.clone(),
                strength: strength_label(entry.score).to_string(),
                reasons: entry
                    .reasons
                    .iter()
                    .take(profile.max_keywords_per_item)
                    .map(|r| humanize_reason(r, object_names))
                    .collect(),
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
        .map(|entry| LlmDominantHouse {
            house: house_ref_from_payload(entry.house_number, &entry.theme_code, payload),
            strength: strength_label(entry.score).to_string(),
            reasons: entry
                .reasons
                .iter()
                .take(profile.max_keywords_per_item)
                .map(|r| humanize_reason(r, object_names))
                .collect(),
            score: profile.include_scores.then_some(entry.score),
        })
        .collect();

    let objects = payload
        .chart_emphasis
        .dominant_objects
        .iter()
        .take(profile.max_dominant_objects)
        .map(|entry| LlmDominantObject {
            object: object_names
                .get(&entry.object_code)
                .cloned()
                .unwrap_or_else(|| title_case_sign(&entry.object_code)),
            strength: strength_label(entry.score).to_string(),
            reasons: entry
                .reasons
                .iter()
                .take(profile.max_keywords_per_item)
                .map(|r| humanize_reason(r, object_names))
                .collect(),
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
            if !out.contains(&kw) {
                out.push(kw);
            }
            if out.len() >= limit {
                return out;
            }
        }
    }
    out
}

fn build_placements(
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
    object_names: &HashMap<String, String>,
) -> LlmPlacementsGroup {
    let core_codes: HashSet<&str> = ["sun", "moon"].into_iter().collect();
    let angle_codes: HashSet<&str> = ["ascendant", "descendant", "mc", "ic"].into_iter().collect();
    let reading_codes: HashSet<String> = payload
        .reading_plan
        .iter()
        .flat_map(|item| item.primary_signal_keys.clone())
        .filter_map(|key| key.strip_prefix("object_position:").map(str::to_string))
        .collect();

    let mut primary = Vec::new();
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
        item.conditions = position_conditions(position, chart_sect(payload));
        item.importance = Some(importance_for_object(&position.object_code, &reading_codes).to_string());

        if core_codes.contains(position.object_code.as_str()) {
            primary.push(item);
        } else if reading_codes.contains(&position.object_code) && supporting.len() < profile.max_supporting_placements {
            supporting.push(item);
        } else if background.len() < profile.max_supporting_placements {
            background.push(item);
        } else if supporting.len() < profile.max_supporting_placements {
            supporting.push(item);
        }
        let _ = object_names;
    }

    LlmPlacementsGroup {
        primary,
        supporting: supporting
            .into_iter()
            .take(profile.max_supporting_placements)
            .collect(),
        background: background
            .into_iter()
            .take(profile.max_supporting_placements)
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
        .take(profile.max_dominant_objects.max(1))
        .map(|d| LlmEssentialDignity {
            object: d.object_name.clone(),
            dignity: d.dignity_label.clone(),
            sign: d.sign_name.clone(),
            effect: dignity_effect(&d.dignity_type).to_string(),
            strength_score: profile.include_scores.then_some(d.strength_score),
        })
        .collect();

    let accidental_conditions = if profile.include_accidental_conditions {
        payload
            .accidental_dignities
            .iter()
            .map(|entry| accidental_to_llm(entry, profile))
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
    profile: &LlmProjectionProfile,
) -> LlmAccidentalCondition {
    LlmAccidentalCondition {
        object: entry.object_name.clone(),
        overall: accidental_overall_label(&entry.expression_quality, &entry.overall_polarity),
        conditions: entry
            .conditions
            .iter()
            .map(|c| humanize_condition_code(&c.condition_code, None))
            .collect(),
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
        let modern = r
            .ruler_sources
            .iter()
            .find(|s| s.astral_system_code == "modern")
            .map(|s| title_case_sign(&s.object_code));
        LlmAscendantRulerNetwork {
            ascendant_sign: title_case_sign(&r.sign_code),
            main_ruler: title_case_sign(&r.ruler_object_code),
            modern_ruler: modern,
            ruler_placement: ruler_placement_text(payload, &r.ruler_object_code),
            meaning: r.interpretive_hint.clone(),
        }
    });

    let midheaven_ruler = payload.rulership_context.mc_ruler.as_ref().map(|r| LlmMcRulerNetwork {
        midheaven_sign: title_case_sign(&r.sign_code),
        ruler: title_case_sign(&r.ruler_object_code),
        ruler_placement: ruler_placement_text(payload, &r.ruler_object_code),
    });

    let dominant_dispositor = payload
        .rulership_context
        .final_dispositors
        .first()
        .map(|d| LlmDominantDispositor {
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
        });

    let mutual_receptions = payload
        .rulership_context
        .mutual_receptions
        .iter()
        .map(|m| LlmMutualReception {
            objects: m
                .object_codes
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

    LlmRelationshipNetwork {
        ascendant_ruler,
        midheaven_ruler,
        dominant_dispositor,
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

fn build_dynamics(payload: &BasicPayload, profile: &LlmProjectionProfile) -> LlmDynamics {
    let lunar_phase = payload.lunar_phase_context.as_ref().map(|phase| {
        LlmLunarPhase {
            phase: phase.phase_label.clone(),
            cycle: title_case_sign(&phase.cycle_family),
            sun_moon_angle_degrees: phase.sun_moon_angle_deg,
            keywords: phase
                .semantic_tags
                .iter()
                .take(profile.max_keywords_per_item)
                .map(|t| t.replace('_', " "))
                .collect(),
        }
    });

    let mut aspect_signals: Vec<&BasicSignal> = payload
        .signals
        .iter()
        .filter(|s| s.signal_key.starts_with("aspect:"))
        .collect();
    aspect_signals.sort_by(|a, b| {
        b.priority_score
            .partial_cmp(&a.priority_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let major_aspects = aspect_signals
        .into_iter()
        .take(profile.max_aspects)
        .filter_map(|signal| aspect_signal_to_llm(signal, profile.max_keywords_per_item))
        .collect();

    LlmDynamics {
        lunar_phase,
        major_aspects,
    }
}

fn aspect_signal_to_llm(signal: &BasicSignal, keyword_limit: usize) -> Option<LlmMajorAspect> {
    let ctx = signal.aspect_context.as_ref()?;
    let orb = ctx.get("orb_deg")?.as_f64()?;
    let phase = ctx
        .get("phase_state")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let quality = ctx
        .get("dynamic_quality")
        .and_then(|v| v.as_str())
        .map(title_case_sign)
        .unwrap_or_else(|| "Dynamic".to_string());
    let keywords = signal
        .semantic_tags
        .iter()
        .take(keyword_limit)
        .map(|t| t.replace('_', " "))
        .collect();

    Some(LlmMajorAspect {
        aspect: signal.title.clone(),
        quality,
        orb_degrees: orb,
        phase: title_case_sign(phase),
        keywords,
    })
}

fn build_house_axes(
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
    axis_refs: &[HouseAxisReference],
) -> Vec<LlmHouseAxis> {
    payload
        .house_axis_emphasis
        .iter()
        .take(profile.max_house_axes)
        .map(|axis| house_axis_to_llm(axis, axis_refs, payload))
        .collect()
}

fn house_axis_to_llm(
    axis: &BasicHouseAxisEmphasis,
    axis_refs: &[HouseAxisReference],
    payload: &BasicPayload,
) -> LlmHouseAxis {
    let axis_title = house_axis_label(&axis.axis_code, axis_refs);
    let houses: Vec<LlmHouseRef> = axis
        .house_scores
        .iter()
        .map(|score| house_ref_from_payload(score.house_number, &score.theme_code, payload))
        .collect();

    LlmHouseAxis {
        axis: axis_title,
        houses,
        balance: axis_balance_label(&axis.polarity_balance, axis.primary_house, axis.secondary_house),
        importance: axis_importance(axis.axis_score).to_string(),
        summary: axis.interpretive_hint.clone(),
    }
}

fn build_keywords(payload: &BasicPayload, profile: &LlmProjectionProfile) -> LlmKeywords {
    let mut main = Vec::new();
    let mut by_area: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for position in &payload.positions {
        if angle_codes().contains(position.object_code.as_str()) {
            continue;
        }
        let area = position
            .house_context
            .as_ref()
            .and_then(|ctx| ctx.get("theme_code"))
            .and_then(|v| v.as_str())
            .unwrap_or("general")
            .to_string();
        let kws = limited_keywords(position, profile.max_keywords_per_item);
        for kw in &kws {
            if main.len() < profile.max_keywords_per_item * 2 && !main.contains(kw) {
                main.push(kw.clone());
            }
        }
        let area_entry = by_area.entry(area).or_default();
        for kw in kws {
            if area_entry.len() < profile.max_keywords_per_item && !area_entry.contains(&kw) {
                area_entry.push(kw);
            }
        }
    }

    LlmKeywords { main, by_area }
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
    position
        .sign_context
        .as_ref()
        .and_then(|ctx| ctx.get("keywords"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .take(limit)
                .collect()
        })
        .unwrap_or_default()
}

fn position_conditions(position: &BasicObjectPosition, chart_sect: Option<&str>) -> Vec<String> {
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
            humanize_condition_code(horizon, chart_sect)
        } else {
            title_case_sign(horizon)
        };
        push_unique(&mut out, label);
    }
    if let Some(motion) = position.motion_context.as_ref() {
        if let Some(label) = motion.get("label").and_then(|v| v.as_str()) {
            push_unique(&mut out, label.to_string());
        }
    }
    for summary in &position.accidental_dignity_context {
        push_unique(
            &mut out,
            humanize_condition_code(&summary.condition_code, chart_sect),
        );
    }
    out
}

fn motion_label(position: &BasicObjectPosition) -> Option<String> {
    position
        .motion_context
        .as_ref()
        .and_then(|ctx| ctx.get("label"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
}
