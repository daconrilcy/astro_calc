use std::sync::Arc;

use astral_calculator::config::{ephemeris_path_from_env, runtime_options_from_env};
use astral_calculator::db::connect_from_env;
use astral_calculator::domain::ObjectPositionFact;
use astral_calculator::ephemeris::SwissEphemerisEngine;
use astral_calculator::horoscope::{
    calculate_horoscope_period_natal, calculate_horoscope_period_natal_from_positions,
    calculate_horoscope_period_natal_from_transits, normalize_horoscope_period_request_utc,
    HoroscopePeriodCalculationRequest,
};
use astral_calculator::runtime::ChartCalculationRuntimeService;
use astral_calculator_api::{
    build_app, config::AppConfig, reference_status::check_reference_status,
    schema_registry::SchemaRegistry, state::AppState,
};

async fn build_test_state() -> Option<AppState> {
    dotenvy::dotenv().ok();
    let pool = connect_from_env().await.ok()?;
    let config = AppConfig::from_env();
    let ephemeris = SwissEphemerisEngine::new(ephemeris_path_from_env());
    let service =
        ChartCalculationRuntimeService::new(pool.clone(), ephemeris, runtime_options_from_env());
    let schema_registry = SchemaRegistry::from_dir(&config.schemas_dir).ok()?;

    Some(AppState {
        config,
        pool,
        service: Arc::new(service),
        schema_registry: Arc::new(schema_registry),
    })
}

#[test]
fn horoscope_period_calculator_from_positions_never_uses_fake_source() {
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
    .unwrap();
    let positions = vec![
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
    ];

    let response = calculate_horoscope_period_natal_from_positions(request, &positions);
    assert_eq!(response.snapshots.len(), 7);
    for snapshot in response.snapshots {
        for fact in snapshot.transits_to_natal {
            assert!(!fact.source.starts_with("fake"));
            assert_eq!(fact.source, "derived_period_calculator_v1");
        }
    }
}

#[test]
fn horoscope_period_calculator_public_function_never_uses_fake_source() {
    let response = calculate_horoscope_period_natal(period_calculator_request());
    for snapshot in response.snapshots {
        for fact in snapshot.transits_to_natal {
            assert!(!fact.source.starts_with("fake"));
            assert_eq!(fact.source, "derived_period_calculator_v1");
        }
    }
}

#[test]
fn horoscope_period_calculator_with_transits_uses_swisseph_source() {
    let request = period_calculator_request();
    let positions = sample_natal_positions();
    let transit_snapshots = request
        .scan_plan
        .snapshots
        .iter()
        .map(|snapshot| (snapshot.snapshot_key.clone(), positions.clone()))
        .collect::<Vec<_>>();

    let response =
        calculate_horoscope_period_natal_from_transits(request, &positions, &transit_snapshots);
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
        longitude_deg: 178.0,
        latitude_deg: None,
        apparent_speed_deg_per_day: Some(1.0),
        altitude_deg: None,
        is_visible: None,
        facts_json: None,
    });
    let transit_snapshots = request
        .scan_plan
        .snapshots
        .iter()
        .map(|snapshot| (snapshot.snapshot_key.clone(), transit_positions.clone()))
        .collect::<Vec<_>>();

    let response =
        calculate_horoscope_period_natal_from_transits(request, &positions, &transit_snapshots);
    let venus_fact = response.snapshots[1].transits_to_natal.first().unwrap();
    assert_ne!(venus_fact.fact_type, "transit_to_natal");
    assert!(venus_fact.aspect.is_none());
    assert!(venus_fact.orb_deg.is_none());
}

#[test]
fn horoscope_period_calculator_outputs_context_fact_when_no_valid_aspect() {
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
        longitude_deg: 178.0,
        latitude_deg: None,
        apparent_speed_deg_per_day: Some(1.0),
        altitude_deg: None,
        is_visible: None,
        facts_json: None,
    });
    let transit_snapshots = request
        .scan_plan
        .snapshots
        .iter()
        .map(|snapshot| (snapshot.snapshot_key.clone(), transit_positions.clone()))
        .collect::<Vec<_>>();

    let response =
        calculate_horoscope_period_natal_from_transits(request, &positions, &transit_snapshots);
    let venus_snapshot = &response.snapshots[1];
    let venus_fact = venus_snapshot.transits_to_natal.first().unwrap();
    assert_eq!(venus_fact.fact_type, "transit_context");
    assert!(venus_fact.evidence_key.contains(":context:"));
    assert!(venus_fact.orb_deg.is_none());
    assert_eq!(venus_snapshot.current_sky_aspects[0]["aspect"], "context");
    assert!(venus_snapshot.current_sky_aspects[0]["orb_deg"].is_null());
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

    let response = calculate_horoscope_period_natal(request);

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
    let Some(state) = build_test_state().await else {
        eprintln!("SKIP health_live_ok_without_readiness_checks: database unavailable");
        return;
    };
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
    let Some(state) = build_test_state().await else {
        eprintln!("SKIP validate_rejects_invalid_natal_request: database unavailable");
        return;
    };
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
async fn contracts_and_schema_discovery() {
    let Some(state) = build_test_state().await else {
        eprintln!("SKIP contracts_and_schema_discovery: database unavailable");
        return;
    };
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
    let Some(state) = build_test_state().await else {
        eprintln!("SKIP calculate_natal_paris_1990_when_ready: database unavailable");
        return;
    };

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
    let Some(state) = build_test_state().await else {
        eprintln!("SKIP validate_rejects_invalid_simplified_request: database unavailable");
        return;
    };
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
    let Some(state) = build_test_state().await else {
        eprintln!("SKIP calculate_simplified_date_only_when_ready: database unavailable");
        return;
    };
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
    let Some(state) = build_test_state().await else {
        eprintln!("SKIP health_ready_returns_503_when_reference_missing: database unavailable");
        return;
    };

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
    let Some(mut state) = build_test_state().await else {
        eprintln!("SKIP auth_rejects_protected_route_when_api_key_required");
        return;
    };
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
