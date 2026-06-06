//! Tests jobs d'intégration : validation, hashing, idempotence logique.

use astral_llm_api::integration_routes::{calculate_job_expires_at, service_has_v1_orchestrator};
use astral_llm_application::IntegrationJobValidator;
use astral_llm_domain::integration::{IntegrationService, ServiceAvailability};
use astral_llm_domain::{CalculationMode, GenerationErrorCode};
use astral_llm_infra::canonical_json_hash::{canonical_json_hash, job_logical_payload};
use chrono::{Duration, TimeZone, Utc};

fn sample_simplified_service() -> IntegrationService {
    IntegrationService {
        service_code: "natal_simplified".into(),
        profile_code: "natal_simplified".into(),
        product_code: "natal_prompter".into(),
        label_fr: "Test".into(),
        description_fr: "Test".into(),
        orchestration_mode: "unified_from_birth".into(),
        calculation_mode: CalculationMode::SimplifiedNatal,
        service_request_contract: "integration_job_request_v1".into(),
        payload_contract: "astro_simplified_natal_request_v1".into(),
        service_response_contract: "integration_job_status_v1".into(),
        calculation_output_contract: Some("astro_simplified_natal_response_v1".into()),
        reading_output_contract: "generate_reading_response_v1".into(),
        sync_endpoint: None,
        async_endpoint: "POST /v1/jobs".into(),
        supports_async: true,
        supports_sync_legacy: true,
        supports_mercure: false,
        availability: ServiceAvailability::Active,
        example_request_json: None,
        sort_order: 1,
    }
}

fn from_payload_service() -> IntegrationService {
    IntegrationService {
        service_code: "natal_basic_from_payload".into(),
        profile_code: "natal_basic".into(),
        product_code: "natal_prompter".into(),
        label_fr: "Test".into(),
        description_fr: "Test".into(),
        orchestration_mode: "interpretation_only".into(),
        calculation_mode: CalculationMode::None,
        service_request_contract: "integration_job_request_v1".into(),
        payload_contract: "generate_reading_request_v1".into(),
        service_response_contract: "integration_job_status_v1".into(),
        calculation_output_contract: None,
        reading_output_contract: "generate_reading_response_v1".into(),
        sync_endpoint: None,
        async_endpoint: "POST /v1/jobs".into(),
        supports_async: true,
        supports_sync_legacy: true,
        supports_mercure: false,
        availability: ServiceAvailability::Planned,
        example_request_json: None,
        sort_order: 2,
    }
}

fn unsupported_active_service() -> IntegrationService {
    IntegrationService {
        service_code: "solar_return".into(),
        profile_code: "natal_basic".into(),
        product_code: "natal_prompter".into(),
        label_fr: "Test".into(),
        description_fr: "Test".into(),
        orchestration_mode: "unsupported".into(),
        calculation_mode: CalculationMode::FullNatal,
        service_request_contract: "integration_job_request_v1".into(),
        payload_contract: "astro_engine_request_v1".into(),
        service_response_contract: "integration_job_status_v1".into(),
        calculation_output_contract: Some("astro_engine_response_v1".into()),
        reading_output_contract: "generate_reading_response_v1".into(),
        sync_endpoint: None,
        async_endpoint: "POST /v1/jobs".into(),
        supports_async: true,
        supports_sync_legacy: false,
        supports_mercure: false,
        availability: ServiceAvailability::Active,
        example_request_json: None,
        sort_order: 99,
    }
}

#[test]
fn job_logical_hash_differs_on_user_language() {
    let a = serde_json::json!({
        "service_code": "natal_simplified",
        "payload": { "birth": { "date": "1990-01-01" } },
        "user_language": "fr",
        "audience_level": "beginner"
    });
    let mut b = a.clone();
    b["user_language"] = serde_json::json!("en");
    assert_ne!(
        canonical_json_hash(&job_logical_payload(&a)),
        canonical_json_hash(&job_logical_payload(&b))
    );
}

#[test]
fn job_logical_hash_stable_on_key_order() {
    let a = serde_json::json!({
        "service_code": "natal_simplified",
        "payload": { "z": 1, "a": 2 },
        "user_language": "fr",
        "audience_level": "beginner"
    });
    let b = serde_json::json!({
        "audience_level": "beginner",
        "service_code": "natal_simplified",
        "user_language": "fr",
        "payload": { "a": 2, "z": 1 }
    });
    assert_eq!(
        canonical_json_hash(&job_logical_payload(&a)),
        canonical_json_hash(&job_logical_payload(&b))
    );
}

