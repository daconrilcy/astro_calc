use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Stdout,
    File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputContract {
    /// Enveloppe `astro_engine_response_v1` (defaut 4A).
    Engine,
    /// Payload audit brut `natal_structured_v13` (scripts golden v13).
    AuditOnly,
}

pub struct CliOptions {
    pub output_mode: OutputMode,
    pub output_contract: OutputContract,
}

pub fn cli_options_from_args(
    args: impl IntoIterator<Item = String>,
    default_mode: OutputMode,
) -> Result<CliOptions, Box<dyn std::error::Error>> {
    let mut output_mode = default_mode;
    let mut output_contract = output_contract_from_env();
    let mut saw_engine = false;
    let mut saw_audit = false;

    for arg in args {
        match arg.as_str() {
            "--file" => output_mode = OutputMode::File,
            "--audit-only" => {
                output_contract = OutputContract::AuditOnly;
                saw_audit = true;
            }
            "--engine" => {
                output_contract = OutputContract::Engine;
                saw_engine = true;
            }
            "--help" | "-h" => {
                return Err(
                    "usage: cargo run -- [--file] [--engine|--audit-only]\n\
                     default: astro_engine_response_v1 envelope (4A)\n\
                     --audit-only: raw natal_structured_v13 payload"
                        .into(),
                );
            }
            other => {
                return Err(format!(
                    "unknown argument {other}; usage: cargo run -- [--file] [--engine|--audit-only]"
                )
                .into());
            }
        }
    }

    if saw_engine && saw_audit {
        return Err(
            "cannot use --engine and --audit-only together; choose one output contract".into(),
        );
    }

    Ok(CliOptions {
        output_mode,
        output_contract,
    })
}

pub fn output_mode_from_args(
    args: impl IntoIterator<Item = String>,
    default_mode: OutputMode,
) -> Result<OutputMode, Box<dyn std::error::Error>> {
    Ok(cli_options_from_args(args, default_mode)?.output_mode)
}

pub fn output_contract_from_env() -> OutputContract {
    match std::env::var("ASTRAL_OUTPUT_CONTRACT")
        .ok()
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("audit" | "audit_only" | "natal_structured_v13" | "v13") => {
            OutputContract::AuditOnly
        }
        Some("engine" | "astro_engine_response_v1" | "4a") => OutputContract::Engine,
        _ => OutputContract::Engine,
    }
}

pub fn output_mode_from_env() -> OutputMode {
    if env_flag_enabled("ASTRAL_OUTPUT_FILE")
        || std::env::var("ASTRAL_OUTPUT_MODE")
            .ok()
            .is_some_and(|value| value.eq_ignore_ascii_case("file"))
    {
        OutputMode::File
    } else {
        OutputMode::Stdout
    }
}

pub fn root_output_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")))
        .join("output")
}

pub fn timestamped_output_filename(datetime: DateTime<Utc>, contract: OutputContract) -> String {
    let stem = match contract {
        OutputContract::Engine => "astro_engine_response",
        OutputContract::AuditOnly => "basic_payload",
    };
    format!("{stem}_{}.json", datetime.format("%Y%m%d_%H%M%S"))
}

pub fn write_timestamped_output_file(
    output_dir: impl AsRef<Path>,
    json: &str,
    contract: OutputContract,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let output_dir = output_dir.as_ref();
    std::fs::create_dir_all(output_dir)?;

    let path = output_dir.join(timestamped_output_filename(Utc::now(), contract));
    std::fs::write(&path, json)?;
    Ok(path)
}

fn env_flag_enabled(name: &str) -> bool {
    std::env::var(name).ok().is_some_and(|value| {
        matches!(
            value.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}
