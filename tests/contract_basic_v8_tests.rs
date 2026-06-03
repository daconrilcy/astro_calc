use std::collections::{HashMap, HashSet};
use std::fs;

use jsonschema::JSONSchema;
use serde_json::Value;

use rust_sqlx_connection_test::domain::BasicPayload;
use rust_sqlx_connection_test::runtime::is_current_basic_payload;

const GOLDEN_PAYLOAD_PATH: &str = "../tests/golden/natal_payload_v11_paris_1990.json";
const SCHEMA_PATH: &str = "schemas/natal_structured_v11.schema.json";
const PAYLOAD_UNDER_TEST_ENV: &str = "NATAL_V11_SCHEMA_PAYLOAD_PATH";
const V10_GOLDEN_PAYLOAD_PATH: &str = "../tests/golden/natal_payload_v10_paris_1990.json";
const V10_SCHEMA_PATH: &str = "schemas/natal_structured_v10.schema.json";
const V8_GOLDEN_PAYLOAD_PATH: &str = "../tests/golden/basic_payload_v8_paris_1990.json";
const V8_SCHEMA_PATH: &str = "schemas/basic_natal_structured_v8.schema.json";
const V8_PAYLOAD_UNDER_TEST_ENV: &str = "BASIC_V8_SCHEMA_PAYLOAD_PATH";

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
    validate_with_schema_at(payload_json, SCHEMA_PATH)
}

fn validate_with_schema_at(payload_json: &Value, schema_path: &str) -> Vec<String> {
    let schema_raw = fs::read_to_string(schema_path).expect("schema should exist");
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

fn find_axis<'a>(payload: &'a Value, axis_code: &str) -> &'a Value {
    array(payload, "house_axis_emphasis")
        .iter()
        .find(|axis| axis["axis_code"] == axis_code)
        .unwrap_or_else(|| panic!("missing house axis {axis_code}"))
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
fn golden_payload_matches_json_schema_v11() {
    let payload_json = load_golden_payload();
    let validation_errors = validate_with_schema(&payload_json);

    assert!(
        validation_errors.is_empty(),
        "golden payload does not match natal_structured_v11 schema:\n{}",
        validation_errors.join("\n")
    );
}

#[test]
fn historical_v10_golden_payload_matches_json_schema_v10() {
    let raw = fs::read_to_string(V10_GOLDEN_PAYLOAD_PATH).expect("v10 golden payload should exist");
    let payload_json: Value =
        serde_json::from_str(&raw).expect("v10 golden payload should be JSON");
    let validation_errors = validate_with_schema_at(&payload_json, V10_SCHEMA_PATH);

    assert!(
        validation_errors.is_empty(),
        "historical v10 golden payload does not match natal_structured_v10 schema:\n{}",
        validation_errors.join("\n")
    );
}

#[test]
fn historical_v8_golden_payload_matches_json_schema_v8() {
    let raw = fs::read_to_string(V8_GOLDEN_PAYLOAD_PATH).expect("v8 golden payload should exist");
    let payload_json: Value = serde_json::from_str(&raw).expect("v8 golden payload should be JSON");
    let validation_errors = validate_with_schema_at(&payload_json, V8_SCHEMA_PATH);

    assert!(
        validation_errors.is_empty(),
        "historical v8 golden payload does not match basic_natal_structured_v8 schema:\n{}",
        validation_errors.join("\n")
    );
}

#[test]
fn external_payload_matches_json_schema_v11_when_requested() {
    let Ok(path) = std::env::var(PAYLOAD_UNDER_TEST_ENV) else {
        return;
    };
    let payload_json = load_payload_from_path(&path);
    let validation_errors = validate_with_schema(&payload_json);

    assert!(
        validation_errors.is_empty(),
        "external payload does not match natal_structured_v11 schema:\n{}",
        validation_errors.join("\n")
    );
}

#[test]
fn external_payload_matches_json_schema_v8_when_requested() {
    let Ok(path) = std::env::var(V8_PAYLOAD_UNDER_TEST_ENV) else {
        return;
    };
    let payload_json = load_payload_from_path(&path);
    let validation_errors = validate_with_schema_at(&payload_json, V8_SCHEMA_PATH);

    assert!(
        validation_errors.is_empty(),
        "external payload does not match basic_natal_structured_v8 schema:\n{}",
        validation_errors.join("\n")
    );
}

