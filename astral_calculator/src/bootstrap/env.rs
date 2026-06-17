use std::path::{Path, PathBuf};

use crate::domain::RuntimeOptions;

pub fn load_dotenv() {
    for path in dotenv_candidates() {
        if path.is_file() {
            dotenvy::from_path(path).ok();
            return;
        }
    }
    dotenvy::dotenv().ok();
}

pub fn runtime_options_from_env() -> RuntimeOptions {
    RuntimeOptions {
        engine_version: std::env::var("ASTRAL_ENGINE_VERSION")
            .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string()),
        ephemeris_version: std::env::var("ASTRAL_EPHEMERIS_VERSION")
            .unwrap_or_else(|_| "se-2026a".to_string()),
        stale_after_seconds: std::env::var("ASTRAL_STALE_AFTER_SECONDS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(900),
    }
}

pub fn ephemeris_path_from_env() -> PathBuf {
    std::env::var("ASTRAL_EPHEMERIS_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("..").join("ephe").join("se-2026a"))
}

fn dotenv_candidates() -> Vec<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut candidates = Vec::new();

    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(current_dir.join(".env"));
        if let Some(parent) = current_dir.parent() {
            candidates.push(parent.join(".env"));
        }
    }

    candidates.push(manifest_dir.join(".env"));
    if let Some(parent) = manifest_dir.parent() {
        candidates.push(parent.join(".env"));
    }

    candidates
}
