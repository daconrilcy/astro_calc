use std::fs;
use std::path::{Path, PathBuf};

use astral_llm_api::api_contracts::load_published_schema;
use astral_llm_api::integration_routes::service_has_v1_orchestrator;
use astral_llm_application::horoscope::{
    build_period_writer_request, fake_period_writer_response, period_writer_messages,
    postprocess_period_provider_response, score_calculation, validate_period_public_request,
    validate_period_response_contract, validate_period_writer_request_schema,
    HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE, HOROSCOPE_FREE_DAILY_SERVICE_CODE,
    HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
    HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
    HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE, HOROSCOPE_SERVICE_CODE,
};
use astral_llm_domain::integration::{CalculationMode, IntegrationService, ServiceAvailability};
use serde_json::{json, Value};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root")
}

fn load_json(relative: &str) -> Value {
    let path = repo_root().join(relative);
    serde_json::from_str(&fs::read_to_string(path).expect("read fixture")).expect("json fixture")
}

fn period_public_request() -> astral_llm_application::horoscope::HoroscopePeriodPublicRequest {
    validate_period_public_request(&json!({
        "anchor_date": "2026-06-07",
        "timezone": "Europe/Paris",
        "chart_calculation_id": "chart_123",
        "target_language_code": "fr",
        "audience_level": "general",
        "astrologer_persona": {
            "persona_id": "narrative_fr",
            "tone": ["concret", "nuance"],
            "interpretation_style": "Mettre l'accent sur l'usage pratique."
        }
    }))
    .expect("valid public request")
}

fn period_writer_request(relative_fixture: &str) -> Value {
    let public = period_public_request();
    let calculation = load_json(relative_fixture);
    build_period_writer_request(&public, &calculation).expect("writer request")
}

#[test]
fn daily_scoring_accepts_current_supported_object_seed_contract() {
    let calculation =
        load_json("tests/golden/horoscope_calculation_response_free_daily_paris_1990.json");
    let scored = score_calculation(&calculation).expect("daily scoring");

    assert!(
        scored
            .iter()
            .any(|signal| signal.transiting_object == "moon"),
        "daily scoring must load active objects from horoscope_supported_objects.json"
    );
}

fn sample_service(
    service_code: &str,
    payload_contract: &str,
    calculation_output_contract: &str,
    reading_output_contract: &str,
) -> IntegrationService {
    IntegrationService {
        service_code: service_code.into(),
        profile_code: "natal_basic".into(),
        product_code: "horoscope".into(),
        label_fr: "Horoscope".into(),
        description_fr: "Test".into(),
        orchestration_mode: if service_code.contains("next_7_days") {
            "horoscope_period_natal".into()
        } else if service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
            "horoscope_premium_daily_local".into()
        } else if service_code == HOROSCOPE_FREE_DAILY_SERVICE_CODE {
            "horoscope_daily_natal".into()
        } else {
            "horoscope_basic_daily_natal".into()
        },
        orchestration_mode_typed: None,
        calculation_mode: CalculationMode::None,
        service_request_contract: "integration_job_request_v1".into(),
        payload_contract: payload_contract.into(),
        service_response_contract: "integration_job_status_v1".into(),
        public_request_contract: None,
        calculator_request_contract: None,
        llm_request_contract: None,
        public_response_contract: None,
        calculation_output_contract: Some(calculation_output_contract.into()),
        reading_output_contract: reading_output_contract.into(),
        sync_endpoint: None,
        async_endpoint: "POST /v1/jobs".into(),
        supports_async: true,
        supports_sync_legacy: false,
        supports_mercure: false,
        availability: ServiceAvailability::Active,
        example_request_json: None,
        sort_order: 10,
    }
}

#[test]
fn horoscope_services_keep_active_orchestrators_after_contract_rename() {
    let services = [
        sample_service(
            HOROSCOPE_SERVICE_CODE,
            "horoscope_basic_daily_natal_request",
            "horoscope_calculation_response",
            "horoscope_response",
        ),
        sample_service(
            HOROSCOPE_FREE_DAILY_SERVICE_CODE,
            "horoscope_daily_natal_request",
            "horoscope_calculation_response",
            "horoscope_response",
        ),
        sample_service(
            HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
            "horoscope_premium_daily_local_request",
            "horoscope_calculation_response",
            "horoscope_response",
        ),
        sample_service(
            HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
            "horoscope_period_natal_request",
            "horoscope_period_calculation_response",
            "horoscope_period_response",
        ),
        sample_service(
            HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE,
            "horoscope_period_natal_request",
            "horoscope_period_calculation_response",
            "horoscope_period_response",
        ),
        sample_service(
            HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE,
            "horoscope_period_natal_request",
            "horoscope_period_calculation_response",
            "horoscope_period_response",
        ),
    ];

    for service in services {
        assert!(service_has_v1_orchestrator(&service));
    }
}

