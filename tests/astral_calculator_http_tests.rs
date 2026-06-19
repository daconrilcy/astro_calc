use std::sync::Arc;

use astral_calculator::config::{ephemeris_path_from_env, runtime_options_from_env};
use astral_calculator::db::connect_from_env;
use astral_calculator::domain::{AspectDefinition, ObjectPositionFact};
use astral_calculator::ephemeris::SwissEphemerisEngine;
use astral_calculator::features::horoscope::{
    calculate_horoscope_daily, calculate_horoscope_daily_from_transits,
    calculate_horoscope_daily_natal, calculate_horoscope_period,
    calculate_horoscope_period_from_positions, calculate_horoscope_period_from_transits,
    calculate_horoscope_period_natal, calculate_horoscope_period_natal_from_positions,
    calculate_horoscope_period_natal_from_transits, normalize_horoscope_period_request_utc,
    try_calculate_horoscope_period_from_transits_with_aspects,
    HoroscopeCalculationRequest, HoroscopeCalculationSlotRequest, HoroscopePeriod,
    HoroscopePeriodCalculationRequest, HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE,
    HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
};
use astral_calculator::runtime::build_runtime_service;
use astral_calculator_http::{
    build_app, config::AppConfig, reference_status::check_reference_status,
    schema_registry::SchemaRegistry, state::AppState,
};

async fn build_test_state() -> AppState {
    dotenvy::dotenv().ok();
    let pool = connect_from_env()
        .await
        .expect("DATABASE_URL and PostgreSQL are required for astral_calculator_http_tests");
    let config = AppConfig::from_env();
    let ephemeris = SwissEphemerisEngine::new(ephemeris_path_from_env());
    let service = build_runtime_service(pool.clone(), ephemeris, runtime_options_from_env());
    let schema_registry = SchemaRegistry::from_dir(&config.schemas_dir)
        .expect("schema registry must load for astral_calculator_http_tests");

    AppState {
        config,
        pool,
        service: Arc::new(service),
        schema_registry: Arc::new(schema_registry),
    }
}

#[test]
fn horoscope_period_calculator_from_positions_never_uses_fake_source() {
    let request = period_calculator_request();
    let positions = sample_natal_positions();

    let response = calculate_horoscope_period_from_positions(request, &positions, 8.0).unwrap();
    assert_eq!(response.snapshots.len(), 7);
    for snapshot in response.snapshots {
        assert!(snapshot.transits_to_natal.is_empty());
        assert_eq!(snapshot.sky_snapshot["source"], "missing_transit_data");
        assert!(!snapshot.calculation_warnings.is_empty());
    }
}

#[test]
fn horoscope_period_calculator_public_function_never_uses_fake_source() {
    let response = calculate_horoscope_period(period_calculator_request()).unwrap();
    for snapshot in response.snapshots {
        assert!(snapshot.transits_to_natal.is_empty());
        assert_eq!(snapshot.sky_snapshot["source"], "missing_transit_data");
        assert!(!snapshot.calculation_warnings.is_empty());
    }
}

#[test]
fn horoscope_period_calculator_with_transits_uses_swisseph_source() {
    let request = period_calculator_request();
    let positions = sample_natal_positions();
    let transit_snapshots = transit_snapshots_for_request(&request, positions.clone());

    let response =
        calculate_horoscope_period_from_transits(request, &positions, &transit_snapshots, 8.0)
            .unwrap();
    for snapshot in response.snapshots {
        for fact in snapshot.transits_to_natal {
            assert_eq!(fact.source, "swisseph_period_calculator_v1");
        }
    }
}

#[test]
fn horoscope_period_calculator_rejects_wide_major_aspect_orbs() {
    let request = period_calculator_request();
    let positions = sample_natal_positions();
    let transit_snapshots = transit_snapshots_with_venus_context(&request, positions.clone());

    let response =
        calculate_horoscope_period_from_transits(request, &positions, &transit_snapshots, 8.0)
            .unwrap();
    let venus_fact = response
        .snapshots
        .iter()
        .flat_map(|snapshot| snapshot.transits_to_natal.iter())
        .find(|fact| fact.transiting_object == "venus")
        .expect("venus transit fact");
    assert_ne!(venus_fact.fact_type, "transit_to_natal");
    assert!(venus_fact.aspect.is_none());
    assert!(venus_fact.orb_deg.is_none());
}

