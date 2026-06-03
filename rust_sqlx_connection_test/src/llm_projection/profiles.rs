use super::types::{LlmEffectiveLimits, LlmProjectionLimitsEnvelope, LlmProjectionProfile};
use crate::repositories::RuntimeRepository;
use crate::runtime::RuntimeError;

const PROFILES_JSON: &str = include_str!("../../../json_db/astral_llm_projection_profiles.json");

pub fn profile_from_level(level: &str) -> Result<LlmProjectionProfile, RuntimeError> {
    all_profiles_from_seed()
        .into_iter()
        .find(|profile| profile.level_code == level)
        .ok_or_else(|| RuntimeError::InvalidEngineRequest(format!("unknown projection level: {level}")))
}

pub fn all_profiles_from_seed() -> Vec<LlmProjectionProfile> {
    let table: serde_json::Value =
        serde_json::from_str(PROFILES_JSON).expect("astral_llm_projection_profiles.json must parse");
    table["data"]
        .as_array()
        .expect("profiles data array")
        .iter()
        .map(|row| LlmProjectionProfile {
            contract_version: row["contract_version"]
                .as_str()
                .expect("contract_version")
                .to_string(),
            level_code: row["level_code"].as_str().expect("level_code").to_string(),
            max_keywords_per_item: row["max_keywords_per_item"]
                .as_u64()
                .expect("max_keywords_per_item") as usize,
            max_core_placements: row["max_core_placements"]
                .as_u64()
                .expect("max_core_placements") as usize,
            max_supporting_placements: row["max_supporting_placements"]
                .as_u64()
                .expect("max_supporting_placements") as usize,
            max_dominant_signs: row["max_dominant_signs"]
                .as_u64()
                .expect("max_dominant_signs") as usize,
            max_dominant_houses: row["max_dominant_houses"]
                .as_u64()
                .expect("max_dominant_houses") as usize,
            max_dominant_objects: row["max_dominant_objects"]
                .as_u64()
                .expect("max_dominant_objects") as usize,
            max_house_axes: row["max_house_axes"]
                .as_u64()
                .expect("max_house_axes") as usize,
            max_aspects: row["max_aspects"].as_u64().expect("max_aspects") as usize,
            max_background_placements: row["max_background_placements"]
                .as_u64()
                .unwrap_or_else(|| {
                    default_max_background_placements_u64(
                        row["level_code"].as_str().expect("level_code"),
                    )
                }) as usize,
            max_accidental_conditions_per_object: row["max_accidental_conditions_per_object"]
                .as_u64()
                .unwrap_or_else(|| {
                    default_max_accidental_conditions_u64(
                        row["level_code"].as_str().expect("level_code"),
                    )
                }) as usize,
            include_accidental_conditions: row["include_accidental_conditions"]
                .as_bool()
                .expect("include_accidental_conditions"),
            include_rulership_details: row["include_rulership_details"]
                .as_bool()
                .expect("include_rulership_details"),
            include_minor_evidence: row["include_minor_evidence"]
                .as_bool()
                .expect("include_minor_evidence"),
            include_degrees: row["include_degrees"].as_bool().expect("include_degrees"),
            include_scores: row["include_scores"].as_bool().expect("include_scores"),
        })
        .collect()
}

pub async fn resolve_projection_profile(
    repository: &RuntimeRepository,
    contract_version: &str,
    level: &str,
) -> Result<LlmProjectionProfile, RuntimeError> {
    match repository
        .llm_projection_profile(contract_version, level)
        .await
    {
        Ok(profile) => Ok(merge_seed_limits(profile)),
        Err(RuntimeError::Database(error)) if missing_relation(&error, "astral_llm_projection_profiles") => {
            profile_from_level(level)
        }
        Err(RuntimeError::InvalidEngineRequest(message))
            if message.contains("unknown llm projection profile") =>
        {
            profile_from_level(level)
        }
        Err(error) => Err(error),
    }
}

pub fn default_max_background_placements(level: &str) -> usize {
    default_max_background_placements_u64(level) as usize
}

fn default_max_background_placements_u64(level: &str) -> u64 {
    match level {
        "compact" => 0,
        "standard" => 3,
        "rich" => 5,
        "expert" => 8,
        _ => 3,
    }
}

pub fn default_max_accidental_conditions(level: &str) -> usize {
    default_max_accidental_conditions_u64(level) as usize
}

fn default_max_accidental_conditions_u64(level: &str) -> u64 {
    match level {
        "compact" => 2,
        "standard" => 3,
        "rich" => 4,
        "expert" => 6,
        _ => 3,
    }
}

/// DB rows may predate seed fields (background limits, accidental caps). Overlay from seed.
fn merge_seed_limits(mut profile: LlmProjectionProfile) -> LlmProjectionProfile {
    let Ok(seed) = profile_from_level(&profile.level_code) else {
        return profile;
    };
    profile.max_background_placements = seed.max_background_placements;
    profile.max_accidental_conditions_per_object = seed.max_accidental_conditions_per_object;
    profile
}

fn missing_relation(error: &sqlx::Error, table: &str) -> bool {
    if let sqlx::Error::Database(db) = error {
        return db.code().as_deref() == Some("42P01")
            || db
                .message()
                .to_ascii_lowercase()
                .contains(&table.to_ascii_lowercase());
    }
    false
}

pub fn limits_envelope(profile: &LlmProjectionProfile) -> LlmProjectionLimitsEnvelope {
    LlmProjectionLimitsEnvelope {
        level: profile.level_code.clone(),
        effective_limits: LlmEffectiveLimits {
            max_keywords_per_item: profile.max_keywords_per_item,
            max_core_placements: profile.max_core_placements,
            max_supporting_placements: profile.max_supporting_placements,
            max_dominant_signs: profile.max_dominant_signs,
            max_dominant_houses: profile.max_dominant_houses,
            max_dominant_objects: profile.max_dominant_objects,
            max_house_axes: profile.max_house_axes,
            max_aspects: profile.max_aspects,
            max_background_placements: profile.max_background_placements,
            max_accidental_conditions_per_object: profile.max_accidental_conditions_per_object,
            include_rulership_context: profile.include_rulership_details,
            include_accidental_dignities: profile.include_accidental_conditions,
            include_minor_evidence: profile.include_minor_evidence,
            include_degrees: profile.include_degrees,
            include_scores: profile.include_scores,
        },
    }
}