#[test]
fn published_horoscope_contracts_are_available_without_v1_suffixes() {
    for contract in [
        "horoscope_daily_natal_request",
        "horoscope_basic_daily_natal_request",
        "horoscope_premium_daily_local_request",
        "horoscope_interpretation_request",
        "horoscope_response",
        "horoscope_period_natal_request",
        "horoscope_period_interpretation_request",
        "horoscope_period_writer_request",
        "horoscope_period_response",
        "horoscope_calculation_request",
        "horoscope_calculation_response",
        "horoscope_period_calculation_request",
        "horoscope_period_calculation_response",
    ] {
        assert!(
            load_published_schema(contract).is_some(),
            "missing published contract {contract}"
        );
    }
}

#[test]
fn period_writer_request_uses_unified_contract_for_all_subscription_levels() {
    let fixtures = [
        (
            HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
            "tests/golden/horoscope_period_calculation_response_free_next_7_days_paris_1990.json",
            "free_compact",
        ),
        (
            HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE,
            "tests/golden/horoscope_period_calculation_response_basic_next_7_days_paris_1990.json",
            "basic_standard",
        ),
        (
            HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE,
            "tests/golden/horoscope_period_calculation_response_premium_next_7_days_paris_1990.json",
            "premium_rich",
        ),
    ];

    for (service_code, fixture, detail_profile_code) in fixtures {
        let request = period_writer_request(fixture);
        assert_eq!(
            request["contract_version"],
            "horoscope_period_writer_request"
        );
        assert_eq!(request["service_code"], service_code);
        assert_eq!(request["detail_profile_code"], detail_profile_code);
        assert_eq!(
            request["output_contract_version"],
            "horoscope_period_response"
        );
        assert_eq!(request["target_language_code"], "fr");
        assert!(request["semantic_brief"].is_object());
        assert!(request["evidence"]
            .as_array()
            .is_some_and(|items| !items.is_empty()));
        validate_period_writer_request_schema(&request).expect("writer request schema");
    }
}

#[test]
fn period_writer_request_preserves_astrologer_persona_injection() {
    let request = period_writer_request(
        "tests/golden/horoscope_period_calculation_response_premium_next_7_days_paris_1990.json",
    );
    assert_eq!(request["astrologer_persona"]["persona_id"], "narrative_fr");
    assert_eq!(request["target_language_code"], "fr");
}

#[test]
fn fake_period_writer_response_matches_contract_for_each_period_profile() {
    let fixtures = [
        "tests/golden/horoscope_period_calculation_response_free_next_7_days_paris_1990.json",
        "tests/golden/horoscope_period_calculation_response_basic_next_7_days_paris_1990.json",
        "tests/golden/horoscope_period_calculation_response_premium_next_7_days_paris_1990.json",
    ];

    for fixture in fixtures {
        let request = period_writer_request(fixture);
        let response = fake_period_writer_response(&request).expect("fake response");
        validate_period_response_contract(&request, &response).expect("response contract");
    }
}

#[test]
fn period_postprocess_removes_provider_artifact_suffixes() {
    let request = period_writer_request(
        "tests/golden/horoscope_period_calculation_response_basic_next_7_days_paris_1990.json",
    );
    let mut response = fake_period_writer_response(&request).expect("fake period response");
    response["week_overview"]["trajectory"] = json!(
        "ouverture -> mise en mouvement -> pivot -> consolidation -> cloture (verification des limites)\"}'}]}</structured_reading> PMID:INVALID.json. The output seems malformed: I accidentally included extra braces and invalid parts. Need produce valid JSON per schema. Time's up."
    );

    let processed = postprocess_period_provider_response(&request, response);
    let trajectory = processed["week_overview"]["trajectory"]
        .as_str()
        .expect("trajectory text");

    assert!(trajectory.contains("passage à l'action"));
    assert!(!trajectory.contains("</structured_reading>"));
    assert!(!trajectory.contains("PMID:INVALID.json"));
    assert!(!trajectory.contains("The output seems malformed"));
    validate_period_response_contract(&request, &processed).expect("processed contract");
}