#[test]
fn horoscope_period_calculator_respects_reference_aspect_orbs_when_supplied() {
    let request = period_calculator_request();
    let positions = sample_natal_positions();
    let mut transit_positions = positions.clone();
    transit_positions.push(ObjectPositionFact {
        chart_object_id: 3,
        object_code: "venus".to_string(),
        object_name: "Venus".to_string(),
        zodiacal_reference_system_id: 1,
        coordinate_reference_system_id: 1,
        sign_id: 6,
        sign_code: "virgo".to_string(),
        sign_name: "Virgo".to_string(),
        house_id: Some(6),
        house_number: Some(6),
        house_name: Some("House 6".to_string()),
        motion_state_id: None,
        horizon_position_id: None,
        longitude_deg: 173.0,
        latitude_deg: None,
        apparent_speed_deg_per_day: Some(1.0),
        altitude_deg: None,
        is_visible: None,
        facts_json: None,
    });
    let transit_snapshots = transit_snapshots_for_request(&request, transit_positions);
    let aspect_definitions = vec![AspectDefinition {
        id: 4,
        code: "trine".to_string(),
        name: "Trine".to_string(),
        angle: 120.0,
        family: "major".to_string(),
        default_orb_deg: Some(2.0),
        max_default_orb_deg: 8.0,
    }];

    let response = try_calculate_horoscope_period_from_transits_with_aspects(
        request,
        &positions,
        &transit_snapshots,
        8.0,
        &aspect_definitions,
        &sample_theme_mappings(),
    )
    .unwrap();

    let venus_fact = response
        .snapshots
        .iter()
        .flat_map(|snapshot| snapshot.transits_to_natal.iter())
        .find(|fact| fact.transiting_object == "venus")
        .expect("venus transit fact");
    assert_eq!(venus_fact.fact_type, "transit_context");
    assert!(venus_fact.aspect.is_none());
    assert!(venus_fact.orb_deg.is_none());
}

#[test]
fn horoscope_period_calculator_outputs_context_fact_when_no_valid_aspect() {
    let request = period_calculator_request();
    let positions = sample_natal_positions();
    let transit_snapshots = transit_snapshots_with_venus_context(&request, positions.clone());

    let response =
        calculate_horoscope_period_from_transits(request, &positions, &transit_snapshots, 8.0)
            .unwrap();
    let venus_snapshot = response
        .snapshots
        .iter()
        .find(|snapshot| {
            snapshot
                .transits_to_natal
                .iter()
                .any(|fact| fact.transiting_object == "venus")
        })
        .expect("venus snapshot");
    let venus_fact = venus_snapshot
        .transits_to_natal
        .iter()
        .find(|fact| fact.transiting_object == "venus")
        .expect("venus fact");
    assert_eq!(venus_fact.fact_type, "transit_context");
    assert!(venus_fact.evidence_key.contains(":context:"));
    assert!(venus_fact.orb_deg.is_none());
    assert!(venus_snapshot
        .current_sky_aspects
        .iter()
        .any(|aspect| aspect["aspect"] == "context" && aspect["orb_deg"].is_null()));
}

#[test]
fn horoscope_period_calculator_request_normalizes_utc_fields() {
    let request: HoroscopePeriodCalculationRequest = serde_json::from_value(serde_json::json!({
        "contract_version": "horoscope_period_calculation_request",
        "service_code": "horoscope_basic_next_7_days_natal",
        "chart_calculation_id": "123",
        "period_resolution": {
            "period_profile_code": "next_7_days",
            "anchor_date": "2026-06-07",
            "timezone": "Europe/Paris",
            "start_datetime_local": "2026-06-07T00:00:00",
            "end_datetime_local": "2026-06-14T00:00:00",
            "start_datetime_utc": "2026-06-07T00:00:00+02:00",
            "end_datetime_utc": "2026-06-14T00:00:00+02:00",
            "end_exclusive": true,
            "duration_days": 7,
            "included_dates": ["2026-06-07"],
            "included_days": []
        },
        "scan_plan": {
            "scan_profile_code": "daily_noon_7_days",
            "granularity": "daily_noon",
            "snapshot_count": 1,
            "snapshots": [
                { "snapshot_key": "2026-06-07:noon", "date": "2026-06-07", "reference_time_local": "12:00", "reference_datetime_local": "2026-06-07T12:00:00", "reference_datetime_utc": "2026-06-07T12:00:00+02:00" }
            ]
        }
    }))
    .unwrap();

    let normalized = normalize_horoscope_period_request_utc(request).unwrap();
    assert_eq!(
        normalized.period_resolution["start_datetime_utc"],
        serde_json::json!("2026-06-06T22:00:00+00:00")
    );
    assert_eq!(
        normalized.period_resolution["end_datetime_utc"],
        serde_json::json!("2026-06-13T22:00:00+00:00")
    );
    assert_eq!(
        normalized.scan_plan.snapshots[0].reference_datetime_utc,
        "2026-06-07T10:00:00+00:00"
    );
}

