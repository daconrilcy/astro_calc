use std::collections::HashMap;
use std::sync::OnceLock;

use crate::domain::HouseAxisReference;

const AXIS_DEFINITIONS_JSON: &str =
    include_str!("../../../json_db/astral_house_axis_definitions.json");

fn seed_axis_labels() -> &'static HashMap<String, String> {
    static LABELS: OnceLock<HashMap<String, String>> = OnceLock::new();
    LABELS.get_or_init(|| {
        let table: serde_json::Value =
            serde_json::from_str(AXIS_DEFINITIONS_JSON).expect("house axis definitions json");
        table["data"]
            .as_array()
            .expect("axis definitions data")
            .iter()
            .map(|row| {
                (
                    row["key"].as_str().expect("axis key").to_string(),
                    row["title"].as_str().expect("axis title").to_string(),
                )
            })
            .collect()
    })
}

pub fn house_axis_label(axis_code: &str, refs: &[HouseAxisReference]) -> String {
    refs.iter()
        .find(|axis| axis.axis_code == axis_code)
        .map(|axis| axis.label.clone())
        .or_else(|| seed_axis_labels().get(axis_code).cloned())
        .unwrap_or_else(|| axis_code.replace('_', " "))
}
