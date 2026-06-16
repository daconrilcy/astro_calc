use super::types::{LlmEffectiveLimits, LlmProjectionLimitsEnvelope, LlmProjectionProfile};
use crate::repositories::RuntimeRepository;
use crate::runtime::RuntimeError;

pub fn profile_from_level(level: &str) -> Result<LlmProjectionProfile, RuntimeError> {
    let repository = load_repository()?;
    tokio::runtime::Handle::current()
        .block_on(repository.llm_projection_profile("llm_projection_natal_v1", level))
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
        Ok(profile) => Ok(profile),
        Err(error) => Err(error),
    }
}

fn load_repository() -> Result<RuntimeRepository, RuntimeError> {
    let pool = run_blocking(crate::db::connect_from_env()).map_err(RuntimeError::Database)?;
    Ok(RuntimeRepository::new(pool))
}

fn run_blocking<F, T>(future: F) -> Result<T, sqlx::Error>
where
    F: std::future::Future<Output = Result<T, sqlx::Error>>,
{
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.block_on(future)
    } else {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime")
            .block_on(future)
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
