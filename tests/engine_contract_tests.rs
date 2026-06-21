mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::sync::OnceLock;

use jsonschema::JSONSchema;
use serde_json::{json, Value};

use astral_calculator::bootstrap::db::connect_from_env;
use astral_calculator::domain::{
    AccidentalDignityConditionReference, AnglePointReference, BasicPayload,
    EssentialDignityRuleReference, HouseAxisReference, HouseReference, MotionStateReference,
    ProjectionLabelDefinition, ProjectionReasonDefinition, RuntimeOptions,
};
use astral_calculator::engine::projection::{
    build_llm_projection_natal_v1, is_active_major_aspect_signal, LlmProjectionBuildContext,
    LlmProjectionProfile,
};
use astral_calculator::engine::{
    build_engine_response, AstroEngineResponse, ResolvedEngineRequest, RESPONSE_CONTRACT_VERSION,
};
use astral_calculator::infra::db::projection_repository::ProjectionRepository;
use common::natal_catalog::test_catalog;

const V14_GOLDEN: &str = "../tests/golden/natal_payload_v14_paris_1990.json";
const LLM_SCHEMA: &str = "../contracts/calculator/llm_projection_natal_v1.schema.json";
const REQUEST_SCHEMA: &str = "../contracts/calculator/astro_engine_request_v1.schema.json";
const RESPONSE_SCHEMA: &str = "../contracts/calculator/astro_engine_response_v1.schema.json";
const ENGINE_RESPONSE_GOLDEN_RICH: &str =
    "../tests/golden/astro_engine_response_v1_paris_1990_rich.json";

const LLM_GOLDEN_COMPACT: &str = "../tests/golden/llm_projection_natal_v1_paris_1990_compact.json";
const LLM_GOLDEN_STANDARD: &str =
    "../tests/golden/llm_projection_natal_v1_paris_1990_standard.json";
const LLM_GOLDEN_RICH: &str = "../tests/golden/llm_projection_natal_v1_paris_1990_rich.json";
const PROJECTION_LABELS_JSON: &str =
    include_str!("../json_db/astral_projection_label_definitions.json");
const HOUSE_AXIS_DEFINITIONS_JSON: &str =
    include_str!("../json_db/astral_house_axis_definitions.json");
const HOUSE_AXIS_MEMBERS_JSON: &str = include_str!("../json_db/astral_house_axis_members.json");
const HOUSES_JSON: &str = include_str!("../json_db/astral_houses.json");
const ANGLE_POINTS_JSON: &str = include_str!("../json_db/astral_angle_points.json");
const MOTION_STATES_JSON: &str = include_str!("../json_db/astral_object_motion_states.json");
const ACCIDENTAL_CONDITIONS_JSON: &str =
    include_str!("../json_db/astral_accidental_dignity_condition_definitions.json");

