use std::collections::{HashMap, HashSet};
use std::fs;

use jsonschema::JSONSchema;
use serde_json::Value;

use rust_sqlx_connection_test::domain::BasicPayload;
use rust_sqlx_connection_test::runtime::is_current_basic_payload;

const GOLDEN_PAYLOAD_PATH: &str = "../tests/golden/basic_payload_v8_paris_1990.json";
const SCHEMA_PATH: &str = "schemas/basic_natal_structured_v8.schema.json";
const PAYLOAD_UNDER_TEST_ENV: &str = "BASIC_V8_SCHEMA_PAYLOAD_PATH";

fn load_golden_payload() -> Value {
    let raw = fs::read_to_string(GOLDEN_PAYLOAD_PATH).expect("golden payload should exist");
    serde_json::from_str(&raw).expect("golden payload should be valid JSON")
}

fn load_payload_from_path(path: &str) -> Value {
    let raw = fs::read_to_string(path).expect("payload under schema test should exist");
    serde_json::from_str(raw.trim_start_matches('\u{feff}'))
        .expect("payload under schema test should be valid JSON")
}

fn validate_with_schema(payload_json: &Value) -> Vec<String> {
    let schema_raw = fs::read_to_string(SCHEMA_PATH).expect("schema should exist");
    let schema_json: Value = serde_json::from_str(&schema_raw).expect("schema should be JSON");

    let compiled = JSONSchema::options()
        .compile(&schema_json)
        .expect("schema should compile");

    compiled
        .validate(payload_json)
        .err()
        .map(|errors| errors.map(|error| error.to_string()).collect::<Vec<_>>())
        .unwrap_or_default()
}

fn array<'a>(value: &'a Value, key: &str) -> &'a Vec<Value> {
    value[key]
        .as_array()
        .unwrap_or_else(|| panic!("{key} should be an array"))
}

fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key]
        .as_str()
        .unwrap_or_else(|| panic!("{key} should be a string"))
}

fn find_signal<'a>(payload: &'a Value, signal_key: &str) -> &'a Value {
    array(payload, "signals")
        .iter()
        .find(|signal| signal["signal_key"] == signal_key)
        .unwrap_or_else(|| panic!("missing signal {signal_key}"))
}

fn signal_exists(payload: &Value, signal_key: &str) -> bool {
    array(payload, "signals")
        .iter()
        .any(|signal| signal["signal_key"] == signal_key)
}

fn find_slot<'a>(payload: &'a Value, plan_name: &str, slot: &str) -> &'a Value {
    array(payload, plan_name)
        .iter()
        .find(|item| item["slot"] == slot)
        .unwrap_or_else(|| panic!("missing {plan_name} slot {slot}"))
}

fn source_keys(item: &Value) -> Vec<&str> {
    array(item, "source_signal_keys")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("source signal key should be a string")
        })
        .collect()
}

fn assert_source(item: &Value, signal_key: &str) {
    assert!(
        source_keys(item).contains(&signal_key),
        "slot {} should contain source {signal_key}",
        string(item, "slot")
    );
}

fn assert_source_prefix(item: &Value, prefix: &str) {
    assert!(
        source_keys(item).iter().any(|key| key.starts_with(prefix)),
        "slot {} should contain source prefix {prefix}",
        string(item, "slot")
    );
}

#[test]
fn golden_payload_matches_json_schema_v8() {
    let payload_json = load_golden_payload();
    let validation_errors = validate_with_schema(&payload_json);

    assert!(
        validation_errors.is_empty(),
        "golden payload does not match basic_natal_structured_v8 schema:\n{}",
        validation_errors.join("\n")
    );
}

#[test]
fn external_payload_matches_json_schema_v8_when_requested() {
    let Ok(path) = std::env::var(PAYLOAD_UNDER_TEST_ENV) else {
        return;
    };
    let payload_json = load_payload_from_path(&path);
    let validation_errors = validate_with_schema(&payload_json);

    assert!(
        validation_errors.is_empty(),
        "external payload does not match basic_natal_structured_v8 schema:\n{}",
        validation_errors.join("\n")
    );
}