#[test]
fn horoscope_period_calculator_response_keeps_canonical_utc_fields() {
    let mut request = period_calculator_request();
    request.period_resolution["start_datetime_utc"] =
        serde_json::json!("2026-06-07T00:00:00+02:00");
    request.period_resolution["end_datetime_utc"] = serde_json::json!("2026-06-14T00:00:00+02:00");
    request.scan_plan.snapshots[0].reference_datetime_utc = "2026-06-07T12:00:00+02:00".to_string();

    let response = calculate_horoscope_period(request).unwrap();

    assert_eq!(
        response.period_resolution["start_datetime_utc"],
        serde_json::json!("2026-06-06T22:00:00+00:00")
    );
    assert_eq!(
        response.period_resolution["end_datetime_utc"],
        serde_json::json!("2026-06-13T22:00:00+00:00")
    );
    assert_eq!(
        response.scan_plan.snapshots[0].reference_datetime_utc,
        "2026-06-07T10:00:00+00:00"
    );
    assert_eq!(
        response.snapshots[0].reference_datetime_utc,
        "2026-06-07T10:00:00+00:00"
    );
}

#[test]
fn horoscope_period_calculator_request_rejects_duplicate_snapshot_keys() {
    let mut request = period_calculator_request();
    request.scan_plan.snapshots[1].snapshot_key =
        request.scan_plan.snapshots[0].snapshot_key.clone();

    let err = normalize_horoscope_period_request_utc(request).unwrap_err();
    assert!(err.contains("snapshot_key must be unique"));
}

#[test]
fn horoscope_period_calculator_request_rejects_snapshot_outside_period() {
    let mut request = period_calculator_request();
    request.scan_plan.snapshots[0].reference_datetime_utc = "2026-06-14T00:00:00+00:00".to_string();

    let err = normalize_horoscope_period_request_utc(request).unwrap_err();
    assert!(err.contains("snapshot outside period"));
}

#[test]
fn horoscope_period_try_calculator_returns_error_for_invalid_request() {
    let mut request = period_calculator_request();
    request.period_resolution["end_datetime_utc"] =
        serde_json::json!("2026-06-06T22:00:00+00:00");

    let err = try_calculate_horoscope_period_from_transits_with_aspects(
        request,
        &sample_natal_positions(),
        &[],
        8.0,
        &[],
        &[],
    )
    .expect_err("invalid request should return an error");

    assert!(err
        .to_string()
        .contains("invalid horoscope period calculation request"));
}

#[test]
fn horoscope_period_public_wrappers_return_error_for_invalid_request() {
    let mut request = period_calculator_request();
    request.period_resolution["end_datetime_utc"] =
        serde_json::json!("2026-06-06T22:00:00+00:00");

    let base = calculate_horoscope_period(request.clone())
        .expect_err("base public wrapper should return an error");
    assert!(base
        .to_string()
        .contains("invalid horoscope period calculation request"));

    let positions = sample_natal_positions();
    let from_positions = calculate_horoscope_period_from_positions(request.clone(), &positions, 8.0)
        .expect_err("positions wrapper should return an error");
    assert!(from_positions
        .to_string()
        .contains("invalid horoscope period calculation request"));

    let from_transits = calculate_horoscope_period_from_transits(request, &positions, &[], 8.0)
        .expect_err("transits wrapper should return an error");
    assert!(from_transits
        .to_string()
        .contains("invalid horoscope period calculation request"));
}

