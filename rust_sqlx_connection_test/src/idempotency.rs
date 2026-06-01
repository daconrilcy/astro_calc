use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::domain::{NatalChartInput, RuntimeOptions};
use crate::runtime::RuntimeError;

#[derive(Debug, Serialize)]
struct StableIdempotencyDocument<'a> {
    chart_type: &'static str,
    input: StableCalculationInput<'a>,
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

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn input() -> NatalChartInput {
        NatalChartInput {
            subject_label: Some("Ada".to_string()),
            birth_datetime_utc: chrono::Utc.with_ymd_and_hms(1990, 1, 2, 3, 4, 5).unwrap(),
            latitude_deg: 48.8566,
            longitude_deg: 2.3522,
            altitude_m: Some(35.0),
            reference_version_id: 1,
            calculation_profile_id: Some(1),
            zodiacal_reference_system_id: 1,
            coordinate_reference_system_id: 1,
            house_system_id: 1,
            product_code: Some("basic".to_string()),
            language_id: Some(2),
        }
    }

    #[test]
    fn advisory_lock_is_stable() {
        assert_eq!(
            advisory_lock_key("runtime-key"),
            advisory_lock_key("runtime-key")
        );
    }

    #[test]
    fn idempotency_changes_when_engine_changes() {
        let input = input();
        let mut left = RuntimeOptions::default();
        left.engine_version = "a".to_string();
        let mut right = RuntimeOptions::default();
        right.engine_version = "b".to_string();

        assert_ne!(
            idempotency_key(&input, &left).unwrap(),
            idempotency_key(&input, &right).unwrap()
        );
    }

    #[test]
    fn idempotency_ignores_generation_options() {
        let mut left = input();
        let mut right = input();
        left.product_code = Some("basic".to_string());
        left.language_id = Some(1);
        right.product_code = Some("premium".to_string());
        right.language_id = Some(2);

        assert_eq!(
            idempotency_key(&left, &RuntimeOptions::default()).unwrap(),
            idempotency_key(&right, &RuntimeOptions::default()).unwrap()
        );
        assert_eq!(input_hash(&left).unwrap(), input_hash(&right).unwrap());
    }
}
