use astral_calculator::horoscope::{
    build_horoscope_daily_calculation_request_from_public,
    build_horoscope_period_calculation_request_from_public,
    HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE, HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
    HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
};
use serde_json::json;

fn db_available() -> bool {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    runtime.block_on(async { astral_calculator::db::connect_from_env().await.is_ok() })
}

#[test]
fn free_daily_builder_keeps_public_surface_minimal() {
    if !db_available() {
        return;
    }
    let request = build_horoscope_daily_calculation_request_from_public(
        HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE,
        &json!({
            "date": "2026-06-14",
            "timezone": "Europe/Paris",
            "chart_calculation_id": "chart-1",
            "location": {
                "latitude": 48.8566,
                "longitude": 2.3522,
                "label": "Paris"
            }
        }),
    )
    .expect("daily request");

    assert_eq!(request.contract_version, "horoscope_calculation_request");
    assert_eq!(
        request.service_code,
        HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE
    );
    assert!(request.location.is_none());
    assert!(request.house_system_code.is_none());
    assert!(request.slot_profile_code.is_none());
    assert!(request.calculation_features.is_empty());
    assert!(!request.slots.is_empty());
}

#[test]
fn premium_daily_builder_requires_location_and_enables_local_features() {
    if !db_available() {
        return;
    }
    let err = build_horoscope_daily_calculation_request_from_public(
        HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
        &json!({
            "date": "2026-06-14",
            "timezone": "Europe/Paris",
            "chart_calculation_id": "chart-1"
        }),
    )
    .expect_err("location required");
    assert_eq!(err, "HOROSCOPE_LOCATION_REQUIRED");

    let request = build_horoscope_daily_calculation_request_from_public(
        HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
        &json!({
            "date": "2026-06-14",
            "timezone": "Europe/Paris",
            "chart_calculation_id": "chart-1",
            "location": {
                "latitude": 48.8566,
                "longitude": 2.3522,
                "label": "Paris"
            }
        }),
    )
    .expect("premium daily request");

    assert_eq!(request.slot_profile_code.as_deref(), Some("daily_2h_slots"));
    assert!(request.location.is_some());
    assert_eq!(request.house_system_code.as_deref(), Some("placidus"));
    assert!(request
        .calculation_features
        .iter()
        .any(|feature| feature == "local_chart"));
}

#[test]
fn daily_builder_rejects_invalid_timezone_and_service_code() {
    if !db_available() {
        return;
    }
    let invalid_timezone = build_horoscope_daily_calculation_request_from_public(
        HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE,
        &json!({
            "date": "2026-06-14",
            "timezone": "Bad/Timezone",
            "chart_calculation_id": "chart-1"
        }),
    )
    .expect_err("timezone must fail");
    assert!(invalid_timezone.contains("timezone"));

    let invalid_service = build_horoscope_daily_calculation_request_from_public(
        "legacy_sync_daily",
        &json!({
            "date": "2026-06-14",
            "timezone": "Europe/Paris",
            "chart_calculation_id": "chart-1"
        }),
    )
    .expect_err("service must fail");
    assert_eq!(invalid_service, "HOROSCOPE_SERVICE_NOT_IMPLEMENTED");
}

#[test]
fn period_builder_creates_canonical_scan_plan_for_v2_service() {
    if !db_available() {
        return;
    }
    let request = build_horoscope_period_calculation_request_from_public(
        HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        &json!({
            "anchor_date": "2026-06-14",
            "timezone": "Europe/Paris",
            "chart_calculation_id": "chart-7"
        }),
    )
    .expect("period request");

    assert_eq!(
        request.contract_version,
        "horoscope_period_calculation_request"
    );
    assert_eq!(
        request.service_code,
        HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE
    );
    assert_eq!(request.scan_plan.scan_profile_code, "daily_noon_7_days");
    assert_eq!(request.scan_plan.snapshot_count, 7);
    assert_eq!(request.scan_plan.snapshots.len(), 7);
    assert_eq!(
        request.period_resolution["included_dates"]
            .as_array()
            .expect("included_dates")
            .len(),
        7
    );
}

#[test]
fn period_builder_rejects_invalid_anchor_date() {
    if !db_available() {
        return;
    }
    let err = build_horoscope_period_calculation_request_from_public(
        HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        &json!({
            "anchor_date": "2026/06/14",
            "timezone": "Europe/Paris",
            "chart_calculation_id": "chart-7"
        }),
    )
    .expect_err("anchor_date must fail");

    assert_eq!(err, "HOROSCOPE_PERIOD_ANCHOR_DATE_REQUIRED");
}