#[test]
fn schema_rejects_extra_must_use_item() {
    let mut payload = load_golden_payload();
    payload["llm_handoff_contract"]["must_use"]
        .as_array_mut()
        .expect("must_use should be an array")
        .push(Value::String("extra_block".to_string()));

    assert!(
        !validate_with_schema(&payload).is_empty(),
        "schema should reject additional must_use items"
    );
}

#[test]
fn schema_rejects_null_required_signal_contract_field() {
    let mut payload = load_golden_payload();
    payload["signals"][0]["interpretive_hint"] = Value::Null;

    assert!(
        !validate_with_schema(&payload).is_empty(),
        "schema should reject null interpretive_hint on an active signal"
    );
}

#[test]
fn schema_rejects_null_required_position_context() {
    let mut payload = load_golden_payload();
    payload["positions"][0]["sign_context"] = Value::Null;

    assert!(
        !validate_with_schema(&payload).is_empty(),
        "schema should reject null sign_context on a position"
    );
}

#[test]
fn schema_rejects_position_context_without_house_theme_code() {
    let mut payload = load_golden_payload();
    payload["positions"][0]["house_context"]
        .as_object_mut()
        .expect("house_context should be an object")
        .remove("theme_code");

    assert!(
        !validate_with_schema(&payload).is_empty(),
        "schema should reject house_context without theme_code"
    );
}

#[test]
fn schema_rejects_signal_evidence_without_fact_type() {
    let mut payload = load_golden_payload();
    payload["signals"][0]["evidence"]
        .as_object_mut()
        .expect("evidence should be an object")
        .remove("fact_type");

    assert!(
        !validate_with_schema(&payload).is_empty(),
        "schema should reject evidence without fact_type"
    );
}

#[test]
fn schema_rejects_unknown_signal_key_family() {
    let mut payload = load_golden_payload();
    payload["signals"][0]["signal_key"] = Value::String("unknown_family:foo".to_string());

    assert!(
        !validate_with_schema(&payload).is_empty(),
        "schema should reject unknown signal_key families"
    );
}

#[test]
fn schema_rejects_extra_aspect_context_property() {
    let mut payload = load_golden_payload();
    let aspect = payload["signals"]
        .as_array_mut()
        .expect("signals should be an array")
        .iter_mut()
        .find(|signal| {
            signal["signal_key"]
                .as_str()
                .is_some_and(|key| key.starts_with("aspect:"))
        })
        .expect("golden payload should contain an aspect signal");

    aspect["aspect_context"]["is_structural_axis"] = Value::Bool(false);

    assert!(
        !validate_with_schema(&payload).is_empty(),
        "schema should reject additional aspect_context properties"
    );
}

#[test]
fn golden_payload_is_accepted_by_runtime_reuse_validation() {
    let raw = fs::read_to_string(GOLDEN_PAYLOAD_PATH).expect("golden payload should exist");
    let payload: BasicPayload =
        serde_json::from_str(&raw).expect("golden payload should deserialize");

    assert!(is_current_basic_payload(&payload));
}

#[test]
fn runtime_rejects_v7_contract_version() {
    let mut payload = load_golden_payload();
    payload["llm_handoff_contract"]["contract_version"] =
        Value::String("basic_natal_structured_v7".to_string());

    let parsed: BasicPayload =
        serde_json::from_value(payload).expect("modified payload should deserialize");

    assert!(!is_current_basic_payload(&parsed));
}

