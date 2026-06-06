use astral_llm_api::integration_routes::service_has_v1_orchestrator;
use astral_llm_application::horoscope::{
    aggregate_themes, build_calculation_request, build_calculation_request_for_service,
    build_interpretation_request, score_calculation, validate_horoscope_response_schema,
    validate_interpretation_request_schema, validate_public_request, validate_response_evidence,
    HOROSCOPE_FREE_DAILY_SERVICE_CODE, HOROSCOPE_SERVICE_CODE,
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

fn horoscope_free_service() -> IntegrationService {
    IntegrationService {
        service_code: HOROSCOPE_FREE_DAILY_SERVICE_CODE.into(),
        profile_code: "natal_basic".into(),
        product_code: "horoscope".into(),
        label_fr: "Horoscope free".into(),
        description_fr: "Test".into(),
        orchestration_mode: "horoscope_daily_natal".into(),
        calculation_mode: CalculationMode::None,
        service_request_contract: "integration_job_request_v1".into(),
        payload_contract: "horoscope_daily_natal_request_v1".into(),
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
        sort_order: 210,
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

fn free_calculation() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "golden/horoscope_calculation_response_v1_free_daily_paris_1990.json"
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
                "theme": "Organisation",
                "tone": "focused",
                "text": "La Lune met l'accent sur l'organisation du matin.",
                "advice": "Choisissez une action vérifiable.",
                "best_for": ["organization", "routine"],
                "watch_point": "avoid_opening_too_many_topics",
                "evidence_keys": slot_keys[0]
            },
            {
                "slot_code": "afternoon",
                "title": "Après-midi",
                "theme": "Limites émotionnelles",
                "tone": "careful",
                "text": "Mars forme un aspect tendu avec la Lune natale.",
                "advice": "Reformulez avant de répondre.",
                "best_for": ["reformulation", "boundaries"],
                "watch_point": "avoid_answering_before_the_emotion_settles",
                "evidence_keys": slot_keys[1]
            },
            {
                "slot_code": "evening",
                "title": "Soir",
                "theme": "Dialogue",
                "tone": "softer",
                "text": "Vénus soutient Mercure natal et adoucit le dialogue.",
                "advice": "Revenez sur un point précis.",
                "best_for": ["dialogue", "repair"],
                "watch_point": "avoid_reopening_every_subject_at_once",
                "evidence_keys": slot_keys[2]
            }
        ],
        "watch_points": [],
        "opportunities": [],
        "evidence_summary": [],
        "quality": {}
    })
}

fn interpretation_request() -> serde_json::Value {
    let public = validate_public_request(&public_payload()).unwrap();
    let signals = score_calculation(&calculation()).unwrap();
    build_interpretation_request(&public, &calculation(), &signals).unwrap()
}

fn golden_response() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "golden/horoscope_response_v1_basic_daily_fake.json"
    ))
    .unwrap()
}

fn free_interpretation_request() -> serde_json::Value {
    let public = validate_public_request(&public_payload()).unwrap();
    let signals = score_calculation(&free_calculation()).unwrap();
    build_interpretation_request(&public, &free_calculation(), &signals).unwrap()
}

