use std::path::PathBuf;

use crate::domain::RuntimeOptions;

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
