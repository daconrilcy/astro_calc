use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn collect_rs_files(root: &Path) -> Vec<PathBuf> {
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

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

#[test]
fn domain_does_not_import_infra_db_models() {
    let root = workspace_root().join("astral_calculator/src/domain");
    for file in collect_rs_files(&root) {
        let content = read(&file);
        assert!(
            !content.contains("crate::infra::db::models"),
            "domain file {} imports infra::db::models",
            file.display()
        );
        assert!(
            !content.contains("crate::infra::"),
            "domain file {} imports infra",
            file.display()
        );
    }
}

#[test]
fn non_infra_source_does_not_import_sqlx_models_directly() {
    let root = workspace_root().join("astral_calculator/src");
    for file in collect_rs_files(&root) {
        if file.starts_with(root.join("infra/db")) {
            continue;
        }

        let content = read(&file);
        assert!(
            !content.contains("use crate::infra::db::models"),
            "{} imports crate::infra::db::models directly",
            file.display()
        );
        assert!(
            !content.contains("crate::infra::db::models::"),
            "{} references crate::infra::db::models directly",
            file.display()
        );
    }
}
