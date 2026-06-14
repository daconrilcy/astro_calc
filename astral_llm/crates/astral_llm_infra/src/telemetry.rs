use std::fs::{create_dir_all, OpenOptions};
use std::path::Path;
use std::sync::OnceLock;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

use crate::config::env_bool;

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

pub fn init_tracing() {
    let filter = build_filter();
    let json_logs = log_format_is_json();

    let stdout_layer = if json_logs {
        fmt::layer().json().with_target(true).boxed()
    } else {
        fmt::layer().with_target(true).boxed()
    };

    if let Some(log_file) = log_file_path() {
        if let Some(parent) = Path::new(&log_file).parent() {
            let _ = create_dir_all(parent);
        }
        match OpenOptions::new().create(true).append(true).open(&log_file) {
            Ok(file) => {
                let (writer, guard) = tracing_appender::non_blocking(file);
                let _ = LOG_GUARD.set(guard);
                let file_layer = if json_logs {
                    fmt::layer()
                        .json()
                        .with_writer(writer)
                        .with_target(true)
                        .boxed()
                } else {
                    fmt::layer().with_writer(writer).with_target(true).boxed()
                };
                tracing_subscriber::registry()
                    .with(filter)
                    .with(stdout_layer)
                    .with(file_layer)
                    .init();
                tracing::info!(log_file = %log_file, json = json_logs, "astral_llm logging initialized");
                return;
            }
            Err(err) => {
                eprintln!("ASTRAL_LLM_LOG_FILE={log_file} unreadable: {err}");
            }
        }
    }

    tracing_subscriber::registry()
        .with(filter)
        .with(stdout_layer)
        .init();
    tracing::info!(json = json_logs, "astral_llm logging initialized");
}

fn build_filter() -> EnvFilter {
    if let Ok(raw) = std::env::var("RUST_LOG") {
        if let Ok(filter) = EnvFilter::try_new(raw) {
            return filter;
        }
    }
    let prompt_directive = if env_bool("ASTRAL_LLM_LOG_COMPILED_PROMPTS", true) {
        ",astral_llm.prompt=debug"
    } else {
        ""
    };
    if let Ok(level) = std::env::var("ASTRAL_LLM_LOG_LEVEL") {
        let directive = format!(
            "{level},astral_llm_api=debug,astral_llm.generation=debug,astral_llm.provider=debug{prompt_directive}"
        );
        if let Ok(filter) = EnvFilter::try_new(directive) {
            return filter;
        }
    }
    EnvFilter::try_new(format!(
        "info,astral_llm_api=debug,astral_llm.generation=debug,astral_llm.provider=debug{prompt_directive}"
    ))
    .unwrap_or_else(|_| EnvFilter::new("info"))
}

fn log_format_is_json() -> bool {
    std::env::var("ASTRAL_LLM_LOG_FORMAT")
        .map(|v| v.eq_ignore_ascii_case("json"))
        .unwrap_or(false)
}

fn log_file_path() -> Option<String> {
    std::env::var("ASTRAL_LLM_LOG_FILE")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}
