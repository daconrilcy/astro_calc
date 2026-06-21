#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

pub fn collect_rs_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rs_files_recursive(root, &mut files);
    files
}

fn collect_rs_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files_recursive(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

pub fn collect_governance_text_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_governance_text_files_recursive(root, &mut files);
    files
}

fn collect_governance_text_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_governance_text_files_recursive(&path, files);
            continue;
        }

        let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };

        if matches!(
            extension,
            "rs" | "ps1" | "js" | "md" | "yaml" | "yml" | "toml"
        ) {
            files.push(path);
        }
    }
}

pub fn allows_legacy_calculator_route_reference(relative_path: &Path) -> bool {
    let path = relative_path.to_string_lossy().replace('\\', "/");
    matches!(
        path.as_str(),
        "astral_calculator_http/src/routes.rs"
            | "contracts/README.md"
            | "contracts/calculator/openapi.yaml"
            | "docs/BASIC_PAYLOAD_IMPLEMENTATION.md"
            | "docs/GUIDE_DEBUTANT_DOCKER.md"
            | "docs/integration_api_contract.md"
            | "docs/integration_api_guide.md"
            | "tests/astral_calculator_http_tests.rs"
            | "tests/refactor_governance_review_tests.rs"
            | "tests/refactor_governance_runtime_tests.rs"
            | "tests/refactor_governance_tests.rs"
    ) || path.starts_with("docs/reviews/")
}

pub fn read(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}
