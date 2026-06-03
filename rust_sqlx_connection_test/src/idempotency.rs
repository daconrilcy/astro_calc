use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::domain::{NatalChartInput, RuntimeOptions};
use crate::runtime::RuntimeError;

#[derive(Debug, Serialize)]
struct StableIdempotencyDocument<'a> {
    chart_type: &'static str,
    input: StableCalculationInput<'a>,
    client_idempotency_key: Option<&'a str>,
    reference_version_id: i32,
    calculation_profile_id: Option<i32>,
    engine_version: &'a str,
    ephemeris_version: &'a str,
    zodiacal_reference_system_id: i32,
    coordinate_reference_system_id: i32,
    house_system_id: i32,
}

#[derive(Debug, Serialize)]
struct StableCalculationInput<'a> {
    subject_label: &'a Option<String>,
    birth_datetime_utc: chrono::DateTime<chrono::Utc>,
    latitude_deg: f64,
    longitude_deg: f64,
    altitude_m: Option<f64>,
}

pub fn input_hash(input: &NatalChartInput) -> Result<String, RuntimeError> {
    sha256_json(&stable_calculation_input(input))
}

pub fn idempotency_key(
    input: &NatalChartInput,
    options: &RuntimeOptions,
) -> Result<String, RuntimeError> {
    let document = StableIdempotencyDocument {
        chart_type: "natal",
        input: stable_calculation_input(input),
        client_idempotency_key: input
            .client_idempotency_key
            .as_deref()
            .filter(|key| !key.trim().is_empty()),
        reference_version_id: input.reference_version_id,
        calculation_profile_id: input.calculation_profile_id,
        engine_version: &options.engine_version,
        ephemeris_version: &options.ephemeris_version,
        zodiacal_reference_system_id: input.zodiacal_reference_system_id,
        coordinate_reference_system_id: input.coordinate_reference_system_id,
        house_system_id: input.house_system_id,
    };

    sha256_json(&document)
}

fn stable_calculation_input(input: &NatalChartInput) -> StableCalculationInput<'_> {
    StableCalculationInput {
        subject_label: &input.subject_label,
        birth_datetime_utc: input.birth_datetime_utc,
        latitude_deg: input.latitude_deg,
        longitude_deg: input.longitude_deg,
        altitude_m: input.altitude_m,
    }
}

pub fn advisory_lock_key(idempotency_key: &str) -> i64 {
    let digest = Sha256::digest(idempotency_key.as_bytes());
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    i64::from_be_bytes(bytes)
}

fn sha256_json<T: Serialize>(value: &T) -> Result<String, RuntimeError> {
    let bytes = serde_json::to_vec(value)?;
    Ok(hex::encode(Sha256::digest(bytes)))
}
