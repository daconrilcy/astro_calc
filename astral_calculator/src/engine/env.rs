//! Module astral_calculator\src\engine\env.rs du moteur astral_calculator.

use chrono::{DateTime, Utc};

use crate::application::ports::ReferenceSystemCatalog;
use crate::engine::{
    AstroEngineRequest, EngineBirthLocation, EngineBirthRequest, EngineCalculationRequest,
    EngineProjectionRequest, REQUEST_CONTRACT_VERSION,
};

pub use crate::engine::calculation_refs::{
    coordinate_reference_system_id_from_env, coordinate_reference_system_key_from_env,
    house_system_code_from_env, house_system_id_from_env, zodiacal_reference_system_id_from_env,
    zodiacal_reference_system_key_from_env,
};

/// Fonction birth_datetime_utc_from_env.
pub fn birth_datetime_utc_from_env() -> Result<DateTime<Utc>, Box<dyn std::error::Error>> {
    let date = optional_non_empty_env("ASTRAL_BIRTH_DATE");
    let time = optional_non_empty_env("ASTRAL_BIRTH_TIME");
    let timezone = optional_non_empty_env("ASTRAL_BIRTH_TIMEZONE");

    match (date, time, timezone) {
        (Some(date), Some(time), Some(timezone)) => {
            Ok(crate::engine::local_birth_to_utc(&date, &time, &timezone)?)
        }
        (None, None, None) => {
            let utc_raw = std::env::var("ASTRAL_BIRTH_DATETIME_UTC").map_err(|_| {
                "set ASTRAL_BIRTH_DATE, ASTRAL_BIRTH_TIME and ASTRAL_BIRTH_TIMEZONE together, or ASTRAL_BIRTH_DATETIME_UTC"
            })?;
            Ok(utc_raw.parse::<DateTime<Utc>>()?)
        }
        _ => Err(
            "ASTRAL_BIRTH_DATE, ASTRAL_BIRTH_TIME and ASTRAL_BIRTH_TIMEZONE must all be set together"
                .into(),
        ),
    }
}

const PROJECTION_LEVELS: &[&str] = &["compact", "standard", "rich", "expert"];

/// Fonction engine_request_from_env.
pub async fn engine_request_from_env<R>(
    repository: &R,
) -> Result<AstroEngineRequest, Box<dyn std::error::Error>>
where
    R: ReferenceSystemCatalog,
{
    let (date, time, timezone) = birth_fields_from_env()?;
    let projection_level = projection_level_from_env()?;

    Ok(AstroEngineRequest {
        request_contract_version: REQUEST_CONTRACT_VERSION.to_string(),
        request_id: optional_non_empty_env("ASTRAL_REQUEST_ID"),
        idempotency_key: optional_non_empty_env("ASTRAL_IDEMPOTENCY_KEY"),
        calculation: EngineCalculationRequest {
            calculation_type: "natal".to_string(),
            zodiacal_reference_system: zodiacal_reference_system_key_from_env(repository).await?,
            coordinate_reference_system: coordinate_reference_system_key_from_env(repository)
                .await?,
            house_system: house_system_code_from_env(repository).await?,
        },
        birth: EngineBirthRequest {
            date,
            time,
            timezone,
            location: EngineBirthLocation {
                label: optional_non_empty_env("ASTRAL_LOCATION_LABEL"),
                latitude: required_parse("ASTRAL_LATITUDE_DEG")?,
                longitude: required_parse("ASTRAL_LONGITUDE_DEG")?,
                country_code: optional_non_empty_env("ASTRAL_COUNTRY_CODE"),
            },
            time_precision: optional_non_empty_env("ASTRAL_TIME_PRECISION"),
        },
        projection: EngineProjectionRequest {
            contract_version: Some("llm_projection_natal_v1".to_string()),
            level: projection_level,
            language_code: optional_non_empty_env("ASTRAL_OUTPUT_LANGUAGE"),
        },
    })
}

/// Fonction projection_level_from_env.
fn projection_level_from_env() -> Result<String, Box<dyn std::error::Error>> {
    let level = std::env::var("ASTRAL_PROJECTION_LEVEL").unwrap_or_else(|_| "rich".to_string());
    let level = level.trim().to_string();
    if PROJECTION_LEVELS.contains(&level.as_str()) {
        Ok(level)
    } else {
        Err(format!(
            "ASTRAL_PROJECTION_LEVEL must be one of: {}",
            PROJECTION_LEVELS.join(", ")
        )
        .into())
    }
}

/// Fonction birth_fields_from_env.
fn birth_fields_from_env() -> Result<(String, String, String), Box<dyn std::error::Error>> {
    let date = std::env::var("ASTRAL_BIRTH_DATE").ok();
    let time = std::env::var("ASTRAL_BIRTH_TIME").ok();
    let timezone = std::env::var("ASTRAL_BIRTH_TIMEZONE").ok();

    match (date, time, timezone) {
        (Some(date), Some(time), Some(timezone)) => Ok((date, time, timezone)),
        (None, None, None) => {
            let utc_raw = std::env::var("ASTRAL_BIRTH_DATETIME_UTC").map_err(|_| {
                "set ASTRAL_BIRTH_DATE, ASTRAL_BIRTH_TIME and ASTRAL_BIRTH_TIMEZONE together, or ASTRAL_BIRTH_DATETIME_UTC"
            })?;
            let utc: DateTime<Utc> = utc_raw.parse()?;
            let timezone = std::env::var("ASTRAL_BIRTH_TIMEZONE")
                .unwrap_or_else(|_| "UTC".to_string());
            Ok((
                utc.format("%Y-%m-%d").to_string(),
                utc.format("%H:%M:%S").to_string(),
                timezone,
            ))
        }
        _ => Err(
            "ASTRAL_BIRTH_DATE, ASTRAL_BIRTH_TIME and ASTRAL_BIRTH_TIMEZONE must all be set together"
                .into(),
        ),
    }
}

/// Fonction optional_non_empty_env.
fn optional_non_empty_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Fonction required_parse.
fn required_parse<T>(name: &str) -> Result<T, Box<dyn std::error::Error>>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + 'static,
{
    std::env::var(name)?
        .parse::<T>()
        .map_err(|error| format!("{name} is invalid: {error}").into())
}