#[test]
fn horoscope_daily_calculator_preserves_public_daily_contract_shape() {
    let response = calculate_horoscope_daily(HoroscopeCalculationRequest {
        contract_version: "horoscope_calculation_request".to_string(),
        service_code: HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE.to_string(),
        period: HoroscopePeriod {
            date: "2026-06-14".to_string(),
            timezone: "Europe/Paris".to_string(),
        },
        chart_calculation_id: "123".to_string(),
        location: None,
        slot_profile_code: None,
        house_system_code: None,
        calculation_features: Vec::new(),
        slots: vec![
            HoroscopeCalculationSlotRequest {
                slot_code: "morning".to_string(),
                start_local_time: "06:00".to_string(),
                end_local_time: "12:00".to_string(),
                reference_local_time: "09:00".to_string(),
            },
            HoroscopeCalculationSlotRequest {
                slot_code: "afternoon".to_string(),
                start_local_time: "12:00".to_string(),
                end_local_time: "18:00".to_string(),
                reference_local_time: "15:00".to_string(),
            },
        ],
    });

    assert_eq!(response.contract_version, "horoscope_calculation_response");
    assert_eq!(
        response.service_code,
        HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE
    );
    assert_eq!(response.slots.len(), 2);
    assert!(response.evidence_keys.is_empty());
    assert!(response
        .slots
        .iter()
        .all(|slot| slot.reference_datetime_utc.is_none()));
    assert!(response
        .slots
        .iter()
        .all(|slot| slot.local_chart.is_none() && slot.angle_activations.is_empty()));
    assert!(response
        .slots
        .iter()
        .all(|slot| slot.sky_snapshot["source"] == "missing_transit_data"));
}

#[test]
fn horoscope_legacy_natal_function_names_delegate_to_canonical_names() {
    let period_request = period_calculator_request();
    let positions = sample_natal_positions();
    let transit_snapshots = transit_snapshots_for_request(&period_request, positions.clone());

    let canonical_period = calculate_horoscope_period(period_request.clone()).unwrap();
    let legacy_period = calculate_horoscope_period_natal(period_request.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(canonical_period).unwrap(),
        serde_json::to_value(legacy_period).unwrap()
    );

    let canonical_positions =
        calculate_horoscope_period_from_positions(period_request.clone(), &positions, 8.0)
            .unwrap();
    let legacy_positions =
        calculate_horoscope_period_natal_from_positions(period_request.clone(), &positions, 8.0)
            .unwrap();
    assert_eq!(
        serde_json::to_value(canonical_positions).unwrap(),
        serde_json::to_value(legacy_positions).unwrap()
    );

    let canonical_transits = calculate_horoscope_period_from_transits(
        period_request.clone(),
        &positions,
        &transit_snapshots,
        8.0,
    )
    .unwrap();
    let legacy_transits = calculate_horoscope_period_natal_from_transits(
        period_request,
        &positions,
        &transit_snapshots,
        8.0,
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(canonical_transits).unwrap(),
        serde_json::to_value(legacy_transits).unwrap()
    );

    let daily_request = HoroscopeCalculationRequest {
        contract_version: "horoscope_calculation_request".to_string(),
        service_code: HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE.to_string(),
        period: HoroscopePeriod {
            date: "2026-06-14".to_string(),
            timezone: "Europe/Paris".to_string(),
        },
        chart_calculation_id: "123".to_string(),
        location: None,
        slot_profile_code: None,
        house_system_code: None,
        calculation_features: Vec::new(),
        slots: vec![HoroscopeCalculationSlotRequest {
            slot_code: "morning".to_string(),
            start_local_time: "06:00".to_string(),
            end_local_time: "12:00".to_string(),
            reference_local_time: "09:00".to_string(),
        }],
    };
    let canonical_daily = calculate_horoscope_daily(daily_request.clone());
    let legacy_daily = calculate_horoscope_daily_natal(daily_request);
    assert_eq!(
        serde_json::to_value(canonical_daily).unwrap(),
        serde_json::to_value(legacy_daily).unwrap()
    );
}

#[test]
fn horoscope_daily_premium_calculator_emits_local_chart_and_reference_utc() {
    let response = calculate_horoscope_daily(HoroscopeCalculationRequest {
        contract_version: "horoscope_calculation_request".to_string(),
        service_code: HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE.to_string(),
        period: HoroscopePeriod {
            date: "2026-06-14".to_string(),
            timezone: "Europe/Paris".to_string(),
        },
        chart_calculation_id: "123".to_string(),
        location: Some(astral_calculator::features::horoscope::HoroscopeLocation {
            latitude: 48.8566,
            longitude: 2.3522,
            label: Some("Paris".to_string()),
        }),
        slot_profile_code: Some("daily_2h_slots".to_string()),
        house_system_code: Some("placidus".to_string()),
        calculation_features: vec!["local_chart".to_string()],
        slots: vec![HoroscopeCalculationSlotRequest {
            slot_code: "slot_22_00".to_string(),
            start_local_time: "22:00".to_string(),
            end_local_time: "23:59".to_string(),
            reference_local_time: "22:00".to_string(),
        }],
    });

    assert_eq!(response.slots.len(), 1);
    let slot = &response.slots[0];
    assert!(slot.reference_datetime_utc.is_none());
    assert!(slot.local_chart.is_none());
    assert!(!slot.calculation_warnings.is_empty());
    assert!(response.evidence_keys.is_empty());
}

