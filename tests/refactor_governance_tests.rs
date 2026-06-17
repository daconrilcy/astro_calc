use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
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
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
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

#[test]
fn business_layers_do_not_use_runtime_db_shortcuts() {
    let root = workspace_root().join("astral_calculator/src");
    let restricted_roots = [
        root.join("domain"),
        root.join("engine"),
        root.join("horoscope"),
        root.join("simplified"),
    ];

    for restricted_root in restricted_roots {
        for file in collect_rs_files(&restricted_root) {
            let content = read(&file);
            for forbidden in ["PgPool", "connect_from_env", "block_on", "run_blocking"] {
                assert!(
                    !content.contains(forbidden),
                    "{} contains forbidden runtime/db shortcut {forbidden}",
                    file.display()
                );
            }
        }
    }
}

#[test]
fn simplified_and_horoscope_do_not_import_natal_internal_calculators() {
    let root = workspace_root().join("astral_calculator/src");
    for restricted_root in [root.join("simplified"), root.join("horoscope")] {
        for file in collect_rs_files(&restricted_root) {
            let content = read(&file);
            assert!(
                !content.contains("crate::natal::aspects"),
                "{} imports crate::natal::aspects",
                file.display()
            );
            assert!(
                !content.contains("crate::natal::ephemeris"),
                "{} imports crate::natal::ephemeris",
                file.display()
            );
        }
    }
}

#[test]
fn astrology_module_exists_and_feature_shared_is_not_used_for_astrology() {
    let root = workspace_root().join("astral_calculator/src");
    let astrology_mod = root.join("astrology/mod.rs");
    assert!(
        astrology_mod.exists(),
        "missing {}",
        astrology_mod.display()
    );

    for file in collect_rs_files(&root) {
        let content = read(&file);
        assert!(
            !content.contains("features/shared"),
            "{} references forbidden features/shared path",
            file.display()
        );
    }
}

#[test]
fn feature_boundary_refactor_reviews_are_closed() {
    let review_root =
        workspace_root().join("docs/reviews/astral_calculator_refactor_feature_boundaries");
    let expected_files = [
        "REV-W00-plan.md",
        "REV-W00-adversarial.md",
        "REV-W00-followup-1.md",
        "REV-W01-plan.md",
        "REV-W01-adversarial.md",
        "REV-W01-followup-1.md",
        "REV-W02-plan.md",
        "REV-W02-adversarial.md",
        "REV-W02-followup-1.md",
        "REV-W03-plan.md",
        "REV-W03-adversarial.md",
        "REV-W03-followup-1.md",
        "REV-W04-plan.md",
        "REV-W04-adversarial.md",
        "REV-W04-followup-1.md",
        "REV-GLOBAL-adversarial.md",
        "REV-IMPLEMENTATION-001-adversarial.md",
        "REV-IMPLEMENTATION-002-adversarial.md",
        "REV-IMPLEMENTATION-003-adversarial.md",
        "REV-FINAL.md",
    ];

    for file_name in expected_files {
        let path = review_root.join(file_name);
        assert!(path.exists(), "missing review artifact {}", path.display());
        let content = read(&path);
        assert!(
            content.contains("Statut: closed") || content.contains("Statut final: closed"),
            "{} is not marked closed",
            path.display()
        );
        assert!(
            content.contains("Aucun finding ouvert")
                || content.contains("Findings restants: Aucun"),
            "{} does not record a zero-open-finding state",
            path.display()
        );
    }
}
