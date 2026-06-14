//! Tests catalogue API d'intégration (schémas + domaine).

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use astral_llm_domain::integration::{CalculationMode, IntegrationService, ServiceAvailability};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root")
}

fn load_seed_services() -> HashMap<String, IntegrationService> {
    let path = repo_root().join("json_db/llm_integration_services.json");
    let raw: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("read seed")).expect("json");
    raw.get("data")
        .and_then(|v| v.as_array())
        .expect("data array")
        .iter()
        .filter_map(|row| {
            let service_code = row.get("service_code")?.as_str()?.to_string();
            Some((
                service_code.clone(),
                IntegrationService {
                    service_code,
                    profile_code: row["profile_code"].as_str()?.into(),
                    product_code: row["product_code"].as_str()?.into(),
                    label_fr: row["label_fr"].as_str()?.into(),
                    description_fr: row["description_fr"].as_str()?.into(),
                    orchestration_mode: row["orchestration_mode"].as_str()?.into(),
                    orchestration_mode_typed: None,
                    calculation_mode: CalculationMode::parse(row["calculation_mode"].as_str()?)?,
                    service_request_contract: row["service_request_contract"].as_str()?.into(),
                    payload_contract: row["payload_contract"].as_str()?.into(),
                    service_response_contract: row["service_response_contract"].as_str()?.into(),
                    public_request_contract: row
                        .get("public_request_contract")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    calculator_request_contract: row
                        .get("calculator_request_contract")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    llm_request_contract: row
                        .get("llm_request_contract")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    public_response_contract: row
                        .get("public_response_contract")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    calculation_output_contract: row
                        .get("calculation_output_contract")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    reading_output_contract: row["reading_output_contract"].as_str()?.into(),
                    sync_endpoint: row
                        .get("sync_endpoint")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    async_endpoint: row["async_endpoint"].as_str()?.into(),
                    supports_async: row["supports_async"].as_bool()?,
                    supports_sync_legacy: row["supports_sync_legacy"].as_bool()?,
                    supports_mercure: row["supports_mercure"].as_bool()?,
                    availability: ServiceAvailability::parse(row["availability"].as_str()?)?,
                    example_request_json: row.get("example_request_json").cloned(),
                    sort_order: row["sort_order"].as_i64()? as i16,
                },
            ))
        })
        .collect()
}

#[test]
fn integration_schemas_exist_and_parse() {
    let dir = repo_root().join("contracts/llm");
    let files = [
        "integration_job_request_v1.schema.json",
        "integration_job_response_v1.schema.json",
        "integration_job_status_v1.schema.json",
        "integration_service_v1.schema.json",
        "integration_service_contract_v1.schema.json",
    ];
    for file in files {
        let path = dir.join(file);
        assert!(path.exists(), "missing schema: {}", path.display());
        let raw = fs::read_to_string(&path).expect("read schema");
        let _: serde_json::Value = serde_json::from_str(&raw).expect("valid json");
    }
}

#[test]
fn integration_service_schemas_publish_api_surface() {
    let dir = repo_root().join("contracts/llm");
    for file in [
        "integration_service_v1.schema.json",
        "integration_service_contract_v1.schema.json",
    ] {
        let schema: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join(file)).expect("read schema"))
                .expect("valid json");
        assert!(
            schema
                .get("properties")
                .and_then(|v| v.get("api_surface"))
                .is_some(),
            "{file} must expose api_surface"
        );
        let required = schema
            .get("required")
            .and_then(|v| v.as_array())
            .expect("required array");
        assert!(
            required
                .iter()
                .any(|value| value.as_str() == Some("api_surface")),
            "{file} must require api_surface"
        );
    }
}

#[test]
fn integration_service_public_schema_drops_legacy_sync_fields() {
    let path = repo_root().join("contracts/llm/integration_service_v1.schema.json");
    let schema: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(path).expect("read schema")).expect("valid json");

    let required = schema
        .get("required")
        .and_then(|v| v.as_array())
        .expect("required array");
    assert!(
        !required
            .iter()
            .any(|value| value.as_str() == Some("supports_sync_legacy")),
        "public service schema must no longer require supports_sync_legacy"
    );

    let properties = schema.get("properties").expect("properties");
    assert!(
        properties.get("supports_sync_legacy").is_none(),
        "public service schema must no longer publish supports_sync_legacy"
    );
    assert!(
        properties
            .get("endpoints")
            .and_then(|v| v.get("properties"))
            .and_then(|v| v.get("submit_sync_legacy"))
            .is_none(),
        "public service schema must no longer publish endpoints.submit_sync_legacy"
    );
}

