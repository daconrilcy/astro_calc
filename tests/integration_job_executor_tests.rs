use astral_llm_application::supports_integration_service;
use astral_llm_domain::integration::{CalculationMode, IntegrationService, ServiceAvailability};

fn service(
    service_code: &str,
    orchestration_mode: &str,
    calculation_mode: CalculationMode,
    payload_contract: &str,
) -> IntegrationService {
    IntegrationService {
        service_code: service_code.into(),
        profile_code: "natal_basic".into(),
        product_code: "natal_prompter".into(),
        label_fr: "Service".into(),
        description_fr: "Service".into(),
        orchestration_mode: orchestration_mode.into(),
        orchestration_mode_typed: None,
        calculation_mode,
        service_request_contract: "integration_job_request_v1".into(),
        payload_contract: payload_contract.into(),
        service_response_contract: "integration_job_response_v1".into(),
        public_request_contract: None,
        calculator_request_contract: None,
        llm_request_contract: None,
        public_response_contract: None,
        calculation_output_contract: None,
        reading_output_contract: "generate_reading_response_v1".into(),
        sync_endpoint: None,
        async_endpoint: "/v1/jobs".into(),
        supports_async: true,
        supports_sync_legacy: false,
        supports_mercure: false,
        availability: ServiceAvailability::Active,
        example_request_json: None,
        sort_order: 1,
    }
}

#[test]
fn supports_horoscope_services_from_shared_registry() {
    let service = service(
        astral_contracts::HOROSCOPE_FREE_DAILY_SERVICE_CODE,
        "calculator_then_llm",
        CalculationMode::None,
        "horoscope_daily_natal_request",
    );

    assert!(supports_integration_service(&service));
}

#[test]
fn supports_simplified_and_full_natal_for_current_orchestrations() {
    let simplified = service(
        "natal_simplified",
        "calculator_then_llm",
        CalculationMode::SimplifiedNatal,
        "astro_simplified_natal_request_v1",
    );
    let full = service(
        "natal_basic",
        "legacy_unified",
        CalculationMode::FullNatal,
        "astro_engine_request_v1",
    );

    assert!(supports_integration_service(&simplified));
    assert!(supports_integration_service(&full));
}

#[test]
fn rejects_natal_services_when_contract_or_orchestration_is_wrong() {
    let wrong_contract = service(
        "natal_simplified",
        "calculator_then_llm",
        CalculationMode::SimplifiedNatal,
        "generate_reading_request_v1",
    );
    let wrong_mode = service(
        "natal_basic",
        "public_gateway",
        CalculationMode::FullNatal,
        "astro_engine_request_v1",
    );

    assert!(!supports_integration_service(&wrong_contract));
    assert!(!supports_integration_service(&wrong_mode));
}

#[test]
fn supports_interpretation_only_from_payload_and_rejects_others() {
    let valid = service(
        "natal_basic_from_payload",
        "llm_only",
        CalculationMode::None,
        "generate_reading_request_v1",
    );
    let wrong_contract = service(
        "natal_basic_from_payload",
        "interpretation_only",
        CalculationMode::None,
        "astro_engine_request_v1",
    );
    let none_mode = service(
        "ad_hoc",
        "calculator_then_llm",
        CalculationMode::None,
        "generate_reading_request_v1",
    );

    assert!(supports_integration_service(&valid));
    assert!(!supports_integration_service(&wrong_contract));
    assert!(!supports_integration_service(&none_mode));
}
