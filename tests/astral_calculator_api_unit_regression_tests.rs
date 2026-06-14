use std::{
    fs,
    path::PathBuf,
};

use astral_calculator_api::AppConfig;
use astral_calculator_api::config::validate_path_within;

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("astral_calculator_api_unit_regression_tests");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn validate_path_within_accepts_child_and_rejects_escape() {
    let dir = temp_dir();
    let child = dir.join("openapi.yaml");
    fs::write(&child, "openapi: 3.1.0").expect("write child");
    validate_path_within(&child, &dir).expect("child path allowed");

    let outside_parent = dir.parent().expect("parent");
    let escaped = outside_parent.join("escaped.yaml");
    let err = validate_path_within(&escaped, &dir).expect_err("escape must fail");
    assert!(err.contains("escapes allowed directory"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn calculator_api_requires_auth_only_for_non_empty_keys() {
    let config = AppConfig {
        bind_addr: "127.0.0.1:8080".parse().expect("socket"),
        allow_public_bind: false,
        api_key: Some(" secret ".into()),
        max_body_bytes: 1,
        request_timeout_ms: 1,
        schemas_dir: PathBuf::from("."),
        openapi_path: PathBuf::from("./openapi.yaml"),
    };
    assert!(config.requires_auth());

    let no_auth = AppConfig {
        api_key: Some("   ".into()),
        ..config
    };
    assert!(!no_auth.requires_auth());
}