#[test]
fn integration_job_request_payload_is_opaque_object() {
    let path = repo_root().join("contracts/llm/integration_job_request_v1.schema.json");
    let schema: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(path).expect("read")).expect("json");
    let payload = schema
        .pointer("/properties/payload")
        .expect("payload property");
    assert_eq!(payload.get("type").and_then(|v| v.as_str()), Some("object"));
    assert!(
        payload.get("$ref").is_none(),
        "payload must not $ref business contracts"
    );
}

#[test]
fn seed_integration_services_loads_natal_simplified_active() {
    let services = load_seed_services();
    let simplified = services
        .get("natal_simplified")
        .expect("natal_simplified in seed");
    assert_eq!(simplified.availability, ServiceAvailability::Active);
    assert!(simplified.supports_async);
    assert_eq!(
        simplified.payload_contract,
        "astro_simplified_natal_request_v1"
    );
}

#[test]
fn availability_public_listing_rules() {
    assert!(ServiceAvailability::Active.is_public_listed(false));
    assert!(ServiceAvailability::Beta.is_public_listed(false));
    assert!(!ServiceAvailability::Planned.is_public_listed(false));
    assert!(ServiceAvailability::Planned.is_public_listed(true));
}

#[test]
fn from_payload_services_have_fixed_profile() {
    let services = load_seed_services();
    let expected = [
        ("natal_light_from_payload", "natal_light"),
        ("natal_basic_from_payload", "natal_basic"),
        ("natal_premium_from_payload", "natal_premium"),
        ("natal_premium_plus_from_payload", "natal_premium_plus"),
        ("natal_simplified_from_payload", "natal_simplified"),
    ];

    for (service_code, profile_code) in expected {
        let service = services
            .get(service_code)
            .unwrap_or_else(|| panic!("{service_code}"));
        assert!(service.is_from_payload());
        assert_eq!(service.profile_code, profile_code);
        assert_eq!(service.payload_contract, "generate_reading_request_v1");
        assert_eq!(service.availability, ServiceAvailability::Deprecated);
    }
}

#[test]
fn natal_basic_full_natal_active_in_seed() {
    let services = load_seed_services();
    let basic = services.get("natal_basic").expect("natal_basic");
    assert_eq!(basic.availability, ServiceAvailability::Active);
    assert_eq!(basic.calculation_mode, CalculationMode::FullNatal);
}

#[test]
fn natal_premium_full_natal_beta_in_seed() {
    let services = load_seed_services();
    let premium = services.get("natal_premium").expect("natal_premium");
    assert_eq!(premium.availability, ServiceAvailability::Beta);
    assert_eq!(premium.calculation_mode, CalculationMode::FullNatal);
    assert!(premium.availability.is_public_listed(false));
}

#[test]
fn natal_simplified_supports_mercure_in_seed() {
    let services = load_seed_services();
    assert!(services.get("natal_simplified").unwrap().supports_mercure);
}

#[test]
fn premium_next_7_days_catalog_exposes_v2_ui_entry_without_contract_change() {
    let services = load_seed_services();
    let premium = services
        .get("horoscope_premium_next_7_days_natal")
        .expect("horoscope_premium_next_7_days_natal in seed");

    assert_eq!(premium.label_fr, "Horoscope Premium 7 prochains jours V2");
    assert_eq!(premium.payload_contract, "horoscope_period_natal_request");
    assert_eq!(premium.reading_output_contract, "horoscope_period_response");
    assert_eq!(premium.availability, ServiceAvailability::Beta);
    assert_eq!(premium.sort_order, 240);

    let payload = premium
        .example_request_json
        .as_ref()
        .and_then(|json| json.get("payload"))
        .expect("premium example payload");
    assert_eq!(
        payload.get("target_language_code").and_then(|v| v.as_str()),
        Some("fr")
    );
    assert!(
        payload.get("target_language").is_none(),
        "premium V2 example must not expose legacy target_language"
    );
}
