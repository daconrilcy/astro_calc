use std::collections::HashMap;
use std::sync::OnceLock;

use tokio::runtime::Handle;

use crate::infra::db::reference_repository::ReferenceRepository;

fn zodiac_key_by_id() -> &'static HashMap<i32, String> {
    static MAP: OnceLock<HashMap<i32, String>> = OnceLock::new();
    MAP.get_or_init(load_zodiac_key_by_id)
}

fn coordinate_key_by_id() -> &'static HashMap<i32, String> {
    static MAP: OnceLock<HashMap<i32, String>> = OnceLock::new();
    MAP.get_or_init(load_coordinate_key_by_id)
}

fn house_code_by_id() -> &'static HashMap<i32, String> {
    static MAP: OnceLock<HashMap<i32, String>> = OnceLock::new();
    MAP.get_or_init(load_house_code_by_id)
}

fn load_repository() -> ReferenceRepository {
    let pool = run_blocking(crate::bootstrap::db::connect_from_env())
        .expect("database must be reachable for engine refs");
    ReferenceRepository::new(pool)
}

fn run_blocking<F, T>(future: F) -> Result<T, sqlx::Error>
where
    F: std::future::Future<Output = Result<T, sqlx::Error>>,
{
    if let Ok(handle) = Handle::try_current() {
        handle.block_on(future)
    } else {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime")
            .block_on(future)
    }
}

fn load_zodiac_key_by_id() -> HashMap<i32, String> {
    let repository = load_repository();
    Handle::current()
        .block_on(async move {
            repository
                .zodiacal_reference_systems()
                .await
                .map(|rows| rows.into_iter().map(|row| (row.id, row.key)).collect())
        })
        .expect("astral_zodiacal_reference_systems must load")
}

fn load_coordinate_key_by_id() -> HashMap<i32, String> {
    let repository = load_repository();
    Handle::current()
        .block_on(async move {
            repository
                .coordinate_reference_systems()
                .await
                .map(|rows| rows.into_iter().map(|row| (row.id, row.key)).collect())
        })
        .expect("astral_coordinate_reference_systems must load")
}

fn load_house_code_by_id() -> HashMap<i32, String> {
    let repository = load_repository();
    Handle::current()
        .block_on(async move {
            repository
                .house_systems()
                .await
                .map(|rows| rows.into_iter().map(|row| (row.id, row.code)).collect())
        })
        .expect("astral_house_systems must load")
}

pub fn zodiacal_reference_system_key_from_env() -> Result<String, String> {
    reference_key_from_env(
        "ASTRAL_ZODIACAL_REFERENCE_SYSTEM",
        "ASTRAL_ZODIACAL_REFERENCE_SYSTEM_ID",
        zodiac_key_by_id(),
        "tropical",
    )
}

pub fn coordinate_reference_system_key_from_env() -> Result<String, String> {
    reference_key_from_env(
        "ASTRAL_COORDINATE_REFERENCE_SYSTEM",
        "ASTRAL_COORDINATE_REFERENCE_SYSTEM_ID",
        coordinate_key_by_id(),
        "geocentric",
    )
}

pub fn house_system_code_from_env() -> Result<String, String> {
    reference_key_from_env(
        "ASTRAL_HOUSE_SYSTEM",
        "ASTRAL_HOUSE_SYSTEM_ID",
        house_code_by_id(),
        "placidus",
    )
}

fn reference_key_from_env(
    key_var: &str,
    id_var: &str,
    id_map: &HashMap<i32, String>,
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

fn zodiac_id_by_key() -> &'static HashMap<String, i32> {
    static MAP: OnceLock<HashMap<String, i32>> = OnceLock::new();
    MAP.get_or_init(|| invert_map(zodiac_key_by_id()))
}

fn coordinate_id_by_key() -> &'static HashMap<String, i32> {
    static MAP: OnceLock<HashMap<String, i32>> = OnceLock::new();
    MAP.get_or_init(|| invert_map(coordinate_key_by_id()))
}

fn house_id_by_code() -> &'static HashMap<String, i32> {
    static MAP: OnceLock<HashMap<String, i32>> = OnceLock::new();
    MAP.get_or_init(|| invert_map(house_code_by_id()))
}

fn invert_map(id_to_key: &HashMap<i32, String>) -> HashMap<String, i32> {
    id_to_key
        .iter()
        .map(|(id, key)| (key.clone(), *id))
        .collect()
}

pub fn zodiacal_reference_system_id_from_env() -> Result<i32, String> {
    reference_id_from_env(
        "ASTRAL_ZODIACAL_REFERENCE_SYSTEM",
        "ASTRAL_ZODIACAL_REFERENCE_SYSTEM_ID",
        zodiac_id_by_key(),
        1,
    )
}

pub fn coordinate_reference_system_id_from_env() -> Result<i32, String> {
    reference_id_from_env(
        "ASTRAL_COORDINATE_REFERENCE_SYSTEM",
        "ASTRAL_COORDINATE_REFERENCE_SYSTEM_ID",
        coordinate_id_by_key(),
        1,
    )
}

pub fn house_system_id_from_env() -> Result<i32, String> {
    reference_id_from_env(
        "ASTRAL_HOUSE_SYSTEM",
        "ASTRAL_HOUSE_SYSTEM_ID",
        house_id_by_code(),
        1,
    )
}

fn reference_id_from_env(
    key_var: &str,
    id_var: &str,
    key_map: &HashMap<String, i32>,
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