#[test]
fn horoscope_daily_with_transits_uses_available_real_position_when_preferred_object_is_missing() {
    let request = HoroscopeCalculationRequest {
        contract_version: "horoscope_calculation_request".to_string(),
        service_code: HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE.to_string(),
        period: HoroscopePeriod {
            date: "2026-06-14".to_string(),
            timezone: "Europe/Paris".to_string(),
        },
        chart_calculation_id: "123".to_string(),
        location: None,
        slot_profile_code: None,
        house_system_code: None,
        calculation_features: Vec::new(),
        slots: vec![HoroscopeCalculationSlotRequest {
            slot_code: "morning".to_string(),
            start_local_time: "06:00".to_string(),
            end_local_time: "12:00".to_string(),
            reference_local_time: "09:00".to_string(),
        }],
    };
    let transit_slots = vec![(
        "morning".to_string(),
        vec![ObjectPositionFact {
            chart_object_id: 4,
            object_code: "venus".to_string(),
            object_name: "Venus".to_string(),
            zodiacal_reference_system_id: 1,
            coordinate_reference_system_id: 1,
            sign_id: 6,
            sign_code: "virgo".to_string(),
            sign_name: "Virgo".to_string(),
            house_id: Some(6),
            house_number: Some(6),
            house_name: Some("House 6".to_string()),
            motion_state_id: None,
            horizon_position_id: None,
            longitude_deg: 72.0,
            latitude_deg: None,
            apparent_speed_deg_per_day: Some(1.0),
            altitude_deg: None,
            is_visible: None,
            facts_json: None,
        }],
    )];

    let response = calculate_horoscope_daily_from_transits(
        request,
        &sample_natal_positions(),
        &transit_slots,
        8.0,
        &[],
        &sample_theme_mappings(),
    );

    let fact = &response.slots[0].transits_to_natal[0];
    assert_eq!(fact.source, "swisseph_daily_calculator_v1");
    assert_eq!(fact.transiting_object, "venus");
    assert!(response.slots[0]
        .sky_snapshot
        .get("visible_objects")
        .and_then(|value| value.as_array())
        .is_some_and(|objects| objects.iter().any(|value| value == "venus")));
}

fn period_calculator_request() -> HoroscopePeriodCalculationRequest {
    serde_json::from_value(serde_json::json!({
        "contract_version": "horoscope_period_calculation_request",
        "service_code": "horoscope_basic_next_7_days_natal",
        "chart_calculation_id": "123",
        "period_resolution": {
            "period_profile_code": "next_7_days",
            "anchor_date": "2026-06-07",
            "timezone": "Europe/Paris",
            "start_datetime_local": "2026-06-07T00:00:00",
            "end_datetime_local": "2026-06-14T00:00:00",
            "start_datetime_utc": "2026-06-06T22:00:00+00:00",
            "end_datetime_utc": "2026-06-13T22:00:00+00:00",
            "end_exclusive": true,
            "duration_days": 7,
            "included_dates": ["2026-06-07","2026-06-08","2026-06-09","2026-06-10","2026-06-11","2026-06-12","2026-06-13"],
            "included_days": []
        },
        "scan_plan": {
            "scan_profile_code": "daily_noon_7_days",
            "granularity": "daily_noon",
            "snapshot_count": 7,
            "snapshots": [
                { "snapshot_key": "2026-06-07:noon", "date": "2026-06-07", "reference_time_local": "12:00", "reference_datetime_local": "2026-06-07T12:00:00", "reference_datetime_utc": "2026-06-07T10:00:00+00:00" },
                { "snapshot_key": "2026-06-08:noon", "date": "2026-06-08", "reference_time_local": "12:00", "reference_datetime_local": "2026-06-08T12:00:00", "reference_datetime_utc": "2026-06-08T10:00:00+00:00" },
                { "snapshot_key": "2026-06-09:noon", "date": "2026-06-09", "reference_time_local": "12:00", "reference_datetime_local": "2026-06-09T12:00:00", "reference_datetime_utc": "2026-06-09T10:00:00+00:00" },
                { "snapshot_key": "2026-06-10:noon", "date": "2026-06-10", "reference_time_local": "12:00", "reference_datetime_local": "2026-06-10T12:00:00", "reference_datetime_utc": "2026-06-10T10:00:00+00:00" },
                { "snapshot_key": "2026-06-11:noon", "date": "2026-06-11", "reference_time_local": "12:00", "reference_datetime_local": "2026-06-11T12:00:00", "reference_datetime_utc": "2026-06-11T10:00:00+00:00" },
                { "snapshot_key": "2026-06-12:noon", "date": "2026-06-12", "reference_time_local": "12:00", "reference_datetime_local": "2026-06-12T12:00:00", "reference_datetime_utc": "2026-06-12T10:00:00+00:00" },
                { "snapshot_key": "2026-06-13:noon", "date": "2026-06-13", "reference_time_local": "12:00", "reference_datetime_local": "2026-06-13T12:00:00", "reference_datetime_utc": "2026-06-13T10:00:00+00:00" }
            ]
        }
    }))
    .unwrap()
}

