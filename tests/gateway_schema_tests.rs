use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn gateway_common_and_public_schemas_exist_and_parse() {
    let root = repo_root();
    let files = [
        root.join("contracts/common/request_context_common_v1.schema.json"),
        root.join("contracts/common/location_common_v1.schema.json"),
        root.join("contracts/common/birth_input_common_v1.schema.json"),
        root.join("contracts/common/response_metadata_common_v1.schema.json"),
        root.join("contracts/common/quality_metadata_common_v1.schema.json"),
        root.join("contracts/common/error_response_common_v1.schema.json"),
        root.join("contracts/public/natal_reading_request_v2.schema.json"),
        root.join("contracts/public/natal_reading_response_v2.schema.json"),
    ];

    for path in files {
        assert!(path.exists(), "missing schema: {}", path.display());
        let raw = fs::read_to_string(&path).expect("read schema");
        let _: serde_json::Value = serde_json::from_str(&raw).expect("valid json");
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root")
}
