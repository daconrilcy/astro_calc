use std::sync::Arc;

use astral_calculator::config::{ephemeris_path_from_env, runtime_options_from_env};
use astral_calculator::db::connect_from_env;
use astral_calculator::ephemeris::SwissEphemerisEngine;
use astral_calculator::runtime::ChartCalculationRuntimeService;
use astral_calculator_api::{
    build_app, config::AppConfig, reference_status::check_reference_status, schema_registry::SchemaRegistry,
    state::AppState,
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

    assert!(response.status().is_success(), "status={}", response.status());
    let body: serde_json::Value = response.json().await.expect("json");
    assert_eq!(body["response_contract_version"], "astro_engine_response_v1");
}

#[tokio::test]
async fn health_ready_returns_503_when_reference_missing() {
    let Some(state) = build_test_state().await else {
        eprintln!("SKIP health_ready_returns_503_when_reference_missing: database unavailable");
        return;
    };

    let status = check_reference_status(&state.pool).await;
    if status.status == "ready" {
        eprintln!("SKIP health_ready_returns_503_when_reference_missing: environment is fully ready");
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
