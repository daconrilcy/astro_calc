use chrono::TimeZone;

use rust_sqlx_connection_test::domain::{NatalChartInput, RuntimeOptions};
use rust_sqlx_connection_test::idempotency::{advisory_lock_key, idempotency_key, input_hash};

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
        client_idempotency_key: None,
    }
}

#[test]
fn client_idempotency_key_changes_hash() {
    let mut with_key = input();
    with_key.client_idempotency_key = Some("client-abc".to_string());
    let mut without_key = input();
    without_key.client_idempotency_key = None;

    let key_a = rust_sqlx_connection_test::idempotency::idempotency_key(
        &with_key,
        &rust_sqlx_connection_test::domain::RuntimeOptions::default(),
    )
    .expect("key");
    let key_b = rust_sqlx_connection_test::idempotency::idempotency_key(
        &without_key,
        &rust_sqlx_connection_test::domain::RuntimeOptions::default(),
    )
    .expect("key");
    assert_ne!(key_a, key_b);
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
    let left = RuntimeOptions {
        engine_version: "a".to_string(),
        ..RuntimeOptions::default()
    };
    let right = RuntimeOptions {
        engine_version: "b".to_string(),
        ..RuntimeOptions::default()
    };

    assert_ne!(
        idempotency_key(&input, &left).unwrap(),
        idempotency_key(&input, &right).unwrap()
    );
}

#[test]
fn idempotency_ignores_product_options() {
    let mut left = input();
    let mut right = input();
    left.product_code = Some("basic".to_string());
    right.product_code = Some("premium".to_string());

    assert_eq!(
        idempotency_key(&left, &RuntimeOptions::default()).unwrap(),
        idempotency_key(&right, &RuntimeOptions::default()).unwrap()
    );
    assert_eq!(input_hash(&left).unwrap(), input_hash(&right).unwrap());
}
