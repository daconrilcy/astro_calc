use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use astral_llm_application::SchemaRegistry;
use astral_llm_domain::{GenerateReadingRequest, GenerateReadingResponse};
use schemars::schema_for;

#[test]
fn integration_schemas_exist_in_contracts_dir() {
    let root = repo_root();
    let dir = root.join("contracts/llm");
    let files = [
        "integration_job_request_v1.schema.json",
        "integration_job_response_v1.schema.json",
        "integration_job_status_v1.schema.json",
        "integration_service_v1.schema.json",
        "integration_service_contract_v1.schema.json",
    ];
    for file in files {
        let path = dir.join(file);
        assert!(
            path.exists(),
            "missing integration schema: {}",
            path.display()
        );
        let _: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("json");
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root")
}

#[test]
fn calculator_schemas_exist_in_contracts_dir() {
    let root = repo_root();
    let dir = root.join("contracts/calculator");

    let files = [
        "astro_engine_request_v1.schema.json",
        "astro_engine_response_v1.schema.json",
        "natal_structured_v13.schema.json",
        "llm_projection_natal_v1.schema.json",
        "astro_simplified_natal_request_v1.schema.json",
        "astro_simplified_natal_response_v1.schema.json",
        "natal_simplified_structured_v1.schema.json",
        "llm_projection_natal_simplified_v1.schema.json",
        "horoscope_calculation_request.schema.json",
        "horoscope_calculation_response.schema.json",
        "horoscope_period_calculation_request.schema.json",
        "horoscope_period_calculation_response.schema.json",
    ];

    for file in files {
        let path = dir.join(file);
        assert!(
            path.exists(),
            "missing calculator schema: {}",
            path.display()
        );
        let _: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("json");
    }
}

#[test]
fn llm_schemas_match_published_contracts() {
    let root = repo_root();
    let published = root.join("contracts/llm");

    let expected: HashMap<&str, &str> = HashMap::from([
        (
            "generate_reading_request_v1.schema.json",
            "generate_reading_request_v1",
        ),
        (
            "generate_reading_response_v1.schema.json",
            "generate_reading_response_v1",
        ),
        ("natal_reading_v1.schema.json", "natal_reading_v1"),
        ("chapter_provider_v1.schema.json", "chapter_provider_v1"),
        ("summary_provider_v1.schema.json", "summary_provider_v1"),
    ]);

    for (file, key) in expected {
        let path = published.join(file);
        assert!(path.exists(), "missing published LLM schema: {file}");
        let fresh = fresh_llm_schema(key);
        let committed: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read schema")).expect("json");
        assert_eq!(fresh, committed, "LLM schema drift for {key}");
    }
}

fn fresh_llm_schema(key: &str) -> serde_json::Value {
    match key {
        "generate_reading_request_v1" => {
            serde_json::to_value(schema_for!(GenerateReadingRequest)).expect("schema")
        }
        "generate_reading_response_v1" => {
            serde_json::to_value(schema_for!(GenerateReadingResponse)).expect("schema")
        }
        "natal_reading_v1" => SchemaRegistry::new()
            .get("natal_reading_v1")
            .cloned()
            .expect("natal_reading_v1"),
        "chapter_provider_v1" => SchemaRegistry::new()
            .get("chapter_provider_v1")
            .cloned()
            .expect("chapter_provider_v1"),
        "summary_provider_v1" => SchemaRegistry::new()
            .get("summary_provider_v1")
            .cloned()
            .expect("summary_provider_v1"),
        other => panic!("unknown schema key: {other}"),
    }
}

#[test]
#[ignore = "run manually to refresh contracts/llm/*.schema.json"]
fn export_llm_schemas() {
    let root = repo_root();
    let out = root.join("contracts/llm");
    fs::create_dir_all(&out).expect("mkdir");

    let exports = [
        (
            "generate_reading_request_v1.schema.json",
            fresh_llm_schema("generate_reading_request_v1"),
        ),
        (
            "generate_reading_response_v1.schema.json",
            fresh_llm_schema("generate_reading_response_v1"),
        ),
        (
            "natal_reading_v1.schema.json",
            fresh_llm_schema("natal_reading_v1"),
        ),
        (
            "summary_provider_v1.schema.json",
            fresh_llm_schema("summary_provider_v1"),
        ),
        (
            "chapter_provider_v1.schema.json",
            fresh_llm_schema("chapter_provider_v1"),
        ),
    ];

    for (name, schema) in exports {
        let path = out.join(name);
        fs::write(
            &path,
            serde_json::to_string_pretty(&schema).expect("serialize"),
        )
        .expect("write");
        eprintln!("exported {}", path.display());
    }
}

#[test]
fn llm_openapi_excludes_removed_sync_legacy_routes() {
    let root = repo_root();
    let openapi = fs::read_to_string(root.join("contracts/llm/openapi.yaml")).expect("read");
    assert!(
        !openapi.contains("/v1/readings/generate:"),
        "generate route must be removed from published OpenAPI"
    );
    assert!(
        !openapi.contains("/v1/readings/natal/simplified:"),
        "simplified natal route must be removed from published OpenAPI"
    );
}

#[test]
fn integration_service_public_schema_excludes_legacy_sync_fields() {
    let root = repo_root();
    let schema: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("contracts/llm/integration_service_v1.schema.json"))
            .expect("read schema"),
    )
    .expect("json");

    let required = schema["required"].as_array().expect("required array");
    assert!(
        !required
            .iter()
            .any(|value| value.as_str() == Some("supports_sync_legacy")),
        "public schema must not require supports_sync_legacy"
    );
    assert!(
        schema["properties"].get("supports_sync_legacy").is_none(),
        "public schema must not expose supports_sync_legacy"
    );
    assert!(
        schema["properties"]["endpoints"]["properties"]
            .get("submit_sync_legacy")
            .is_none(),
        "public schema must not expose endpoints.submit_sync_legacy"
    );
}
