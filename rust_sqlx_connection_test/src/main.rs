use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use rust_sqlx_connection_test::config::{
    ephemeris_path_from_env, load_dotenv, runtime_options_from_env,
};
use rust_sqlx_connection_test::db::connect_from_env;
use rust_sqlx_connection_test::domain::NatalChartInput;
use rust_sqlx_connection_test::ephemeris::SwissEphemerisEngine;
use rust_sqlx_connection_test::runtime::ChartCalculationRuntimeService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    load_dotenv();
    let output_mode = output_mode_from_args(std::env::args().skip(1), output_mode_from_env())?;
    let input = natal_input_from_env()?;
    let pool = connect_from_env().await?;
    let ephemeris = SwissEphemerisEngine::new(ephemeris_path_from_env());
    let service = ChartCalculationRuntimeService::new(pool, ephemeris, runtime_options_from_env());

    let output = service.calculate_natal_basic(input).await?;
    let json = serde_json::to_string_pretty(&output)?;
    match output_mode {
        OutputMode::Stdout => println!("{json}"),
        OutputMode::File => {
            let path = write_timestamped_output_file(root_output_dir(), &json)?;
            println!("JSON payload written to {}", path.display());
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputMode {
    Stdout,
    File,
}

fn output_mode_from_args(
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

fn output_mode_from_env() -> OutputMode {
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

fn env_flag_enabled(name: &str) -> bool {
    std::env::var(name).ok().is_some_and(|value| {
        matches!(
            value.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn root_output_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")))
        .join("output")
}

fn write_timestamped_output_file(
    output_dir: impl AsRef<Path>,
    json: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let output_dir = output_dir.as_ref();
    std::fs::create_dir_all(output_dir)?;

    let path = output_dir.join(timestamped_output_filename(Utc::now()));
    std::fs::write(&path, json)?;
    Ok(path)
}

fn timestamped_output_filename(datetime: DateTime<Utc>) -> String {
    format!("basic_payload_{}.json", datetime.format("%Y%m%d_%H%M%S"))
}

fn natal_input_from_env() -> Result<NatalChartInput, Box<dyn std::error::Error>> {
    Ok(NatalChartInput {
        subject_label: std::env::var("ASTRAL_SUBJECT_LABEL").ok(),
        birth_datetime_utc: required("ASTRAL_BIRTH_DATETIME_UTC")?.parse::<DateTime<Utc>>()?,
        latitude_deg: required("ASTRAL_LATITUDE_DEG")?.parse()?,
        longitude_deg: required("ASTRAL_LONGITUDE_DEG")?.parse()?,
        altitude_m: optional_parse("ASTRAL_ALTITUDE_M")?,
        reference_version_id: optional_parse("ASTRAL_REFERENCE_VERSION_ID")?.unwrap_or(1),
        calculation_profile_id: optional_parse("ASTRAL_CALCULATION_PROFILE_ID")?,
        zodiacal_reference_system_id: optional_parse("ASTRAL_ZODIACAL_REFERENCE_SYSTEM_ID")?
            .unwrap_or(1),
        coordinate_reference_system_id: optional_parse("ASTRAL_COORDINATE_REFERENCE_SYSTEM_ID")?
            .unwrap_or(1),
        house_system_id: optional_parse("ASTRAL_HOUSE_SYSTEM_ID")?.unwrap_or(1),
        product_code: Some(
            std::env::var("ASTRAL_PRODUCT_CODE").unwrap_or_else(|_| "basic".to_string()),
        ),
    })
}

fn required(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    std::env::var(name).map_err(|_| format!("{name} must be set").into())
}

fn optional_parse<T>(name: &str) -> Result<Option<T>, Box<dyn std::error::Error>>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + 'static,
{
    std::env::var(name)
        .ok()
        .map(|value| value.parse::<T>().map_err(Into::into))
        .transpose()
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn output_mode_defaults_to_stdout() {
        assert_eq!(
            output_mode_from_args(Vec::<String>::new(), OutputMode::Stdout).unwrap(),
            OutputMode::Stdout
        );
    }

    #[test]
    fn output_mode_accepts_file_flag() {
        assert_eq!(
            output_mode_from_args(["--file".to_string()], OutputMode::Stdout).unwrap(),
            OutputMode::File
        );
    }

    #[test]
    fn output_mode_can_default_to_file_from_env() {
        assert_eq!(
            output_mode_from_args(Vec::<String>::new(), OutputMode::File).unwrap(),
            OutputMode::File
        );
    }

    #[test]
    fn timestamped_output_filename_is_json_and_filesystem_safe() {
        let datetime = Utc.with_ymd_and_hms(2026, 6, 2, 12, 34, 56).unwrap();

        assert_eq!(
            timestamped_output_filename(datetime),
            "basic_payload_20260602_123456.json"
        );
    }

    #[test]
    fn root_output_dir_is_parent_of_crate_output_dir() {
        assert_eq!(
            root_output_dir(),
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("crate should have parent")
                .join("output")
        );
    }
}
