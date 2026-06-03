use std::collections::BTreeSet;
use std::fs;

use jsonschema::JSONSchema;
use serde_json::{json, Value};

use rust_sqlx_connection_test::domain::BasicPayload;
use rust_sqlx_connection_test::llm_projection::{
    build_llm_projection_natal_v1, profile_from_level, LlmProjectionBuildContext,
};

const V13_GOLDEN: &str = "../tests/golden/natal_payload_v13_paris_1990.json";
const LLM_SCHEMA: &str = "schemas/llm_projection_natal_v1.schema.json";
const REQUEST_SCHEMA: &str = "schemas/astro_engine_request_v1.schema.json";
const RESPONSE_SCHEMA: &str = "schemas/astro_engine_response_v1.schema.json";

const LLM_GOLDEN_COMPACT: &str = "../tests/golden/llm_projection_natal_v1_paris_1990_compact.json";
const LLM_GOLDEN_STANDARD: &str = "../tests/golden/llm_projection_natal_v1_paris_1990_standard.json";
const LLM_GOLDEN_RICH: &str = "../tests/golden/llm_projection_natal_v1_paris_1990_rich.json";

fn load_v13_golden() -> BasicPayload {
    let raw = fs::read_to_string(V13_GOLDEN).expect("v13 golden");
    serde_json::from_str(&raw).expect("v13 golden json")
}

fn validate_schema(value: &Value, schema_path: &str) -> Vec<String> {
    let schema_raw = fs::read_to_string(schema_path).expect("schema file");
    let schema_json: Value = serde_json::from_str(&schema_raw).expect("schema json");
    let compiled = JSONSchema::options()
        .compile(&schema_json)
        .expect("compile schema");
    compiled
        .validate(value)
        .err()
        .map(|errors| errors.map(|e| e.to_string()).collect())
        .unwrap_or_default()
}

fn projection_context() -> LlmProjectionBuildContext<'static> {
    LlmProjectionBuildContext {
        birth_location_label: "Paris, France",
        zodiac_label: "Tropical",
        coordinate_label: "Geocentric",
        house_system_label: "Placidus",
        house_axes: &[],
    }
}

fn build_level(level: &str) -> Value {
    let payload = load_v13_golden();
    let profile = profile_from_level(level).expect("profile");
    let projection = build_llm_projection_natal_v1(&payload, &profile, &projection_context());
    serde_json::to_value(projection).expect("projection json")
}

fn top_level_keys(value: &Value) -> BTreeSet<String> {
    value
        .as_object()
        .expect("object")
        .keys()
        .cloned()
        .collect()
}

