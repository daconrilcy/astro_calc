use std::{
    fs,
    path::{Path, PathBuf},
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root")
}

fn collect_rs_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

#[test]
fn workspace_core_contains_no_inline_unit_tests() {
    let root = repo_root();
    let src_roots = [
        root.join("astral_calculator/src"),
        root.join("astral_calculator_api/src"),
        root.join("astral_contracts/src"),
        root.join("astral_gateway/src"),
        root.join("astral_time_window/src"),
        root.join("astral_llm/crates/astral_llm_api/src"),
        root.join("astral_llm/crates/astral_llm_application/src"),
        root.join("astral_llm/crates/astral_llm_domain/src"),
        root.join("astral_llm/crates/astral_llm_infra/src"),
        root.join("astral_llm/crates/astral_llm_providers/src"),
        root.join("astral_llm/crates/astral_llm_worker/src"),
    ];

    let mut offenders = Vec::new();
    for dir in src_roots {
        let mut files = Vec::new();
        collect_rs_files(&dir, &mut files);
        for file in files {
            let raw = fs::read_to_string(&file).expect("read source");
            if raw.contains("#[cfg(test)]") || raw.contains("mod tests") {
                offenders.push(
                    file.strip_prefix(&root)
                        .unwrap_or(&file)
                        .display()
                        .to_string(),
                );
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "inline tests must live under tests/: {:?}",
        offenders
    );
}