fn free_golden_response() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "golden/horoscope_response_v1_free_daily_fake.json"
    ))
    .unwrap()
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
fn horoscope_free_payload_schema_accepts_request() {
    let validator = IntegrationJobValidator::new();
    let body = serde_json::json!({
        "service_code": HOROSCOPE_FREE_DAILY_SERVICE_CODE,
        "payload": public_payload(),
        "user_language": "fr",
        "audience_level": "beginner"
    });
    let validated = validator
        .validate_job(&body, &horoscope_free_service())
        .expect("valid free horoscope job");
    assert_eq!(validated.service_code, HOROSCOPE_FREE_DAILY_SERVICE_CODE);
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
fn horoscope_free_payload_rejects_inline_birth_data() {
    let mut payload = public_payload();
    payload["birth_data"] = serde_json::json!({
        "date": "1990-06-15",
        "time": "14:30"
    });
    let validator = IntegrationJobValidator::new();
    let body = serde_json::json!({
        "service_code": HOROSCOPE_FREE_DAILY_SERVICE_CODE,
        "payload": payload
    });
    let err = validator
        .validate_job(&body, &horoscope_free_service())
        .unwrap_err();
    assert_eq!(
        err.detail().code,
        astral_llm_domain::GenerationErrorCode::SchemaValidationFailed
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
fn horoscope_free_daily_builds_single_day_calculation_request() {
    let public = validate_public_request(&public_payload()).unwrap();
    let request =
        build_calculation_request_for_service(HOROSCOPE_FREE_DAILY_SERVICE_CODE, &public).unwrap();
    let slots = request["slots"].as_array().unwrap();
    assert_eq!(request["service_code"], HOROSCOPE_FREE_DAILY_SERVICE_CODE);
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0]["slot_code"], "day");
    assert_eq!(slots[0]["reference_local_time"], "12:00");
}

#[test]
fn horoscope_unknown_service_code_is_rejected_before_calculation_request() {
    let public = validate_public_request(&public_payload()).unwrap();
    let err = build_calculation_request_for_service("horoscope_free_daily_general", &public)
        .expect_err("unknown horoscope service must not be silently routed");
    assert_eq!(err.detail().message, "HOROSCOPE_SERVICE_NOT_IMPLEMENTED");
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
    assert!(request.get("debug_aspects").is_none());
    assert!(request["main_signals"].as_array().unwrap().len() <= 6);
    assert!(request["evidence"].as_array().unwrap().len() <= 8);
}

#[test]
fn horoscope_interpretation_request_contains_slot_shortlists() {
    let request = interpretation_request();
    let slots = request["slots"].as_array().unwrap();
    assert_eq!(slots.len(), 3);
    assert_eq!(slots[0]["slot_code"], "morning");
    assert_eq!(slots[0]["slot_label"], "Matin");
    assert_eq!(slots[0]["specificity"], "specific");
    assert_eq!(
        slots[0]["required_evidence_keys"],
        serde_json::json!(["slot:morning:moon:natal_house:6"])
    );
    assert_eq!(slots[1]["slot_label"], "Après-midi");
    assert_eq!(slots[2]["advice_axis"], "reopen_simple_dialogue");
}

#[test]
fn horoscope_free_daily_interpretation_uses_single_internal_day_slot() {
    let request = free_interpretation_request();
    let slots = request["slots"].as_array().unwrap();
    assert_eq!(request["service_code"], HOROSCOPE_FREE_DAILY_SERVICE_CODE);
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0]["slot_code"], "day");
    assert_eq!(slots[0]["slot_label"], "Aujourd’hui");
    assert_eq!(
        slots[0]["required_evidence_keys"],
        serde_json::json!(["slot:day:moon:natal_house:6"])
    );
    assert!(request["main_signals"].as_array().unwrap().len() <= 2);
    assert!(request["evidence"].as_array().unwrap().len() <= 3);
}

