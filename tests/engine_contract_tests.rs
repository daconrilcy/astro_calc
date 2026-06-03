use std::collections::BTreeSet;
use std::fs;

use jsonschema::JSONSchema;
use serde_json::{json, Value};

use astral_calculator::domain::{BasicPayload, RuntimeOptions};
use astral_calculator::engine::{
    build_engine_response, ResolvedEngineRequest, RESPONSE_CONTRACT_VERSION,
};
use astral_calculator::llm_projection::{
    build_llm_projection_natal_v1, is_active_major_aspect_signal, profile_from_level,
    LlmProjectionBuildContext,
};

const V13_GOLDEN: &str = "../tests/golden/natal_payload_v13_paris_1990.json";
const LLM_SCHEMA: &str = "schemas/llm_projection_natal_v1.schema.json";
const REQUEST_SCHEMA: &str = "schemas/astro_engine_request_v1.schema.json";
const RESPONSE_SCHEMA: &str = "schemas/astro_engine_response_v1.schema.json";
const ENGINE_RESPONSE_GOLDEN_RICH: &str =
    "../tests/golden/astro_engine_response_v1_paris_1990_rich.json";

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

fn sample_resolved(level: &str, payload: &BasicPayload) -> ResolvedEngineRequest {
    ResolvedEngineRequest {
        natal_input: astral_calculator::domain::NatalChartInput {
            subject_label: Some("Test".to_string()),
            birth_datetime_utc: payload.birth_datetime_utc,
            latitude_deg: 48.8566,
            longitude_deg: 2.3522,
            altitude_m: None,
            reference_version_id: payload.reference_version_id,
            calculation_profile_id: None,
            zodiacal_reference_system_id: 1,
            coordinate_reference_system_id: 1,
            house_system_id: 1,
            product_code: Some("basic".to_string()),
            client_idempotency_key: None,
        },
        projection_level: level.to_string(),
        birth_datetime_local: "1990-01-02T03:04:05".to_string(),
        birth_timezone: "UTC".to_string(),
        birth_datetime_utc: payload.birth_datetime_utc,
        location_label: "Paris, France".to_string(),
        zodiac_key: "tropical".to_string(),
        coordinate_key: "geocentric".to_string(),
        house_system_code: "placidus".to_string(),
        calculation_type: "natal".to_string(),
    }
}

fn build_engine_envelope(level: &str) -> Value {
    let payload = load_v13_golden();
    let profile = profile_from_level(level).expect("profile");
    let resolved = sample_resolved(level, &payload);
    let response = build_engine_response(
        &resolved,
        payload,
        &RuntimeOptions::default(),
        "Tropical",
        "Geocentric",
        "Placidus",
        &[],
        &profile,
    )
    .expect("engine response");
    serde_json::to_value(response).expect("engine json")
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
    const FORBIDDEN_KEYS: &[&str] = &[
        "signal_key",
        "source_weight",
        "priority_score",
        "confidence_score",
        "aggregation_group",
        "evidence",
        "chart_object_id",
        "reference_version_id",
        "product_code",
        "context_key",
        "source_kind",
        "source_code",
        "ruler_sources",
        "ruler_object_code",
        "ruler_position_signal_key",
        "ruler_house_number",
        "ruler_sign_code",
        "interpretive_role",
        "astral_system_id",
        "astral_system_code",
        "dispositor_signal_key",
        "axis_code",
        "theme_codes",
        "source_signal_keys",
        "source_context_keys",
        "polarity_balance",
        "axis_score",
        "slot",
        "primary_signal_keys",
        "secondary_slot_candidates",
        "source_signal_keys",
    ];
    const FORBIDDEN_SUFFIXES: &[&str] = &["_id", "_code"];

    fn walk(
        node: &Value,
        path: &str,
        forbidden_keys: &[&str],
        forbidden_suffixes: &[&str],
    ) {
        match node {
            Value::Object(map) => {
                for (key, child) in map {
                    let child_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    if forbidden_keys.contains(&key.as_str()) {
                        panic!("forbidden technical key {child_path}");
                    }
                    for suffix in forbidden_suffixes {
                        if key.ends_with(suffix) {
                            panic!("forbidden technical key {child_path}");
                        }
                    }
                    walk(child, &child_path, forbidden_keys, forbidden_suffixes);
                }
            }
            Value::Array(items) => {
                for (index, child) in items.iter().enumerate() {
                    walk(
                        child,
                        &format!("{path}[{index}]"),
                        forbidden_keys,
                        forbidden_suffixes,
                    );
                }
            }
            _ => {}
        }
    }
    walk(value, "", FORBIDDEN_KEYS, FORBIDDEN_SUFFIXES);
}

fn assert_no_language_or_tone(value: &Value) {
    let serialized = value.to_string().to_ascii_lowercase();
    for token in ["target_language", "\"tone\"", "prompt", "writing_guidance"] {
        assert!(
            !serialized.contains(token),
            "llm_payload must not contain {token}"
        );
    }
}