#[test]
fn period_postprocess_repairs_internal_phase_trajectory_artifacts() {
    let request = period_writer_request(
        "tests/golden/horoscope_period_calculation_response_basic_next_7_days_paris_1990.json",
    );
    let mut response = fake_period_writer_response(&request).expect("fake period response");
    response["week_overview"]["trajectory"] =
        json!("ouverture mise_en_mouvement pivot consolidation clôture (prudente) } } (removed)");

    let processed = postprocess_period_provider_response(&request, response);
    let trajectory = processed["week_overview"]["trajectory"]
        .as_str()
        .expect("trajectory text");

    assert!(!trajectory.contains("mise_en_mouvement"));
    assert!(!trajectory.contains('{'));
    assert!(!trajectory.contains('}'));
    assert!(!trajectory.to_lowercase().contains("(removed)"));
    assert!(trajectory.contains("passage à l'action"));
    validate_period_response_contract(&request, &processed).expect("processed contract");
}

#[test]
fn period_postprocess_runs_text_reprocessing_on_public_fields_only() {
    let request = period_writer_request(
        "tests/golden/horoscope_period_calculation_response_basic_next_7_days_paris_1990.json",
    );
    let mut response = fake_period_writer_response(&request).expect("fake period response");
    response["daily_timeline"][0]["text"] = json!("l impression demande d ajuster.");
    response["daily_timeline"][0]["evidence_keys"] =
        json!(["period:2026-06-14:2026-06-14:noon:moon:natal_house:10"]);

    let processed = postprocess_period_provider_response(&request, response);

    assert!(processed["daily_timeline"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .contains("l'impression"));
    assert_eq!(
        processed["daily_timeline"][0]["evidence_keys"],
        json!(["period:2026-06-14:2026-06-14:noon:moon:natal_house:10"])
    );
}

#[test]
fn period_postprocess_replaces_em_dash_in_public_text() {
    let request = period_writer_request(
        "tests/golden/horoscope_period_calculation_response_basic_next_7_days_paris_1990.json",
    );
    let mut response = fake_period_writer_response(&request).expect("fake period response");
    response["week_overview"]["text"] =
        json!("La semaine commence par un cadrage — puis avance vers une clarification.");

    let processed = postprocess_period_provider_response(&request, response);
    let text = processed["week_overview"]["text"]
        .as_str()
        .expect("week overview text");

    assert!(text.contains("cadrage - puis"));
    assert!(!text.contains('—'));
}

#[test]
fn period_system_prompts_forbid_meta_comments_about_output() {
    let basic_request = period_writer_request(
        "tests/golden/horoscope_period_calculation_response_basic_next_7_days_paris_1990.json",
    );
    let premium_request = period_writer_request(
        "tests/golden/horoscope_period_calculation_response_premium_next_7_days_paris_1990.json",
    );

    let basic_messages = period_writer_messages(&basic_request).expect("basic prompts");
    let premium_messages = period_writer_messages(&premium_request).expect("premium prompts");

    let basic_system = &basic_messages[0].content;
    let premium_system = &premium_messages[0].content;

    for prompt in [basic_system, premium_system] {
        assert!(
            prompt.contains("Never comment on the result")
                || prompt.contains("Ne commente jamais le résultat")
        );
        assert!(prompt.contains("JSON"));
        assert!(prompt.contains("schema") || prompt.contains("schéma"));
        assert!(prompt.contains("timeout"));
        assert!(prompt.contains("truncation") || prompt.contains("troncature"));
        assert!(prompt.contains("meta-commentary") || prompt.contains("méta-commentaire"));
        assert!(prompt.contains("trajectory must be one natural public sentence"));
        assert!(prompt.contains("never underscores"));
        assert!(prompt.contains("(removed)"));
    }
}

#[test]
fn period_writer_prompt_targets_renamed_public_contract() {
    let request = period_writer_request(
        "tests/golden/horoscope_period_calculation_response_premium_next_7_days_paris_1990.json",
    );
    let prompt = period_writer_messages(&request)
        .expect("writer messages")
        .into_iter()
        .map(|message| message.content)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(prompt.contains("horoscope_period_response"));
    assert!(!prompt.contains("horoscope_period_response_v1"));
}

#[test]
fn renamed_period_response_golden_still_validates() {
    let request = period_writer_request(
        "tests/golden/horoscope_period_calculation_response_premium_next_7_days_paris_1990.json",
    );
    let response =
        load_json("tests/golden/horoscope_period_response_premium_next_7_days_fake.json");
    validate_period_response_contract(&request, &response).expect("golden response contract");
}