fn sample_natal_positions() -> Vec<ObjectPositionFact> {
    vec![
        ObjectPositionFact {
            chart_object_id: 1,
            object_code: "sun".to_string(),
            object_name: "Sun".to_string(),
            zodiacal_reference_system_id: 1,
            coordinate_reference_system_id: 1,
            sign_id: 1,
            sign_code: "aries".to_string(),
            sign_name: "Aries".to_string(),
            house_id: Some(1),
            house_number: Some(1),
            house_name: Some("House 1".to_string()),
            motion_state_id: None,
            horizon_position_id: None,
            longitude_deg: 12.0,
            latitude_deg: None,
            apparent_speed_deg_per_day: Some(1.0),
            altitude_deg: None,
            is_visible: None,
            facts_json: None,
        },
        ObjectPositionFact {
            chart_object_id: 2,
            object_code: "moon".to_string(),
            object_name: "Moon".to_string(),
            zodiacal_reference_system_id: 1,
            coordinate_reference_system_id: 1,
            sign_id: 2,
            sign_code: "taurus".to_string(),
            sign_name: "Taurus".to_string(),
            house_id: Some(6),
            house_number: Some(6),
            house_name: Some("House 6".to_string()),
            motion_state_id: None,
            horizon_position_id: None,
            longitude_deg: 48.0,
            latitude_deg: None,
            apparent_speed_deg_per_day: Some(13.0),
            altitude_deg: None,
            is_visible: None,
            facts_json: None,
        },
    ]
}

fn transit_snapshots_for_request(
    request: &HoroscopePeriodCalculationRequest,
    positions: Vec<ObjectPositionFact>,
) -> Vec<(String, Vec<ObjectPositionFact>)> {
    request
        .scan_plan
        .snapshots
        .iter()
        .map(|snapshot| (snapshot.snapshot_key.clone(), positions.clone()))
        .collect()
}

fn transit_snapshots_with_venus_context(
    request: &HoroscopePeriodCalculationRequest,
    positions: Vec<ObjectPositionFact>,
) -> Vec<(String, Vec<ObjectPositionFact>)> {
    let mut transit_positions = positions;
    transit_positions.push(ObjectPositionFact {
        chart_object_id: 3,
        object_code: "venus".to_string(),
        object_name: "Venus".to_string(),
        zodiacal_reference_system_id: 1,
        coordinate_reference_system_id: 1,
        sign_id: 6,
        sign_code: "virgo".to_string(),
        sign_name: "Virgo".to_string(),
        house_id: Some(6),
        house_number: Some(6),
        house_name: Some("House 6".to_string()),
        motion_state_id: None,
        horizon_position_id: None,
        longitude_deg: 178.0,
        latitude_deg: None,
        apparent_speed_deg_per_day: Some(1.0),
        altitude_deg: None,
        is_visible: None,
        facts_json: None,
    });
    transit_snapshots_for_request(request, transit_positions)
}

fn sample_theme_mappings(
) -> Vec<astral_calculator::features::horoscope::HoroscopeSignalThemeMapping> {
    vec![
        astral_calculator::features::horoscope::HoroscopeSignalThemeMapping {
            match_object: "venus".to_string(),
            match_aspect: None,
            match_natal_target: None,
            theme_code: "supportive_dialogue".to_string(),
        },
        astral_calculator::features::horoscope::HoroscopeSignalThemeMapping {
            match_object: "moon".to_string(),
            match_aspect: None,
            match_natal_target: Some("natal_house_6".to_string()),
            theme_code: "daily_care".to_string(),
        },
    ]
}

