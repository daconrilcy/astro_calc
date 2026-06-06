use astral_llm_api::integration_routes::service_has_v1_orchestrator;
use astral_llm_application::horoscope::{
    aggregate_themes, build_calculation_request, build_interpretation_request, score_calculation,
    validate_public_request, validate_response_evidence, HOROSCOPE_SERVICE_CODE,
};
use astral_llm_application::IntegrationJobValidator;
use astral_llm_domain::integration::{CalculationMode, IntegrationService, ServiceAvailability};

fn horoscope_service() -> IntegrationService {
    IntegrationService {
        service_code: HOROSCOPE_SERVICE_CODE.into(),
        profile_code: "natal_basic".into(),
        product_code: "horoscope".into(),
        label_fr: "Horoscope".into(),
        description_fr: "Test".into(),
        orchestration_mode: "horoscope_basic_daily_natal".into(),
        calculation_mode: CalculationMode::None,
        service_request_contract: "integration_job_request_v1".into(),
        payload_contract: "horoscope_basic_daily_natal_request_v1".into(),
        service_response_contract: "integration_job_status_v1".into(),
        calculation_output_contract: Some("horoscope_calculation_response_v1".into()),
        reading_output_contract: "horoscope_response_v1".into(),
        sync_endpoint: None,
        async_endpoint: "POST /v1/jobs".into(),
        supports_async: true,
        supports_sync_legacy: false,
        supports_mercure: false,
        availability: ServiceAvailability::Beta,
        example_request_json: None,
        sort_order: 200,
    }
}

fn public_payload() -> serde_json::Value {
    serde_json::json!({
        "date": "2026-06-06",
        "timezone": "Europe/Paris",
        "target_language": "fr",
        "chart_calculation_id": "123",
        "audience_level": "general"
    })
}

fn calculation() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "golden/horoscope_calculation_response_v1_basic_daily_paris_1990.json"
    ))
    .unwrap()
}

fn valid_response_with_slot_keys(slot_keys: [serde_json::Value; 3]) -> serde_json::Value {
    serde_json::json!({
        "contract_version": "horoscope_response_v1",
        "service_code": HOROSCOPE_SERVICE_CODE,
        "period": {
            "date": "2026-06-06",
            "timezone": "Europe/Paris"
        },
        "summary": {
            "title": "Une journee a ajuster avec precision",
            "text": "La journee met l'accent sur les rythmes ordinaires, les reactions emotionnelles et la qualite du dialogue."
        },
        "slots": [
            {
                "slot_code": "morning",
                "title": "Matin",
                "text": "Le matin invite a poser une action utile.",
                "advice": "Priorisez une action mesurable.",
                "evidence_keys": slot_keys[0]
            },
            {
                "slot_code": "afternoon",
                "title": "Apres-midi",
                "text": "L'apres-midi demande de ralentir les reactions.",
                "advice": "Repondez lentement si une tension monte.",
                "evidence_keys": slot_keys[1]
            },
            {
                "slot_code": "evening",
                "title": "Soir",
                "text": "Le soir aide a rouvrir une parole plus simple.",
                "advice": "Rouvrez le dialogue sur un point concret.",
                "evidence_keys": slot_keys[2]
            }
        ],
        "watch_points": [],
        "opportunities": [],
        "evidence_summary": [],
        "quality": {}
    })
}

#[test]
fn horoscope_payload_schema_accepts_v1_request() {
    let validator = IntegrationJobValidator::new();
    let body = serde_json::json!({
        "service_code": HOROSCOPE_SERVICE_CODE,
        "payload": public_payload(),
        "user_language": "fr",
        "audience_level": "beginner"
    });
    let validated = validator
        .validate_job(&body, &horoscope_service())
        .expect("valid horoscope job");
    assert_eq!(validated.service_code, HOROSCOPE_SERVICE_CODE);
}

#[test]
fn horoscope_payload_requires_chart_calculation_id() {
    let mut payload = public_payload();
    payload
        .as_object_mut()
        .unwrap()
        .remove("chart_calculation_id");
    let validator = IntegrationJobValidator::new();
    let body = serde_json::json!({
        "service_code": HOROSCOPE_SERVICE_CODE,
        "payload": payload
    });
    let err = validator
        .validate_job(&body, &horoscope_service())
        .unwrap_err();
    assert_eq!(
        err.detail().code,
        astral_llm_domain::GenerationErrorCode::SchemaValidationFailed
    );
}

#[test]
fn horoscope_payload_rejects_inline_birth_data() {
    let mut payload = public_payload();
    payload["birth_data"] = serde_json::json!({
        "date": "1990-06-15",
        "time": "14:30"
    });
    let err = validate_public_request(&payload).unwrap_err();
    assert_eq!(
        err.detail().code,
        astral_llm_domain::GenerationErrorCode::InvalidInput
    );
}

#[test]
fn horoscope_calculation_request_uses_seeded_three_slots() {
    let public = validate_public_request(&public_payload()).unwrap();
    let request = build_calculation_request(&public).unwrap();
    let slots = request["slots"].as_array().unwrap();
    assert_eq!(slots.len(), 3);
    assert_eq!(slots[0]["slot_code"], "morning");
    assert_eq!(slots[0]["reference_local_time"], "09:00");
    assert_eq!(slots[2]["slot_code"], "evening");
}

