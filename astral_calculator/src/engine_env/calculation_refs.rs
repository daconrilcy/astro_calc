use std::collections::HashMap;
use std::sync::OnceLock;

const ZODIAC_JSON: &str = include_str!("../../../json_db/astral_zodiacal_reference_systems.json");
const COORDINATE_JSON: &str =
    include_str!("../../../json_db/astral_coordinate_reference_systems.json");
const HOUSE_SYSTEM_JSON: &str = include_str!("../../../json_db/astral_house_systems.json");

fn zodiac_key_by_id() -> &'static HashMap<i32, String> {
    static MAP: OnceLock<HashMap<i32, String>> = OnceLock::new();
    MAP.get_or_init(|| id_key_map(ZODIAC_JSON, "key"))
}

fn coordinate_key_by_id() -> &'static HashMap<i32, String> {
    static MAP: OnceLock<HashMap<i32, String>> = OnceLock::new();
    MAP.get_or_init(|| id_key_map(COORDINATE_JSON, "key"))
}

fn house_code_by_id() -> &'static HashMap<i32, String> {
    static MAP: OnceLock<HashMap<i32, String>> = OnceLock::new();
    MAP.get_or_init(|| id_key_map(HOUSE_SYSTEM_JSON, "code"))
}

fn id_key_map(json: &str, key_field: &str) -> HashMap<i32, String> {
    key_by_id_map(json, key_field)
}

fn key_by_id_map(json: &str, key_field: &str) -> HashMap<i32, String> {
    let table: serde_json::Value = serde_json::from_str(json).expect("json_db table must parse");
    table["data"]
        .as_array()
        .expect("json_db data array")
        .iter()
        .map(|row| {
            let id = row["id"].as_i64().expect("row id") as i32;
            let key = row[key_field].as_str().expect("row key").to_string();
            (id, key)
        })
        .collect()
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
            .ok_or_else(|| format!("{id_var}={id} has no matching entry in json_db seed"));
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
            return key_map.get(trimmed).copied().ok_or_else(|| {
                format!("{key_var}={trimmed} has no matching entry in json_db seed")
            });
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