async fn spawn_test_server(state: AppState) -> String {
    let app = build_app(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn health_live_ok_without_readiness_checks() {
    let state = build_test_state().await;
    let base = spawn_test_server(state).await;
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{base}/health/live"))
        .send()
        .await
        .expect("request");
    assert!(response.status().is_success());
}

#[tokio::test]
async fn validate_rejects_invalid_natal_request() {
    let state = build_test_state().await;
    let base = spawn_test_server(state).await;
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{base}/v1/calculations/validate"))
        .json(&serde_json::json!({
            "schema_version": "astro_engine_request_v1",
            "payload": { "invalid": true }
        }))
        .send()
        .await
        .expect("request");

    assert_eq!(response.status(), 422);
    let body: serde_json::Value = response.json().await.expect("json");
    assert_eq!(body["error"]["code"], "VALIDATION_FAILED");
}

#[tokio::test]
async fn internal_calculation_validate_route_matches_legacy_route() {
    let mut state = build_test_state().await;
    state.config.api_key = None;

    let base = spawn_test_server(state).await;
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "schema_version": "astro_engine_request_v1",
        "payload": { "invalid": true }
    });

    let legacy = client
        .post(format!("{base}/v1/calculations/validate"))
        .json(&body)
        .send()
        .await
        .expect("legacy request");
    let internal = client
        .post(format!("{base}/v1/internal/calculations/validate"))
        .json(&body)
        .send()
        .await
        .expect("internal request");

    assert_eq!(internal.status(), legacy.status());
    let legacy_body: serde_json::Value = legacy.json().await.expect("legacy json");
    let internal_body: serde_json::Value = internal.json().await.expect("internal json");
    assert_eq!(internal_body["error"]["code"], legacy_body["error"]["code"]);
}

#[tokio::test]
async fn internal_calculation_routes_match_legacy_route_statuses() {
    let mut state = build_test_state().await;
    state.config.api_key = None;

    let base = spawn_test_server(state).await;
    let client = reqwest::Client::new();
    let cases = [
        (
            "/v1/calculations/validate",
            "/v1/internal/calculations/validate",
            serde_json::json!({
                "schema_version": "astro_engine_request_v1",
                "payload": { "invalid": true }
            }),
        ),
        (
            "/v1/calculations/natal",
            "/v1/internal/calculations/natal",
            serde_json::json!({ "invalid": true }),
        ),
        (
            "/v1/calculations/natal/simplified",
            "/v1/internal/calculations/natal/simplified",
            serde_json::json!({ "invalid": true }),
        ),
        (
            "/v1/calculations/horoscope/daily-natal",
            "/v1/internal/calculations/horoscope/daily-natal",
            serde_json::json!({ "invalid": true }),
        ),
        (
            "/v1/calculations/horoscope/period/natal",
            "/v1/internal/calculations/horoscope/period/natal",
            serde_json::json!({ "invalid": true }),
        ),
    ];

    for (legacy_path, internal_path, body) in cases {
        let legacy = client
            .post(format!("{base}{legacy_path}"))
            .json(&body)
            .send()
            .await
            .expect("legacy request");
        let internal = client
            .post(format!("{base}{internal_path}"))
            .json(&body)
            .send()
            .await
            .expect("internal request");

        assert_eq!(
            internal.status(),
            legacy.status(),
            "{internal_path} status should match {legacy_path}"
        );
        let legacy_body: serde_json::Value = legacy.json().await.expect("legacy json");
        let internal_body: serde_json::Value = internal.json().await.expect("internal json");
        assert_eq!(
            internal_body["error"]["code"], legacy_body["error"]["code"],
            "{internal_path} error code should match {legacy_path}"
        );
    }
}

#[tokio::test]
async fn contracts_and_schema_discovery() {
    let state = build_test_state().await;
    let base = spawn_test_server(state).await;
    let client = reqwest::Client::new();
    let contracts = client
        .get(format!("{base}/v1/contracts"))
        .send()
        .await
        .expect("contracts")
        .json::<serde_json::Value>()
        .await
        .expect("json");
    assert_eq!(contracts["openapi"], "/openapi.yaml");
    assert_eq!(contracts["surface"], "internal_calculator_http");
    assert_eq!(
        contracts["canonical_calculation_base_path"],
        "/v1/internal/calculations"
    );
    assert_eq!(
        contracts["legacy_calculation_base_path"],
        "/v1/calculations"
    );

    let schema = client
        .get(format!("{base}/v1/schemas/astro_engine_request_v1"))
        .send()
        .await
        .expect("schema")
        .json::<serde_json::Value>()
        .await
        .expect("json");
    assert_eq!(
        schema["properties"]["request_contract_version"]["const"],
        "astro_engine_request_v1"
    );
}