#[test]
fn horoscope_scoring_is_deterministic_and_theme_aggregation_is_stable() {
    let signals = score_calculation(&calculation()).unwrap();
    assert_eq!(signals.len(), 3);
    assert_eq!(
        signals[0].evidence_key,
        "slot:afternoon:mars:square:natal_moon"
    );
    assert_eq!(signals[0].theme_code, "emotional_boundaries");
    assert_eq!(signals[0].priority_score, 2.06);

    let themes = aggregate_themes(&signals);
    assert_eq!(themes[0]["theme_code"], "emotional_boundaries");
    assert!(themes.len() >= 2);
}

#[test]
fn horoscope_interpretation_request_is_shortlisted_not_raw_dump() {
    let public = validate_public_request(&public_payload()).unwrap();
    let signals = score_calculation(&calculation()).unwrap();
    let request = build_interpretation_request(&public, &calculation(), &signals).unwrap();
    assert!(request.get("raw_transits").is_none());
    assert!(request.get("all_transits").is_none());
    assert!(request["main_signals"].as_array().unwrap().len() <= 6);
    assert!(request["evidence"].as_array().unwrap().len() <= 8);
}

#[test]
fn horoscope_interpretation_request_matches_golden() {
    let public = validate_public_request(&public_payload()).unwrap();
    let signals = score_calculation(&calculation()).unwrap();
    let request = build_interpretation_request(&public, &calculation(), &signals).unwrap();
    let golden: serde_json::Value = serde_json::from_str(include_str!(
        "golden/horoscope_interpretation_request_v1_basic_daily_paris_1990.json"
    ))
    .unwrap();
    assert_eq!(request, golden);
}

#[test]
fn horoscope_response_golden_passes_schema_and_evidence_guard() {
    let public = validate_public_request(&public_payload()).unwrap();
    let signals = score_calculation(&calculation()).unwrap();
    let request = build_interpretation_request(&public, &calculation(), &signals).unwrap();
    let response: serde_json::Value = serde_json::from_str(include_str!(
        "golden/horoscope_response_v1_basic_daily_fake.json"
    ))
    .unwrap();
    validate_response_evidence(&request, &response).unwrap();
}

#[test]
fn horoscope_evidence_guard_rejects_invented_key() {
    let public = validate_public_request(&public_payload()).unwrap();
    let signals = score_calculation(&calculation()).unwrap();
    let request = build_interpretation_request(&public, &calculation(), &signals).unwrap();
    let response = valid_response_with_slot_keys([
        serde_json::json!(["slot:morning:moon:natal_house:6"]),
        serde_json::json!(["invented:key"]),
        serde_json::json!(["slot:evening:venus:trine:natal_mercury"]),
    ]);
    let err = validate_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().code,
        astral_llm_domain::GenerationErrorCode::PostSafetyValidationFailed
    );
}

#[test]
fn horoscope_evidence_guard_rejects_slot_without_evidence() {
    let public = validate_public_request(&public_payload()).unwrap();
    let signals = score_calculation(&calculation()).unwrap();
    let request = build_interpretation_request(&public, &calculation(), &signals).unwrap();
    let response = valid_response_with_slot_keys([
        serde_json::json!(["slot:morning:moon:natal_house:6"]),
        serde_json::json!([]),
        serde_json::json!(["slot:evening:venus:trine:natal_mercury"]),
    ]);
    let err = validate_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().code,
        astral_llm_domain::GenerationErrorCode::SchemaValidationFailed
    );
}

#[test]
fn horoscope_evidence_guard_rejects_non_string_key() {
    let public = validate_public_request(&public_payload()).unwrap();
    let signals = score_calculation(&calculation()).unwrap();
    let request = build_interpretation_request(&public, &calculation(), &signals).unwrap();
    let response = valid_response_with_slot_keys([
        serde_json::json!(["slot:morning:moon:natal_house:6"]),
        serde_json::json!([123]),
        serde_json::json!(["slot:evening:venus:trine:natal_mercury"]),
    ]);
    let err = validate_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().code,
        astral_llm_domain::GenerationErrorCode::SchemaValidationFailed
    );
}

#[test]
fn horoscope_evidence_guard_rejects_malformed_response_even_with_valid_keys() {
    let public = validate_public_request(&public_payload()).unwrap();
    let signals = score_calculation(&calculation()).unwrap();
    let request = build_interpretation_request(&public, &calculation(), &signals).unwrap();
    let response = serde_json::json!({
        "contract_version": "horoscope_response_v1",
        "service_code": HOROSCOPE_SERVICE_CODE,
        "slots": [
            { "evidence_keys": ["slot:morning:moon:natal_house:6"] },
            { "evidence_keys": ["slot:afternoon:mars:square:natal_moon"] },
            { "evidence_keys": ["slot:evening:venus:trine:natal_mercury"] }
        ]
    });
    let err = validate_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().code,
        astral_llm_domain::GenerationErrorCode::SchemaValidationFailed
    );
}

#[test]
fn horoscope_payload_rejects_unknown_timezone() {
    let mut payload = public_payload();
    payload["timezone"] = serde_json::json!("Europe/Atlantis");
    let err = validate_public_request(&payload).unwrap_err();
    assert_eq!(
        err.detail().code,
        astral_llm_domain::GenerationErrorCode::InvalidInput
    );
}

#[test]
fn horoscope_service_has_v1_orchestrator() {
    assert!(service_has_v1_orchestrator(&horoscope_service()));
}
