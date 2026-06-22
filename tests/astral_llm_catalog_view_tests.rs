use std::sync::Arc;

use astral_llm_application::reading_catalog::ReadingCatalog;
use astral_llm_domain::integration::{CalculationMode, IntegrationService, ServiceAvailability};
use astral_llm_infra::{bootstrap_evidence_catalog, CanonicalCatalog};

#[test]
fn reading_catalog_exposes_application_owned_reading_views() {
    let mut base = CanonicalCatalog::default();
    base.integration_services.insert(
        "natal".into(),
        IntegrationService {
            service_code: "natal".into(),
            profile_code: "natal_basic".into(),
            product_code: "natal_prompter".into(),
            label_fr: "Natal".into(),
            description_fr: "Natal".into(),
            orchestration_mode: "calculator_then_llm".into(),
            orchestration_mode_typed: None,
            calculation_mode: CalculationMode::FullNatal,
            service_request_contract: "integration_job_request_v1".into(),
            payload_contract: "astro_engine_request_v1".into(),
            service_response_contract: "integration_job_status_v1".into(),
            public_request_contract: None,
            calculator_request_contract: None,
            llm_request_contract: None,
            public_response_contract: None,
            calculation_output_contract: Some("astro_engine_response_v1".into()),
            reading_output_contract: "generate_reading_response_v1".into(),
            sync_endpoint: None,
            async_endpoint: "POST /v1/jobs".into(),
            supports_async: true,
            supports_sync_legacy: false,
            supports_mercure: false,
            availability: ServiceAvailability::Active,
            example_request_json: None,
            sort_order: 1,
        },
    );

    let catalog = ReadingCatalog::new(Arc::new(base));
    let service = catalog
        .integration_service("natal")
        .expect("integration service available through reading catalog");

    assert_eq!(service.product_code, "natal_prompter");
    assert_eq!(
        catalog
            .integration_service("natal")
            .expect("reading catalog keeps integration service lookup")
            .service_code,
        "natal"
    );
}

#[test]
fn reading_catalog_views_expose_role_and_evidence_access_without_direct_infra_types() {
    let mut base = CanonicalCatalog::default();
    base.astro_basis_roles.insert("core".into());
    base.astro_basis_roles.insert("supporting".into());
    base.evidence = bootstrap_evidence_catalog();

    let catalog = ReadingCatalog::new(Arc::new(base));
    let roles = catalog.astro_basis_roles_view().allowed_roles();
    let identity_slots = catalog
        .evidence_catalog_view()
        .slots_for_chapter("identity");

    assert!(roles.contains("core"));
    assert!(roles.contains("supporting"));
    assert!(
        !identity_slots.is_empty(),
        "identity chapter should expose evidence slots through the application view"
    );
}