fn placement_object_names(group: &Value) -> Vec<String> {
    ["primary", "supporting", "background"]
        .iter()
        .flat_map(|bucket| {
            group[*bucket]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|entry| entry["object"].as_str().map(str::to_string))
        })
        .collect()
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
        assert_no_language_or_tone(level.1);
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
        assert_no_technical_ids(&generated);
        assert_no_language_or_tone(&generated);
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
fn engine_envelope_is_not_flat_v13_payload() {
    let envelope = build_engine_envelope("rich");
    assert_eq!(
        envelope["response_contract_version"], "astro_engine_response_v1"
    );
    assert!(envelope.get("product_code").is_none());
    assert!(envelope.get("audit_payload").is_some());
    assert!(envelope.get("llm_payload").is_some());
}

#[test]
fn engine_envelope_golden_rich_matches_built_sample() {
    let generated = build_engine_envelope("rich");
    let golden_raw =
        fs::read_to_string(ENGINE_RESPONSE_GOLDEN_RICH).unwrap_or_else(|_| {
            panic!(
                "missing {ENGINE_RESPONSE_GOLDEN_RICH}; run UPDATE_ENGINE_RESPONSE_GOLDEN=1 cargo test --test engine_contract_tests write_engine_response_golden_when_env_set"
            )
        });
    let golden: Value = serde_json::from_str(&golden_raw).expect("golden json");
    assert_eq!(generated, golden, "engine response rich golden mismatch");
}

#[test]
fn write_engine_response_golden_when_env_set() {
    if std::env::var("UPDATE_ENGINE_RESPONSE_GOLDEN").ok().as_deref() != Some("1") {
        return;
    }
    let json = serde_json::to_string_pretty(&build_engine_envelope("rich")).expect("serialize");
    fs::write(ENGINE_RESPONSE_GOLDEN_RICH, format!("{json}\n")).expect("write golden");
}

#[test]
fn audit_payload_identical_across_projection_levels_in_envelope() {
    let compact = build_engine_envelope("compact");
    let rich = build_engine_envelope("rich");
    assert_eq!(compact["audit_payload"], rich["audit_payload"]);
    assert_ne!(compact["llm_payload"], rich["llm_payload"]);
}

