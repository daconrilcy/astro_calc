use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use astral_llm_application::SchemaRegistry;
use astral_llm_domain::{GenerateReadingRequest, GenerateReadingResponse};
use schemars::schema_for;
use sha2::{Digest, Sha256};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..").join("..").join("..")
        .canonicalize()
        .expect("repo root")
}

fn sha256_file(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|_| panic!("missing file: {}", path.display()));
    format!("{:x}", Sha256::digest(bytes))
}

fn assert_schemas_match(source: &Path, published: &Path) {
    assert!(
        published.exists(),
        "published schema missing: {}",
        published.display()
    );
    assert_eq!(
        sha256_file(source),
        sha256_file(published),
        "schema drift: {} vs {}",
        source.display(),
        published.display()
    );
}

#[test]
fn calculator_schemas_match_published_contracts() {
    let root = repo_root();
    let source_dir = root.join("astral_calculator/schemas");
    let published_dir = root.join("contracts/calculator");

    let pairs = [
        ("astro_engine_request_v1.schema.json", "astro_engine_request_v1.schema.json"),
        ("astro_engine_response_v1.schema.json", "astro_engine_response_v1.schema.json"),
        ("natal_structured_v13.schema.json", "natal_structured_v13.schema.json"),
        ("llm_projection_natal_v1.schema.json", "llm_projection_natal_v1.schema.json"),
        ("astro_simplified_natal_request_v1.schema.json", "astro_simplified_natal_request_v1.schema.json"),
        ("astro_simplified_natal_response_v1.schema.json", "astro_simplified_natal_response_v1.schema.json"),
        ("natal_simplified_structured_v1.schema.json", "natal_simplified_structured_v1.schema.json"),
        ("llm_projection_natal_simplified_v1.schema.json", "llm_projection_natal_simplified_v1.schema.json"),
    ];

    for (src, dst) in pairs {
        assert_schemas_match(&source_dir.join(src), &published_dir.join(dst));
    }
}

#[test]
fn llm_schemas_match_published_contracts() {
    let root = repo_root();
    let published = root.join("contracts/llm");

    let expected: HashMap<&str, &str> = HashMap::from([
        ("generate_reading_request_v1.schema.json", "generate_reading_request_v1"),
        ("generate_reading_response_v1.schema.json", "generate_reading_response_v1"),
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