fn assert_compact_profile_rules(value: &Value) {
    assert!(
        value["core_identity"]["ascendant"]
            .get("ruler")
            .is_none_or(|r| r.is_null())
    );
    assert_eq!(
        value["relationship_network"].as_object().unwrap().len(),
        0
    );
    assert_eq!(
        value["strengths"]["accidental_conditions"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert!(!value.to_string().contains("overall_score"));
}

fn assert_no_technical_ids(value: &Value) {
    let forbidden_suffixes = ["_id", "_code", "signal_key", "chart_object_id"];
    fn walk(node: &Value, path: &str, forbidden_suffixes: &[&str]) {
        match node {
            Value::Object(map) => {
                for (key, child) in map {
                    let child_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    for suffix in forbidden_suffixes {
                        if key.ends_with(suffix) || key.contains("signal_key") {
                            panic!("forbidden technical key {child_path}");
                        }
                    }
                    walk(child, &child_path, forbidden_suffixes);
                }
            }
            Value::Array(items) => {
                for (index, child) in items.iter().enumerate() {
                    walk(child, &format!("{path}[{index}]"), forbidden_suffixes);
                }
            }
            _ => {}
        }
    }
    walk(value, "", &forbidden_suffixes);
}

#[test]
fn sample_engine_request_matches_schema() {
    let request = json!({
        "request_contract_version": "astro_engine_request_v1",
        "request_id": "req_20260603_001",
        "calculation": {
            "type": "natal",
            "zodiacal_reference_system": "tropical",
            "coordinate_reference_system": "geocentric",
            "house_system": "placidus"
        },
        "birth": {
            "date": "1990-01-02",
            "time": "03:04:05",
            "timezone": "UTC",
            "location": {
                "label": "Paris, France",
                "latitude": 48.8566,
                "longitude": 2.3522,
                "country_code": "FR"
            },
            "time_precision": "exact"
        },
        "projection": {
            "contract_version": "llm_projection_natal_v1",
            "level": "rich"
        }
    });
    let errors = validate_schema(&request, REQUEST_SCHEMA);
    assert!(errors.is_empty(), "request schema errors:\n{}", errors.join("\n"));
}

#[test]
fn llm_projection_levels_share_identical_structure() {
    let compact = build_level("compact");
    let standard = build_level("standard");
    let rich = build_level("rich");

    assert_eq!(
        top_level_keys(&compact),
        top_level_keys(&standard),
        "compact vs standard top-level keys"
    );
    assert_eq!(
        top_level_keys(&standard),
        top_level_keys(&rich),
        "standard vs rich top-level keys"
    );

    for level in [("compact", &compact), ("standard", &standard), ("rich", &rich)] {
        let errors = validate_schema(level.1, LLM_SCHEMA);
        assert!(
            errors.is_empty(),
            "{} llm schema errors:\n{}",
            level.0,
            errors.join("\n")
        );
        assert_no_technical_ids(level.1);
        assert_eq!(
            level.1["contract_version"], "llm_projection_natal_v1",
            "{} contract_version",
            level.0
        );
        assert_eq!(level.1["projection_level"], level.0);
        if level.0 == "compact" {
            assert_compact_profile_rules(level.1);
        }
    }

    assert!(
        compact["placements"]["supporting"]
            .as_array()
            .unwrap()
            .len()
            <= standard["placements"]["supporting"]
                .as_array()
                .unwrap()
                .len()
    );
    assert!(
        standard["placements"]["supporting"]
            .as_array()
            .unwrap()
            .len()
            <= rich["placements"]["supporting"].as_array().unwrap().len()
    );
}

#[test]
fn llm_projection_golden_compact_standard_rich() {
    let cases = [
        ("compact", LLM_GOLDEN_COMPACT),
        ("standard", LLM_GOLDEN_STANDARD),
        ("rich", LLM_GOLDEN_RICH),
    ];

    for (level, path) in cases {
        let generated = build_level(level);
        let golden_raw = fs::read_to_string(path).unwrap_or_else(|_| {
            panic!(
                "missing golden {path}; run UPDATE_LLM_GOLDENS=1 cargo test --test engine_contract_tests llm_projection_golden_compact_standard_rich"
            )
        });
        let golden: Value = serde_json::from_str(&golden_raw).expect("golden json");
        assert_eq!(generated, golden, "llm projection {level} differs from {path}");
    }
}

#[test]
fn write_llm_projection_goldens_when_env_set() {
    if std::env::var("UPDATE_LLM_GOLDENS").ok().as_deref() != Some("1") {
        return;
    }
    for (level, path) in [
        ("compact", LLM_GOLDEN_COMPACT),
        ("standard", LLM_GOLDEN_STANDARD),
        ("rich", LLM_GOLDEN_RICH),
    ] {
        let json = serde_json::to_string_pretty(&build_level(level)).expect("serialize");
        fs::write(path, format!("{json}\n")).expect("write golden");
    }
}

#[test]
fn engine_response_envelope_shape_from_v13_golden() {
    let payload = load_v13_golden();
    let profile = profile_from_level("rich").expect("profile");
    let llm = build_llm_projection_natal_v1(&payload, &profile, &projection_context());
    let llm_value = serde_json::to_value(&llm).expect("llm json");

    let response = json!({
        "response_contract_version": "astro_engine_response_v1",
        "request_echo": {
            "calculation_type": "natal",
            "birth_datetime_local": "1990-01-02T03:04:05",
            "birth_timezone": "UTC",
            "birth_datetime_utc": payload.birth_datetime_utc.to_rfc3339(),
            "location": {
                "label": "Paris, France",
                "latitude": 48.8566,
                "longitude": 2.3522
            },
            "projection_level": "rich"
        },
        "calculation_result": {
            "status": "completed",
            "chart_calculation_id": payload.chart_calculation_id,
            "engine_version": "test",
            "ephemeris_version": "test",
            "raw_payload_contract_version": "natal_structured_v13",
            "llm_projection_contract_version": "llm_projection_natal_v1"
        },
        "audit_payload": {
            "contract_version": "natal_structured_v13",
            "payload": payload
        },
        "llm_payload": llm_value
    });

    let errors = validate_schema(&response, RESPONSE_SCHEMA);
    assert!(
        errors.is_empty(),
        "response schema errors:\n{}",
        errors.join("\n")
    );

    let llm_errors = validate_schema(&llm_value, LLM_SCHEMA);
    assert!(
        llm_errors.is_empty(),
        "llm_payload schema errors:\n{}",
        llm_errors.join("\n")
    );
}