#[test]
fn v8_handoff_contract_is_strict() {
    let payload = load_golden_payload();
    let contract = &payload["llm_handoff_contract"];

    assert_eq!(contract["contract_version"], "basic_natal_structured_v8");
    assert_eq!(contract["payload_language_code"], "en");
    assert_eq!(
        contract["target_language_policy"],
        "provided_by_llm_service"
    );
    assert_eq!(contract["audience_level"], "beginner");
    assert_eq!(contract["output_format"], "structured_sections");

    let must_use = array(contract, "must_use");
    for expected in [
        "chart_context",
        "chart_emphasis",
        "dignities",
        "angles",
        "signals",
        "reading_plan",
        "drafting_plan",
    ] {
        assert!(
            must_use.iter().any(|value| value == expected),
            "must_use should contain {expected}"
        );
    }
}

#[test]
fn v8_contains_four_canonical_angles() {
    let payload = load_golden_payload();
    let angles = array(&payload, "angles");

    assert_eq!(angles.len(), 4);

    for code in ["ascendant", "descendant", "mc", "ic"] {
        assert!(
            angles.iter().any(|angle| angle["angle_code"] == code),
            "missing angle {code}"
        );
    }

    assert_angle_opposite(angles, "ascendant", "descendant");
    assert_angle_opposite(angles, "descendant", "ascendant");
    assert_angle_opposite(angles, "mc", "ic");
    assert_angle_opposite(angles, "ic", "mc");
}

fn assert_angle_opposite(angles: &[Value], angle_code: &str, opposite_angle_code: &str) {
    let angle = angles
        .iter()
        .find(|angle| angle["angle_code"] == angle_code)
        .unwrap_or_else(|| panic!("missing angle {angle_code}"));

    assert_eq!(angle["opposite_angle_code"], opposite_angle_code);
}

#[test]
fn core_identity_contains_sun_moon_ascendant() {
    let payload = load_golden_payload();
    let core = find_slot(&payload, "reading_plan", "core_identity");

    assert_source(core, "object_position:sun");
    assert_source(core, "object_position:moon");
    assert_source_prefix(core, "angle:ascendant:sign:");
}

#[test]
fn background_contains_mc_when_mc_signal_is_active() {
    let payload = load_golden_payload();
    let mc_key = array(&payload, "signals")
        .iter()
        .filter_map(|signal| signal["signal_key"].as_str())
        .find(|signal_key| signal_key.starts_with("angle:mc:sign:"))
        .map(ToString::to_string);

    if let Some(mc_key) = mc_key {
        let background = find_slot(&payload, "reading_plan", "background_factors");
        assert_source(background, &mc_key);
    }
}

#[test]
fn angle_signal_evidence_contains_long_opposite_object_code() {
    let payload = load_golden_payload();

    let ascendant = find_signal(&payload, "angle:ascendant:sign:scorpio");
    assert_eq!(
        ascendant["evidence"]["opposite_angle_object_code"],
        "descendant"
    );

    let mc = find_signal(&payload, "angle:mc:sign:leo");
    assert_eq!(mc["evidence"]["opposite_angle_object_code"], "ic");
}

#[test]
fn no_active_angle_angle_aspect_signal() {
    let payload = load_golden_payload();
    let angle_codes = ["ascendant", "descendant", "mc", "ic"];

    for signal in array(&payload, "signals") {
        let key = string(signal, "signal_key");
        if !key.starts_with("aspect:") {
            continue;
        }

        let parts: Vec<&str> = key.split(':').collect();
        assert!(parts.len() >= 4, "invalid aspect signal key {key}");

        let is_angle_angle = angle_codes.contains(&parts[1]) && angle_codes.contains(&parts[2]);
        assert!(
            !is_angle_angle,
            "angle-angle aspect should not be active in Basic payload: {key}"
        );
    }
}

#[test]
fn preserves_non_structural_dynamic_aspect() {
    let payload = load_golden_payload();
    let signal_key = "aspect:jupiter:uranus:opposition";

    assert!(
        signal_exists(&payload, signal_key),
        "Jupiter-Uranus opposition should remain active"
    );

    let main = find_slot(&payload, "reading_plan", "main_tension_or_support");
    assert_source(main, signal_key);

    let signal = find_signal(&payload, signal_key);
    assert_eq!(signal["aspect_context"]["dynamic_quality"], "tension");
    assert_eq!(signal["aspect_context"]["primary_valence"], "polarizing");
}

