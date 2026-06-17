use astral_calculator::cli::{
    cli_options_from_args, output_mode_from_env, root_output_dir, write_timestamped_output_file,
    OutputContract, OutputMode,
};
use astral_calculator::config::{ephemeris_path_from_env, load_dotenv, runtime_options_from_env};
use astral_calculator::db::connect_from_env;
use astral_calculator::domain::NatalChartInput;
use astral_calculator::engine::{
    birth_datetime_utc_from_env, coordinate_reference_system_id_from_env, house_system_id_from_env,
    zodiacal_reference_system_id_from_env,
};
use astral_calculator::engine_request_from_env;
use astral_calculator::ephemeris::SwissEphemerisEngine;
use astral_calculator::infra::db::reference_repository::ReferenceRepository;
use astral_calculator::runtime::build_runtime_service;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    load_dotenv();
    let cli = cli_options_from_args(std::env::args().skip(1), output_mode_from_env())?;
    let pool = connect_from_env().await?;
    let references = ReferenceRepository::new(pool.clone());
    let ephemeris = SwissEphemerisEngine::new(ephemeris_path_from_env());
    let service = build_runtime_service(pool, ephemeris, runtime_options_from_env());

    let json = match cli.output_contract {
        OutputContract::Engine => {
            let request = engine_request_from_env(&references).await?;
            let response = service.calculate_natal_engine(request).await?;
            serde_json::to_string_pretty(&response)?
        }
        OutputContract::AuditOnly => {
            let input = natal_input_from_env(&references).await?;
            let output = service.calculate_natal_basic(input).await?;
            serde_json::to_string_pretty(&output)?
        }
    };

    match cli.output_mode {
        OutputMode::Stdout => println!("{json}"),
        OutputMode::File => {
            let path =
                write_timestamped_output_file(root_output_dir(), &json, cli.output_contract)?;
            let label = match cli.output_contract {
                OutputContract::Engine => "astro_engine_response_v1",
                OutputContract::AuditOnly => "natal_structured_v13 audit payload",
            };
            println!("{label} written to {}", path.display());
        }
    }
    Ok(())
}

async fn natal_input_from_env(
    references: &ReferenceRepository,
) -> Result<NatalChartInput, Box<dyn std::error::Error>> {
    let idempotency_key = std::env::var("ASTRAL_IDEMPOTENCY_KEY")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    Ok(NatalChartInput {
        subject_label: std::env::var("ASTRAL_SUBJECT_LABEL").ok(),
        birth_datetime_utc: birth_datetime_utc_from_env()?,
        latitude_deg: required_parse("ASTRAL_LATITUDE_DEG")?,
        longitude_deg: required_parse("ASTRAL_LONGITUDE_DEG")?,
        altitude_m: optional_parse("ASTRAL_ALTITUDE_M")?,
        reference_version_id: optional_parse("ASTRAL_REFERENCE_VERSION_ID")?.unwrap_or(1),
        calculation_profile_id: optional_parse("ASTRAL_CALCULATION_PROFILE_ID")?,
        zodiacal_reference_system_id: zodiacal_reference_system_id_from_env(references).await?,
        coordinate_reference_system_id: coordinate_reference_system_id_from_env(references).await?,
        house_system_id: house_system_id_from_env(references).await?,
        product_code: Some(
            std::env::var("ASTRAL_PRODUCT_CODE").unwrap_or_else(|_| "basic".to_string()),
        ),
        client_idempotency_key: idempotency_key,
    })
}

fn required_parse<T>(name: &str) -> Result<T, Box<dyn std::error::Error>>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + 'static,
{
    std::env::var(name)?
        .parse::<T>()
        .map_err(|error| format!("{name} is invalid: {error}").into())
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