#[test]
fn job_ttl_uses_configured_hours() {
    let now = Utc.with_ymd_and_hms(2026, 6, 6, 12, 0, 0).unwrap();

    assert_eq!(calculate_job_expires_at(now, 24), now + Duration::hours(24));
    assert_eq!(
        calculate_job_expires_at(now, 168),
        now + Duration::hours(168)
    );
    assert_eq!(calculate_job_expires_at(now, 0), now + Duration::hours(1));
}

#[test]
fn job_persistence_defines_terminal_purge_query() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap();
    let source = std::fs::read_to_string(
        root.join("astral_llm/crates/astral_llm_infra/src/job_persistence.rs"),
    )
    .unwrap();

    assert!(source.contains("pub async fn purge_expired_terminal_jobs"));
    assert!(source.contains("DELETE FROM llm_jobs"));
    assert!(source.contains("expires_at < NOW()"));
    for status in [
        "completed",
        "failed",
        "safety_rejected",
        "cancelled",
        "expired",
    ] {
        assert!(
            source.contains(status),
            "missing purge terminal status: {status}"
        );
    }
}

#[test]
fn job_persistence_checks_api_key_before_idempotent_replay() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap();
    let source = std::fs::read_to_string(
        root.join("astral_llm/crates/astral_llm_infra/src/job_persistence.rs"),
    )
    .unwrap();

    assert!(source.contains("SELECT run_id, service_code, api_key_id"));
    assert!(source.contains("api_key_id != job.api_key_id"));
    assert!(source.contains("IdempotentJobClaim::ApiKeyMismatch"));
}

#[test]
fn from_payload_gate_rejects_profile_mismatch() {
    let validator = IntegrationJobValidator::new();
    let mut service = from_payload_service();
    service.availability = ServiceAvailability::Active;
    let body = serde_json::json!({
        "service_code": "natal_basic_from_payload",
        "payload": {
            "product_context": {
                "interpretation_profile_code": "natal_premium"
            },
            "astro_result": {
                "contract_version": "natal_structured_v13",
                "chart_type": "natal",
                "data": {}
            }
        },
        "user_language": "fr",
        "audience_level": "beginner"
    });
    let err = validator.validate_job(&body, &service).unwrap_err();
    assert_eq!(
        err.detail().code,
        GenerationErrorCode::SchemaValidationFailed
    );
}

#[test]
fn envelope_service_code_must_match_catalogue_service() {
    let validator = IntegrationJobValidator::new();
    let mut service = from_payload_service();
    service.availability = ServiceAvailability::Active;
    let body = serde_json::json!({
        "service_code": "natal_premium_from_payload",
        "payload": {
            "product_context": {
                "interpretation_profile_code": "natal_basic"
            },
            "astro_result": {
                "contract_version": "natal_structured_v13",
                "chart_type": "natal",
                "data": {}
            }
        },
        "user_language": "fr",
        "audience_level": "beginner"
    });

    let err = validator.validate_job(&body, &service).unwrap_err();
    assert_eq!(err.detail().code, GenerationErrorCode::InvalidInput);
}

#[test]
fn active_service_without_v1_orchestrator_is_not_executable() {
    assert!(service_has_v1_orchestrator(&sample_simplified_service()));

    let mut from_payload = from_payload_service();
    from_payload.availability = ServiceAvailability::Active;
    assert!(service_has_v1_orchestrator(&from_payload));

    assert!(!service_has_v1_orchestrator(&unsupported_active_service()));
}

#[test]
fn planned_service_not_submittable() {
    let service = from_payload_service();
    assert!(!service.availability.is_submittable());
}

#[test]
fn simplified_envelope_requires_service_code() {
    let validator = IntegrationJobValidator::new();
    let service = sample_simplified_service();
    let body = serde_json::json!({
        "payload": {},
        "user_language": "fr"
    });
    let err = validator.validate_job(&body, &service).unwrap_err();
    assert_eq!(err.detail().code, GenerationErrorCode::InvalidInput);
}

#[test]
fn engine_reading_golden_minimal_response() {
    use astral_llm_application::{build_reading_request_from_engine, validate_engine_response};
    use std::fs;
    use std::path::PathBuf;

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap();
    let engine_path =
        root.join("contracts/integration/examples/natal_calculation_response_v1.paris_1990.json");
    if !engine_path.exists() {
        return;
    }
    let engine: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&engine_path).unwrap()).unwrap();
    validate_engine_response(&engine).expect("valid engine response");
    let req = build_reading_request_from_engine(
        &engine,
        "natal_basic",
        "fr",
        astral_llm_domain::generation_request::AudienceLevel::Beginner,
        None,
        None,
    )
    .expect("build reading request");
    assert_eq!(
        req.product_context.interpretation_profile_code.as_deref(),
        Some("natal_basic")
    );
    assert_eq!(req.astro_result.contract_version, "natal_structured_v13");
}
