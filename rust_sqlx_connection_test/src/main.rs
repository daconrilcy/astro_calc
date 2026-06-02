use chrono::{DateTime, Utc};
use rust_sqlx_connection_test::cli::{
    output_mode_from_args, output_mode_from_env, root_output_dir, write_timestamped_output_file,
    OutputMode,
};
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
