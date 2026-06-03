use rust_sqlx_connection_test::models::AspectDefinition;
use serde_json::Value;

const ASTRAL_ASPECTS_JSON: &str = include_str!("../../json_db/astral_aspects.json");
const ASTRAL_ASPECT_FAMILIES_JSON: &str = include_str!("../../json_db/astral_aspect_families.json");

pub fn astral_aspects_table() -> Value {
    serde_json::from_str(ASTRAL_ASPECTS_JSON).expect("json_db/astral_aspects.json must parse")
}

pub fn major_aspect_family_expected_count_from_json_db_seed() -> usize {
    let table: Value =
        serde_json::from_str(ASTRAL_ASPECT_FAMILIES_JSON).expect("astral_aspect_families.json");
    let row = table["data"]
        .as_array()
        .expect("astral_aspect_families.data")
        .iter()
        .find(|row| row["name"].as_str() == Some("major"))
        .expect("major family must exist in json_db seed");
    row["expected_aspect_count"]
        .as_u64()
        .expect("major family must define expected_aspect_count in json_db seed")
        as usize
}

pub fn major_aspect_family_max_default_orb_deg_from_json_db_seed() -> f64 {
    let table: Value =
        serde_json::from_str(ASTRAL_ASPECT_FAMILIES_JSON).expect("astral_aspect_families.json");
    let row = table["data"]
        .as_array()
        .expect("astral_aspect_families.data")
        .iter()
        .find(|row| row["name"].as_str() == Some("major"))
        .expect("major family must exist in json_db seed");
    row["max_default_orb_deg"]
        .as_f64()
        .expect("major family must define max_default_orb_deg in json_db seed")
}

pub fn major_aspect_definitions_from_json_db_seed() -> Vec<AspectDefinition> {
    let max_default_orb_deg = major_aspect_family_max_default_orb_deg_from_json_db_seed();
    let table = astral_aspects_table();
    let rows = table["data"]
        .as_array()
        .expect("astral_aspects.data must be an array");

    let mut majors: Vec<AspectDefinition> = rows
        .iter()
        .filter(|row| row["family"].as_str() == Some("major"))
        .map(|row| {
            let id = row["id"].as_i64().expect("major aspect id") as i32;
            let code = row["code"].as_str().expect("major aspect code").to_string();
            let name = row["name"].as_str().unwrap_or(code.as_str()).to_string();
            let angle = row["angle"].as_f64().expect("major aspect angle");
            let family = row["family"].as_str().expect("major aspect family").to_string();
            let default_orb_deg = row["default_orb_deg"]
                .as_f64()
                .filter(|orb| orb.is_finite() && *orb > 0.0);
            AspectDefinition {
                id,
                code,
                name,
                angle,
                family,
                default_orb_deg,
                max_default_orb_deg,
            }
        })
        .collect();
    majors.sort_by_key(|aspect| aspect.id);
    majors
}
