use std::collections::HashMap;
use std::sync::OnceLock;

use crate::infra::db::reference_repository::ReferenceRepository;

pub async fn zodiacal_reference_system_key_from_env(
    repository: &ReferenceRepository,
) -> Result<String, String> {
    reference_key_from_env(
        "ASTRAL_ZODIACAL_REFERENCE_SYSTEM",
        "ASTRAL_ZODIACAL_REFERENCE_SYSTEM_ID",
        zodiac_key_by_id(repository).await?,
        "tropical",
    )
}

pub async fn coordinate_reference_system_key_from_env(
    repository: &ReferenceRepository,
) -> Result<String, String> {
    reference_key_from_env(
        "ASTRAL_COORDINATE_REFERENCE_SYSTEM",
        "ASTRAL_COORDINATE_REFERENCE_SYSTEM_ID",
        coordinate_key_by_id(repository).await?,
        "geocentric",
    )
}

pub async fn house_system_code_from_env(repository: &ReferenceRepository) -> Result<String, String> {
    reference_key_from_env(
        "ASTRAL_HOUSE_SYSTEM",
        "ASTRAL_HOUSE_SYSTEM_ID",
        house_code_by_id(repository).await?,
        "placidus",
    )
}

fn reference_key_from_env(
    key_var: &str,
    id_var: &str,
    id_map: HashMap<i32, String>,
    default: &str,
) -> Result<String, String> {
    if let Ok(key) = std::env::var(key_var) {
        let trimmed = key.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    if let Ok(id_raw) = std::env::var(id_var) {
        let id: i32 = id_raw
            .parse()
            .map_err(|error| format!("{id_var} is invalid: {error}"))?;
        return id_map
            .get(&id)
            .cloned()
            .ok_or_else(|| format!("{id_var}={id} has no matching entry in database"));
    }

    Ok(default.to_string())
}

fn invert_map(id_to_key: &HashMap<i32, String>) -> HashMap<String, i32> {
    id_to_key
        .iter()
        .map(|(id, key)| (key.clone(), *id))
        .collect()
}

pub async fn zodiacal_reference_system_id_from_env(
    repository: &ReferenceRepository,
) -> Result<i32, String> {
    reference_id_from_env(
        "ASTRAL_ZODIACAL_REFERENCE_SYSTEM",
        "ASTRAL_ZODIACAL_REFERENCE_SYSTEM_ID",
        invert_map(&zodiac_key_by_id(repository).await?),
        1,
    )
}

pub async fn coordinate_reference_system_id_from_env(
    repository: &ReferenceRepository,
) -> Result<i32, String> {
    reference_id_from_env(
        "ASTRAL_COORDINATE_REFERENCE_SYSTEM",
        "ASTRAL_COORDINATE_REFERENCE_SYSTEM_ID",
        invert_map(&coordinate_key_by_id(repository).await?),
        1,
    )
}

pub async fn house_system_id_from_env(repository: &ReferenceRepository) -> Result<i32, String> {
    reference_id_from_env(
        "ASTRAL_HOUSE_SYSTEM",
        "ASTRAL_HOUSE_SYSTEM_ID",
        invert_map(&house_code_by_id(repository).await?),
        1,
    )
}

fn reference_id_from_env(
    key_var: &str,
    id_var: &str,
    key_map: HashMap<String, i32>,
    default_id: i32,
) -> Result<i32, String> {
    if let Ok(key) = std::env::var(key_var) {
        let trimmed = key.trim();
        if !trimmed.is_empty() {
            return key_map
                .get(trimmed)
                .copied()
                .ok_or_else(|| format!("{key_var}={trimmed} has no matching entry in database"));
        }
    }

    if let Ok(id_raw) = std::env::var(id_var) {
        let id: i32 = id_raw
            .parse()
            .map_err(|error| format!("{id_var} is invalid: {error}"))?;
        return Ok(id);
    }

    Ok(default_id)
}

async fn zodiac_key_by_id(repository: &ReferenceRepository) -> Result<HashMap<i32, String>, String> {
    static MAP: OnceLock<HashMap<i32, String>> = OnceLock::new();
    if let Some(map) = MAP.get() {
        return Ok(map.clone());
    }

    let rows = repository
        .zodiacal_reference_systems()
        .await
        .map_err(|err| err.to_string())?;
    let map: HashMap<i32, String> = rows.into_iter().map(|row| (row.id, row.key)).collect();
    let _ = MAP.set(map.clone());
    Ok(map)
}

async fn coordinate_key_by_id(
    repository: &ReferenceRepository,
) -> Result<HashMap<i32, String>, String> {
    static MAP: OnceLock<HashMap<i32, String>> = OnceLock::new();
    if let Some(map) = MAP.get() {
        return Ok(map.clone());
    }

    let rows = repository
        .coordinate_reference_systems()
        .await
        .map_err(|err| err.to_string())?;
    let map: HashMap<i32, String> = rows.into_iter().map(|row| (row.id, row.key)).collect();
    let _ = MAP.set(map.clone());
    Ok(map)
}

async fn house_code_by_id(repository: &ReferenceRepository) -> Result<HashMap<i32, String>, String> {
    static MAP: OnceLock<HashMap<i32, String>> = OnceLock::new();
    if let Some(map) = MAP.get() {
        return Ok(map.clone());
    }

    let rows = repository
        .house_systems()
        .await
        .map_err(|err| err.to_string())?;
    let map: HashMap<i32, String> = rows.into_iter().map(|row| (row.id, row.code)).collect();
    let _ = MAP.set(map.clone());
    Ok(map)
}
