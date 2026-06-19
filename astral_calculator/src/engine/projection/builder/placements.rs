use super::*;
use crate::shared::error::RuntimeError;

pub(super) fn build_placements(
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<LlmPlacementsGroup, RuntimeError> {
    let core_codes: HashSet<&str> = ["sun", "moon"].into_iter().collect();
    let angle_codes = angle_codes();
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
        let mut item = placement_from_position(position, profile.include_degrees, resolver)?;
        item.house = position
            .house_number
            .map(|n| house_ref_from_payload(n, house_theme_code(position), resolver))
            .transpose()?;
        item.keywords = limited_keywords(position, profile.max_keywords_per_item);
        item.conditions = position_conditions(position, chart_sect(payload), profile, resolver)?;
        item.importance =
            Some(importance_for_object(&position.object_code, &reading_codes).to_string());

        if core_codes.contains(position.object_code.as_str()) {
            continue;
        } else if supporting.len() < profile.max_supporting_placements {
            supporting.push(item);
        } else if background.len() < profile.max_background_placements {
            background.push(item);
        }
    }

    Ok(LlmPlacementsGroup {
        primary,
        supporting: supporting
            .into_iter()
            .take(profile.max_supporting_placements)
            .collect(),
        background: background
            .into_iter()
            .take(profile.max_background_placements)
            .collect(),
    })
}

pub(super) fn build_angles(payload: &BasicPayload, profile: &LlmProjectionProfile) -> LlmAngles {
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
