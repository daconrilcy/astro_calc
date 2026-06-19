use std::collections::BTreeMap;

use super::*;
use crate::shared::error::RuntimeError;

pub(super) fn build_keywords(
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
    dynamics: &LlmDynamics,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<LlmKeywords, RuntimeError> {
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
        push_unique(
            &mut main,
            super::super::humanize::title_case_sign(&sign.sign_code),
        );
    }
    if let Some(house) = payload.chart_emphasis.dominant_houses.first() {
        let theme = house_ref_from_payload(house.house_number, &house.theme_code, resolver)?;
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

    Ok(LlmKeywords {
        main: limit_keywords(&main, profile.max_keywords_per_item * 2),
        by_area,
    })
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