#[test]
fn drafting_plan_is_aligned_with_reading_plan() {
    let payload = load_golden_payload();
    let reading = array(&payload, "reading_plan");
    let drafting = array(&payload, "drafting_plan");

    assert_eq!(reading.len(), drafting.len());

    for (reading_item, drafting_item) in reading.iter().zip(drafting.iter()) {
        assert_eq!(reading_item["slot"], drafting_item["slot"]);
        assert_eq!(
            reading_item["source_signal_keys"],
            drafting_item["source_signal_keys"]
        );
        assert_eq!(
            reading_item["primary_signal_keys"],
            drafting_item["primary_signal_keys"]
        );
        assert_eq!(
            reading_item["secondary_slot_candidates"],
            drafting_item["secondary_slot_candidates"]
        );
    }
}

#[test]
fn no_empty_reading_or_drafting_slots() {
    let payload = load_golden_payload();

    for item in array(&payload, "reading_plan") {
        assert!(
            !array(item, "source_signal_keys").is_empty(),
            "reading_plan slot has no source_signal_keys: {}",
            string(item, "slot")
        );
        assert!(
            !array(item, "primary_signal_keys").is_empty(),
            "reading_plan slot has no primary_signal_keys: {}",
            string(item, "slot")
        );
    }

    for item in array(&payload, "drafting_plan") {
        assert!(
            !array(item, "source_signal_keys").is_empty(),
            "drafting_plan slot has no source_signal_keys: {}",
            string(item, "slot")
        );
    }
}

#[test]
fn every_plan_source_exists_in_signals() {
    let payload = load_golden_payload();
    let signal_keys: HashSet<&str> = array(&payload, "signals")
        .iter()
        .map(|signal| string(signal, "signal_key"))
        .collect();

    for plan_name in ["reading_plan", "drafting_plan"] {
        for item in array(&payload, plan_name) {
            for key in source_keys(item) {
                assert!(
                    signal_keys.contains(key),
                    "{plan_name} references missing signal {key}"
                );
            }
        }
    }
}

#[test]
fn primary_signal_appears_in_only_one_reading_slot() {
    let payload = load_golden_payload();
    let mut seen = HashMap::<&str, &str>::new();

    for item in array(&payload, "reading_plan") {
        let slot = string(item, "slot");

        for key in array(item, "primary_signal_keys").iter().map(|value| {
            value
                .as_str()
                .expect("primary signal key should be a string")
        }) {
            if let Some(previous_slot) = seen.insert(key, slot) {
                panic!("primary signal {key} appears in both {previous_slot} and {slot}");
            }
        }
    }
}

#[test]
fn emphasis_refs_are_only_on_dominant_cluster_or_fallback_core_identity() {
    let payload = load_golden_payload();
    let has_dominant_cluster = array(&payload, "drafting_plan")
        .iter()
        .any(|item| item["slot"] == "dominant_cluster");

    for item in array(&payload, "drafting_plan") {
        let slot = string(item, "slot");
        let refs = &item["emphasis_refs"];
        let has_refs = !array(refs, "dominant_signs").is_empty()
            || !array(refs, "dominant_houses").is_empty()
            || !array(refs, "dominant_objects").is_empty();

        if has_refs && has_dominant_cluster {
            assert_eq!(slot, "dominant_cluster");
        } else if has_refs {
            assert_eq!(slot, "core_identity");
        }
    }
}

#[test]
fn every_drafting_item_forbids_chart_emphasis_section() {
    let payload = load_golden_payload();

    for item in array(&payload, "drafting_plan") {
        let avoid = array(item, "avoid");
        assert!(
            avoid.iter().any(
                |value| value.as_str() == Some("turn chart_emphasis into a standalone section")
            ),
            "drafting_plan item must forbid chart_emphasis standalone section: {}",
            string(item, "slot")
        );
    }
}