#[test]
fn schema_rejects_llm_handoff_contract_property() {
    let mut payload = load_golden_payload();
    payload["llm_handoff_contract"] = serde_json::json!({
        "contract_version": "natal_structured_v11"
    });

    assert!(
        !validate_with_schema(&payload).is_empty(),
        "schema should reject llm_handoff_contract"
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
fn schema_rejects_mobile_visibility_context_without_calculated_altitude() {
    let mut payload = load_golden_payload();
    let position = payload["positions"]
        .as_array_mut()
        .expect("positions should be an array")
        .iter_mut()
        .find(|position| {
            position["object_context"]["role"]
                .as_str()
                .is_some_and(|role| role != "angle")
        })
        .expect("golden payload should contain a mobile position");
    position["visibility_context"]["altitude_deg"] = Value::Null;
    position["visibility_context"]["is_visible"] = Value::Null;

    assert!(
        !validate_with_schema(&payload).is_empty(),
        "schema should reject null altitude/is_visible on mobile positions"
    );
}

#[test]
fn schema_rejects_mobile_signal_visibility_context_without_calculated_altitude() {
    let mut payload = load_golden_payload();
    let signal = payload["signals"]
        .as_array_mut()
        .expect("signals should be an array")
        .iter_mut()
        .find(|signal| {
            signal["signal_key"]
                .as_str()
                .is_some_and(|key| key.starts_with("object_position:"))
                && signal["evidence"]["placement_context"]["object_context"]["role"]
                    .as_str()
                    .is_some_and(|role| role != "angle")
        })
        .expect("golden payload should contain a mobile placement signal");
    let visibility = &mut signal["evidence"]["placement_context"]["visibility_context"];
    visibility["altitude_deg"] = Value::Null;
    visibility["is_visible"] = Value::Null;

    assert!(
        !validate_with_schema(&payload).is_empty(),
        "schema should reject null altitude/is_visible on mobile placement signals"
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
fn schema_rejects_invalid_axis_house_pair() {
    let mut payload = load_golden_payload();
    payload["house_axis_emphasis"][0]["houses"] = serde_json::json!([2, 7]);

    assert!(
        !validate_with_schema(&payload).is_empty(),
        "schema should reject a non-canonical house pair for an axis"
    );
}

#[test]
fn schema_rejects_axis_score_out_of_range() {
    let mut payload = load_golden_payload();
    payload["house_axis_emphasis"][0]["axis_score"] = serde_json::json!(1.1);

    assert!(
        !validate_with_schema(&payload).is_empty(),
        "schema should reject axis_score greater than 1"
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
fn runtime_rejects_v10_payload_contract_version() {
    let mut payload = load_golden_payload();
    payload["chart_context"]["payload_contract"]["contract_version"] =
        Value::String("natal_structured_v10".to_string());

    let parsed: BasicPayload =
        serde_json::from_value(payload).expect("modified payload should deserialize");

    assert!(!is_current_basic_payload(&parsed));
}

#[test]
fn runtime_rejects_v10_without_house_axis_emphasis() {
    let raw = fs::read_to_string(V10_GOLDEN_PAYLOAD_PATH).expect("v10 golden payload should exist");
    let payload: BasicPayload =
        serde_json::from_str(&raw).expect("v10 golden payload should deserialize");

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn runtime_rejects_axis_source_signal_key_that_does_not_exist() {
    let mut payload = load_golden_payload();
    payload["house_axis_emphasis"][0]["source_signal_keys"]
        .as_array_mut()
        .expect("source_signal_keys should be an array")
        .push(Value::String("object_position:not_active".to_string()));
    let parsed: BasicPayload =
        serde_json::from_value(payload).expect("modified payload should deserialize");

    assert!(!is_current_basic_payload(&parsed));
}

#[test]
fn runtime_rejects_axis_theme_codes_that_do_not_match_canonical_axis() {
    let mut payload = load_golden_payload();
    payload["house_axis_emphasis"][0]["theme_codes"] =
        serde_json::json!(["identity", "relationships"]);
    let parsed: BasicPayload =
        serde_json::from_value(payload).expect("modified payload should deserialize");

    assert!(!is_current_basic_payload(&parsed));
}

#[test]
fn runtime_rejects_axis_primary_house_inconsistent_with_house_scores() {
    let mut payload = load_golden_payload();
    payload["house_axis_emphasis"][0]["primary_house"] = serde_json::json!(8);
    payload["house_axis_emphasis"][0]["secondary_house"] = serde_json::json!(2);
    let parsed: BasicPayload =
        serde_json::from_value(payload).expect("modified payload should deserialize");

    assert!(!is_current_basic_payload(&parsed));
}

#[test]
fn runtime_rejects_axis_score_inconsistent_with_house_scores() {
    let mut payload = load_golden_payload();
    payload["house_axis_emphasis"][0]["axis_score"] = serde_json::json!(0.5);
    let parsed: BasicPayload =
        serde_json::from_value(payload).expect("modified payload should deserialize");

    assert!(!is_current_basic_payload(&parsed));
}

#[test]
fn runtime_rejects_duplicate_axis_source_signal_keys() {
    let mut payload = load_golden_payload();
    let duplicate = payload["house_axis_emphasis"][0]["source_signal_keys"][0].clone();
    payload["house_axis_emphasis"][0]["source_signal_keys"]
        .as_array_mut()
        .expect("source_signal_keys should be an array")
        .push(duplicate);
    let parsed: BasicPayload =
        serde_json::from_value(payload).expect("modified payload should deserialize");

    assert!(!is_current_basic_payload(&parsed));
}

#[test]
fn v11_payload_omits_llm_and_drafting_instructions() {
    let payload = load_golden_payload();

    assert!(payload.get("llm_handoff_contract").is_none());
    assert!(payload.get("drafting_plan").is_none());
    assert!(payload["chart_context"]["payload_contract"]
        .get("writing_contract")
        .is_none());
    assert!(array(&payload, "signals")
        .iter()
        .all(|signal| signal.get("writing_guidance").is_none()));
}

#[test]
fn v11_contains_house_axis_emphasis() {
    let payload = load_golden_payload();
    let axes = array(&payload, "house_axis_emphasis");

    assert!(!axes.is_empty());
    assert!(axes.len() <= 3);
}

#[test]
fn resources_sharing_axis_is_detected() {
    let payload = load_golden_payload();
    let axis = find_axis(&payload, "resources_sharing");

    assert_eq!(axis["houses"], serde_json::json!([2, 8]));
    assert_eq!(axis["primary_house"], 2);
    assert!(array(axis, "source_signal_keys")
        .iter()
        .any(|value| value == "cluster:capricorn:house_2"));
}

#[test]
fn self_relationship_axis_is_detected() {
    let payload = load_golden_payload();
    let axis = find_axis(&payload, "self_relationship");

    assert_eq!(axis["houses"], serde_json::json!([1, 7]));
    assert_eq!(axis["primary_house"], 1);
    assert!(array(axis, "source_signal_keys")
        .iter()
        .any(|value| value == "angle:ascendant:sign:scorpio"));
}

#[test]
fn axis_source_signal_keys_exist() {
    let payload = load_golden_payload();
    let signal_keys: HashSet<&str> = array(&payload, "signals")
        .iter()
        .map(|signal| string(signal, "signal_key"))
        .collect();

    for axis in array(&payload, "house_axis_emphasis") {
        for key in source_keys(axis) {
            assert!(
                signal_keys.contains(key),
                "house axis references missing signal {key}"
            );
        }
    }
}

#[test]
fn axis_scores_are_sorted_desc() {
    let payload = load_golden_payload();
    let mut previous = f64::INFINITY;

    for axis in array(&payload, "house_axis_emphasis") {
        let score = axis["axis_score"]
            .as_f64()
            .expect("axis_score should be a number");
        assert!(score <= previous);
        previous = score;
    }
}

#[test]
fn axis_count_is_limited_to_three() {
    let payload = load_golden_payload();

    assert!(array(&payload, "house_axis_emphasis").len() <= 3);
}

#[test]
fn axis_houses_are_opposites() {
    let payload = load_golden_payload();

    for axis in array(&payload, "house_axis_emphasis") {
        let houses = array(axis, "houses");
        let first = houses[0].as_i64().expect("house should be an integer");
        let second = houses[1].as_i64().expect("house should be an integer");
        assert_eq!(first + 6, second);
    }
}

#[test]
fn v11_contains_four_canonical_angles() {
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

#[test]
fn v11_rulership_sources_map_doctrines_to_ruler_objects() {
    let payload = load_golden_payload();
    let rulership = &payload["rulership_context"];

    for key in ["ascendant_ruler", "mc_ruler"] {
        assert_ruler_sources_map_to_objects(&rulership[key]);
    }
    for key in ["dominant_house_rulers", "dominant_sign_rulers"] {
        for context in array(rulership, key) {
            assert_ruler_sources_map_to_objects(context);
        }
    }
    for link in array(rulership, "dispositor_links") {
        for source in array(link, "ruler_sources") {
            assert!(
                source["object_code"]
                    .as_str()
                    .is_some_and(|value| !value.trim().is_empty()),
                "dispositor source should expose its ruler object_code"
            );
        }
    }
}

#[test]
fn v11_rulership_routes_mc_and_uses_consistent_modern_scorpio_ruler() {
    let payload = load_golden_payload();
    let rulership = &payload["rulership_context"];
    let ascendant_ruler = &rulership["ascendant_ruler"];

    assert_eq!(ascendant_ruler["sign_code"], "scorpio");
    assert!(array(ascendant_ruler, "ruler_object_codes")
        .iter()
        .any(|value| value == "pluto"));
    assert!(!array(ascendant_ruler, "ruler_object_codes")
        .iter()
        .any(|value| value == "uranus"));
    assert!(array(ascendant_ruler, "ruler_sources")
        .iter()
        .any(|source| {
            source["astral_system_code"] == "modern" && source["object_code"] == "pluto"
        }));

    assert!(rulership["mc_ruler"].is_object());
}

#[test]
fn v11_rulership_uses_current_modern_outer_planet_rulers() {
    let payload = load_golden_payload();
    let rulership = &payload["rulership_context"];
    let ascendant_ruler = &rulership["ascendant_ruler"];

    assert_eq!(ascendant_ruler["sign_code"], "scorpio");
    assert!(array(ascendant_ruler, "ruler_object_codes")
        .iter()
        .any(|value| value == "mars"));
    assert!(array(ascendant_ruler, "ruler_object_codes")
        .iter()
        .any(|value| value == "pluto"));
    assert_modern_ruler_source(ascendant_ruler, "pluto");

    assert_modern_ruler_source(find_dispositor_link(rulership, "pluto", "scorpio"), "pluto");
    assert_modern_ruler_source(find_dispositor_link(rulership, "moon", "pisces"), "neptune");
    assert_modern_ruler_source(
        find_dispositor_link(rulership, "venus", "aquarius"),
        "uranus",
    );
}

#[test]
fn v11_rulership_splits_final_dispositors_from_mutual_receptions() {
    let payload = load_golden_payload();
    let rulership = &payload["rulership_context"];

    let final_dispositors = array(rulership, "final_dispositors");
    assert!(!final_dispositors.is_empty());
    for final_dispositor in final_dispositors {
        assert!(
            final_dispositor.get("disposition_type").is_none(),
            "final_dispositors must not carry mutual_reception/cycle endpoints"
        );
        assert!(
            !array(final_dispositor, "source_objects").is_empty(),
            "final_dispositor should keep source_objects"
        );
    }

    let mutual_receptions = array(rulership, "mutual_receptions");
    assert!(
        !mutual_receptions.is_empty(),
        "mutual receptions should be exposed separately"
    );
    for reception in mutual_receptions {
        assert_eq!(array(reception, "object_codes").len(), 2);
        assert!(!array(reception, "source_objects").is_empty());
    }
}

#[test]
fn runtime_rejects_final_dispositors_not_matching_chains() {
    let mut payload = load_golden_payload();
    payload["rulership_context"]["final_dispositors"][0]["source_objects"]
        .as_array_mut()
        .expect("source_objects should be an array")
        .push(Value::String("moon".to_string()));

    let parsed: BasicPayload =
        serde_json::from_value(payload).expect("modified payload should deserialize");

    assert!(!is_current_basic_payload(&parsed));
}

#[test]
fn runtime_rejects_mutual_receptions_not_matching_chains() {
    let mut payload = load_golden_payload();
    payload["rulership_context"]["mutual_receptions"][0]["source_objects"]
        .as_array_mut()
        .expect("source_objects should be an array")
        .retain(|value| value != "moon");

    let parsed: BasicPayload =
        serde_json::from_value(payload).expect("modified payload should deserialize");

    assert!(!is_current_basic_payload(&parsed));
}

fn assert_ruler_sources_map_to_objects(context: &Value) {
    let ruler_object_codes = array(context, "ruler_object_codes")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("ruler object code should be a string")
        })
        .collect::<HashSet<_>>();

    for source in array(context, "ruler_sources") {
        let object_code = source["object_code"]
            .as_str()
            .expect("ruler source should expose object_code");
        assert!(
            ruler_object_codes.contains(object_code),
            "ruler source object_code {object_code} should be listed in ruler_object_codes"
        );
    }
}

fn find_dispositor_link<'a>(
    rulership: &'a Value,
    object_code: &str,
    object_sign_code: &str,
) -> &'a Value {
    array(rulership, "dispositor_links")
        .iter()
        .find(|link| {
            link["object_code"] == object_code && link["object_sign_code"] == object_sign_code
        })
        .unwrap_or_else(|| panic!("missing dispositor link {object_code}/{object_sign_code}"))
}

fn assert_modern_ruler_source(context: &Value, expected_object_code: &str) {
    assert!(
        array(context, "ruler_sources").iter().any(|source| {
            source["astral_system_code"] == "modern"
                && source["object_code"] == expected_object_code
        }),
        "missing modern ruler source {expected_object_code}"
    );
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
fn no_empty_reading_slots() {
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
}

#[test]
fn every_plan_source_exists_in_signals() {
    let payload = load_golden_payload();
    let signal_keys: HashSet<&str> = array(&payload, "signals")
        .iter()
        .map(|signal| string(signal, "signal_key"))
        .collect();

    for item in array(&payload, "reading_plan") {
        for key in source_keys(item) {
            assert!(
                signal_keys.contains(key),
                "reading_plan references missing signal {key}"
            );
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
