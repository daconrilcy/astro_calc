use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Stdout,
    File,
}

pub fn output_mode_from_args(
    args: impl IntoIterator<Item = String>,
    default_mode: OutputMode,
) -> Result<OutputMode, Box<dyn std::error::Error>> {
    let mut output_mode = default_mode;

    for arg in args {
        match arg.as_str() {
            "--file" => output_mode = OutputMode::File,
            "--help" | "-h" => {
                return Err("usage: cargo run -- [--file]".into());
            }
            other => {
                return Err(
                    format!("unknown argument {other}; usage: cargo run -- [--file]").into(),
                );
            }
        }
    }

    Ok(output_mode)
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

pub fn timestamped_output_filename(datetime: DateTime<Utc>) -> String {
    format!("basic_payload_{}.json", datetime.format("%Y%m%d_%H%M%S"))
}

pub fn write_timestamped_output_file(
    output_dir: impl AsRef<Path>,
    json: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let output_dir = output_dir.as_ref();
    std::fs::create_dir_all(output_dir)?;

    let path = output_dir.join(timestamped_output_filename(Utc::now()));
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