#[test]
fn engine_response_envelope_shape_from_v13_golden() {
    let payload = load_v13_golden();
    let profile = profile_from_level("rich").expect("profile");
    let llm = build_llm_projection_natal_v1(&payload, &profile, &projection_context());
    let llm_value = serde_json::to_value(&llm).expect("llm json");

    assert_eq!(RESPONSE_CONTRACT_VERSION, "astro_engine_response_v1");

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

#[test]
fn llm_projection_contains_all_top_level_sections() {
    let rich = build_level("rich");
    for key in [
        "chart",
        "reading_order",
        "core_identity",
        "dominant_themes",
        "placements",
        "angles",
        "strengths",
        "relationship_network",
        "dynamics",
        "house_axes",
        "keywords",
    ] {
        assert!(rich.get(key).is_some(), "missing section {key}");
    }
}

#[test]
fn llm_projection_includes_active_major_aspect() {
    let payload = load_v13_golden();
    let aspect_count = payload
        .signals
        .iter()
        .filter(|s| is_active_major_aspect_signal(s))
        .count();
    assert!(aspect_count >= 1, "golden must contain at least one major aspect signal");

    let rich = build_level("rich");
    let aspects = rich["dynamics"]["major_aspects"]
        .as_array()
        .expect("major_aspects array");
    assert!(!aspects.is_empty(), "rich projection must include major aspects");
}

#[test]
fn llm_projection_maps_jupiter_uranus_opposition() {
    let rich = build_level("rich");
    let aspects = rich["dynamics"]["major_aspects"]
        .as_array()
        .expect("major_aspects");
    let jupiter_uranus = aspects
        .iter()
        .find(|aspect| aspect["aspect"] == "Jupiter opposition Uranus")
        .expect("Jupiter opposition Uranus must be projected");
    assert_eq!(jupiter_uranus["objects"], json!(["Jupiter", "Uranus"]));
    assert_eq!(jupiter_uranus["quality"], "Tension");
    assert_eq!(jupiter_uranus["valence"], "Polarizing");
    assert_eq!(jupiter_uranus["phase"], "Separating");
    let orb = jupiter_uranus["orb_degrees"].as_f64().expect("orb");
    assert!((orb - 0.76).abs() < 0.01, "orb expected near 0.76, got {orb}");
}

#[test]
fn llm_projection_humanizes_dominant_theme_reasons() {
    let rich = build_level("rich");
    let factors = rich["dominant_themes"]["objects"][0]["supporting_factors"]
        .as_array()
        .expect("supporting_factors");
    let joined = factors
        .iter()
        .filter_map(|v| v.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(!joined.contains("strong_aspect_participant"));
    assert!(!joined.contains("accidental_context"));
}

#[test]
fn llm_projection_humanizes_accidental_conditions() {
    let standard = build_level("standard");
    let conditions = standard["strengths"]["accidental_conditions"]
        .as_array()
        .expect("accidental_conditions");
    assert!(!conditions.is_empty());
    let first = &conditions[0]["conditions"]
        .as_array()
        .expect("conditions")[0];
    let label = first.as_str().expect("condition label");
    assert!(!label.contains('_'));
}

#[test]
fn llm_projection_reading_order_has_no_signal_keys() {
    let rich = build_level("rich");
    let serialized = rich["reading_order"].to_string();
    assert!(!serialized.contains("signal_key"));
    assert!(!serialized.contains("object_position:"));
    assert!(!serialized.contains("aspect:jupiter"));
}

#[test]
fn non_expert_does_not_include_scores() {
    for level in ["compact", "standard", "rich"] {
        let projection = build_level(level);
        assert!(
            !projection.to_string().contains("strength_score"),
            "{level} must not expose scores"
        );
        assert!(
            !projection.to_string().contains("overall_score"),
            "{level} must not expose scores"
        );
    }
}

#[test]
fn expert_may_include_scores() {
    let expert = build_level("expert");
    assert!(
        expert.to_string().contains("strength_score")
            || expert["dominant_themes"]["signs"]
                .as_array()
                .and_then(|s| s.first())
                .and_then(|s| s.get("score"))
                .is_some(),
        "expert should expose numeric scores when available"
    );
}

#[test]
fn compact_has_fewer_keywords_than_rich() {
    let compact = build_level("compact");
    let rich = build_level("rich");
    assert!(
        compact["keywords"]["main"].as_array().unwrap().len()
            <= rich["keywords"]["main"].as_array().unwrap().len()
    );
}

#[test]
fn llm_projection_placements_exclude_core_luminaries() {
    let rich = build_level("rich");
    let names = placement_object_names(&rich["placements"]);
    assert!(!names.iter().any(|name| name == "Sun"));
    assert!(!names.iter().any(|name| name == "Moon"));
}

#[test]
fn compact_has_zero_background_placements() {
    let compact = build_level("compact");
    assert_eq!(
        compact["placements"]["background"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn llm_projection_accidental_conditions_are_deduplicated() {
    let standard = build_level("standard");
    for entry in standard["strengths"]["accidental_conditions"]
        .as_array()
        .unwrap()
    {
        let labels: Vec<&str> = entry["conditions"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        let normalized: Vec<String> = labels.iter().map(|s| s.to_ascii_lowercase()).collect();
        assert_eq!(
            normalized.len(),
            normalized.iter().collect::<std::collections::HashSet<_>>().len(),
            "duplicate accidental labels: {labels:?}"
        );
    }
}

#[test]
fn llm_projection_axis_summary_has_no_snake_case_themes() {
    let rich = build_level("rich");
    let summary = rich["house_axes"][0]["summary"]
        .as_str()
        .expect("summary");
    assert!(
        !summary.contains("shared_resources"),
        "summary must not contain snake_case theme codes: {summary}"
    );
    assert!(
        summary.contains("house 8 (Transformation)"),
        "expected human house 8 theme label: {summary}"
    );
}

#[test]
fn llm_projection_humanizes_axis_supporting_factors() {
    let rich = build_level("rich");
    let factors = rich["house_axes"][1]["supporting_factors"]
        .as_array()
        .expect("supporting_factors");
    let joined = factors
        .iter()
        .filter_map(|v| v.as_str())
        .collect::<Vec<_>>()
        .join("|");
    assert!(!joined.contains("ascendant angle in house"));
    assert!(!joined.contains("identity theme"));
    assert!(joined.contains("Ascendant emphasizes this house"));
    assert!(joined.contains("Identity theme emphasized"));
}

#[test]
fn llm_projection_conditions_exclude_redundant_direct_motion() {
    let rich = build_level("rich");
    for bucket in ["primary", "supporting", "background"] {
        for placement in rich["placements"][bucket].as_array().unwrap() {
            let motion = placement["motion"].as_str();
            let conditions = placement["conditions"].as_array().unwrap();
            if motion == Some("Direct motion") {
                for condition in conditions {
                    assert_ne!(
                        condition.as_str(),
                        Some("Direct motion"),
                        "redundant Direct motion in conditions for {}",
                        placement["object"]
                    );
                }
            }
        }
    }
    let mars = rich["placements"]["supporting"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["object"] == "Mars")
        .expect("Mars placement");
    assert_eq!(mars["motion"], "Direct motion");
    let mars_conditions: Vec<&str> = mars["conditions"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(mars_conditions.contains(&"Angular house"));
    assert!(!mars_conditions.contains(&"Direct motion"));
}

#[test]
fn compact_has_fewer_placements_than_rich() {
    let compact = build_level("compact");
    let rich = build_level("rich");
    let compact_total = compact["placements"]["primary"].as_array().unwrap().len()
        + compact["placements"]["supporting"].as_array().unwrap().len()
        + compact["placements"]["background"].as_array().unwrap().len();
    let rich_total = rich["placements"]["primary"].as_array().unwrap().len()
        + rich["placements"]["supporting"].as_array().unwrap().len()
        + rich["placements"]["background"].as_array().unwrap().len();
    assert!(compact_total <= rich_total);
}