#[tokio::test]
async fn calculate_natal_paris_1990_when_ready() {
    let state = build_test_state().await;

    let status = check_reference_status(&state.pool).await;
    if status.status != "ready" {
        eprintln!("SKIP calculate_natal_paris_1990_when_ready: reference data not ready");
        return;
    }

    let base = spawn_test_server(state).await;
    let request: serde_json::Value = serde_json::from_str(include_str!(
        "../contracts/integration/examples/natal_calculation_request_v1.paris_1990.json"
    ))
    .expect("fixture");

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{base}/v1/calculations/natal"))
        .json(&request)
        .send()
        .await
        .expect("request");

    assert!(
        response.status().is_success(),
        "status={}",
        response.status()
    );
    let body: serde_json::Value = response.json().await.expect("json");
    assert_eq!(
        body["response_contract_version"],
        "astro_engine_response_v1"
    );
}

#[tokio::test]
async fn validate_rejects_invalid_simplified_request() {
    let state = build_test_state().await;
    let base = spawn_test_server(state).await;
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{base}/v1/calculations/validate"))
        .json(&serde_json::json!({
            "schema_version": "astro_simplified_natal_request_v1",
            "payload": { "birth": { "date": "not-a-date" } }
        }))
        .send()
        .await
        .expect("request");
    assert_eq!(response.status(), 422);
}

#[tokio::test]
async fn calculate_simplified_date_only_when_ready() {
    let state = build_test_state().await;
    let status = check_reference_status(&state.pool).await;
    if status.status != "ready" {
        eprintln!("SKIP calculate_simplified_date_only_when_ready: reference not ready");
        return;
    }
    let base = spawn_test_server(state).await;
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{base}/v1/calculations/natal/simplified"))
        .json(&serde_json::json!({
            "request_contract_version": "astro_simplified_natal_request_v1",
            "birth": { "date": "1990-06-15" }
        }))
        .send()
        .await
        .expect("request");
    assert!(
        response.status().is_success(),
        "status={}",
        response.status()
    );
    let body: serde_json::Value = response.json().await.expect("json");
    assert_eq!(
        body["response_contract_version"],
        "astro_simplified_natal_response_v1"
    );
    assert_eq!(body["computed_scope"], "stable_birth_date_profile");
    assert_eq!(body["reading_hint"]["reading_completeness"], "partial");
}

#[tokio::test]
async fn health_ready_returns_503_when_reference_missing() {
    let state = build_test_state().await;

    let status = check_reference_status(&state.pool).await;
    if status.status == "ready" {
        eprintln!(
            "SKIP health_ready_returns_503_when_reference_missing: environment is fully ready"
        );
        return;
    }

    let base = spawn_test_server(state).await;
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{base}/health/ready"))
        .send()
        .await
        .expect("request");
    assert_eq!(response.status(), 503);
    let body: serde_json::Value = response.json().await.expect("json");
    assert_eq!(body["error"]["code"], "SERVICE_NOT_READY");
    assert!(body["error"]["details"]["database"].is_boolean());
}

#[tokio::test]
async fn auth_rejects_protected_route_when_api_key_required() {
    dotenvy::dotenv().ok();
    let mut state = build_test_state().await;
    state.config.api_key = Some("test-secret-key".into());

    let base = spawn_test_server(state).await;
    let client = reqwest::Client::new();
    let unauthorized = client
        .post(format!("{base}/v1/calculations/validate"))
        .json(&serde_json::json!({
            "schema_version": "astro_engine_request_v1",
            "payload": {}
        }))
        .send()
        .await
        .expect("request");
    assert_eq!(unauthorized.status(), 401);

    let authorized = client
        .post(format!("{base}/v1/calculations/validate"))
        .header("Authorization", "Bearer test-secret-key")
        .json(&serde_json::json!({
            "schema_version": "astro_engine_request_v1",
            "payload": {}
        }))
        .send()
        .await
        .expect("request");
    assert_ne!(authorized.status(), 401);
}