fn load_v14_golden() -> BasicPayload {
    let raw = fs::read_to_string(V14_GOLDEN).expect("v14 golden");
    serde_json::from_str(&raw).expect("v14 golden json")
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
    static REASON_DEFINITIONS: OnceLock<Vec<ProjectionReasonDefinition>> = OnceLock::new();
    static LABEL_DEFINITIONS: OnceLock<Vec<ProjectionLabelDefinition>> = OnceLock::new();
    static HOUSE_REFERENCES: OnceLock<Vec<HouseReference>> = OnceLock::new();
    static HOUSE_AXES: OnceLock<Vec<HouseAxisReference>> = OnceLock::new();
    static ANGLE_POINTS: OnceLock<Vec<AnglePointReference>> = OnceLock::new();
    static MOTION_STATES: OnceLock<Vec<MotionStateReference>> = OnceLock::new();
    static ACCIDENTAL_CONDITIONS: OnceLock<Vec<AccidentalDignityConditionReference>> =
        OnceLock::new();
    static ESSENTIAL_DIGNITIES: OnceLock<Vec<EssentialDignityRuleReference>> = OnceLock::new();
    LlmProjectionBuildContext {
        birth_location_label: "Paris, France",
        zodiac_label: "Tropical",
        coordinate_label: "Geocentric",
        house_system_label: "Placidus",
        house_axes: HOUSE_AXES.get_or_init(house_axes_from_seed),
        projection_reason_definitions: REASON_DEFINITIONS
            .get_or_init(|| test_catalog().projection_reason_definitions),
        projection_label_definitions: LABEL_DEFINITIONS
            .get_or_init(projection_label_definitions_from_seed),
        house_references: HOUSE_REFERENCES.get_or_init(house_references_from_seed),
        angle_points: ANGLE_POINTS.get_or_init(angle_points_from_seed),
        motion_states: MOTION_STATES.get_or_init(motion_states_from_seed),
        accidental_condition_definitions: ACCIDENTAL_CONDITIONS
            .get_or_init(accidental_conditions_from_seed),
        essential_dignity_rules: ESSENTIAL_DIGNITIES
            .get_or_init(|| test_catalog().essential_dignity_rules),
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

fn load_profile_from_db(
    level: &str,
) -> astral_calculator::engine::projection::LlmProjectionProfile {
    static PROFILE_CACHE: OnceLock<Result<BTreeMap<String, LlmProjectionProfile>, String>> =
        OnceLock::new();

    let profiles = PROFILE_CACHE.get_or_init(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        runtime.block_on(async {
            let pool = connect_from_env().await.map_err(|error| {
                format!(
                    "DATABASE_URL and PostgreSQL are required for engine_contract_tests: {error}"
                )
            })?;
            let repository = ProjectionRepository::new(pool);
            let mut profiles = BTreeMap::new();
            for level in ["compact", "standard", "rich", "expert"] {
                let profile = repository
                    .llm_projection_profile("llm_projection_natal_v1", level)
                    .await
                    .map_err(|error| {
                        format!(
                            "llm projection profile {level} must exist for engine_contract_tests: {error}"
                        )
                    })?;
                profiles.insert(level.to_string(), profile);
            }
            Ok(profiles)
        })
    });

    match profiles {
        Ok(profiles) => profiles
            .get(level)
            .cloned()
            .unwrap_or_else(|| panic!("missing cached llm projection profile for level {level}")),
        Err(message) => panic!("{message}"),
    }
}

fn build_engine_envelope(level: &str) -> Value {
    serde_json::to_value(build_engine_response_sample(level)).expect("engine json")
}

fn build_engine_response_sample(level: &str) -> AstroEngineResponse {
    let payload = load_v14_golden();
    let profile = load_profile_from_db(level);
    let resolved = sample_resolved(level, &payload);
    let catalog = test_catalog();
    build_engine_response(
        &resolved,
        payload,
        &RuntimeOptions::default(),
        "Tropical",
        "Geocentric",
        "Placidus",
        projection_context().house_references,
        projection_context().house_axes,
        projection_context().angle_points,
        projection_context().motion_states,
        projection_context().accidental_condition_definitions,
        &catalog.essential_dignity_rules,
        &catalog.projection_reason_definitions,
        &catalog.projection_label_definitions,
        &profile,
    )
    .expect("engine response")
}

fn build_level(level: &str) -> Value {
    let payload = load_v14_golden();
    let profile = load_profile_from_db(level);
    let projection = build_llm_projection_natal_v1(&payload, &profile, &projection_context())
        .expect("llm projection build must succeed for engine_contract_tests");
    serde_json::to_value(projection).expect("projection json")
}

fn local_projection_profile() -> LlmProjectionProfile {
    LlmProjectionProfile {
        contract_version: "llm_projection_natal_v1".to_string(),
        level_code: "rich".to_string(),
        max_keywords_per_item: 5,
        max_core_placements: 3,
        max_supporting_placements: 3,
        max_dominant_signs: 3,
        max_dominant_houses: 3,
        max_dominant_objects: 3,
        max_house_axes: 3,
        max_aspects: 3,
        max_background_placements: 3,
        max_accidental_conditions_per_object: 3,
        include_accidental_conditions: true,
        include_rulership_details: true,
        include_minor_evidence: true,
        include_degrees: true,
        include_scores: true,
    }
}

fn assert_no_underscore_strings(values: &[Value], label: &str) {
    for value in values {
        let text = value.as_str().expect(label);
        assert!(
            !text.contains('_'),
            "{label} must not contain snake_case fallback: {text}"
        );
    }
}

fn seed_rows(json: &str) -> Vec<Value> {
    serde_json::from_str::<Value>(json)
        .expect("seed json")
        .get("data")
        .and_then(Value::as_array)
        .expect("seed data array")
        .clone()
}

fn projection_label_definitions_from_seed() -> Vec<ProjectionLabelDefinition> {
    seed_rows(PROJECTION_LABELS_JSON)
        .into_iter()
        .map(|row| ProjectionLabelDefinition {
            label_family: row["label_family"]
                .as_str()
                .expect("label_family")
                .to_string(),
            label_code: row["label_code"].as_str().expect("label_code").to_string(),
            label_template_en: row["label_template_en"]
                .as_str()
                .expect("label_template_en")
                .to_string(),
            is_active: row["is_active"].as_bool().expect("is_active"),
            sort_order: row["sort_order"].as_i64().expect("sort_order") as i32,
        })
        .collect()
}

fn house_references_from_seed() -> Vec<HouseReference> {
    seed_rows(HOUSES_JSON)
        .into_iter()
        .map(|row| HouseReference {
            id: row["id"].as_i64().expect("id") as i32,
            number: row["number"].as_i64().expect("number") as i32,
            name: row["name"].as_str().expect("name").to_string(),
            theme_code: row["theme_code"].as_str().expect("theme_code").to_string(),
            modality_code: None,
            modality_label: None,
            accidental_strength: None,
            modality_priority_delta: None,
            interpretation_weight: None,
        })
        .collect()
}

fn house_axes_from_seed() -> Vec<HouseAxisReference> {
    let houses_by_id = seed_rows(HOUSES_JSON)
        .into_iter()
        .map(|house| {
            (
                house["id"].as_i64().expect("house id") as i32,
                (
                    house["number"].as_i64().expect("house number") as i32,
                    house["theme_code"]
                        .as_str()
                        .expect("theme_code")
                        .to_string(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let members = seed_rows(HOUSE_AXIS_MEMBERS_JSON);

    let mut axes = seed_rows(HOUSE_AXIS_DEFINITIONS_JSON)
        .into_iter()
        .map(|axis| {
            let axis_id = axis["id"].as_i64().expect("axis id") as i32;
            let member = members
                .iter()
                .find(|member| {
                    if member["axis_id"].as_i64().expect("member axis_id") as i32 != axis_id {
                        return false;
                    }
                    let house_a_id = member["house_id"].as_i64().expect("house_id") as i32;
                    let house_b_id = member["opposite_house_id"]
                        .as_i64()
                        .expect("opposite_house_id") as i32;
                    let house_a_number = houses_by_id
                        .get(&house_a_id)
                        .unwrap_or_else(|| panic!("missing house reference for id {house_a_id}"))
                        .0;
                    let house_b_number = houses_by_id
                        .get(&house_b_id)
                        .unwrap_or_else(|| panic!("missing house reference for id {house_b_id}"))
                        .0;
                    house_a_number < house_b_number
                })
                .unwrap_or_else(|| {
                    panic!("missing canonical house axis member for axis id {axis_id}")
                });
            let house_a_id = member["house_id"].as_i64().expect("house_id") as i32;
            let house_b_id = member["opposite_house_id"]
                .as_i64()
                .expect("opposite_house_id") as i32;
            let (house_a_number, theme_a_code) = houses_by_id
                .get(&house_a_id)
                .cloned()
                .unwrap_or_else(|| panic!("missing house reference for id {house_a_id}"));
            let (house_b_number, theme_b_code) = houses_by_id
                .get(&house_b_id)
                .cloned()
                .unwrap_or_else(|| panic!("missing house reference for id {house_b_id}"));

            HouseAxisReference {
                axis_code: axis["key"].as_str().expect("axis key").to_string(),
                house_a_number,
                house_b_number,
                theme_a_code,
                theme_b_code,
                label: axis["title"].as_str().expect("axis title").to_string(),
                description: axis["summary"].as_str().expect("axis summary").to_string(),
            }
        })
        .collect::<Vec<_>>();
    axes.sort_by_key(|axis| axis.house_a_number);
    axes
}

fn angle_points_from_seed() -> Vec<AnglePointReference> {
    seed_rows(ANGLE_POINTS_JSON)
        .into_iter()
        .map(|row| {
            let code = row["code"].as_str().expect("code");
            let (chart_object_code, chart_object_name, sort_order) = match code {
                "asc" => ("ascendant", "Ascendant", 1),
                "dsc" => ("descendant", "Descendant", 2),
                "mc" => ("mc", "Midheaven", 3),
                "ic" => ("ic", "IC", 4),
                other => (other, other, 99),
            };
            AnglePointReference {
                id: row["id"].as_i64().expect("id") as i32,
                code: code.to_string(),
                short_label: row["short_label"]
                    .as_str()
                    .expect("short_label")
                    .to_string(),
                full_name: row["full_name"].as_str().expect("full_name").to_string(),
                axis: row["axis"].as_str().expect("axis").to_string(),
                opposite_angle_code: row["opposite_angle_code"].as_str().map(str::to_string),
                associated_house: row["associated_house"].as_i64().expect("associated_house")
                    as i32,
                description: row["description"]
                    .as_str()
                    .expect("description")
                    .to_string(),
                chart_object_id: row["id"].as_i64().expect("id") as i32,
                chart_object_code: chart_object_code.to_string(),
                chart_object_name: chart_object_name.to_string(),
                chart_object_sort_order: sort_order,
            }
        })
        .collect()
}

fn motion_states_from_seed() -> Vec<MotionStateReference> {
    seed_rows(MOTION_STATES_JSON)
        .into_iter()
        .map(|row| MotionStateReference {
            id: row["id"].as_i64().expect("id") as i32,
            code: row["code"].as_str().expect("code").to_string(),
            label: row["label"].as_str().expect("label").to_string(),
            motion_family: row["motion_family"]
                .as_str()
                .expect("motion_family")
                .to_string(),
        })
        .collect()
}

fn accidental_conditions_from_seed() -> Vec<AccidentalDignityConditionReference> {
    seed_rows(ACCIDENTAL_CONDITIONS_JSON)
        .into_iter()
        .map(|row| AccidentalDignityConditionReference {
            condition_code: row["condition_code"]
                .as_str()
                .expect("condition_code")
                .to_string(),
            condition_family: row["condition_family"]
                .as_str()
                .expect("condition_family")
                .to_string(),
            label: row["label"].as_str().expect("label").to_string(),
            polarity: row["polarity"].as_str().expect("polarity").to_string(),
            strength_score: row["strength_score"].as_f64().expect("strength_score"),
            score_delta: row["score_delta"].as_f64().expect("score_delta"),
            description: row["description"]
                .as_str()
                .expect("description")
                .to_string(),
        })
        .collect()
}

fn top_level_keys(value: &Value) -> BTreeSet<String> {
    value.as_object().expect("object").keys().cloned().collect()
}

fn assert_compact_profile_rules(value: &Value) {
    assert!(value["core_identity"]["ascendant"]
        .get("ruler")
        .is_none_or(|r| r.is_null()));
    assert_eq!(value["relationship_network"].as_object().unwrap().len(), 0);
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

    fn walk(node: &Value, path: &str, forbidden_keys: &[&str], forbidden_suffixes: &[&str]) {
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
    assert!(
        errors.is_empty(),
        "request schema errors:\n{}",
        errors.join("\n")
    );
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

    for level in [
        ("compact", &compact),
        ("standard", &standard),
        ("rich", &rich),
    ] {
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
        assert_eq!(
            generated, golden,
            "llm projection {level} differs from {path}"
        );
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
        let value = build_level(level);
        let json = serde_json::to_string_pretty(&value).expect("serialize");
        fs::write(path, format!("{json}\n")).expect("write golden");
    }
}

#[test]
fn engine_envelope_is_not_flat_v14_payload() {
    let envelope = build_engine_envelope("rich");
    assert_eq!(
        envelope["response_contract_version"],
        "astro_engine_response_v1"
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
    if std::env::var("UPDATE_ENGINE_RESPONSE_GOLDEN")
        .ok()
        .as_deref()
        != Some("1")
    {
        return;
    }
    let response = build_engine_response_sample("rich");
    let json = serde_json::to_string_pretty(&response).expect("serialize");
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
fn engine_response_envelope_shape_from_v14_golden() {
    let payload = load_v14_golden();
    let profile = load_profile_from_db("rich");
    let llm =
        build_llm_projection_natal_v1(&payload, &profile, &projection_context()).expect("llm");
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
            "raw_payload_contract_version": "natal_structured_v14",
            "llm_projection_contract_version": "llm_projection_natal_v1"
        },
        "audit_payload": {
            "contract_version": "natal_structured_v14",
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
    let payload = load_v14_golden();
    let aspect_count = payload
        .signals
        .iter()
        .filter(|s| is_active_major_aspect_signal(s))
        .count();
    assert!(
        aspect_count >= 1,
        "golden must contain at least one major aspect signal"
    );

    let rich = build_level("rich");
    let aspects = rich["dynamics"]["major_aspects"]
        .as_array()
        .expect("major_aspects array");
    assert!(
        !aspects.is_empty(),
        "rich projection must include major aspects"
    );
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
    assert!(
        (orb - 0.76).abs() < 0.01,
        "orb expected near 0.76, got {orb}"
    );
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
fn llm_projection_fails_when_reason_definition_is_missing() {
    let payload = load_v14_golden();
    let profile = load_profile_from_db("rich");
    let mut definitions = test_catalog().projection_reason_definitions;
    definitions.retain(|definition| definition.reason_code != "essential_dignity");

    let result = build_llm_projection_natal_v1(
        &payload,
        &profile,
        &LlmProjectionBuildContext {
            birth_location_label: "Paris, France",
            zodiac_label: "Tropical",
            coordinate_label: "Geocentric",
            house_system_label: "Placidus",
            house_axes: &[],
            projection_reason_definitions: &definitions,
            projection_label_definitions: projection_context().projection_label_definitions,
            house_references: projection_context().house_references,
            angle_points: projection_context().angle_points,
            motion_states: projection_context().motion_states,
            accidental_condition_definitions: projection_context().accidental_condition_definitions,
            essential_dignity_rules: projection_context().essential_dignity_rules,
        },
    );

    let error = result.expect_err("projection should fail without runtime definition");
    assert_eq!(error.code(), "invalid_projection_reason_definition");
}

#[test]
fn llm_projection_fails_when_projection_label_definition_is_missing() {
    let payload = load_v14_golden();
    let profile = load_profile_from_db("rich");
    let mut labels = projection_label_definitions_from_seed();
    labels.retain(|definition| {
        !(definition.label_family == "dynamic_quality" && definition.label_code == "tension")
    });
    let ctx = projection_context();

    let result = build_llm_projection_natal_v1(
        &payload,
        &profile,
        &LlmProjectionBuildContext {
            birth_location_label: ctx.birth_location_label,
            zodiac_label: ctx.zodiac_label,
            coordinate_label: ctx.coordinate_label,
            house_system_label: ctx.house_system_label,
            house_axes: ctx.house_axes,
            projection_reason_definitions: ctx.projection_reason_definitions,
            projection_label_definitions: &labels,
            house_references: ctx.house_references,
            angle_points: ctx.angle_points,
            motion_states: ctx.motion_states,
            accidental_condition_definitions: ctx.accidental_condition_definitions,
            essential_dignity_rules: ctx.essential_dignity_rules,
        },
    );

    let error = result.expect_err("projection should fail without runtime projection label");
    assert_eq!(error.code(), "invalid_projection_label_definition");
}

#[test]
fn llm_projection_fails_when_accidental_condition_reference_is_missing() {
    let payload = load_v14_golden();
    let profile = load_profile_from_db("rich");
    let mut conditions = accidental_conditions_from_seed();
    conditions.retain(|definition| definition.condition_code != "angular_house");
    let ctx = projection_context();

    let result = build_llm_projection_natal_v1(
        &payload,
        &profile,
        &LlmProjectionBuildContext {
            birth_location_label: ctx.birth_location_label,
            zodiac_label: ctx.zodiac_label,
            coordinate_label: ctx.coordinate_label,
            house_system_label: ctx.house_system_label,
            house_axes: ctx.house_axes,
            projection_reason_definitions: ctx.projection_reason_definitions,
            projection_label_definitions: ctx.projection_label_definitions,
            house_references: ctx.house_references,
            angle_points: ctx.angle_points,
            motion_states: ctx.motion_states,
            accidental_condition_definitions: &conditions,
            essential_dignity_rules: ctx.essential_dignity_rules,
        },
    );

    let error = result.expect_err("projection should fail without accidental condition reference");
    assert_eq!(error.code(), "invalid_projection_label_definition");
}

#[test]
fn llm_projection_fails_when_house_reference_is_missing() {
    let payload = load_v14_golden();
    let profile = load_profile_from_db("rich");
    let mut houses = house_references_from_seed();
    houses.retain(|reference| reference.theme_code != "shared_resources");
    let ctx = projection_context();

    let result = build_llm_projection_natal_v1(
        &payload,
        &profile,
        &LlmProjectionBuildContext {
            birth_location_label: ctx.birth_location_label,
            zodiac_label: ctx.zodiac_label,
            coordinate_label: ctx.coordinate_label,
            house_system_label: ctx.house_system_label,
            house_axes: ctx.house_axes,
            projection_reason_definitions: ctx.projection_reason_definitions,
            projection_label_definitions: ctx.projection_label_definitions,
            house_references: &houses,
            angle_points: ctx.angle_points,
            motion_states: ctx.motion_states,
            accidental_condition_definitions: ctx.accidental_condition_definitions,
            essential_dignity_rules: ctx.essential_dignity_rules,
        },
    );

    let error = result.expect_err("projection should fail without house reference");
    assert_eq!(error.code(), "invalid_projection_label_definition");
}

#[test]
fn llm_projection_never_contains_reason_fallback_marker() {
    let rich = build_level("rich");
    assert!(
        !rich.to_string().contains("reason:"),
        "projection must not contain technical reason fallback markers"
    );
}

#[test]
fn llm_projection_keeps_object_name_in_essential_dignity_reasons() {
    let rich = build_level("rich");
    let objects = rich["dominant_themes"]["objects"]
        .as_array()
        .expect("dominant objects");
    let saturn = objects
        .iter()
        .find(|item| item["name"] == "Saturn")
        .expect("Saturn dominant object");
    let factors = saturn["supporting_factors"]
        .as_array()
        .expect("supporting_factors");
    let labels = factors
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>();

    assert!(
        labels.contains(&"Saturn in domicile"),
        "expected rendered dignity label with object name, got {labels:?}"
    );
    assert!(
        !labels.contains(&"In domicile"),
        "truncated dignity label leaked into projection: {labels:?}"
    );
}

#[test]
fn llm_projection_humanizes_accidental_conditions() {
    let standard = build_level("standard");
    let conditions = standard["strengths"]["accidental_conditions"]
        .as_array()
        .expect("accidental_conditions");
    assert!(!conditions.is_empty());
    let first = &conditions[0]["conditions"].as_array().expect("conditions")[0];
    let label = first.as_str().expect("condition label");
    assert!(!label.contains('_'));
}

#[test]
fn llm_projection_db_backed_labels_do_not_leak_snake_case() {
    let rich = build_level("rich");

    for entry in rich["strengths"]["accidental_conditions"]
        .as_array()
        .expect("accidental_conditions")
    {
        assert_no_underscore_strings(
            entry["conditions"].as_array().expect("conditions"),
            "condition",
        );
    }

    for entry in rich["dominant_themes"]["objects"]
        .as_array()
        .expect("dominant objects")
    {
        assert_no_underscore_strings(
            entry["supporting_factors"]
                .as_array()
                .expect("supporting_factors"),
            "supporting_factor",
        );
    }

    for entry in rich["house_axes"].as_array().expect("house_axes") {
        let summary = entry["summary"].as_str().expect("summary");
        assert!(
            !summary.contains('_'),
            "house axis summary must not contain snake_case fallback: {summary}"
        );
    }

    for aspect in rich["dynamics"]["major_aspects"]
        .as_array()
        .expect("major_aspects")
    {
        for key in ["quality", "valence", "phase"] {
            let value = aspect[key].as_str().expect("aspect label");
            assert!(
                !value.contains('_'),
                "aspect {key} must not contain snake_case fallback: {value}"
            );
        }
    }
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
            normalized
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            "duplicate accidental labels: {labels:?}"
        );
    }
}

#[test]
fn llm_projection_axis_summary_has_no_snake_case_themes() {
    let rich = build_level("rich");
    let summary = rich["house_axes"][0]["summary"].as_str().expect("summary");
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
fn llm_projection_secondary_axis_balance_matches_summary_house() {
    let mut payload = load_v14_golden();
    let axis = payload
        .house_axis_emphasis
        .get_mut(0)
        .expect("house axis fixture");
    axis.primary_house = 8;
    axis.secondary_house = 2;
    axis.polarity_balance = "secondary_house_dominant".to_string();
    axis.interpretive_hint =
        "Resources and Sharing is activated mainly through house 8 (shared_resources), with house 2 (resources) present as a secondary counterpoint."
            .to_string();

    let projection =
        build_llm_projection_natal_v1(&payload, &local_projection_profile(), &projection_context())
            .expect("projection");
    let rich = serde_json::to_value(projection).expect("projection json");
    let axis = &rich["house_axes"][0];

    assert_eq!(axis["balance"].as_str(), Some("Mainly house 8"));
    let summary = axis["summary"].as_str().expect("summary");
    assert!(
        summary.contains("mainly through house 8 (Transformation)"),
        "summary must match the displayed dominant balance: {summary}"
    );
}

#[test]
fn llm_projection_humanizes_axis_supporting_factors() {
    let rich = build_level("rich");
    let axis = rich["house_axes"]
        .as_array()
        .expect("house_axes")
        .iter()
        .find(|axis| axis["axis"].as_str() == Some("Self and Relationship"))
        .expect("self relationship axis");
    let factors = axis["supporting_factors"]
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
    assert!(joined.contains("Self theme emphasized"));
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
        + compact["placements"]["supporting"]
            .as_array()
            .unwrap()
            .len()
        + compact["placements"]["background"]
            .as_array()
            .unwrap()
            .len();
    let rich_total = rich["placements"]["primary"].as_array().unwrap().len()
        + rich["placements"]["supporting"].as_array().unwrap().len()
        + rich["placements"]["background"].as_array().unwrap().len();
    assert!(compact_total <= rich_total);
}