#[test]
fn horoscope_each_slot_has_required_evidence() {
    let request = interpretation_request();
    let evidence = request["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item["evidence_key"].as_str())
        .collect::<std::collections::HashSet<_>>();
    for slot in request["slots"].as_array().unwrap() {
        assert_eq!(slot["specificity"], "specific");
        let keys = slot["required_evidence_keys"].as_array().unwrap();
        assert!(!keys.is_empty());
        for key in keys {
            assert!(evidence.contains(key.as_str().unwrap()));
        }
    }
}

#[test]
fn horoscope_interpretation_request_does_not_contain_raw_transit_dump() {
    let request = interpretation_request();
    assert!(request.get("raw_transits").is_none());
    assert!(request.get("all_transits").is_none());
    assert!(request.get("debug_aspects").is_none());
    assert_eq!(request["slots"].as_array().unwrap().len(), 3);
    assert!(request["slots"]
        .as_array()
        .unwrap()
        .iter()
        .all(|slot| slot["main_signal_keys"].as_array().unwrap().len() <= 2));
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
fn horoscope_free_interpretation_request_matches_golden() {
    let request = free_interpretation_request();
    let golden: serde_json::Value = serde_json::from_str(include_str!(
        "golden/horoscope_interpretation_request_v1_free_daily_paris_1990.json"
    ))
    .unwrap();
    assert_eq!(request, golden);
}

#[test]
fn horoscope_interpretation_schema_rejects_basic_with_single_slot() {
    let mut request = free_interpretation_request();
    request["service_code"] = serde_json::json!(HOROSCOPE_SERVICE_CODE);
    assert!(validate_interpretation_request_schema(&request).is_err());
}

#[test]
fn horoscope_interpretation_schema_rejects_free_with_three_slots() {
    let mut request = interpretation_request();
    request["service_code"] = serde_json::json!(HOROSCOPE_FREE_DAILY_SERVICE_CODE);
    assert!(validate_interpretation_request_schema(&request).is_err());
}

#[test]
fn horoscope_response_golden_passes_schema_and_evidence_guard() {
    let request = interpretation_request();
    let response = golden_response();
    validate_response_evidence(&request, &response).unwrap();
}

#[test]
fn horoscope_free_daily_response_golden_has_no_public_slots() {
    let request = free_interpretation_request();
    let response = free_golden_response();
    validate_response_evidence(&request, &response).unwrap();
    assert!(response.get("slots").is_none());
    assert!(response.get("summary").is_some());
    assert!(response.get("advice").is_some());
    assert!(response.get("watch_point").is_some());
    assert_eq!(
        response["evidence_keys"],
        serde_json::json!(["slot:day:moon:natal_house:6"])
    );
}

#[test]
fn horoscope_response_schema_accepts_free_shape() {
    validate_horoscope_response_schema(&free_golden_response()).unwrap();
}

#[test]
fn horoscope_response_schema_accepts_basic_shape() {
    validate_horoscope_response_schema(&golden_response()).unwrap();
}

#[test]
fn horoscope_response_schema_rejects_free_with_public_slots() {
    let mut response = free_golden_response();
    response["slots"] = serde_json::json!([]);
    assert!(validate_horoscope_response_schema(&response).is_err());
}

#[test]
fn horoscope_response_schema_rejects_basic_without_three_slots() {
    let mut response = golden_response();
    response.as_object_mut().unwrap().remove("slots");
    assert!(validate_horoscope_response_schema(&response).is_err());
}

#[test]
fn horoscope_basic_daily_does_not_use_free_summary_shape() {
    let response = golden_response();
    assert!(response.get("advice").is_none());
    assert!(response.get("watch_point").is_none());
    assert!(response.get("evidence_keys").is_none());
    assert_eq!(response["slots"].as_array().unwrap().len(), 3);
}

#[test]
fn horoscope_free_daily_does_not_use_basic_slots_shape() {
    let response = free_golden_response();
    assert!(response.get("slots").is_none());
    assert!(response.get("watch_points").is_none());
    assert!(response.get("opportunities").is_none());
    assert!(response.get("evidence_summary").is_none());
}

#[test]
fn horoscope_rejects_repeated_slot_bodies() {
    let request = interpretation_request();
    let mut response = golden_response();
    let repeated = response["slots"][0]["text"].clone();
    response["slots"][1]["text"] = repeated.clone();
    response["slots"][2]["text"] = repeated;
    let err = validate_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().code,
        astral_llm_domain::GenerationErrorCode::PostSafetyValidationFailed
    );
    assert_eq!(err.detail().message, "HOROSCOPE_SLOT_REPETITION_FAILED");
}

#[test]
fn horoscope_rejects_day_overview_copied_into_slots() {
    let request = interpretation_request();
    let mut response = golden_response();
    response["slots"][0]["text"] = request["day_overview"]["summary_hint"].clone();
    let err = validate_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_SLOT_REPETITION_FAILED");
}

#[test]
fn horoscope_rejects_generic_signal_wording() {
    let request = interpretation_request();
    let mut response = golden_response();
    response["slots"][0]["text"] = serde_json::json!(
        "La Lune est presente, mais les signaux du jour invitent a rester concret et nuance."
    );
    let err = validate_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_SLOT_TOO_GENERIC");
}

#[test]
fn horoscope_rejects_public_slot_codes_in_markdown() {
    let request = interpretation_request();
    let mut response = golden_response();
    response["slots"][0]["title"] = serde_json::json!("Matin [morning]");
    let err = validate_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PUBLIC_SLOT_CODE_LEAK");
}

#[test]
fn horoscope_free_daily_rejects_public_slot_code_day() {
    let request = free_interpretation_request();
    let mut response = free_golden_response();
    response["summary"]["text"] = serde_json::json!(
        "La Lune soutient l'organisation, mais le slot:day ne doit jamais être visible."
    );
    let err = validate_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PUBLIC_SLOT_CODE_LEAK");
}

#[test]
fn horoscope_free_daily_rejects_public_word_day() {
    let request = free_interpretation_request();
    let mut response = free_golden_response();
    response["summary"]["text"] = serde_json::json!(
        "La Lune soutient l'organisation, mais le code day ne doit jamais être visible dans la lecture publique."
    );
    let err = validate_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PUBLIC_SLOT_CODE_LEAK");
}

#[test]
fn horoscope_free_daily_rejects_technical_editorial_explanation() {
    let request = free_interpretation_request();
    let mut response = free_golden_response();
    response["summary"]["text"] = serde_json::json!(
        "La Lune soutient l'organisation. Cette lecture reste volontairement synthétique, avec une preuve astrologique centrale plutôt qu'un découpage horaire."
    );
    let err = validate_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_SLOT_TOO_GENERIC");
}

#[test]
fn horoscope_applies_french_typography() {
    let request = interpretation_request();
    let response = golden_response();
    validate_response_evidence(&request, &response).unwrap();
    assert_eq!(response["slots"][1]["title"], "Après-midi");
    assert!(response["summary"]["title"]
        .as_str()
        .unwrap()
        .contains("journée"));
}

#[test]
fn horoscope_requires_distinct_advice_axes() {
    let request = interpretation_request();
    let slots = request["slots"].as_array().unwrap();
    let axes = slots
        .iter()
        .filter_map(|slot| slot["advice_axis"].as_str())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(axes.len(), 3);
}

#[test]
fn horoscope_fake_writer_uses_slot_specific_evidence() {
    let request = interpretation_request();
    let response = golden_response();
    let slots = response["slots"].as_array().unwrap();
    for response_slot in slots {
        let slot_code = response_slot["slot_code"].as_str().unwrap();
        let request_slot = request["slots"]
            .as_array()
            .unwrap()
            .iter()
            .find(|slot| slot["slot_code"].as_str() == Some(slot_code))
            .unwrap();
        assert_eq!(
            response_slot["evidence_keys"],
            request_slot["required_evidence_keys"]
        );
    }
}

#[test]
fn horoscope_response_quality_flags_are_set() {
    let response = golden_response();
    assert_eq!(response["quality"]["evidence_coverage"], 1.0);
    assert_eq!(response["quality"]["slot_diversity_passed"], true);
    assert_eq!(response["quality"]["french_typography_passed"], true);
    assert_eq!(response["quality"]["generic_language_passed"], true);
}

#[test]
fn horoscope_slot_without_evidence_requires_fallback_reason() {
    let mut request = interpretation_request();
    request["slots"][0]["specificity"] = serde_json::json!("fallback");
    request["slots"][0]["fallback_reason"] = serde_json::Value::Null;
    let response = golden_response();
    let err = validate_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_SLOT_FALLBACK_INVALID");
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
fn horoscope_free_daily_evidence_guard_rejects_invented_key() {
    let request = free_interpretation_request();
    let mut response = free_golden_response();
    response["evidence_keys"] = serde_json::json!(["invented:key"]);
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
    assert!(service_has_v1_orchestrator(&horoscope_free_service()));
}
