use astral_llm_api::integration_routes::service_has_v1_orchestrator;
use astral_llm_application::horoscope::repair_period_response_shape;
use astral_llm_application::horoscope::{
    aggregate_themes, build_calculation_request, build_calculation_request_for_service,
    build_interpretation_request, build_period_calculation_request,
    build_period_calculation_request_for_service, build_period_interpretation_request,
    fake_period_writer_response, period_response_provider_schema,
    period_writer_prompt_text_for_test, postprocess_period_provider_response,
    prune_period_response_variant_fields, public_watch_point_for_theme,
    reprocess_horoscope_period_payload, score_calculation, validate_horoscope_response_schema,
    validate_interpretation_request_schema, validate_period_interpretation_request_schema,
    validate_period_provider_public_payload, validate_period_public_request,
    validate_period_response_evidence, validate_period_response_schema, validate_public_request,
    validate_response_evidence, validate_scan_plan, HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE,
    HOROSCOPE_FREE_DAILY_SERVICE_CODE, HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
    HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
    HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE, HOROSCOPE_SERVICE_CODE,
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

fn horoscope_premium_service() -> IntegrationService {
    IntegrationService {
        service_code: HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE.into(),
        profile_code: "natal_premium".into(),
        product_code: "horoscope".into(),
        label_fr: "Horoscope premium".into(),
        description_fr: "Test".into(),
        orchestration_mode: "horoscope_premium_daily_local".into(),
        calculation_mode: CalculationMode::None,
        service_request_contract: "integration_job_request_v1".into(),
        payload_contract: "horoscope_premium_daily_local_request_v1".into(),
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
        sort_order: 220,
    }
}

fn horoscope_period_service() -> IntegrationService {
    IntegrationService {
        service_code: HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE.into(),
        profile_code: "natal_basic".into(),
        product_code: "horoscope".into(),
        label_fr: "Horoscope period".into(),
        description_fr: "Test".into(),
        orchestration_mode: "horoscope_period_natal".into(),
        calculation_mode: CalculationMode::None,
        service_request_contract: "integration_job_request_v1".into(),
        payload_contract: "horoscope_period_natal_request_v1".into(),
        service_response_contract: "integration_job_status_v1".into(),
        calculation_output_contract: Some("horoscope_period_calculation_response_v1".into()),
        reading_output_contract: "horoscope_period_response_v1".into(),
        sync_endpoint: None,
        async_endpoint: "POST /v1/jobs".into(),
        supports_async: true,
        supports_sync_legacy: false,
        supports_mercure: false,
        availability: ServiceAvailability::Beta,
        example_request_json: None,
        sort_order: 230,
    }
}

fn horoscope_free_period_service() -> IntegrationService {
    IntegrationService {
        service_code: HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE.into(),
        profile_code: "natal_basic".into(),
        product_code: "horoscope".into(),
        label_fr: "Horoscope period free".into(),
        description_fr: "Test".into(),
        orchestration_mode: "horoscope_period_natal".into(),
        calculation_mode: CalculationMode::None,
        service_request_contract: "integration_job_request_v1".into(),
        payload_contract: "horoscope_period_natal_request_v1".into(),
        service_response_contract: "integration_job_status_v1".into(),
        calculation_output_contract: Some("horoscope_period_calculation_response_v1".into()),
        reading_output_contract: "horoscope_period_response_v1".into(),
        sync_endpoint: None,
        async_endpoint: "POST /v1/jobs".into(),
        supports_async: true,
        supports_sync_legacy: false,
        supports_mercure: false,
        availability: ServiceAvailability::Active,
        example_request_json: None,
        sort_order: 225,
    }
}

fn horoscope_premium_period_service() -> IntegrationService {
    IntegrationService {
        service_code: HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE.into(),
        profile_code: "natal_premium".into(),
        product_code: "horoscope".into(),
        label_fr: "Horoscope period premium".into(),
        description_fr: "Test".into(),
        orchestration_mode: "horoscope_period_natal".into(),
        calculation_mode: CalculationMode::None,
        service_request_contract: "integration_job_request_v1".into(),
        payload_contract: "horoscope_period_natal_request_v1".into(),
        service_response_contract: "integration_job_status_v1".into(),
        calculation_output_contract: Some("horoscope_period_calculation_response_v1".into()),
        reading_output_contract: "horoscope_period_response_v1".into(),
        sync_endpoint: None,
        async_endpoint: "POST /v1/jobs".into(),
        supports_async: true,
        supports_sync_legacy: false,
        supports_mercure: false,
        availability: ServiceAvailability::Beta,
        example_request_json: None,
        sort_order: 240,
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

fn premium_public_payload() -> serde_json::Value {
    serde_json::json!({
        "date": "2026-06-06",
        "timezone": "Europe/Paris",
        "target_language": "fr",
        "chart_calculation_id": "123",
        "location": {
            "latitude": 48.8566,
            "longitude": 2.3522,
            "label": "Paris"
        },
        "audience_level": "general",
        "detail_level": "premium_rich"
    })
}

fn premium_public_payload_without_label() -> serde_json::Value {
    serde_json::json!({
        "date": "2026-06-06",
        "timezone": "Europe/Paris",
        "target_language": "fr",
        "chart_calculation_id": "123",
        "location": {
            "latitude": 48.8566,
            "longitude": 2.3522
        },
        "audience_level": "general",
        "detail_level": "premium_rich"
    })
}

fn period_public_payload() -> serde_json::Value {
    serde_json::json!({
        "anchor_date": "2026-06-07",
        "timezone": "Europe/Paris",
        "target_language": "fr",
        "chart_calculation_id": "123",
        "audience_level": "general"
    })
}

fn period_calculation() -> serde_json::Value {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let request = build_period_calculation_request(&public).unwrap();
    let snapshots = request["scan_plan"]["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(idx, snapshot)| {
            let date = snapshot["date"].as_str().unwrap();
            let object = match idx % 4 {
                0 => "moon",
                1 => "venus",
                2 => "mars",
                _ => "mercury",
            };
            let aspect = if object == "mars" { "square" } else { "trine" };
            let key = if object == "moon" {
                format!("period:{date}:{}:moon:natal_house:6", snapshot["snapshot_key"].as_str().unwrap())
            } else {
                format!("period:{date}:{}:{object}:{aspect}:natal_moon", snapshot["snapshot_key"].as_str().unwrap())
            };
            serde_json::json!({
                "snapshot_key": snapshot["snapshot_key"],
                "date": date,
                "reference_datetime_utc": snapshot["reference_datetime_utc"],
                "sky_snapshot": { "visible_objects": ["sun", "moon", "venus", "mars", "mercury"] },
                "moon_context": { "moon_sign": "virgo", "natal_house": 6 },
                "transits_to_natal": [{
                    "evidence_key": key,
                    "fact_type": if object == "moon" { "moon_house_by_day" } else { "transit_to_natal" },
                    "source": "test_period",
                    "transiting_object": object,
                    "natal_target": if object == "moon" { serde_json::Value::Null } else { serde_json::json!("natal_moon") },
                    "aspect": if object == "moon" { serde_json::Value::Null } else { serde_json::json!(aspect) },
                    "orb_deg": 0.8,
                    "natal_house": if object == "moon" { serde_json::json!(6) } else { serde_json::Value::Null }
                }],
                "current_sky_aspects": [],
                "natal_house_activations": [],
                "calculation_warnings": []
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "contract_version": "horoscope_period_calculation_response_v1",
        "service_code": HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        "period_resolution": request["period_resolution"],
        "scan_plan": request["scan_plan"],
        "snapshots": snapshots,
        "calculation_warnings": [],
        "evidence_keys": []
    })
}

fn free_period_calculation() -> serde_json::Value {
    let mut calculation = period_calculation();
    calculation["service_code"] = serde_json::json!(HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE);
    calculation
}

fn free_period_interpretation_request() -> serde_json::Value {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    build_period_interpretation_request(&public, &free_period_calculation()).unwrap()
}

fn free_period_interpretation_request_from_calculation(
    calculation: &serde_json::Value,
) -> serde_json::Value {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    build_period_interpretation_request(&public, calculation).unwrap()
}

fn free_period_calculation_without_tension() -> serde_json::Value {
    let mut calculation = free_period_calculation();
    for snapshot in calculation["snapshots"].as_array_mut().unwrap() {
        let date = snapshot["date"].as_str().unwrap().to_string();
        let snapshot_key = snapshot["snapshot_key"].as_str().unwrap().to_string();
        let fact = &mut snapshot["transits_to_natal"][0];
        if fact["fact_type"] == "transit_to_natal" {
            let object = fact["transiting_object"].as_str().unwrap().to_string();
            fact["aspect"] = serde_json::json!("trine");
            fact["evidence_key"] = serde_json::json!(format!(
                "period:{}:{}:{}:trine:natal_moon",
                date, snapshot_key, object
            ));
        }
    }
    calculation
}

fn free_period_calculation_active_profile() -> serde_json::Value {
    let mut calculation = free_period_calculation();
    let objects = ["mars", "mercury", "sun", "venus", "mars", "mercury", "moon"];
    for (index, snapshot) in calculation["snapshots"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        let date = snapshot["date"].as_str().unwrap().to_string();
        let snapshot_key = snapshot["snapshot_key"].as_str().unwrap().to_string();
        let object = objects[index % objects.len()];
        let fact = &mut snapshot["transits_to_natal"][0];
        if object == "moon" {
            fact["fact_type"] = serde_json::json!("moon_house_by_day");
            fact["transiting_object"] = serde_json::json!("moon");
            fact["natal_target"] = serde_json::Value::Null;
            fact["aspect"] = serde_json::Value::Null;
            fact["natal_house"] = serde_json::json!(6);
            fact["evidence_key"] = serde_json::json!(format!(
                "period:{}:{}:moon:natal_house:6",
                date, snapshot_key
            ));
        } else {
            fact["fact_type"] = serde_json::json!("transit_to_natal");
            fact["transiting_object"] = serde_json::json!(object);
            fact["natal_target"] = serde_json::json!("natal_moon");
            fact["aspect"] = serde_json::json!("square");
            fact["orb_deg"] = serde_json::json!(0.6);
            fact["evidence_key"] = serde_json::json!(format!(
                "period:{}:{}:{}:square:natal_moon",
                date, snapshot_key, object
            ));
        }
    }
    calculation
}

fn free_period_public_word_count(response: &serde_json::Value) -> usize {
    let mut text = String::new();
    for pointer in [
        "/summary/title",
        "/summary/text",
        "/dominant_theme/theme",
        "/dominant_theme/text",
        "/watch_summary/text",
        "/advice",
    ] {
        if let Some(value) = response.pointer(pointer).and_then(|value| value.as_str()) {
            text.push_str(value);
            text.push('\n');
        }
    }
    for field in ["key_days", "evidence_summary"] {
        for item in response[field].as_array().into_iter().flatten() {
            for key in ["title", "reason", "label"] {
                if let Some(value) = item.get(key).and_then(|value| value.as_str()) {
                    text.push_str(value);
                    text.push('\n');
                }
            }
        }
    }
    text.split_whitespace().count()
}

fn premium_period_calculation() -> serde_json::Value {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let request = build_period_calculation_request_for_service(
        HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        &public,
    )
    .unwrap();
    let objects = [
        ("venus", "trine", "natal_moon"),
        ("mars", "square", "natal_mercury"),
        ("mercury", "trine", "natal_mars"),
        ("sun", "sextile", "natal_venus"),
        ("moon", "context", "natal_house_6"),
        ("jupiter", "trine", "natal_saturn"),
        ("saturn", "opposition", "natal_sun"),
    ];
    let snapshots = request["scan_plan"]["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(idx, snapshot)| {
            let date = snapshot["date"].as_str().unwrap();
            let snapshot_key = snapshot["snapshot_key"].as_str().unwrap();
            let (object, aspect, target) = objects[idx % objects.len()];
            let is_moon = object == "moon";
            let evidence_key = if is_moon {
                format!("period:{date}:{snapshot_key}:moon:natal_house:6")
            } else {
                format!("period:{date}:{snapshot_key}:{object}:{aspect}:{target}")
            };
            serde_json::json!({
                "snapshot_key": snapshot["snapshot_key"],
                "date": date,
                "reference_datetime_utc": snapshot["reference_datetime_utc"],
                "sky_snapshot": { "visible_objects": ["sun", "moon", "venus", "mars", "mercury", "jupiter", "saturn"] },
                "moon_context": { "moon_sign": "virgo", "natal_house": 6 },
                "transits_to_natal": [{
                    "evidence_key": evidence_key,
                    "fact_type": if is_moon { "moon_house_by_day" } else { "transit_to_natal" },
                    "source": "test_period_premium",
                    "transiting_object": object,
                    "natal_target": if is_moon { serde_json::Value::Null } else { serde_json::json!(target) },
                    "aspect": if is_moon { serde_json::Value::Null } else { serde_json::json!(aspect) },
                    "orb_deg": if is_moon { serde_json::Value::Null } else { serde_json::json!(0.7) },
                    "natal_house": if is_moon { serde_json::json!(6) } else { serde_json::Value::Null }
                }],
                "current_sky_aspects": [],
                "natal_house_activations": [],
                "calculation_warnings": []
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "contract_version": "horoscope_period_calculation_response_v1",
        "service_code": HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        "period_resolution": request["period_resolution"],
        "scan_plan": request["scan_plan"],
        "snapshots": snapshots,
        "calculation_warnings": [],
        "evidence_keys": []
    })
}

fn premium_period_context_only_calculation() -> serde_json::Value {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let request = build_period_calculation_request_for_service(
        HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        &public,
    )
    .unwrap();
    let objects = ["moon", "venus", "mars", "sun", "mercury", "moon", "jupiter"];
    let snapshots = request["scan_plan"]["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(idx, snapshot)| {
            let date = snapshot["date"].as_str().unwrap();
            let snapshot_key = snapshot["snapshot_key"].as_str().unwrap();
            let object = objects[idx % objects.len()];
            let is_moon = object == "moon";
            let target = match object {
                "venus" => "natal_moon",
                "mars" => "natal_mercury",
                "sun" => "natal_venus",
                "mercury" => "natal_mars",
                "jupiter" => "natal_saturn",
                _ => "natal_house",
            };
            let evidence_key = if is_moon {
                format!("period:{date}:{snapshot_key}:moon:natal_house:{}", (idx % 12) + 1)
            } else {
                format!("period:{date}:{snapshot_key}:{object}:context:{target}")
            };
            serde_json::json!({
                "snapshot_key": snapshot["snapshot_key"],
                "date": date,
                "reference_datetime_utc": snapshot["reference_datetime_utc"],
                "sky_snapshot": { "visible_objects": ["sun", "moon", "venus", "mars", "mercury", "jupiter"] },
                "moon_context": { "moon_sign": "virgo", "natal_house": (idx % 12) + 1 },
                "transits_to_natal": [{
                    "evidence_key": evidence_key,
                    "fact_type": if is_moon { "moon_house_by_day" } else { "transit_context" },
                    "source": "test_period_premium",
                    "transiting_object": object,
                    "natal_target": if is_moon { serde_json::Value::Null } else { serde_json::json!(target) },
                    "aspect": serde_json::Value::Null,
                    "orb_deg": serde_json::Value::Null,
                    "natal_house": if is_moon { serde_json::json!((idx % 12) + 1) } else { serde_json::json!((idx % 10) + 1) }
                }],
                "current_sky_aspects": [],
                "natal_house_activations": [],
                "calculation_warnings": []
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "contract_version": "horoscope_period_calculation_response_v1",
        "service_code": HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        "period_resolution": request["period_resolution"],
        "scan_plan": request["scan_plan"],
        "snapshots": snapshots,
        "calculation_warnings": [],
        "evidence_keys": []
    })
}

fn period_interpretation_request() -> serde_json::Value {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    build_period_interpretation_request(&public, &period_calculation()).unwrap()
}

fn premium_period_interpretation_request() -> serde_json::Value {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    build_period_interpretation_request(&public, &premium_period_calculation()).unwrap()
}

fn period_response_from_request(request: &serde_json::Value) -> serde_json::Value {
    let timeline = request["daily_plans"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(index, day)| {
            let theme = day["theme_label"].as_str().unwrap_or("organisation");
            let focus = match theme {
                "relations" => "les relations directes",
                "énergie" => "la manière d'agir",
                "communication" => "la manière de communiquer",
                "clarté" => "la manière de penser",
                "intégration" => "les habitudes à consolider",
                _ => "la responsabilité concrète",
            };
            serde_json::json!({
                "date": day["date"],
                "day_label": day["day_label"],
                "theme": theme,
                "tone": match day["tone"].as_str().unwrap_or("focused") {
                    "supportive" => "soutenant",
                    "careful" => "vigilant",
                    "active" => "dynamique",
                    _ => "concentré",
                },
                "text": format!("Cette journée numéro {} s'inscrit dans une progression de période, garde un lien clair avec {} et appuie {}.", index + 1, theme, focus),
                "advice": format!("Choisir une priorité liée à {} et la traiter sans isoler {} du reste de la période.", theme, focus),
                "evidence_keys": day["evidence_keys"]
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "contract_version": "horoscope_period_response_v1",
        "service_code": HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        "period_resolution": request["period_resolution"],
        "week_overview": {
            "title": "Vos 7 prochains jours",
            "text": "La semaine suit une trajectoire globale lisible, avec des appuis, des jours clés et une attention portée aux relations directes.",
            "trajectory": "La progression aide à consolider la responsabilité concrète et le rythme de travail."
        },
        "key_days": request["key_days"],
        "best_days": request["best_days"],
        "watch_days": request["watch_days"],
        "watch_summary": request["watch_summary_plan"],
        "daily_timeline": timeline,
        "domain_sections": request["domain_sections"].as_array().unwrap().iter().map(|section| serde_json::json!({
            "domain": section["domain"],
            "title": section["title"],
            "text": "Ce domaine reste relié aux relations directes et à la responsabilité concrète, avec un geste précis pour la semaine.",
            "evidence_keys": section["evidence_keys"]
        })).collect::<Vec<_>>(),
        "advice": {
            "main": "Gardez une progression simple.",
            "best_use": "Planifier et ajuster.",
            "avoid": "Isoler chaque journée du mouvement d'ensemble."
        },
        "evidence_summary": request["evidence"].as_array().unwrap().iter().map(|item| serde_json::json!({
            "date": item["date"],
            "evidence_key": item["evidence_key"],
            "label": item["human_label"]
        })).collect::<Vec<_>>(),
        "quality": {
            "daily_timeline_count": 7,
            "evidence_guard_passed": true,
            "best_watch_overlap_passed": true,
            "provider": "fake",
            "model": "fake-model",
            "fallback_used": false,
            "period_contract": "basic_next_7_days"
        }
    })
}

fn premium_period_response_from_request(request: &serde_json::Value) -> serde_json::Value {
    let mut response = period_response_from_request(request);
    response["service_code"] = serde_json::json!(HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE);
    response["key_days"] = request["key_days"].clone();
    response["best_days"] = request["best_days"].clone();
    response["watch_days"] = request["watch_days"].clone();
    response["best_windows"] = request["best_windows"].clone();
    response["watch_windows"] = request["watch_windows"].clone();
    response["strategy"] = serde_json::json!({
        "title": "Stratégie de semaine",
        "text": "La stratégie consiste à utiliser les fenêtres favorables pour poser des gestes courts, puis à ralentir dans les créneaux de vigilance. Le thème natal sert de repère concret pour relier action, communication et récupération.",
        "best_use": "Réserver les meilleurs créneaux aux échanges clairs, aux décisions simples et aux actions qui respectent le rythme de travail.",
        "recovery": "Après une fenêtre tendue, revenir aux besoins émotionnels et laisser une marge avant de conclure.",
        "evidence_keys": request["strategy"]["evidence_keys"]
    });
    response["domain_sections"] = request["domain_sections"].as_array().unwrap().iter().take(5).map(|section| serde_json::json!({
        "domain": section["domain"],
        "title": section["title"],
        "text": "Ce domaine relie les relations directes, les besoins émotionnels et la responsabilité concrète. Il donne une manière d'utiliser la semaine sans disperser l'énergie ni isoler les décisions du thème natal.",
        "evidence_keys": section["evidence_keys"]
    })).collect::<serde_json::Value>();
    response["quality"]["period_contract"] = serde_json::json!("premium_next_7_days");
    response
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

fn premium_calculation() -> serde_json::Value {
    let slots = [
        (
            "slot_00_02",
            "00:00–02:00",
            "01:00",
            "2026-06-05T23:00:00+00:00",
        ),
        (
            "slot_02_04",
            "02:00–04:00",
            "03:00",
            "2026-06-06T01:00:00+00:00",
        ),
        (
            "slot_04_06",
            "04:00–06:00",
            "05:00",
            "2026-06-06T03:00:00+00:00",
        ),
        (
            "slot_06_08",
            "06:00–08:00",
            "07:00",
            "2026-06-06T05:00:00+00:00",
        ),
        (
            "slot_08_10",
            "08:00–10:00",
            "09:00",
            "2026-06-06T07:00:00+00:00",
        ),
        (
            "slot_10_12",
            "10:00–12:00",
            "11:00",
            "2026-06-06T09:00:00+00:00",
        ),
        (
            "slot_12_14",
            "12:00–14:00",
            "13:00",
            "2026-06-06T11:00:00+00:00",
        ),
        (
            "slot_14_16",
            "14:00–16:00",
            "15:00",
            "2026-06-06T13:00:00+00:00",
        ),
        (
            "slot_16_18",
            "16:00–18:00",
            "17:00",
            "2026-06-06T15:00:00+00:00",
        ),
        (
            "slot_18_20",
            "18:00–20:00",
            "19:00",
            "2026-06-06T17:00:00+00:00",
        ),
        (
            "slot_20_22",
            "20:00–22:00",
            "21:00",
            "2026-06-06T19:00:00+00:00",
        ),
        (
            "slot_22_00",
            "22:00–00:00",
            "23:00",
            "2026-06-06T21:00:00+00:00",
        ),
    ]
    .into_iter()
    .enumerate()
    .map(|(idx, (slot_code, _label, local_time, utc))| {
        let aspect = if idx % 3 == 2 { "square" } else { "trine" };
        let object = match idx % 3 {
            0 => "moon",
            1 => "venus",
            _ => "mars",
        };
        let key = format!("slot:{slot_code}:{object}:{aspect}:natal_moon");
        serde_json::json!({
            "slot_code": slot_code,
            "reference_local_time": local_time,
            "reference_datetime_utc": utc,
            "sky_snapshot": { "visible_objects": ["moon", "venus", "mars"] },
            "moon_context": { "sign": "virgo", "natal_house": 6, "local_house": 2 },
            "transits_to_natal": [{
                "evidence_key": key,
                "fact_type": "transit_to_natal",
                "source": "test",
                "transiting_object": object,
                "natal_target": "natal_moon",
                "aspect": aspect,
                "orb_deg": 1.0,
                "natal_house": 6
            }],
            "current_sky_aspects": [],
            "natal_house_activations": [],
            "local_chart": {
                "house_system_code": "placidus",
                "ascendant": { "sign": "leo", "longitude_deg": 132.4 },
                "midheaven": { "sign": "taurus", "longitude_deg": 41.2 },
                "houses": (1..=12).map(|house| serde_json::json!({
                    "house": house,
                    "longitude_deg": house * 30
                })).collect::<Vec<_>>()
            },
            "local_house_placements": [],
            "angle_activations": [],
            "calculation_warnings": []
        })
    })
    .collect::<Vec<_>>();
    serde_json::json!({
        "contract_version": "horoscope_calculation_response_v1",
        "service_code": HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
        "period": {
            "date": "2026-06-06",
            "timezone": "Europe/Paris"
        },
        "slots": slots,
        "calculation_warnings": [],
        "evidence_keys": []
    })
}

fn calculator_response_schema() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../astral_calculator/schemas/horoscope_calculation_response_v1.schema.json"
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
                "watch_point": "Évitez d'ouvrir trop de sujets à la fois.",
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
                "watch_point": "Attendez que l'émotion se calme avant de répondre.",
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
                "watch_point": "Ne rouvrez pas tous les sujets en même temps.",
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

fn premium_interpretation_request() -> serde_json::Value {
    let public = validate_public_request(&premium_public_payload()).unwrap();
    let signals = score_calculation(&premium_calculation()).unwrap();
    build_interpretation_request(&public, &premium_calculation(), &signals).unwrap()
}

fn premium_response_from_request(request: &serde_json::Value) -> serde_json::Value {
    let timeline = request["slots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|slot| {
            serde_json::json!({
                "slot_label": slot["slot_label"],
                "title": "Clarté pratique",
                "theme": "Organisation",
                "tone": slot["tone"],
                "text": "La Lune donne un repère concret pour organiser une priorité sans disperser l'attention.",
                "advice": "Choisissez une tâche utile et terminez-la avant d'en ouvrir une autre.",
                "best_for": slot["best_for"],
                "watch_point": premium_public_watch_point(slot),
                "evidence_keys": slot["required_evidence_keys"]
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "contract_version": "horoscope_response_v1",
        "service_code": HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
        "period": request["period"],
        "summary": {
            "title": "Votre météo astrologique détaillée",
            "text": "La journée se lit par créneaux courts et reste reliée aux preuves astrologiques retenues."
        },
        "best_slots": [request["best_slots"][0].clone()],
        "watch_slots": [request["watch_slots"][0].clone()],
        "timeline": timeline,
        "domain_sections": request["domain_sections"],
        "advice": {
            "main": "Utilisez les créneaux fluides pour les décisions concrètes.",
            "best_use": "Planifier, prioriser et formuler les échanges importants.",
            "avoid": "Transformer un signal bref en certitude."
        },
        "evidence_summary": [],
        "quality": {}
    })
}

fn premium_public_watch_point(slot: &serde_json::Value) -> String {
    public_watch_point_for_theme(slot["theme_code"].as_str().unwrap_or(""))
        .unwrap()
        .unwrap_or_else(|| "Gardez un repère simple et vérifiable.".to_string())
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
fn horoscope_premium_payload_schema_accepts_request() {
    let validator = IntegrationJobValidator::new();
    let body = serde_json::json!({
        "service_code": HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
        "payload": premium_public_payload(),
        "user_language": "fr",
        "audience_level": "beginner"
    });
    let validated = validator
        .validate_job(&body, &horoscope_premium_service())
        .expect("valid premium horoscope job");
    assert_eq!(
        validated.service_code,
        HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE
    );
}

#[test]
fn horoscope_period_payload_schema_accepts_request() {
    let validator = IntegrationJobValidator::new();
    let body = serde_json::json!({
        "service_code": HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        "payload": period_public_payload()
    });
    let validated = validator
        .validate_job(&body, &horoscope_period_service())
        .expect("valid period horoscope job");
    assert_eq!(
        validated.service_code,
        HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE
    );
}

#[test]
fn horoscope_premium_next_7_days_requires_chart_calculation_id() {
    let mut payload = period_public_payload();
    payload
        .as_object_mut()
        .unwrap()
        .remove("chart_calculation_id");
    let validator = IntegrationJobValidator::new();
    let body = serde_json::json!({
        "service_code": HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        "payload": payload
    });
    let err = validator
        .validate_job(&body, &horoscope_premium_period_service())
        .unwrap_err();
    assert_eq!(
        err.detail().code,
        astral_llm_domain::GenerationErrorCode::SchemaValidationFailed
    );
}

#[test]
fn horoscope_period_payload_rejects_profile_override() {
    let mut payload = period_public_payload();
    payload["period_profile_code"] = serde_json::json!("current_workweek_monday_friday");
    let validator = IntegrationJobValidator::new();
    let body = serde_json::json!({
        "service_code": HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        "payload": payload
    });
    let err = validator
        .validate_job(&body, &horoscope_period_service())
        .unwrap_err();
    assert_eq!(
        err.detail().code,
        astral_llm_domain::GenerationErrorCode::SchemaValidationFailed
    );
}

#[test]
fn horoscope_period_anchor_date_is_local_date() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let request = build_period_calculation_request(&public).unwrap();
    assert_eq!(
        request["period_resolution"]["start_datetime_local"],
        "2026-06-07T00:00:00"
    );
    assert_eq!(request["period_resolution"]["timezone"], "Europe/Paris");
}

#[test]
fn horoscope_period_next_7_days_has_exclusive_end() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let request = build_period_calculation_request(&public).unwrap();
    assert_eq!(request["period_resolution"]["end_exclusive"], true);
    assert_eq!(
        request["period_resolution"]["end_datetime_local"],
        "2026-06-14T00:00:00"
    );
}

#[test]
fn horoscope_period_next_7_days_has_7_included_dates() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let request = build_period_calculation_request(&public).unwrap();
    let dates = request["period_resolution"]["included_dates"]
        .as_array()
        .unwrap();
    assert_eq!(dates.len(), 7);
    assert_eq!(dates[0], "2026-06-07");
    assert_eq!(dates[6], "2026-06-13");
}

#[test]
fn horoscope_period_scan_plan_has_unique_snapshot_keys() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let request = build_period_calculation_request(&public).unwrap();
    let keys = request["scan_plan"]["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|snapshot| snapshot["snapshot_key"].as_str().unwrap())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(keys.len(), 7);
}

#[test]
fn horoscope_period_daily_noon_has_one_snapshot_per_included_date() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let request = build_period_calculation_request(&public).unwrap();
    let snapshots = request["scan_plan"]["snapshots"].as_array().unwrap();
    assert_eq!(request["scan_plan"]["snapshot_count"], 7);
    assert_eq!(snapshots.len(), 7);
    assert!(snapshots
        .iter()
        .all(|snapshot| snapshot["reference_time_local"] == "12:00"));
}

#[test]
fn horoscope_free_next_7_days_uses_daily_noon_scan() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let request = build_period_calculation_request_for_service(
        HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        &public,
    )
    .unwrap();
    assert_eq!(
        request["scan_plan"]["scan_profile_code"],
        "daily_noon_7_days"
    );
    assert_eq!(request["scan_plan"]["snapshot_count"], 7);
    assert!(request["scan_plan"]["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .all(|snapshot| snapshot["reference_time_local"] == "12:00"));
}

#[test]
fn horoscope_free_next_7_days_interpretation_is_free_compact() {
    let request = free_period_interpretation_request();
    validate_period_interpretation_request_schema(&request).unwrap();
    assert_eq!(
        request["service_code"],
        HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE
    );
    assert_eq!(request["detail_profile_code"], "free_compact");
    assert_eq!(request["scan_plan"]["snapshot_count"], 7);
    assert!(request["daily_plans"].as_array().unwrap().is_empty());
    assert!((1..=2).contains(&request["key_days"].as_array().unwrap().len()));
    assert!(request["best_days"].as_array().unwrap().is_empty());
    assert!(request["watch_days"].as_array().unwrap().is_empty());
    assert!(request["key_days"]
        .as_array()
        .unwrap()
        .iter()
        .all(|day| day["title"] == "Jour à retenir"));
    assert!(request["domain_sections"].as_array().unwrap().is_empty());
    assert!(request.get("best_windows").is_none());
    assert!(request.get("watch_windows").is_none());
    assert!(request.get("strategy").is_none());
}

#[test]
fn horoscope_free_next_7_days_response_has_compact_shape() {
    let request = free_period_interpretation_request();
    let response = fake_period_writer_response(&request).unwrap();
    validate_period_response_schema(&response).unwrap();
    validate_period_response_evidence(&request, &response).unwrap();
    assert!(response.get("summary").is_some());
    assert!(response.get("dominant_theme").is_some());
    assert!(response
        .get("advice")
        .and_then(|value| value.as_str())
        .is_some());
    assert!((1..=2).contains(&response["key_days"].as_array().unwrap().len()));
    assert!((1..=3).contains(&response["evidence_summary"].as_array().unwrap().len()));
    for forbidden in [
        "daily_timeline",
        "best_days",
        "watch_days",
        "best_windows",
        "watch_windows",
        "domain_sections",
        "strategy",
    ] {
        assert!(response.get(forbidden).is_none(), "{forbidden} leaked");
    }
}

#[test]
fn horoscope_free_next_7_days_public_words_stay_within_free_bounds_across_profiles() {
    let requests = [
        free_period_interpretation_request(),
        free_period_interpretation_request_from_calculation(
            &free_period_calculation_without_tension(),
        ),
        free_period_interpretation_request_from_calculation(
            &free_period_calculation_active_profile(),
        ),
    ];

    for request in requests {
        let response = fake_period_writer_response(&request).unwrap();
        validate_period_response_evidence(&request, &response).unwrap();
        let words = free_period_public_word_count(&response);
        assert!(
            (140..=450).contains(&words),
            "free public response should stay between 140 and 450 words, got {words}"
        );
    }
}

#[test]
fn horoscope_free_next_7_days_goldens_validate_shape_and_evidence() {
    let calculation: serde_json::Value = serde_json::from_str(include_str!(
        "golden/horoscope_period_calculation_response_v1_free_next_7_days_paris_1990.json"
    ))
    .unwrap();
    assert_eq!(
        calculation["service_code"],
        HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE
    );
    assert_eq!(calculation["scan_plan"]["snapshot_count"], 7);

    let interpretation: serde_json::Value = serde_json::from_str(include_str!(
        "golden/horoscope_period_interpretation_request_v1_free_next_7_days_paris_1990.json"
    ))
    .unwrap();
    validate_period_interpretation_request_schema(&interpretation).unwrap();
    assert_eq!(interpretation["detail_profile_code"], "free_compact");

    let response: serde_json::Value = serde_json::from_str(include_str!(
        "golden/horoscope_period_response_v1_free_next_7_days_fake.json"
    ))
    .unwrap();
    validate_period_response_schema(&response).unwrap();
    validate_period_response_evidence(&interpretation, &response).unwrap();
}

#[test]
fn horoscope_free_next_7_days_rejects_basic_or_premium_leaks() {
    let request = free_period_interpretation_request();
    let mut response = fake_period_writer_response(&request).unwrap();
    response["daily_timeline"] = serde_json::json!([]);
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_FREE_DAILY_TIMELINE_LEAK"
    );

    let mut response = fake_period_writer_response(&request).unwrap();
    response["week_overview"] = serde_json::json!({
        "title": "Semaine",
        "text": "Texte",
        "trajectory": "Progression"
    });
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_FREE_WEEK_OVERVIEW_LEAK"
    );
}

#[test]
fn horoscope_free_next_7_days_watch_summary_present_requires_evidence() {
    let request = free_period_interpretation_request();
    let mut response = fake_period_writer_response(&request).unwrap();
    response["watch_summary"]["status"] = serde_json::json!("present");
    response["watch_summary"]["evidence_keys"] = serde_json::json!([]);
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PERIOD_EVIDENCE_MISSING");
}

#[test]
fn horoscope_free_next_7_days_rejects_active_watch_summary_status() {
    let request = free_period_interpretation_request();
    let mut response = fake_period_writer_response(&request).unwrap();
    response["watch_summary"]["status"] = serde_json::json!("active");
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert!(err
        .detail()
        .message
        .starts_with("HOROSCOPE_PERIOD_RESPONSE_INVALID"));
}

#[test]
fn horoscope_free_next_7_days_repair_preserves_present_watch_summary_with_allowed_evidence() {
    let request = free_period_interpretation_request();
    let mut response = fake_period_writer_response(&request).unwrap();
    let evidence_key = request["evidence"][0]["evidence_key"].clone();
    response["watch_summary"] = serde_json::json!({
        "status": "present",
        "text": "Un point demande de ralentir la réponse avant de conclure.",
        "evidence_keys": [evidence_key]
    });
    repair_period_response_shape(&request, &mut response);
    assert_eq!(response["watch_summary"]["status"], "present");
    assert_eq!(
        response["watch_summary"]["evidence_keys"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    validate_period_response_evidence(&request, &response).unwrap();
}

#[test]
fn horoscope_free_next_7_days_repair_enriches_none_watch_summary() {
    let request = free_period_interpretation_request_from_calculation(
        &free_period_calculation_without_tension(),
    );
    let mut response = fake_period_writer_response(&request).unwrap();
    response["watch_summary"] = serde_json::json!({
        "status": "none",
        "text": "",
        "evidence_keys": [request["evidence"][0]["evidence_key"].clone()]
    });
    repair_period_response_shape(&request, &mut response);

    assert_eq!(response["watch_summary"]["status"], "none");
    assert!(response["watch_summary"]["evidence_keys"]
        .as_array()
        .unwrap()
        .is_empty());
    let text = response["watch_summary"]["text"].as_str().unwrap();
    assert!(text.contains("marge d'observation"));
    assert!(text.split_whitespace().count() >= 14);
    validate_period_response_evidence(&request, &response).unwrap();
}

#[test]
fn horoscope_free_next_7_days_rejects_thin_none_watch_summary() {
    let request = free_period_interpretation_request_from_calculation(
        &free_period_calculation_without_tension(),
    );
    let mut response = fake_period_writer_response(&request).unwrap();
    response["watch_summary"] = serde_json::json!({
        "status": "none",
        "text": "Aucun point particulier cette semaine.",
        "evidence_keys": []
    });

    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_FREE_WATCH_SUMMARY_TOO_THIN"
    );
}

#[test]
fn horoscope_free_next_7_days_rejects_basic_key_day_title() {
    let request = free_period_interpretation_request();
    let mut response = fake_period_writer_response(&request).unwrap();
    response["key_days"][0]["title"] = serde_json::json!("Jour favorable");
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_FREE_KEY_DAY_TITLE_INVALID"
    );
}

#[test]
fn horoscope_free_next_7_days_repair_normalizes_key_day_title() {
    let request = free_period_interpretation_request();
    let mut response = fake_period_writer_response(&request).unwrap();
    response["key_days"][0]["title"] = serde_json::json!("Jour favorable");
    repair_period_response_shape(&request, &mut response);
    assert_eq!(response["key_days"][0]["title"], "Jour à retenir");
    validate_period_response_evidence(&request, &response).unwrap();
}

#[test]
fn horoscope_free_next_7_days_rejects_best_day_disguised_as_key_day() {
    let request = free_period_interpretation_request();
    let mut response = fake_period_writer_response(&request).unwrap();
    response["key_days"][0]["reason"] = serde_json::json!(
        "C'est le meilleur créneau favorable pour profiter d'une opportunité idéale."
    );
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_FREE_KEY_DAY_BEST_DAY_LEAK"
    );
}

#[test]
fn horoscope_free_next_7_days_rejects_thin_key_day_reason() {
    let request = free_period_interpretation_request();
    let mut response = fake_period_writer_response(&request).unwrap();
    response["key_days"][0]["reason"] = serde_json::json!("Jour important.");
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_FREE_KEY_DAY_TOO_THIN"
    );
}

#[test]
fn horoscope_free_next_7_days_missing_required_fields_use_free_guard_codes() {
    let request = free_period_interpretation_request();
    let mut response = fake_period_writer_response(&request).unwrap();
    response.as_object_mut().unwrap().remove("summary");
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_FREE_MISSING_SUMMARY"
    );

    let mut response = fake_period_writer_response(&request).unwrap();
    response["key_days"] = serde_json::json!([]);
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_FREE_MISSING_KEY_DAY"
    );

    let mut response = fake_period_writer_response(&request).unwrap();
    response["dominant_theme"]["theme"] = serde_json::json!("");
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_FREE_MISSING_DOMINANT_THEME"
    );
}

#[test]
fn horoscope_free_next_7_days_rejects_summary_with_three_french_dates() {
    let request = free_period_interpretation_request();
    let mut response = fake_period_writer_response(&request).unwrap();
    response["summary"]["text"] = serde_json::json!(
        "Le 7 juin donne un premier repère. Le 8 juin demande un ajustement simple. Le 9 juin confirme la tendance générale sans fournir de timeline complète."
    );
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_FREE_SUMMARY_TOO_MANY_EXPLICIT_DATES"
    );
}

#[test]
fn horoscope_free_next_7_days_real_response_above_hard_limit_uses_too_long_guard() {
    let request = free_period_interpretation_request();
    let mut response = fake_period_writer_response(&request).unwrap();
    response["quality"]["provider"] = serde_json::json!("openai");
    response["advice"] = serde_json::json!(std::iter::repeat("Phrase publique simple.")
        .take(180)
        .collect::<Vec<_>>()
        .join(" "));
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PERIOD_FREE_TOO_LONG");
}

#[test]
fn horoscope_free_next_7_days_rejects_too_generic_public_text() {
    let request = free_period_interpretation_request();
    let mut response = fake_period_writer_response(&request).unwrap();
    response["summary"]["text"] = serde_json::json!(
        "Les prochains jours peuvent aider à avancer avec prudence. Prenez le temps de choisir ce qui compte et gardez une approche simple."
    );
    response["dominant_theme"]["theme"] = serde_json::json!("repère");
    response["dominant_theme"]["text"] =
        serde_json::json!("Un repère général aide à garder une direction simple.");
    response["advice"] = serde_json::json!("Avancez doucement et observez ce qui évolue.");
    response["watch_summary"]["text"] =
        serde_json::json!("Une attention calme suffit pour traverser la période.");
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PERIOD_FREE_TOO_GENERIC");
}

#[test]
fn horoscope_period_schema_rejects_basic_without_timeline_shape() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response.as_object_mut().unwrap().remove("daily_timeline");
    let err = validate_period_response_schema(&response).unwrap_err();
    assert!(err
        .detail()
        .message
        .starts_with("HOROSCOPE_PERIOD_RESPONSE_INVALID"));
}

#[test]
fn horoscope_period_schema_rejects_basic_present_watch_summary_status() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["watch_summary"]["status"] = serde_json::json!("present");
    let err = validate_period_response_schema(&response).unwrap_err();
    assert!(err
        .detail()
        .message
        .starts_with("HOROSCOPE_PERIOD_RESPONSE_INVALID"));
}

#[test]
fn horoscope_period_repair_removes_free_fields_from_basic_response() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["summary"] = serde_json::json!({
        "title": "Vos 7 prochains jours",
        "text": "Cette lecture reste indicative et reformule les points utiles sans ajouter de certitude."
    });
    response["dominant_theme"] = serde_json::json!({
        "theme": "Organisation",
        "text": "Un theme compact ne doit pas rester dans la variante Basic."
    });

    repair_period_response_shape(&request, &mut response);

    assert!(response.get("summary").is_none());
    assert!(response.get("dominant_theme").is_none());
    validate_period_response_schema(&response).unwrap();
}

#[test]
fn horoscope_period_postprocess_prunes_summary_added_by_text_reprocessing() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    repair_period_response_shape(&request, &mut response);
    response = reprocess_horoscope_period_payload(response);
    assert!(response.get("summary").is_some());

    prune_period_response_variant_fields(&request, &mut response);

    assert!(response.get("summary").is_none());
    assert!(response.get("dominant_theme").is_none());
    validate_period_response_schema(&response).unwrap();
}

#[test]
fn horoscope_period_repair_restores_empty_provider_evidence_keys_from_request() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["key_days"][0]["evidence_keys"] = serde_json::json!([]);
    response["best_days"][0]["evidence_keys"] = serde_json::json!([""]);
    response["domain_sections"][0]["evidence_keys"] = serde_json::json!([""]);

    repair_period_response_shape(&request, &mut response);

    assert!(!response["key_days"][0]["evidence_keys"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(!response["best_days"][0]["evidence_keys"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(!response["domain_sections"][0]["evidence_keys"]
        .as_array()
        .unwrap()
        .is_empty());
    validate_period_response_evidence(&request, &response).unwrap();
}

#[test]
fn horoscope_premium_period_postprocess_preserves_word_bounds_after_normalization() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["week_overview"]["trajectory"] = serde_json::json!("Clarifier puis clarifier encore.");
    response["advice"]["main"] = serde_json::json!("Clarifier avant de clarifier.");
    response["quality"]["provider"] = serde_json::json!("openai");

    repair_period_response_shape(&request, &mut response);
    let response = postprocess_period_provider_response(&request, response);

    let public = response.to_string().to_lowercase();
    assert!(public.matches("clarifier").count() <= 2, "{public}");
    validate_period_provider_public_payload(&response).unwrap();
}

#[test]
fn horoscope_period_interpretation_schema_rejects_basic_without_domain_sections() {
    let mut request = period_interpretation_request();
    request["domain_sections"] = serde_json::json!([]);
    let err = validate_period_interpretation_request_schema(&request).unwrap_err();
    assert!(err
        .detail()
        .message
        .starts_with("HOROSCOPE_PERIOD_RESPONSE_INVALID"));
}

#[test]
fn horoscope_period_schema_rejects_premium_without_windows_shape() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response.as_object_mut().unwrap().remove("best_windows");
    let err = validate_period_response_schema(&response).unwrap_err();
    assert!(err
        .detail()
        .message
        .starts_with("HOROSCOPE_PERIOD_RESPONSE_INVALID"));
}

#[test]
fn horoscope_premium_next_7_days_uses_six_hour_scan() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let request = build_period_calculation_request_for_service(
        HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        &public,
    )
    .unwrap();
    assert_eq!(
        request["service_code"],
        HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE
    );
    assert_eq!(request["scan_plan"]["scan_profile_code"], "six_hour_7_days");
    assert_eq!(request["scan_plan"]["granularity"], "six_hour");
}

#[test]
fn horoscope_premium_next_7_days_builds_28_snapshots() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let request = build_period_calculation_request_for_service(
        HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        &public,
    )
    .unwrap();
    let snapshots = request["scan_plan"]["snapshots"].as_array().unwrap();
    assert_eq!(request["scan_plan"]["snapshot_count"], 28);
    assert_eq!(snapshots.len(), 28);
    assert_eq!(snapshots[0]["reference_time_local"], "00:00");
    assert_eq!(snapshots[3]["reference_time_local"], "18:00");
}

#[test]
fn horoscope_premium_next_7_days_snapshot_count_matches_duration_times_snapshots_per_day() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let request = build_period_calculation_request_for_service(
        HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        &public,
    )
    .unwrap();
    let duration = request["period_resolution"]["duration_days"]
        .as_u64()
        .unwrap();
    let snapshot_count = request["scan_plan"]["snapshot_count"].as_u64().unwrap();
    assert_eq!(snapshot_count, duration * 4);
}

#[test]
fn horoscope_premium_next_7_days_handles_midnight_utc_shift() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let request = build_period_calculation_request_for_service(
        HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        &public,
    )
    .unwrap();
    let first = &request["scan_plan"]["snapshots"][0];
    assert_eq!(first["date"], "2026-06-07");
    assert_eq!(first["reference_time_local"], "00:00");
    assert_eq!(first["reference_datetime_local"], "2026-06-07T00:00:00");
    assert_eq!(first["reference_datetime_utc"], "2026-06-06T22:00:00+00:00");
    validate_scan_plan(&request["period_resolution"], &request["scan_plan"]).unwrap();
}

#[test]
fn horoscope_premium_next_7_days_outputs_canonical_utc_offsets() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let request = build_period_calculation_request_for_service(
        HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        &public,
    )
    .unwrap();
    for pointer in [
        "/period_resolution/start_datetime_utc",
        "/period_resolution/end_datetime_utc",
    ] {
        assert!(request
            .pointer(pointer)
            .unwrap()
            .as_str()
            .unwrap()
            .ends_with("+00:00"));
    }
    for snapshot in request["scan_plan"]["snapshots"].as_array().unwrap() {
        assert!(snapshot["reference_datetime_utc"]
            .as_str()
            .unwrap()
            .ends_with("+00:00"));
    }
    validate_scan_plan(&request["period_resolution"], &request["scan_plan"]).unwrap();
}

#[test]
fn horoscope_period_snapshots_are_inside_period() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let request = build_period_calculation_request(&public).unwrap();
    validate_scan_plan(&request["period_resolution"], &request["scan_plan"]).unwrap();
}

#[test]
fn horoscope_period_handles_utc_date_shift() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let request = build_period_calculation_request(&public).unwrap();
    assert_eq!(
        request["period_resolution"]["start_datetime_utc"],
        "2026-06-06T22:00:00+00:00"
    );
}

#[test]
fn horoscope_period_rejects_invalid_scan_plan() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let mut request = build_period_calculation_request(&public).unwrap();
    request["scan_plan"]["snapshots"][0]["snapshot_key"] =
        request["scan_plan"]["snapshots"][1]["snapshot_key"].clone();
    let err = validate_scan_plan(&request["period_resolution"], &request["scan_plan"]).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PERIOD_SCAN_PLAN_INVALID");
}

#[test]
fn horoscope_period_interpretation_request_matches_schema() {
    let request = period_interpretation_request();
    assert_eq!(
        request["contract_version"],
        "horoscope_period_interpretation_request_v1"
    );
    assert_eq!(request["daily_plans"].as_array().unwrap().len(), 7);
    assert!(request.get("raw_transits").is_none());
}

#[test]
fn horoscope_period_response_has_exactly_7_daily_timeline_entries() {
    let request = period_interpretation_request();
    let response = period_response_from_request(&request);
    validate_period_response_schema(&response).unwrap();
    validate_period_response_evidence(&request, &response).unwrap();
    assert_eq!(response["daily_timeline"].as_array().unwrap().len(), 7);
}

#[test]
fn horoscope_period_public_tone_uses_french_labels() {
    let request = period_interpretation_request();
    let response = period_response_from_request(&request);
    validate_period_response_evidence(&request, &response).unwrap();
    let allowed = std::collections::HashSet::from([
        "concentré",
        "soutenant",
        "vigilant",
        "dynamique",
        "nuancé",
        "fluide",
        "sous tension",
    ]);
    let tones = response["daily_timeline"]
        .as_array()
        .unwrap()
        .iter()
        .map(|day| day["tone"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(tones.iter().all(|tone| allowed.contains(*tone)));
    assert!(tones.iter().any(|tone| *tone == "soutenant"));
    assert!(tones.iter().any(|tone| *tone == "vigilant"));
    assert!(!tones.iter().any(|tone| matches!(
        *tone,
        "focused" | "focus" | "supportive" | "careful" | "active" | "mixed" | "fluid" | "tense"
    )));
}

#[test]
fn horoscope_period_public_tone_is_forced_from_db_labels() {
    let request = period_interpretation_request();
    let response = period_response_from_request(&request);
    validate_period_response_evidence(&request, &response).unwrap();
    for day in response["daily_timeline"].as_array().unwrap() {
        assert!(matches!(
            day["tone"].as_str().unwrap(),
            "concentré"
                | "soutenant"
                | "vigilant"
                | "dynamique"
                | "nuancé"
                | "fluide"
                | "sous tension"
        ));
    }
}

#[test]
fn horoscope_period_daily_timeline_matches_included_dates() {
    let request = period_interpretation_request();
    let response = period_response_from_request(&request);
    let included = request["period_resolution"]["included_dates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<Vec<_>>();
    let timeline = response["daily_timeline"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value["date"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(timeline, included);
}

#[test]
fn horoscope_period_key_best_watch_days_match_included_dates() {
    let request = period_interpretation_request();
    let response = period_response_from_request(&request);
    validate_period_response_evidence(&request, &response).unwrap();
}

#[test]
fn horoscope_period_rejects_day_in_both_best_and_watch() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["watch_days"][0]["date"] = response["best_days"][0]["date"].clone();
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PERIOD_BEST_WATCH_MISSING");
}

#[test]
fn horoscope_period_response_rejects_public_theme_codes() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["daily_timeline"][0]["text"] =
        serde_json::json!("Le thème organization ne doit jamais sortir tel quel.");
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PERIOD_TECHNICAL_CODE_LEAK");
}

#[test]
fn horoscope_period_public_text_rejects_internal_tone_codes() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["daily_timeline"][0]["tone"] = serde_json::json!("focus");
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PERIOD_TECHNICAL_CODE_LEAK");
}

#[test]
fn horoscope_period_rejects_public_tone_not_in_db_labels() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["daily_timeline"][0]["tone"] = serde_json::json!("posé");
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PERIOD_TECHNICAL_CODE_LEAK");
}

#[test]
fn horoscope_period_rejects_invented_evidence_summary_key() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["evidence_summary"][0]["evidence_key"] = serde_json::json!("period:invented");
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PERIOD_EVIDENCE_MISSING");
}

#[test]
fn horoscope_period_rejects_evidence_summary_date_outside_period() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["evidence_summary"][0]["date"] = serde_json::json!("2026-06-30");
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH");
}

#[test]
fn horoscope_period_provider_payload_requires_real_public_text_before_repair() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["advice"]["main"] = serde_json::json!("");
    let err = validate_period_provider_public_payload(&response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PERIOD_RESPONSE_INVALID");
}

#[test]
fn horoscope_period_provider_payload_rejects_missing_domain_text_before_repair() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["domain_sections"][0]["text"] = serde_json::json!(null);
    let err = validate_period_provider_public_payload(&response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PERIOD_RESPONSE_INVALID");
}

#[test]
fn horoscope_period_provider_payload_accepts_none_watch_summary_without_tension() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let request =
        build_period_interpretation_request(&public, &premium_period_context_only_calculation())
            .unwrap();
    let response = premium_period_response_from_request(&request);
    assert_eq!(response["watch_summary"]["status"], "none");
    assert!(response["watch_windows"].as_array().unwrap().is_empty());
    validate_period_provider_public_payload(&response).unwrap();
}

#[test]
fn horoscope_period_real_response_rejects_too_short_public_text() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["quality"]["provider"] = serde_json::json!("openai");
    response["quality"]["model"] = serde_json::json!("gpt-5-mini");
    response["week_overview"]["text"] =
        serde_json::json!("Vos repères personnels restent le point d'appui.");
    response["week_overview"]["trajectory"] = serde_json::json!("Avancer par étapes.");
    for (index, day) in response["daily_timeline"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        day["text"] = serde_json::json!(format!(
            "Ce jour {} garde vos repères personnels.",
            index + 1
        ));
        day["advice"] = serde_json::json!("Avancer simplement.");
    }
    for section in response["domain_sections"].as_array_mut().unwrap() {
        section["text"] = serde_json::json!("Ce domaine reste relié à vos repères personnels.");
    }
    response["advice"]["main"] = serde_json::json!("Gardez une priorité.");
    response["advice"]["best_use"] = serde_json::json!("Agir simplement.");
    response["advice"]["avoid"] = serde_json::json!("Forcer.");
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_WORD_COUNT_OUT_OF_RANGE"
    );
}

#[test]
fn horoscope_period_real_response_rejects_text_above_hard_limit() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["quality"]["provider"] = serde_json::json!("openai");
    response["quality"]["model"] = serde_json::json!("gpt-5-mini");
    let repeated = (0..1600).map(|_| "mot").collect::<Vec<_>>().join(" ");
    response["week_overview"]["text"] = serde_json::json!(repeated);
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_WORD_COUNT_OUT_OF_RANGE"
    );
}

#[test]
fn horoscope_period_response_rejects_repetitive_timeline() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    let repeated = response["daily_timeline"][0]["text"].clone();
    for day in response["daily_timeline"].as_array_mut().unwrap() {
        day["text"] = repeated.clone();
    }
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_REPETITIVE_DAILY_TEXT"
    );
}

#[test]
fn horoscope_period_rejects_internal_guidance_leak() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["daily_timeline"][0]["text"] = serde_json::json!(
        "Personnaliser ce signal par les relations directes plutôt que rester sur un conseil générique."
    );
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_INTERNAL_GUIDANCE_LEAK"
    );
}

#[test]
fn horoscope_period_rejects_broken_sentences() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["daily_timeline"][0]["text"] = serde_json::json!(
        "Cette journée garde un lien clair avec une zone personnelle du thème natal et."
    );
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PERIOD_BROKEN_SENTENCE");
}

#[test]
fn horoscope_period_repair_removes_broken_sentence_fragments() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["daily_timeline"][0]["text"] = serde_json::json!(
        "Cette journée garde un lien clair avec les besoins émotionnels. Une fin tronquée avec."
    );
    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();
    assert!(!response["daily_timeline"][0]["text"]
        .as_str()
        .unwrap()
        .contains("tronquée avec"));
}

#[test]
fn horoscope_period_repair_replaces_single_broken_sentence() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["advice"]["main"] = serde_json::json!("Une phrase isolée avec.");
    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();
    assert_eq!(
        response["advice"]["main"].as_str().unwrap(),
        "Vos repères personnels gardent un appui concret pour avancer avec mesure."
    );
}

#[test]
fn horoscope_period_repair_removes_typographic_broken_sentence_tail() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["domain_sections"][0]["text"] =
        serde_json::json!("Ce domaine soutient une décision utile. L’");
    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();
    let public = response["domain_sections"][0]["text"].as_str().unwrap();
    assert!(!public.ends_with("L’"));
}

#[test]
fn horoscope_period_repair_removes_elided_broken_sentence_tail() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["domain_sections"][0]["text"] =
        serde_json::json!("Ce domaine soutient une décision utile. Une transition reste liée à d’");
    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();
    let public = response["domain_sections"][0]["text"].as_str().unwrap();
    assert!(!public.ends_with("d’"));
    assert!(!public.contains(" à d’"), "{public}");
}

#[test]
fn horoscope_period_rejects_lowercase_sentence_start_after_period() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["daily_timeline"][0]["text"] = serde_json::json!(
        "Cette journée garde un lien clair avec les relations directes. votre manière d'avancer doit rester lisible."
    );
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PERIOD_BROKEN_SENTENCE");
}

#[test]
fn horoscope_period_repair_capitalizes_lowercase_sentence_starts() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["week_overview"]["text"] = serde_json::json!(
        "La semaine avance avec un repère natal. une priorité se précise. le rythme reste concret."
    );
    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();
    let public = response["week_overview"]["text"].as_str().unwrap();
    assert!(public.contains(". Une priorité"));
    assert!(public.contains(". Le rythme"));
}

#[test]
fn horoscope_period_public_text_does_not_contain_writer_instructions() {
    let request = period_interpretation_request();
    let response = period_response_from_request(&request);
    validate_period_response_evidence(&request, &response).unwrap();
    let public_text = response.to_string().to_lowercase();
    for forbidden in [
        "personnaliser ce signal",
        "relier ce signal",
        "plutôt que rester sur un conseil générique",
        "donne le relief principal",
        "summary_hint",
        "advice_hint",
        "personalization_hint",
        "natal_focus_hint",
    ] {
        assert!(
            !public_text.contains(forbidden),
            "public response leaked writer instruction: {forbidden}"
        );
    }
}

#[test]
fn horoscope_period_rejects_mechanical_lisible_phrase() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["domain_sections"][0]["text"] =
        serde_json::json!("La semaine devient plus lisible quand vous triez les priorités.");

    let err = validate_period_response_evidence(&request, &response).unwrap_err();

    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_INTERNAL_GUIDANCE_LEAK"
    );
}

#[test]
fn horoscope_period_internal_hints_do_not_use_copyable_writer_instructions() {
    let request = period_interpretation_request();
    let serialized = serde_json::to_string(&request["daily_plans"]).unwrap();
    for forbidden in [
        "Le 2026-",
        "donne le relief principal",
        "en prose utilisateur",
        "Exploite explicitement",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "period daily_plans contain copyable writer instruction: {forbidden}"
        );
    }
}

#[test]
fn horoscope_period_week_overview_is_not_repetitive() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["week_overview"]["text"] =
        serde_json::json!("Les relations directes donnent une lecture personnelle.");
    response["week_overview"]["trajectory"] =
        serde_json::json!("Les relations directes soutiennent la progression.");
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PERIOD_OVERVIEW_REPETITION");
}

#[test]
fn horoscope_period_rejects_meta_personalization_language() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["daily_timeline"][0]["text"] = serde_json::json!(
        "Cette nuance reste liée au thème natal, ce qui rend le conseil plus personnel que générique."
    );
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_META_PERSONALIZATION_LEAK"
    );
}

#[test]
fn horoscope_period_rejects_mechanical_personalization_fragment() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["domain_sections"][0]["text"] = serde_json::json!(
        "Dans ce domaine, vos repères personnels liés à nommer ce qui compte, préserver un lien utile, refuser un accord de façade."
    );
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_META_PERSONALIZATION_LEAK"
    );
}

#[test]
fn horoscope_premium_next_7_days_repair_rewrites_domain_templates_and_weak_trajectory() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["week_overview"]["trajectory"] = serde_json::json!(
        "Aller de la sécurisation pratique vers la vérification des limites. Le mouvement relie vos repères personnels, les appuis émotionnels et les choix à consolider."
    );
    response["domain_sections"][0]["text"] = serde_json::json!(
        "Dans ce domaine, les repères les plus utiles consistent à préparer un message court, différer une réponse rapide, vérifier une information, trier deux options concrètes. Et à choisir le bon niveau d'engagement."
    );
    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();
    let trajectory = response["week_overview"]["trajectory"].as_str().unwrap();
    assert!(trajectory.contains("sécurisation pratique"));
    assert!(trajectory.contains("validation plus collective"));
    assert!(!trajectory.contains("Le mouvement relie vos repères personnels"));
    let domain = response["domain_sections"][0]["text"].as_str().unwrap();
    assert!(!domain.contains("Dans ce domaine"));
    assert!(!domain.contains("choisir le bon niveau d'engagement"));
    assert!(domain.contains("ouvre un fil pratique"));
}

#[test]
fn horoscope_period_daily_text_uses_distinct_sentence_patterns() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    for (index, day) in response["daily_timeline"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .take(3)
        .enumerate()
    {
        day["text"] = serde_json::json!(format!(
            "Ce jour met l'accent sur organisation avec une priorité reliée au thème natal numéro {}.",
            index + 1
        ));
    }
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_REPETITIVE_DAILY_TEXT"
    );
}

#[test]
fn horoscope_period_week_overview_does_not_mention_the_reading_process() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["week_overview"]["text"] = serde_json::json!(
        "La lecture relie les choix de la semaine aux zones natales activées dans le thème natal."
    );
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_META_PERSONALIZATION_LEAK"
    );
}

#[test]
fn horoscope_period_advice_is_not_repeated_template() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    for (index, day) in response["daily_timeline"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .take(3)
        .enumerate()
    {
        day["advice"] = serde_json::json!(format!(
            "Choisissez une seule priorité liée au thème natal et avancez sans multiplier les sujets numéro {}.",
            index + 1
        ));
    }
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_REPETITIVE_DAILY_TEXT"
    );
}

#[test]
fn horoscope_period_french_typography_colon_spacing() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["week_overview"]["text"] =
        serde_json::json!("La semaine avance clairement: le thème natal reste présent.");
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_FRENCH_TYPOGRAPHY_FAILED"
    );
}

#[test]
fn horoscope_premium_next_7_days_rejects_bad_french_elision() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["week_overview"]["text"] =
        serde_json::json!("La semaine permet d’réaccorder le cadre avant de confirmer.");
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_FRENCH_TYPOGRAPHY_FAILED"
    );
}

#[test]
fn horoscope_premium_next_7_days_rejects_glued_french_compounds() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["strategy"]["best_use"] = serde_json::json!(
        "Confirmez un rendezvous bref, utilisezles, puis revenezy sans arrêtezvous sur le détail des joursclés."
    );
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_FRENCH_TYPOGRAPHY_FAILED"
    );
}

#[test]
fn horoscope_premium_next_7_days_prompt_prevents_typography_and_template_regressions() {
    let request = premium_period_interpretation_request();
    let prompt = period_writer_prompt_text_for_test(&request).unwrap();

    for expected in [
        "Respecte la typographie française",
        "rendez-vous",
        "phrase-clé",
        "utilisez-les",
        "revenez-y",
        "jours clés",
        "arrêtez-vous",
        "terminez-la",
        "accordez-vous",
        "aucune parenthèse ouverte",
        "Ne recopie jamais les situations associées sous forme de liste",
        "autour de vérifier",
        "appuis concrets aide",
        "Appui concret :",
        "est un point d'appui pour",
        "Ce créneau peut servir",
        "N'utilise pas de structure répétée comme Dans ce domaine",
        "Cette énergie devient utile",
        "les repères les plus utiles consistent",
        "mini-lecture naturelle",
    ] {
        assert!(
            prompt.contains(expected),
            "Premium prompt should contain guard instruction: {expected}"
        );
    }
}

#[test]
fn horoscope_period_repair_fixes_glued_french_compounds() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["daily_timeline"][0]["text"] = serde_json::json!(
        "Confirmez un rendezvous, laissezle reposer, terminezla, puis faitesle valider sans pression."
    );
    response["strategy"]["recovery"] = serde_json::json!(
        "Si un engagement pèse, diminuezle, déléguezla, transformezle, allégezle et retirezvous avant de répondre. Autorisezvous une pause, utilisezles, revenezy et arrêtezvous."
    );
    response["domain_sections"][0]["text"] =
        serde_json::json!("Les joursclés demandent une preuve simple.");
    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();
    let public = serde_json::to_string(&response).unwrap();
    assert!(public.contains("rendez-vous"));
    assert!(public.contains("laissez-le"));
    assert!(public.contains("terminez-la"));
    assert!(public.contains("faites-le"));
    assert!(public.contains("diminuez-le"));
    assert!(public.contains("déléguez-la"));
    assert!(public.contains("transformez-le"));
    assert!(public.contains("allégez-le"));
    assert!(public.contains("retirez-vous"));
    assert!(public.contains("Autorisez-vous"));
    assert!(public.contains("utilisez-les"));
    assert!(public.contains("revenez-y"));
    assert!(public.contains("arrêtez-vous"));
    assert!(public.contains("jours clés"));
    assert!(!public.contains("rendezvous"));
    assert!(!public.contains("laissezle"));
    assert!(!public.contains("terminezla"));
    assert!(!public.contains("faitesle"));
    assert!(!public.contains("diminuezle"));
    assert!(!public.contains("déléguezla"));
    assert!(!public.contains("transformezle"));
    assert!(!public.contains("allégezle"));
    assert!(!public.contains("retirezvous"));
    assert!(!public.contains("Autorisezvous"));
    assert!(!public.contains("utilisezles"));
    assert!(!public.contains("revenezy"));
    assert!(!public.contains("arrêtezvous"));
    assert!(!public.contains("joursclés"));
}

#[test]
fn horoscope_premium_next_7_days_rejects_truncated_example_parenthesis() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["daily_timeline"][2]["text"] =
        serde_json::json!("Côté relations, offrir une assurance courte (par ex.");
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PERIOD_BROKEN_SENTENCE");
}

#[test]
fn horoscope_period_repair_removes_truncated_example_tail() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["daily_timeline"][2]["text"] =
        serde_json::json!("Côté relations, offrir une assurance courte (par ex.");
    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();
    let text = response["daily_timeline"][2]["text"].as_str().unwrap();
    assert!(text.starts_with("Côté relations, offrir une assurance courte."));
    assert!(!text.contains("(par ex."));
}

#[test]
fn horoscope_period_rejects_mechanical_marker_reason_patterns() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["best_days"][0]["reason"] = serde_json::json!(
        "Mercredi 10/06 se prête mieux à une action simple autour de vérifier une ressource : appuis concrets aide à choisir le bon message. ."
    );
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_FRENCH_TYPOGRAPHY_FAILED"
    );
}

#[test]
fn horoscope_period_repair_rewrites_mechanical_marker_reasons() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["best_days"][0]["reason"] = serde_json::json!(
        "Mercredi 10/06 se prête mieux à une action simple autour de vérifier une ressource, réduire une dépense d'énergie : appuis concrets aide à choisir."
    );
    if response["watch_days"].as_array().unwrap().is_empty() {
        response["watch_days"] = serde_json::json!([request["watch_days"][0].clone()]);
    }
    response["watch_days"][0]["reason"] = serde_json::json!(
        "Jeudi 11/06 demande une vigilance précise autour de vérifier un délai, réduire une promesse. ."
    );
    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();
    let public = serde_json::to_string(&response).unwrap();
    assert!(!public.contains("autour de vérifier"));
    assert!(!public.contains(": appuis concrets aide"));
    assert!(!public.contains("Appui concret :"));
    assert!(!public.contains("est un point d'appui pour"));
    assert!(!public.contains(". ."));
    assert!(public.contains("Avant de promettre davantage") || public.contains("Traitez d'abord"));
}

#[test]
fn horoscope_period_repair_naturalizes_fallback_lists_and_repeated_verbs() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["daily_timeline"][0]["text"] = serde_json::json!(
        "Dimanche ouvre le thème appuis concrets. Avec vérifier une ressource, réduire une dépense d'énergie, sécuriser un appui concret. , le plus utile consiste à poser un repère clair."
    );
    response["daily_timeline"][0]["advice"] = serde_json::json!(
        "Posez une priorité claire liée à vérifier une ressource, réduire une dépense d'énergie. , puis avancez."
    );
    response["key_days"][0]["reason"] = serde_json::json!(
        "Mercredi sert de repère. Vérifiez vérifier une ressource et réduire une dépense d'énergie avant de promettre davantage."
    );

    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();

    let public = serde_json::to_string(&response).unwrap();
    assert!(!public.contains("Vérifiez vérifier"));
    assert!(!public.contains(". ,"));
    assert!(!public.contains("Posez une priorité claire liée à"));
    assert!(!public.contains("Avec vérifier une ressource, réduire une dépense"));
    assert!(
        public.contains("Le geste utile consiste")
            || public.contains("Traitez d'abord ce point")
            || public.contains("Avant de promettre davantage")
    );
}

#[test]
fn horoscope_period_repair_rewrites_mechanical_public_fragments_outside_markers() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["daily_timeline"][0]["text"] = serde_json::json!(
        "Mercredi Est un point d’appui pour appuis concrets Autour de vérifier une ressource. APPUI CONCRET : reprendre un repère."
    );
    response["domain_sections"][0]["text"] = serde_json::json!(
        "Cette énergie devient utile quand elle sert à vérifier une information."
    );
    response["domain_sections"][1]["text"] = serde_json::json!(
        "Cette énergie devient utile quand elle sert à trier deux options. Mardi est un point d’appui pour thème imprévu."
    );

    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();

    let public = serde_json::to_string(&response).unwrap();
    assert!(!public.contains("autour de vérifier"));
    assert!(!public.contains("Appui concret :"));
    assert!(!public.contains("est un point d'appui pour"));
    assert!(!public.contains("point d’appui pour"));
    assert!(!public.contains("Cette énergie devient utile quand elle sert à"));
}

#[test]
fn horoscope_premium_next_7_days_repair_rewrites_generic_windows_and_advice() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["best_windows"][0]["reason"] =
        serde_json::json!("Ce créneau peut servir à poser une action simple et vérifiable.");
    response["advice"] = serde_json::json!({
        "main": "Gardez une progression simple.",
        "best_use": "Utiliser les appuis.",
        "avoid": "Éviter de transformer un signal quotidien."
    });

    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();

    let reason = response["best_windows"][0]["reason"].as_str().unwrap();
    assert!(!reason.contains("Ce créneau peut servir"));
    assert!(
        reason.contains("confirmer")
            || reason.contains("action courte")
            || reason.contains("message")
    );
    let advice = serde_json::to_string(&response["advice"]).unwrap();
    assert!(advice.contains("preuve"));
    assert!(advice.contains("confirmation") || advice.contains("échéance"));
}

#[test]
fn horoscope_period_repair_fixes_french_colon_spacing() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["week_overview"]["text"] =
        serde_json::json!("La semaine avance clairement: le thème natal reste présent.");
    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();
    assert!(response["week_overview"]["text"]
        .as_str()
        .unwrap()
        .contains("clairement : le"));
}

#[test]
fn horoscope_period_repair_rewrites_repeated_natal_anchor_phrases() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    for day in response["daily_timeline"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .take(4)
    {
        day["text"] = serde_json::json!(
            "L'appui personnel vient de vos habitudes, à utiliser comme repère concret plutôt que comme explication abstraite."
        );
    }
    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();
    let public = response["daily_timeline"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|day| day["text"].as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        public.matches("L'appui personnel vient de").count() <= 2,
        "repair should reduce repeated natal anchor phrase: {public}"
    );
}

#[test]
fn horoscope_period_rejects_broken_french_fragments() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["domain_sections"][0]["text"] = serde_json::json!(
        "Les moments où tout s’dynamique demandent une reprise simple, avec une preuve astrologique claire et un conseil concret."
    );
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_BROKEN_FRENCH_FRAGMENT"
    );
}

#[test]
fn horoscope_period_repair_rewrites_redynamique_fragment() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["domain_sections"][0]["text"] = serde_json::json!(
        "L’énergie de la semaine est rapide, mentale et rédynamique, avec une preuve astrologique claire et un conseil concret."
    );
    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();
    let public = response["domain_sections"][0]["text"].as_str().unwrap();
    assert!(!public.contains("rédynamique"));
    assert!(public.contains("dynamisante"));
}

#[test]
fn horoscope_period_repair_rewrites_repetitive_public_vocabulary() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    for (index, day) in response["daily_timeline"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .take(5)
        .enumerate()
    {
        day["text"] = serde_json::json!(format!(
            "Cette journée aide à clarifier une priorité personnelle du thème natal avec une nuance concrète numéro {}.",
            index + 1
        ));
        day["advice"] = serde_json::json!(format!(
            "Choisissez une seule priorité dans ce contexte personnel numéro {}.",
            index + 1
        ));
    }
    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();
    let public = response["daily_timeline"]
        .as_array()
        .unwrap()
        .iter()
        .map(|day| {
            format!(
                "{} {}",
                day["text"].as_str().unwrap(),
                day["advice"].as_str().unwrap()
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    assert!(public.matches("clarifier").count() <= 2);
    assert!(public.matches("choisissez une seule priorité").count() <= 2);
}

#[test]
fn horoscope_period_repair_rewrites_repeated_hierarchisez_advice() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    for (index, day) in response["daily_timeline"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .take(5)
        .enumerate()
    {
        day["advice"] = serde_json::json!(format!(
            "Hiérarchisez une priorité et laissez le reste au second plan. Variante {}.",
            index + 1
        ));
    }
    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();
    let public = response["daily_timeline"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|day| day["advice"].as_str())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    assert!(public.matches("hiérarchisez une priorité").count() <= 2);
    assert!(public.contains("retenez une priorité nette"));
}

#[test]
fn horoscope_period_watch_days_created_from_valid_tension_event() {
    let request = period_interpretation_request();
    let watch_days = request["watch_days"].as_array().unwrap();
    assert!(
        !watch_days.is_empty(),
        "period interpretation should expose watch days for valid square/opposition events"
    );
    assert!(watch_days.iter().any(|day| day["date"] == "2026-06-09"));
}

#[test]
fn horoscope_period_event_scores_are_discriminating() {
    let request = period_interpretation_request();
    let scores = request["period_events"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|event| event["score"].as_f64())
        .map(|score| format!("{score:.2}"))
        .collect::<std::collections::HashSet<_>>();
    assert!(
        scores.len() > 1,
        "period event scores should not be flattened to a single value"
    );
    for event in request["period_events"].as_array().unwrap() {
        let score = event["score"].as_f64().unwrap();
        assert!(score > 0.0 && score <= 1.0);
    }
}

#[test]
fn horoscope_period_event_scores_are_sorted_desc() {
    let request = period_interpretation_request();
    let scores = request["period_events"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|event| event["score"].as_f64())
        .collect::<Vec<_>>();
    for window in scores.windows(2) {
        assert!(
            window[0] >= window[1],
            "period events must be sorted by score desc: {scores:?}"
        );
    }
}

#[test]
fn horoscope_period_key_days_respect_score_threshold_and_max_two() {
    let request = period_interpretation_request();
    let key_days = request["key_days"].as_array().unwrap();
    assert!(key_days.len() <= 2);
    let events = request["period_events"].as_array().unwrap();
    let top_score = events[0]["score"].as_f64().unwrap();
    for day in key_days {
        let date = day["date"].as_str().unwrap();
        let score = events
            .iter()
            .find(|event| event["date"].as_str() == Some(date))
            .unwrap()["score"]
            .as_f64()
            .unwrap();
        assert!(score >= 0.60);
        assert!(score >= top_score - 0.08);
    }
}

#[test]
fn horoscope_period_key_days_empty_when_no_clear_peak() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let mut calculation = period_calculation();
    let objects = [
        ("venus", "natal_moon"),
        ("mars", "natal_mercury"),
        ("mercury", "natal_mars"),
        ("jupiter", "natal_saturn"),
        ("sun", "natal_venus"),
        ("saturn", "natal_saturn"),
        ("moon", "natal_moon"),
    ];
    for (index, snapshot) in calculation["snapshots"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        let date = snapshot["date"].as_str().unwrap().to_string();
        let snapshot_key = snapshot["snapshot_key"].as_str().unwrap().to_string();
        let (object, target) = objects[index % objects.len()];
        let fact = &mut snapshot["transits_to_natal"][0];
        fact["fact_type"] = serde_json::json!("transit_context");
        fact["aspect"] = serde_json::Value::Null;
        fact["orb_deg"] = serde_json::Value::Null;
        fact["transiting_object"] = serde_json::json!(object);
        fact["natal_target"] = serde_json::json!(target);
        fact["evidence_key"] = serde_json::json!(format!(
            "period:{}:{}:{}:context:{}",
            date, snapshot_key, object, target
        ));
    }
    let request = build_period_interpretation_request(&public, &calculation).unwrap();
    assert!(request["key_days"].as_array().unwrap().is_empty());
}

#[test]
fn horoscope_period_key_days_use_theme_rarity_as_tiebreaker() {
    let request = period_interpretation_request();
    let key_days = request["key_days"].as_array().unwrap();
    assert!(key_days.len() <= 2);
    let themes = key_days
        .iter()
        .filter_map(|day| {
            let key = day["evidence_keys"][0].as_str()?;
            request["period_events"]
                .as_array()?
                .iter()
                .find(|event| event["evidence_keys"][0].as_str() == Some(key))?["theme_code"]
                .as_str()
        })
        .collect::<Vec<_>>();
    assert!(themes.len() <= 2);
}

#[test]
fn horoscope_period_events_expose_aspect_for_tension_selection() {
    let request = period_interpretation_request();
    let mars_event = request["period_events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| {
            event["evidence_keys"][0]
                .as_str()
                .unwrap()
                .contains(":mars:square:")
        })
        .unwrap();
    assert_eq!(mars_event["aspect"], "square");
    assert_eq!(mars_event["tone"], "careful");
}

#[test]
fn horoscope_period_rejects_calculation_without_events() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let mut calculation = period_calculation();
    for snapshot in calculation["snapshots"].as_array_mut().unwrap() {
        snapshot["transits_to_natal"] = serde_json::json!([]);
    }
    let err = build_period_interpretation_request(&public, &calculation).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PERIOD_EVIDENCE_MISSING");
}

#[test]
fn horoscope_period_no_tension_has_empty_watch_days_and_none_summary() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let mut calculation = period_calculation();
    for snapshot in calculation["snapshots"].as_array_mut().unwrap() {
        let date = snapshot["date"].as_str().unwrap().to_string();
        let snapshot_key = snapshot["snapshot_key"].as_str().unwrap().to_string();
        let fact = &mut snapshot["transits_to_natal"][0];
        if fact["fact_type"] == "transit_to_natal" {
            let object = fact["transiting_object"].as_str().unwrap().to_string();
            fact["aspect"] = serde_json::json!("trine");
            fact["evidence_key"] = serde_json::json!(format!(
                "period:{}:{}:{}:trine:natal_moon",
                date, snapshot_key, object
            ));
        }
    }
    let request = build_period_interpretation_request(&public, &calculation).unwrap();
    let watch_days = request["watch_days"].as_array().unwrap();
    assert!(watch_days.is_empty());
    assert_eq!(request["watch_summary_plan"]["status"], "none");
    assert_eq!(
        request["watch_summary_plan"]["text"],
        "Aucun point de vigilance dominant ne ressort cette semaine. Gardez simplement une marge d'observation si un échange ou une décision demande plus de temps que prévu."
    );
}

#[test]
fn horoscope_period_valid_tension_has_watch_day_and_active_summary() {
    let request = period_interpretation_request();
    assert!(!request["watch_days"].as_array().unwrap().is_empty());
    assert_eq!(request["watch_summary_plan"]["status"], "active");
}

#[test]
fn horoscope_premium_next_7_days_builds_best_windows() {
    let request = premium_period_interpretation_request();
    let windows = request["best_windows"].as_array().unwrap();
    assert!(!windows.is_empty());
    assert!(windows
        .iter()
        .all(|window| !window["evidence_keys"].as_array().unwrap().is_empty()));
}

#[test]
fn horoscope_premium_next_7_days_best_windows_have_distinct_titles_and_best_for() {
    let request = premium_period_interpretation_request();
    let windows = request["best_windows"].as_array().unwrap();
    assert!(windows.len() >= 3);
    let titles = windows
        .iter()
        .filter_map(|window| window["title"].as_str())
        .collect::<std::collections::HashSet<_>>();
    let best_for_sets = windows
        .iter()
        .filter_map(|window| window["best_for"].as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .collect::<Vec<_>>()
                .join("|")
        })
        .collect::<std::collections::HashSet<_>>();
    assert!(
        titles.len() >= 2,
        "expected differentiated titles: {titles:?}"
    );
    assert!(
        best_for_sets.len() >= 2,
        "expected differentiated best_for sets: {best_for_sets:?}"
    );
    assert!(!titles.contains("Fenêtre favorable"));
}

#[test]
fn horoscope_premium_next_7_days_builds_watch_windows_or_none() {
    let request = premium_period_interpretation_request();
    let watch_windows = request["watch_windows"].as_array().unwrap();
    if request["watch_summary_plan"]["status"] != "none" {
        assert!(!watch_windows.is_empty());
    }
}

#[test]
fn horoscope_premium_next_7_days_context_only_does_not_build_fake_watch_windows() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let request =
        build_period_interpretation_request(&public, &premium_period_context_only_calculation())
            .unwrap();
    assert_eq!(request["watch_summary_plan"]["status"], "none");
    assert!(request["watch_windows"].as_array().unwrap().is_empty());
    assert!(!request["best_windows"].as_array().unwrap().is_empty());
    let mut response = premium_period_response_from_request(&request);
    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();
}

#[test]
fn horoscope_premium_next_7_days_returns_no_watch_windows_without_true_tension() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let request =
        build_period_interpretation_request(&public, &premium_period_context_only_calculation())
            .unwrap();
    assert_eq!(request["watch_days"].as_array().unwrap().len(), 0);
    assert_eq!(request["watch_summary_plan"]["status"], "none");
    assert!(request["watch_windows"].as_array().unwrap().is_empty());
    assert!(request["watch_summary_plan"]["evidence_keys"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn horoscope_premium_next_7_days_watch_windows_reference_existing_snapshots_when_present() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let request =
        build_period_interpretation_request(&public, &premium_period_context_only_calculation())
            .unwrap();
    let snapshot_keys = request["scan_plan"]["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|snapshot| snapshot["snapshot_key"].as_str())
        .collect::<std::collections::HashSet<_>>();
    let evidence = request["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item["evidence_key"].as_str())
        .collect::<std::collections::HashSet<_>>();
    for window in request["watch_windows"].as_array().unwrap() {
        for key in window["source_snapshot_keys"].as_array().unwrap() {
            assert!(snapshot_keys.contains(key.as_str().unwrap()));
        }
        for key in window["evidence_keys"].as_array().unwrap() {
            assert!(evidence.contains(key.as_str().unwrap()));
        }
    }
}

#[test]
fn horoscope_premium_next_7_days_watch_windows_do_not_overlap_best_windows_when_present() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let request =
        build_period_interpretation_request(&public, &premium_period_context_only_calculation())
            .unwrap();
    let best_sources = request["best_windows"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|window| window["source_snapshot_keys"].as_array().unwrap())
        .filter_map(|key| key.as_str())
        .collect::<std::collections::HashSet<_>>();
    for window in request["watch_windows"].as_array().unwrap() {
        for key in window["source_snapshot_keys"].as_array().unwrap() {
            assert!(!best_sources.contains(key.as_str().unwrap()));
        }
    }
}

#[test]
fn horoscope_premium_next_7_days_fake_writer_context_only_passes_evidence_guard() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let request =
        build_period_interpretation_request(&public, &premium_period_context_only_calculation())
            .unwrap();
    let response = fake_period_writer_response(&request).unwrap();
    validate_period_response_evidence(&request, &response).unwrap();
}

#[test]
fn horoscope_period_fake_writer_sanitizes_mechanical_marker_fallbacks() {
    let mut request = period_interpretation_request();
    request["best_days"][0]["reason"] = serde_json::json!(
        "Mercredi 10/06 se prête mieux à une action simple autour de vérifier une ressource : appuis concrets aide à choisir."
    );
    request["watch_days"][0]["reason"] = serde_json::json!(
        "Jeudi 11/06 demande une vigilance précise autour de vérifier un délai. ."
    );

    let response = fake_period_writer_response(&request).unwrap();

    validate_period_response_evidence(&request, &response).unwrap();
    let public = serde_json::to_string(&response).unwrap().to_lowercase();
    assert!(!public.contains("autour de vérifier"));
    assert!(!public.contains(": appuis concrets aide"));
    assert!(!public.contains(". ."));
}

#[test]
fn horoscope_period_provider_schema_matches_service_shape() {
    let free = free_period_interpretation_request();
    let free_schema = period_response_provider_schema(&free).unwrap();
    assert!(free_schema.get("allOf").is_none());
    let free_properties = free_schema["properties"].as_object().unwrap();
    for forbidden in [
        "week_overview",
        "best_days",
        "watch_days",
        "daily_timeline",
        "domain_sections",
        "best_windows",
        "watch_windows",
        "strategy",
    ] {
        assert!(!free_properties.contains_key(forbidden));
    }
    assert_eq!(free_properties["advice"]["type"], "string");
    assert_eq!(
        free_properties["watch_summary"]["$ref"],
        "#/definitions/free_watch_summary"
    );

    let basic = period_interpretation_request();
    let basic_schema = period_response_provider_schema(&basic).unwrap();
    assert!(basic_schema.get("allOf").is_none());
    let basic_properties = basic_schema["properties"].as_object().unwrap();
    let basic_required = basic_schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<std::collections::HashSet<_>>();
    for field in basic_properties.keys() {
        assert!(
            basic_required.contains(field.as_str()),
            "basic provider schema property must be required: {field}"
        );
    }
    assert!(!basic_properties.contains_key("best_windows"));
    assert!(!basic_properties.contains_key("watch_windows"));
    assert!(!basic_properties.contains_key("strategy"));
    assert!(!basic_properties.contains_key("summary"));
    assert!(!basic_properties.contains_key("dominant_theme"));
    assert!(!basic_required.contains("summary"));
    assert!(!basic_required.contains("dominant_theme"));
    assert!(!basic_required.contains("best_windows"));
    assert!(!basic_required.contains("watch_windows"));
    assert!(!basic_required.contains("strategy"));
    for field in [
        "week_overview",
        "best_days",
        "watch_days",
        "daily_timeline",
        "domain_sections",
    ] {
        assert!(basic_properties.contains_key(field));
        assert!(basic_required.contains(field));
    }

    let premium = premium_period_interpretation_request();
    let premium_schema = period_response_provider_schema(&premium).unwrap();
    assert!(premium_schema.get("allOf").is_none());
    let premium_properties = premium_schema["properties"].as_object().unwrap();
    let premium_required = premium_schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<std::collections::HashSet<_>>();
    for field in premium_properties.keys() {
        assert!(
            premium_required.contains(field.as_str()),
            "premium provider schema property must be required: {field}"
        );
    }
    for field in ["best_windows", "watch_windows", "strategy"] {
        assert!(premium_properties.contains_key(field));
        assert!(premium_required.contains(field));
    }
    assert!(!premium_properties.contains_key("summary"));
    assert!(!premium_properties.contains_key("dominant_theme"));
    assert!(!premium_required.contains("summary"));
    assert!(!premium_required.contains("dominant_theme"));
    for field in [
        "week_overview",
        "best_days",
        "watch_days",
        "daily_timeline",
        "domain_sections",
    ] {
        assert!(premium_properties.contains_key(field));
        assert!(premium_required.contains(field));
    }
}

#[test]
fn horoscope_period_event_schema_requires_premium_selection_metadata() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../contracts/llm/horoscope_period_interpretation_request_v1.schema.json"
    ))
    .unwrap();
    let required = schema["definitions"]["period_event"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<std::collections::HashSet<_>>();
    for field in [
        "theme_density_score",
        "fact_type",
        "transiting_object",
        "natal_target",
        "natal_house",
        "natal_focus_hint",
        "personalization_hint",
    ] {
        assert!(
            required.contains(field),
            "period_event must require {field}"
        );
    }
}

#[test]
fn horoscope_premium_next_7_days_windows_reference_existing_snapshots() {
    let request = premium_period_interpretation_request();
    let snapshots = request["scan_plan"]["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|snapshot| snapshot["snapshot_key"].as_str())
        .collect::<std::collections::HashSet<_>>();
    for field in ["best_windows", "watch_windows"] {
        for window in request[field].as_array().unwrap() {
            for key in window["source_snapshot_keys"].as_array().unwrap() {
                assert!(snapshots.contains(key.as_str().unwrap()));
            }
        }
    }
}

#[test]
fn horoscope_premium_next_7_days_derived_sections_only_reference_published_evidence() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let mut calculation = premium_period_calculation();
    for (idx, snapshot) in calculation["snapshots"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        let date = snapshot["date"].as_str().unwrap().to_string();
        let snapshot_key = snapshot["snapshot_key"].as_str().unwrap().to_string();
        snapshot["transits_to_natal"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "evidence_key": format!("period:{date}:{snapshot_key}:extra:{idx}"),
                "fact_type": "transit_to_natal",
                "source": "test_period_premium",
                "transiting_object": "venus",
                "natal_target": "natal_sun",
                "aspect": "sextile",
                "orb_deg": 1.1,
                "natal_house": null
            }));
    }
    let request = build_period_interpretation_request(&public, &calculation).unwrap();
    let evidence = request["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item["evidence_key"].as_str())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(evidence.len(), 50);
    for field in [
        "main_events",
        "daily_plans",
        "key_days",
        "best_days",
        "watch_days",
        "best_windows",
        "watch_windows",
        "domain_sections",
    ] {
        for item in request[field].as_array().into_iter().flatten() {
            for key in item["evidence_keys"].as_array().into_iter().flatten() {
                assert!(
                    evidence.contains(key.as_str().unwrap()),
                    "{field} referenced unpublished evidence {key}"
                );
            }
        }
    }
}

#[test]
fn horoscope_premium_next_7_days_response_has_strategy() {
    let request = premium_period_interpretation_request();
    let response = premium_period_response_from_request(&request);
    validate_period_response_evidence(&request, &response).unwrap();
    assert!(response["strategy"]["text"]
        .as_str()
        .unwrap()
        .contains("stratégie"));
}

#[test]
fn horoscope_premium_next_7_days_markers_explain_their_role() {
    let request = premium_period_interpretation_request();
    let marker_text = ["key_days", "best_days", "watch_days"]
        .into_iter()
        .flat_map(|field| request[field].as_array().into_iter().flatten())
        .filter_map(|marker| marker["reason"].as_str())
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();

    assert!(!marker_text.contains("sert de repère utile pour comprendre une priorité concrète"));
    assert!(!marker_text.contains("concentre le relief principal"));
    assert!(!marker_text.contains("devient plus lisible"));
    assert!(!marker_text.contains("timeline quotidienne"));
    assert!(
        marker_text.contains("jour clé")
            || marker_text.contains("geste précis")
            || marker_text.contains("avant de répondre")
    );
}

#[test]
fn horoscope_premium_next_7_days_repair_preserves_provider_marker_prose() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["key_days"][0]["reason"] = serde_json::json!(
        "Mercredi demande de choisir une conversation à traiter franchement, puis de garder une marge avant d'élargir la décision."
    );

    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();

    assert_eq!(
        response["key_days"][0]["reason"],
        "Mercredi demande de choisir une conversation à traiter franchement, puis de garder une marge avant d'élargir la décision."
    );
}

#[test]
fn horoscope_premium_next_7_days_repair_keeps_canonical_marker_evidence() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    let canonical_date = response["key_days"][0]["date"].clone();
    let canonical_keys = response["key_days"][0]["evidence_keys"].clone();
    response["key_days"][0]["date"] = serde_json::json!("2026-06-30");
    response["key_days"][0]["evidence_keys"] = serde_json::json!(["period:invented"]);
    response["key_days"][0]["reason"] = serde_json::json!(
        "Mercredi demande de choisir une conversation à traiter franchement, puis de garder une marge avant d'élargir la décision."
    );

    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();

    assert_eq!(response["key_days"][0]["date"], canonical_date);
    assert_eq!(response["key_days"][0]["evidence_keys"], canonical_keys);
    assert_eq!(
        response["key_days"][0]["reason"],
        "Mercredi demande de choisir une conversation à traiter franchement, puis de garder une marge avant d'élargir la décision."
    );
}

#[test]
fn horoscope_premium_next_7_days_repair_keeps_canonical_evidence_summary_keys() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    let canonical_date = response["evidence_summary"][0]["date"].clone();
    let canonical_key = response["evidence_summary"][0]["evidence_key"].clone();
    response["evidence_summary"][0]["date"] = serde_json::json!("2026-06-30");
    response["evidence_summary"][0]["evidence_key"] = serde_json::json!("period:invented");
    response["evidence_summary"][0]["label"] = serde_json::json!(
        "Une conversation précise peut être cadrée sans rouvrir tout le dossier."
    );

    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();

    assert_eq!(response["evidence_summary"][0]["date"], canonical_date);
    assert_eq!(
        response["evidence_summary"][0]["evidence_key"],
        canonical_key
    );
    assert_eq!(
        response["evidence_summary"][0]["label"],
        "Une conversation précise peut être cadrée sans rouvrir tout le dossier."
    );
}

#[test]
fn horoscope_premium_next_7_days_repair_keeps_llm_evidence_summary_selection() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["evidence_summary"] = serde_json::json!([
        {
            "date": request["evidence"][0]["date"],
            "evidence_key": request["evidence"][0]["evidence_key"],
            "label": "Premier appui concret choisi pour la lecture"
        },
        {
            "date": request["evidence"][3]["date"],
            "evidence_key": request["evidence"][3]["evidence_key"],
            "label": "Deuxième appui concret choisi pour la lecture"
        }
    ]);

    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();

    let summary = response["evidence_summary"].as_array().unwrap();
    assert_eq!(summary.len(), 2);
    assert_eq!(
        summary[0]["label"],
        "Premier appui concret choisi pour la lecture"
    );
    assert_eq!(
        summary[1]["label"],
        "Deuxième appui concret choisi pour la lecture"
    );
}

#[test]
fn horoscope_premium_next_7_days_repair_keeps_canonical_domain_evidence() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    let canonical_keys = response["domain_sections"][0]["evidence_keys"].clone();
    response["domain_sections"][0]["evidence_keys"] = serde_json::json!(["period:invented"]);
    response["domain_sections"][0]["text"] = serde_json::json!(
        "Ce domaine demande de cadrer une conversation précise avec vos repères personnels, sans rouvrir tous les sujets."
    );

    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();

    assert_eq!(
        response["domain_sections"][0]["evidence_keys"],
        canonical_keys
    );
    assert!(response["domain_sections"][0]["text"]
        .as_str()
        .unwrap()
        .starts_with(
            "Ce domaine demande de cadrer une conversation précise avec vos repères personnels, sans rouvrir tous les sujets."
        ));
}

#[test]
fn horoscope_premium_next_7_days_repair_restores_response_evidence_after_provider_loss() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    let canonical_day_keys = response["daily_timeline"][0]["evidence_keys"].clone();
    let canonical_best_day_keys = response["best_days"][0]["evidence_keys"].clone();
    let canonical_domain_keys = response["domain_sections"][0]["evidence_keys"].clone();
    let canonical_window_keys = response["best_windows"][0]["evidence_keys"].clone();
    let canonical_window_sources = response["best_windows"][0]["source_snapshot_keys"].clone();
    let canonical_strategy_keys = request["strategy"]["evidence_keys"].clone();
    response["daily_timeline"][0]["evidence_keys"] = serde_json::json!([]);
    response["key_days"][0]["evidence_keys"] = serde_json::json!([]);
    response["best_days"][0]["evidence_keys"] =
        response["daily_timeline"][1]["evidence_keys"].clone();
    response["watch_days"][0]["evidence_keys"] = serde_json::json!([]);
    response["watch_summary"]["evidence_keys"] = serde_json::json!([]);
    response["domain_sections"][0]["evidence_keys"] =
        response["domain_sections"][1]["evidence_keys"].clone();
    response["best_windows"][0]["evidence_keys"] =
        response["daily_timeline"][2]["evidence_keys"].clone();
    if !response["watch_windows"].as_array().unwrap().is_empty() {
        response["watch_windows"][0]["evidence_keys"] = serde_json::json!([]);
    }
    response["strategy"]["evidence_keys"] = response["daily_timeline"][3]["evidence_keys"].clone();

    repair_period_response_shape(&request, &mut response);
    let response = postprocess_period_provider_response(&request, response);

    validate_period_response_evidence(&request, &response).unwrap();
    assert_eq!(
        response["daily_timeline"][0]["evidence_keys"],
        canonical_day_keys
    );
    assert_eq!(
        response["best_days"][0]["evidence_keys"],
        canonical_best_day_keys
    );
    assert_eq!(
        response["domain_sections"][0]["evidence_keys"],
        canonical_domain_keys
    );
    assert_eq!(
        response["best_windows"][0]["evidence_keys"],
        canonical_window_keys
    );
    assert_eq!(
        response["best_windows"][0]["source_snapshot_keys"],
        canonical_window_sources
    );
    assert_eq!(
        response["strategy"]["evidence_keys"],
        canonical_strategy_keys
    );
}

#[test]
fn horoscope_premium_next_7_days_repair_restores_domain_personalization_after_cleanup() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["domain_sections"][0]["text"] = serde_json::json!(
        "La semaine commence par un besoin net de remettre l'ordre dans vos priorités, puis elle vous demande de le rendre visible, concret et tenable."
    );
    response["domain_sections"][1]["text"] =
        serde_json::json!("Les moments les plus lisibles de la semaine aident à distinguer le vrai désir de la simple habitude.");

    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();

    assert!(response["domain_sections"][0]["text"]
        .as_str()
        .unwrap()
        .contains("direction claire"));
    assert!(response["domain_sections"][1]["text"]
        .as_str()
        .unwrap()
        .contains("direction claire"));
}

#[test]
fn horoscope_premium_next_7_days_accepts_natural_domain_personalization() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["domain_sections"][0]["text"] = serde_json::json!(
        "L'organisation de la semaine tient mieux si vous partez de vous-même et de ce qui doit être visible."
    );
    response["domain_sections"][1]["text"] =
        serde_json::json!("Le coeur de la semaine consiste à remettre les choses à leur place sans alourdir votre agenda.");
    response["domain_sections"][2]["text"] = serde_json::json!(
        "La semaine gagne en clarté quand vous nommez vos priorités avant de répondre."
    );

    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();
}

#[test]
fn horoscope_premium_next_7_days_response_has_3_to_5_domain_sections() {
    let request = premium_period_interpretation_request();
    let response = premium_period_response_from_request(&request);
    validate_period_response_evidence(&request, &response).unwrap();
    let count = response["domain_sections"].as_array().unwrap().len();
    assert!((3..=5).contains(&count));
}

#[test]
fn horoscope_premium_next_7_days_repair_restores_domain_evidence_when_model_renames_domains() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    for (index, section) in response["domain_sections"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        section["domain"] = serde_json::json!(format!("Domaine éditorial {}", index + 1));
        section["evidence_keys"] = serde_json::json!([]);
    }

    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();

    for section in response["domain_sections"].as_array().unwrap() {
        assert!(!section["evidence_keys"].as_array().unwrap().is_empty());
    }
}

#[test]
fn horoscope_premium_next_7_days_repair_personalizes_generic_domain_sections() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    for section in response["domain_sections"].as_array_mut().unwrap() {
        section["text"] = serde_json::json!(
            "Ce domaine donne une manière d'utiliser la semaine sans disperser l'énergie."
        );
    }

    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();

    for section in response["domain_sections"].as_array().unwrap() {
        assert!(section["text"]
            .as_str()
            .unwrap()
            .contains("direction claire"));
    }
    let public = serde_json::to_string(&response["domain_sections"]).unwrap();
    assert!(!public.contains("de accorder"));
    assert!(!public.contains("de éviter"));

    let mut fallback_response = premium_period_response_from_request(&request);
    fallback_response["domain_sections"] = request["domain_sections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|section| {
            serde_json::json!({
                "domain": section["domain"],
                "title": section["title"],
                "text": "Texte fournisseur générique.",
                "evidence_keys": section["evidence_keys"]
            })
        })
        .collect::<serde_json::Value>();
    repair_period_response_shape(&request, &mut fallback_response);
    validate_period_response_evidence(&request, &fallback_response).unwrap();
}

#[test]
fn horoscope_premium_next_7_days_rejects_window_without_evidence() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["best_windows"][0]["evidence_keys"] = serde_json::json!([]);
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_PREMIUM_WINDOW_EVIDENCE_MISSING"
    );
}

#[test]
fn horoscope_premium_next_7_days_rejects_generic_best_windows() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    for window in response["best_windows"].as_array_mut().unwrap() {
        window["title"] = serde_json::json!("Fenêtre favorable");
        window["best_for"] = serde_json::json!([
            "consolider une avancée",
            "revenir à l'essentiel",
            "stabiliser une décision"
        ]);
    }
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_PREMIUM_WINDOWS_TOO_GENERIC"
    );
}

#[test]
fn horoscope_premium_next_7_days_rejects_identical_best_for_sets() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    for (index, window) in response["best_windows"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        window["title"] = serde_json::json!(format!("Fenêtre différenciée {}", index + 1));
        window["best_for"] = serde_json::json!([
            "consolider une avancée",
            "revenir à l'essentiel",
            "stabiliser une décision"
        ]);
    }
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_PREMIUM_WINDOWS_TOO_GENERIC"
    );
}

#[test]
fn horoscope_premium_next_7_days_rejects_window_outside_period() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["best_windows"][0]["date"] = serde_json::json!("2026-06-30");
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH");
}

#[test]
fn horoscope_premium_next_7_days_rejects_best_watch_window_overlap() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    if response["watch_windows"].as_array().unwrap().is_empty() {
        response["watch_windows"] = serde_json::json!([{
            "date": response["best_windows"][0]["date"],
            "time_range_label": response["best_windows"][0]["time_range_label"],
            "source_snapshot_keys": response["best_windows"][0]["source_snapshot_keys"],
            "title": "Fenêtre de vigilance",
            "theme": "communication",
            "tone": "vigilant",
            "watch_point": "Ralentir avant de répondre.",
            "evidence_keys": response["best_windows"][0]["evidence_keys"]
        }]);
    } else {
        response["watch_windows"][0]["date"] = response["best_windows"][0]["date"].clone();
        response["watch_windows"][0]["source_snapshot_keys"] =
            response["best_windows"][0]["source_snapshot_keys"].clone();
    }
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_PREMIUM_WINDOW_OVERLAP"
    );
}

#[test]
fn horoscope_premium_next_7_days_repair_does_not_invent_windows_or_evidence() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["best_windows"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "date": "2026-06-07",
            "time_range_label": "06:00–12:00",
            "source_snapshot_keys": ["not-in-scan"],
            "title": "Fenêtre inventée",
            "theme": "communication",
            "tone": "fluide",
            "reason": "Inventée.",
            "best_for": ["inventer"],
            "evidence_keys": ["invented:evidence"]
        }));
    repair_period_response_shape(&request, &mut response);
    let allowed = request["best_windows"].as_array().unwrap().len();
    assert_eq!(response["best_windows"].as_array().unwrap().len(), allowed);
    let public_evidence = request["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item["evidence_key"].as_str())
        .collect::<std::collections::HashSet<_>>();
    for window in response["best_windows"].as_array().unwrap() {
        for key in window["evidence_keys"].as_array().unwrap() {
            assert!(public_evidence.contains(key.as_str().unwrap()));
        }
    }
}

#[test]
fn horoscope_premium_next_7_days_repair_keeps_missing_windows_missing() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["best_windows"] = serde_json::json!([]);
    repair_period_response_shape(&request, &mut response);
    assert!(response["best_windows"].as_array().unwrap().is_empty());
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_PREMIUM_WINDOWS_MISSING"
    );
}

#[test]
fn horoscope_premium_next_7_days_goldens_validate_shape_and_evidence() {
    let calculation: serde_json::Value = serde_json::from_str(include_str!(
        "golden/horoscope_period_calculation_response_v1_premium_next_7_days_paris_1990.json"
    ))
    .unwrap();
    assert_eq!(
        calculation["service_code"],
        HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE
    );
    assert_eq!(
        calculation["scan_plan"]["scan_profile_code"],
        "six_hour_7_days"
    );
    assert_eq!(calculation["scan_plan"]["snapshot_count"], 28);

    let interpretation: serde_json::Value = serde_json::from_str(include_str!(
        "golden/horoscope_period_interpretation_request_v1_premium_next_7_days_paris_1990.json"
    ))
    .unwrap();
    validate_period_interpretation_request_schema(&interpretation).unwrap();
    assert!(!interpretation["best_windows"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(interpretation["domain_sections"].as_array().unwrap().len() >= 3);

    let response: serde_json::Value = serde_json::from_str(include_str!(
        "golden/horoscope_period_response_v1_premium_next_7_days_fake.json"
    ))
    .unwrap();
    validate_period_response_schema(&response).unwrap();
    validate_period_response_evidence(&interpretation, &response).unwrap();
}

#[test]
fn horoscope_premium_next_7_days_rejects_technical_codes() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["strategy"]["text"] =
        serde_json::json!("Cette stratégie expose snapshot et evidence_key.");
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PERIOD_TECHNICAL_CODE_LEAK");
}

#[test]
fn horoscope_premium_next_7_days_rejects_dates_in_advice_strategy() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["advice"]["main"] =
        serde_json::json!("Le 10/06, avancez vite, puis le 12/06 ralentissez.");
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_PREMIUM_ADVICE_RECALENDARIZED"
    );
}

#[test]
fn horoscope_premium_next_7_days_is_not_basic_shape_with_more_words() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response.as_object_mut().unwrap().remove("strategy");
    response["best_windows"] = serde_json::json!([]);
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    let message = err.detail().message.as_str();
    assert!(
        message.starts_with("HOROSCOPE_PERIOD_RESPONSE_INVALID")
            || matches!(
                message,
                "HOROSCOPE_PERIOD_PREMIUM_WINDOWS_MISSING"
                    | "HOROSCOPE_PERIOD_PREMIUM_INSUFFICIENT_DETAIL"
            )
    );
}

#[test]
fn horoscope_period_best_days_do_not_overlap_watch_days_after_tension_selection() {
    let request = period_interpretation_request();
    let best = request["best_days"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|day| day["date"].as_str())
        .collect::<std::collections::HashSet<_>>();
    for date in request["watch_days"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|day| day["date"].as_str())
    {
        assert!(!best.contains(date), "watch day {date} overlaps best_days");
    }
}

#[test]
fn horoscope_period_best_days_use_distinct_themes() {
    let request = period_interpretation_request();
    let mut themes = std::collections::HashSet::new();
    for day in request["best_days"].as_array().unwrap() {
        let key = day["evidence_keys"][0].as_str().unwrap();
        let theme = request["period_events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["evidence_keys"][0].as_str() == Some(key))
            .unwrap()["theme_code"]
            .as_str()
            .unwrap();
        assert!(themes.insert(theme.to_string()));
    }
}

#[test]
fn horoscope_period_best_days_do_not_duplicate_key_days() {
    let request = period_interpretation_request();
    let key_dates = request["key_days"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|day| day["date"].as_str())
        .collect::<std::collections::HashSet<_>>();
    for date in request["best_days"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|day| day["date"].as_str())
    {
        assert!(!key_dates.contains(date));
    }
}

#[test]
fn horoscope_premium_next_7_days_best_days_are_distinct_dates() {
    let request = premium_period_interpretation_request();
    let mut dates = std::collections::HashSet::new();
    for marker in request["best_days"].as_array().unwrap() {
        assert!(dates.insert(marker["date"].as_str().unwrap().to_string()));
    }
}

#[test]
fn premium_best_days_can_return_two_when_only_two_clear_dates_after_exclusions() {
    let mut request = premium_period_interpretation_request();
    let two_best_days = request["best_days"]
        .as_array()
        .unwrap()
        .iter()
        .take(2)
        .cloned()
        .collect::<Vec<_>>();
    request["best_days"] = serde_json::json!(two_best_days);
    validate_period_interpretation_request_schema(&request).unwrap();

    let mut response = premium_period_response_from_request(&request);
    repair_period_response_shape(&request, &mut response);
    validate_period_response_evidence(&request, &response).unwrap();

    let best_days = response["best_days"].as_array().unwrap();
    assert_eq!(best_days.len(), 2);

    let key_dates = request["key_days"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|day| day["date"].as_str())
        .collect::<std::collections::HashSet<_>>();
    let watch_dates = request["watch_days"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|day| day["date"].as_str())
        .collect::<std::collections::HashSet<_>>();
    for day in best_days {
        let date = day["date"].as_str().unwrap();
        assert!(!key_dates.contains(date));
        assert!(!watch_dates.contains(date));
        assert_eq!(day["fallback_reason"], serde_json::Value::Null);
    }
}

#[test]
fn horoscope_period_response_rejects_best_day_overlapping_key_day() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    if response["key_days"].as_array().unwrap().is_empty() {
        response["key_days"] = serde_json::json!([{
            "date": "2026-06-07",
            "title": "Jour clé",
            "reason": "Dimanche 07/06 ressort par le thème organisation.",
            "evidence_keys": ["period:2026-06-07:2026-06-07:noon:moon:natal_house:7"],
            "fallback_reason": null
        }]);
    }
    response["best_days"][0]["date"] = response["key_days"][0]["date"].clone();
    response["best_days"][0]["evidence_keys"] = response["key_days"][0]["evidence_keys"].clone();
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PERIOD_KEY_DAYS_MISSING");
}

#[test]
fn horoscope_period_response_rejects_duplicate_best_days() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    if response["best_days"].as_array().unwrap().len() < 2 {
        response["best_days"] = serde_json::json!([
            response["best_days"][0].clone(),
            response["best_days"][0].clone()
        ]);
    } else {
        response["best_days"][1]["date"] = response["best_days"][0]["date"].clone();
    }
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_DUPLICATE_DAY_MARKER"
    );
}

#[test]
fn horoscope_period_response_rejects_duplicate_key_days() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    if response["key_days"].as_array().unwrap().len() < 2 {
        response["key_days"] = serde_json::json!([
            response["best_days"][0].clone(),
            response["best_days"][0].clone()
        ]);
    } else {
        response["key_days"][1]["date"] = response["key_days"][0]["date"].clone();
    }
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_DUPLICATE_DAY_MARKER"
    );
}

#[test]
fn horoscope_period_response_rejects_duplicate_watch_days() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    if response["watch_days"].as_array().unwrap().len() < 2 {
        response["watch_days"] = serde_json::json!([
            response["best_days"][0].clone(),
            response["best_days"][0].clone()
        ]);
        response["watch_summary"]["status"] = serde_json::json!("active");
        response["watch_summary"]["evidence_keys"] =
            response["best_days"][0]["evidence_keys"].clone();
    } else {
        response["watch_days"][1]["date"] = response["watch_days"][0]["date"].clone();
    }
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_DUPLICATE_DAY_MARKER"
    );
}

#[test]
fn horoscope_period_daily_plans_have_diverse_internal_tones() {
    let request = period_interpretation_request();
    let tones = request["daily_plans"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|day| day["tone"].as_str())
        .collect::<std::collections::HashSet<_>>();
    assert!(tones.len() >= 2, "expected diversified internal tones");
    assert!(tones.contains("supportive"));
    assert!(tones.contains("careful"));
}

#[test]
fn horoscope_premium_next_7_days_daily_theme_distribution_is_not_over_dominated_when_alternatives_exist(
) {
    let request = premium_period_interpretation_request();
    let mut counts = std::collections::HashMap::<String, usize>::new();
    for day in request["daily_plans"].as_array().unwrap() {
        *counts
            .entry(day["theme_code"].as_str().unwrap().to_string())
            .or_default() += 1;
    }
    let max_count = counts.values().copied().max().unwrap_or(0);
    assert!(
        max_count <= 3,
        "expected premium daily theme distribution not to exceed 3/7 when alternatives exist: {counts:?}"
    );
}

#[test]
fn horoscope_premium_next_7_days_adds_editorial_brief_with_distinct_day_roles() {
    let request = premium_period_interpretation_request();
    let days = request["editorial_brief"]["days"].as_array().unwrap();
    assert_eq!(days.len(), 7);

    let action_modes = days
        .iter()
        .filter_map(|day| day["action_mode"].as_str())
        .collect::<std::collections::HashSet<_>>();
    assert!(
        action_modes.len() >= 4,
        "expected editorial action modes to guide real day differentiation: {action_modes:?}"
    );

    for day in days {
        assert!(!day["public_role"].as_str().unwrap().is_empty());
        assert!(!day["narrative_function"].as_str().unwrap().is_empty());
        assert!(!day["reader_situation"].as_str().unwrap().is_empty());
        assert!(!day["contrast_with_previous_day"]
            .as_str()
            .unwrap()
            .is_empty());
        assert!(!day["avoid_angle_reuse"].as_str().unwrap().is_empty());
    }
}

#[test]
fn horoscope_premium_next_7_days_editorial_brief_has_unique_reader_situations() {
    let request = premium_period_interpretation_request();
    let situations = request["editorial_brief"]["days"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|day| day["reader_situation"].as_str())
        .collect::<Vec<_>>();
    let unique = situations
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        situations.len(),
        unique.len(),
        "reader_situation entries must be human-distinct, not repeated templates: {situations:?}"
    );
}

#[test]
fn horoscope_premium_next_7_days_uses_public_usage_domains() {
    let request = premium_period_interpretation_request();
    let serialized = serde_json::to_string(&request).unwrap();
    assert!(
        !serialized.contains("Intégration") && !serialized.contains("intégration"),
        "period request should not expose abstract integration label publicly"
    );
    assert!(
        serialized.contains("Engagements et limites")
            || serialized.contains("engagements et limites"),
        "expected usage-oriented public domain label"
    );
}

#[test]
fn horoscope_premium_next_7_days_watch_windows_are_not_duplicate_prompts() {
    let request = premium_period_interpretation_request();
    let mut seen = std::collections::HashSet::new();
    for window in request["watch_windows"].as_array().unwrap() {
        let key = format!(
            "{}|{}",
            window["title"].as_str().unwrap_or(""),
            window["watch_point"].as_str().unwrap_or("")
        );
        assert!(
            seen.insert(key.clone()),
            "watch window title/watch_point pair must not be duplicated: {key}"
        );
    }
}

#[test]
fn horoscope_premium_next_7_days_watch_windows_do_not_use_editorial_arc_titles() {
    let request = premium_period_interpretation_request();
    let serialized = serde_json::to_string(&request["watch_windows"])
        .unwrap()
        .to_lowercase();
    for forbidden in [
        "nouvelle facette",
        "répéter le même conseil",
        "fonction narrative",
        "changer l'usage",
        "changer l’usage",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "watch window leaked editorial scaffold: {forbidden}"
        );
    }
}

#[test]
fn horoscope_premium_next_7_days_rejects_meta_watch_window_title() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    if response["watch_windows"].as_array().unwrap().is_empty() {
        response["watch_summary"]["status"] = serde_json::json!("active");
        response["watch_windows"] = serde_json::json!([{
            "date": request["period_resolution"]["included_dates"][0],
            "time_range_label": "12:00–18:00",
            "source_snapshot_keys": request["scan_plan"]["snapshots"][2]["snapshot_key"].as_str().map(|key| vec![key]).unwrap_or_default(),
            "title": "Nouvelle facette de appuis concrets, changer l'usage concret plutôt que répéter le même conseil.",
            "theme": "appuis concrets",
            "tone": "concentré",
            "watch_point": "Rester sur la fonction narrative.",
            "evidence_keys": request["evidence"][0]["evidence_key"].as_str().map(|key| vec![key]).unwrap_or_default()
        }]);
    } else {
        response["watch_windows"][0]["title"] = serde_json::json!(
            "Nouvelle facette de appuis concrets, changer l'usage concret plutôt que répéter le même conseil."
        );
    }
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_PREMIUM_WINDOW_META_LEAK"
    );
}

#[test]
fn horoscope_premium_next_7_days_allows_natural_usage_concret_phrase() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["week_overview"]["text"] = serde_json::json!(
        "La semaine gagne en usage concret quand vous transformez les signaux en choix simples."
    );
    validate_period_response_evidence(&request, &response).unwrap();
}

#[test]
fn horoscope_premium_next_7_days_rejects_meta_public_text_outside_windows() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["week_overview"]["text"] = serde_json::json!(
        "Nouvelle facette de la semaine, à traiter comme fonction narrative plutôt que comme lecture publique."
    );
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_PREMIUM_PUBLIC_META_LEAK"
    );
}

#[test]
fn horoscope_premium_next_7_days_request_hints_are_scene_based_not_meta_instructions() {
    let request = premium_period_interpretation_request();
    let serialized = serde_json::to_string(&request).unwrap().to_lowercase();
    for forbidden in [
        "personnaliser ce signal",
        "relier ce signal",
        "plutôt que rester sur un conseil générique",
        "sans devenir une explication abstraite",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "interpretation request leaked meta hint: {forbidden}"
        );
    }
    assert!(
        serialized.contains("situations associées"),
        "expected scene-based hints in interpretation request"
    );
}

#[test]
fn horoscope_premium_next_7_days_markers_use_editorial_roles_not_raw_theme_sentences() {
    let request = premium_period_interpretation_request();
    let markers = ["key_days", "watch_days"]
        .into_iter()
        .flat_map(|field| request[field].as_array().into_iter().flatten())
        .collect::<Vec<_>>();
    let mut watch_reasons = std::collections::HashSet::new();
    for marker in markers {
        let reason = marker["reason"].as_str().unwrap_or("");
        assert!(!reason.contains(" y pèse davantage"), "{reason}");
        assert!(
            !reason.contains("peut rendre les réactions plus rapides"),
            "{reason}"
        );
        if marker["title"].as_str() == Some("Jour de vigilance") {
            assert!(
                watch_reasons.insert(reason.to_string()),
                "watch day reasons must not repeat: {reason}"
            );
        }
    }
}

#[test]
fn horoscope_period_rejects_editorial_scaffolding_phrase() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["daily_timeline"][0]["text"] = serde_json::json!(
        "Avec vos repères, la journée gagne un repère personnel concret sans devenir une explication abstraite."
    );

    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_META_PERSONALIZATION_LEAK"
    );
}

#[test]
fn horoscope_non_premium_period_requests_do_not_add_editorial_brief() {
    let basic = period_interpretation_request();
    assert!(basic.get("editorial_brief").is_none());

    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let calculation = free_period_calculation();
    let free = build_period_interpretation_request(&public, &calculation).unwrap();
    assert!(free.get("editorial_brief").is_none());
}

#[test]
fn horoscope_premium_next_7_days_uses_less_forced_word_target() {
    let request = premium_period_interpretation_request();
    let mut response = premium_period_response_from_request(&request);
    response["quality"]["provider"] = serde_json::json!("openai");
    response["week_overview"]["text"] = serde_json::json!(vec![
            "Cette phrase allonge volontairement la lecture premium sans changer les preuves.";
            520
        ]
    .join(" "));

    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_WORD_COUNT_OUT_OF_RANGE"
    );
    let details = err.detail().details.as_ref().unwrap();
    assert_eq!(details["target_words_min"], 1600);
    assert_eq!(details["hard_limit_words"], 3200);
}

#[test]
fn horoscope_premium_scores_domain_score_is_not_constant_placeholder() {
    let request = premium_period_interpretation_request();
    let score = request["premium_scores"]["domain_score"].as_f64().unwrap();
    assert!(
        score > 0.0,
        "domain_score should reflect available coverage"
    );
    assert!(
        score < 1.0,
        "domain_score should not be a saturated placeholder: {score}"
    );
}

#[test]
fn horoscope_period_evidence_contains_natal_personalization_hints() {
    let request = period_interpretation_request();
    for item in request["evidence"].as_array().unwrap() {
        assert!(!item["natal_focus_label"].as_str().unwrap().is_empty());
        assert!(!item["natal_focus_hint"].as_str().unwrap().is_empty());
        assert!(!item["personalization_hint"].as_str().unwrap().is_empty());
    }
}

#[test]
fn horoscope_period_domain_sections_cover_two_to_four_scored_themes() {
    let request = period_interpretation_request();
    let domains = request["domain_sections"].as_array().unwrap();
    assert!((2..=4).contains(&domains.len()));
    let mut seen = std::collections::HashSet::new();
    for section in domains {
        assert!(seen.insert(section["domain"].as_str().unwrap().to_string()));
        assert!(!section["personalization_hint"].as_str().unwrap().is_empty());
    }
}

#[test]
fn horoscope_period_response_rejects_duplicate_domain_sections() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["domain_sections"][1]["domain"] = response["domain_sections"][0]["domain"].clone();
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PERIOD_EVIDENCE_MISSING");
}

#[test]
fn horoscope_period_day_markers_use_null_fallback_reason_without_fallback() {
    let request = period_interpretation_request();
    for marker in request["key_days"]
        .as_array()
        .unwrap()
        .iter()
        .chain(request["best_days"].as_array().unwrap())
        .chain(request["watch_days"].as_array().unwrap())
    {
        assert_ne!(marker["fallback_reason"], "");
        if marker["title"] != "Jour de vigilance douce" {
            assert!(marker["fallback_reason"].is_null());
        }
    }
}

#[test]
fn horoscope_period_response_rejects_empty_fallback_reason() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    response["key_days"][0]["fallback_reason"] = serde_json::json!("");
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PERIOD_EVIDENCE_MISSING");
}

#[test]
fn horoscope_period_public_response_requires_natal_personalization() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    for (index, day) in response["daily_timeline"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        day["text"] = serde_json::json!(format!(
            "Cette journée avance dans une progression générale numéro {}.",
            index + 1
        ));
    }
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PERIOD_EVIDENCE_MISSING");
}

#[test]
fn horoscope_period_rejects_repeated_period_vocabulary() {
    let request = period_interpretation_request();
    let mut response = period_response_from_request(&request);
    for day in response["daily_timeline"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .take(3)
    {
        day["text"] = serde_json::json!(
            "Restez concret dans cette journée, avec une nuance reliée au thème natal."
        );
    }
    let err = validate_period_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_REPETITIVE_DAILY_TEXT"
    );
}

#[test]
fn horoscope_period_context_facts_do_not_expose_high_orb() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let mut calculation = period_calculation();
    calculation["snapshots"][1]["transits_to_natal"][0]["fact_type"] =
        serde_json::json!("transit_context");
    calculation["snapshots"][1]["transits_to_natal"][0]["aspect"] = serde_json::Value::Null;
    calculation["snapshots"][1]["transits_to_natal"][0]["orb_deg"] = serde_json::json!(8.4);
    calculation["snapshots"][1]["transits_to_natal"][0]["evidence_key"] =
        serde_json::json!("period:2026-06-08:2026-06-08:noon:venus:context:natal_moon");
    let request = build_period_interpretation_request(&public, &calculation).unwrap();
    let evidence = request["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["fact_type"] == "transit_context")
        .unwrap();
    assert!(evidence["orb_deg"].is_null());
    let event = request["period_events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["evidence_keys"][0] == evidence["evidence_key"])
        .unwrap();
    assert!(event["score"].as_f64().unwrap() < 0.7);
}

#[test]
fn horoscope_period_utc_fields_are_normalized_to_utc_offset() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let request = build_period_calculation_request(&public).unwrap();
    assert!(request["period_resolution"]["start_datetime_utc"]
        .as_str()
        .unwrap()
        .ends_with("+00:00"));
    for snapshot in request["scan_plan"]["snapshots"].as_array().unwrap() {
        assert!(snapshot["reference_datetime_utc"]
            .as_str()
            .unwrap()
            .ends_with("+00:00"));
    }
}

#[test]
fn horoscope_period_rejects_event_outside_window() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let mut calculation = period_calculation();
    calculation["snapshots"][0]["date"] = serde_json::json!("2026-06-20");
    let err = build_period_interpretation_request(&public, &calculation).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PERIOD_EVENT_OUTSIDE_WINDOW"
    );
}

#[test]
fn horoscope_period_application_rejects_wide_named_major_aspect() {
    let public = validate_period_public_request(&period_public_payload()).unwrap();
    let mut calculation = period_calculation();
    calculation["snapshots"][2]["transits_to_natal"][0]["aspect"] = serde_json::json!("square");
    calculation["snapshots"][2]["transits_to_natal"][0]["orb_deg"] = serde_json::json!(6.7);
    let err = build_period_interpretation_request(&public, &calculation).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PERIOD_CALCULATION_FAILED");
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
fn horoscope_premium_requires_location() {
    let mut payload = premium_public_payload();
    payload.as_object_mut().unwrap().remove("location");
    let validator = IntegrationJobValidator::new();
    let body = serde_json::json!({
        "service_code": HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
        "payload": payload
    });
    let err = validator
        .validate_job(&body, &horoscope_premium_service())
        .unwrap_err();
    assert_eq!(
        err.detail().code,
        astral_llm_domain::GenerationErrorCode::SchemaValidationFailed
    );
}

#[test]
fn horoscope_premium_rejects_invalid_latitude_longitude() {
    let mut payload = premium_public_payload();
    payload["location"]["latitude"] = serde_json::json!(91.0);
    let validator = IntegrationJobValidator::new();
    let body = serde_json::json!({
        "service_code": HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
        "payload": payload
    });
    let err = validator
        .validate_job(&body, &horoscope_premium_service())
        .unwrap_err();
    assert_eq!(
        err.detail().code,
        astral_llm_domain::GenerationErrorCode::SchemaValidationFailed
    );
}

#[test]
fn horoscope_premium_rejects_inline_birth_data() {
    let mut payload = premium_public_payload();
    payload["birth_data"] = serde_json::json!({
        "date": "1990-06-15",
        "time": "14:30"
    });
    let validator = IntegrationJobValidator::new();
    let body = serde_json::json!({
        "service_code": HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
        "payload": payload
    });
    let err = validator
        .validate_job(&body, &horoscope_premium_service())
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
fn horoscope_premium_builds_12_local_slots_and_uses_service_house_system() {
    let public = validate_public_request(&premium_public_payload()).unwrap();
    let request = build_calculation_request_for_service(
        HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
        &public,
    )
    .unwrap();
    let slots = request["slots"].as_array().unwrap();
    assert_eq!(
        request["service_code"],
        HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE
    );
    assert_eq!(request["slot_profile_code"], "daily_2h_slots");
    assert_eq!(request["house_system_code"], "placidus");
    assert_eq!(slots.len(), 12);
    assert_eq!(slots[0]["slot_code"], "slot_00_02");
    assert_eq!(slots[0]["reference_local_time"], "01:00");
    assert_eq!(slots[11]["slot_code"], "slot_22_00");
    assert_eq!(request["location"]["latitude"], 48.8566);
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
fn horoscope_premium_interpretation_contains_timeline_inputs() {
    let request = premium_interpretation_request();
    assert_eq!(
        request["service_code"],
        HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE
    );
    assert_eq!(request["slots"].as_array().unwrap().len(), 12);
    assert!(!request["best_slots"].as_array().unwrap().is_empty());
    assert!(!request["watch_slots"].as_array().unwrap().is_empty());
    assert!(!request["domain_sections"].as_array().unwrap().is_empty());
    assert_eq!(request["period"]["location_label"], "Paris");
}

#[test]
fn horoscope_premium_evidence_keeps_all_slot_required_keys_when_main_signals_are_capped() {
    let public = validate_public_request(&premium_public_payload()).unwrap();
    let mut calculation = premium_calculation();
    for slot in calculation["slots"].as_array_mut().unwrap() {
        let slot_code = slot["slot_code"].as_str().unwrap().to_string();
        let facts = slot["transits_to_natal"].as_array_mut().unwrap();
        facts.push(serde_json::json!({
            "evidence_key": format!("slot:{slot_code}:venus:trine:natal_moon:extra"),
            "fact_type": "transit_to_natal",
            "source": "test",
            "transiting_object": "venus",
            "natal_target": "natal_moon",
            "aspect": "trine",
            "orb_deg": 1.1,
            "natal_house": 6
        }));
        facts.push(serde_json::json!({
            "evidence_key": format!("slot:{slot_code}:mars:square:natal_moon:extra"),
            "fact_type": "transit_to_natal",
            "source": "test",
            "transiting_object": "mars",
            "natal_target": "natal_moon",
            "aspect": "square",
            "orb_deg": 1.2,
            "natal_house": 6
        }));
    }
    let signals = score_calculation(&calculation).unwrap();
    let request = build_interpretation_request(&public, &calculation, &signals).unwrap();
    let evidence = request["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item["evidence_key"].as_str())
        .collect::<std::collections::HashSet<_>>();
    assert!(request["main_signals"].as_array().unwrap().len() <= 24);
    for slot in request["slots"].as_array().unwrap() {
        for key in slot["required_evidence_keys"].as_array().unwrap() {
            assert!(
                evidence.contains(key.as_str().unwrap()),
                "missing planned premium evidence key: {key}"
            );
        }
    }
}

#[test]
fn horoscope_premium_does_not_invent_location_label() {
    let public = validate_public_request(&premium_public_payload_without_label()).unwrap();
    let signals = score_calculation(&premium_calculation()).unwrap();
    let request = build_interpretation_request(&public, &premium_calculation(), &signals).unwrap();
    assert!(request["period"].get("location_label").is_none());
    let response = premium_response_from_request(&request);
    assert!(response["period"].get("location_label").is_none());
    validate_response_evidence(&request, &response).unwrap();
}

#[test]
fn horoscope_premium_timeline_has_exact_ordered_public_labels() {
    let request = premium_interpretation_request();
    let response = premium_response_from_request(&request);
    validate_response_evidence(&request, &response).unwrap();
    let labels = response["timeline"]
        .as_array()
        .unwrap()
        .iter()
        .map(|slot| slot["slot_label"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        labels,
        vec![
            "00:00–02:00",
            "02:00–04:00",
            "04:00–06:00",
            "06:00–08:00",
            "08:00–10:00",
            "10:00–12:00",
            "12:00–14:00",
            "14:00–16:00",
            "16:00–18:00",
            "18:00–20:00",
            "20:00–22:00",
            "22:00–00:00"
        ]
    );
}

#[test]
fn horoscope_premium_rejects_slot_in_both_best_and_watch() {
    let request = premium_interpretation_request();
    let mut response = premium_response_from_request(&request);
    response["watch_slots"][0] = response["best_slots"][0].clone();
    let err = validate_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PREMIUM_CONTRADICTORY_SLOT_CLASSIFICATION"
    );
}

#[test]
fn horoscope_premium_rejects_unknown_best_slot_label() {
    let request = premium_interpretation_request();
    let mut response = premium_response_from_request(&request);
    response["best_slots"][0]["slot_label"] = serde_json::json!("Demain matin");
    let err = validate_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PREMIUM_UNKNOWN_SLOT_CLASSIFICATION"
    );
}

#[test]
fn horoscope_premium_rejects_best_slot_with_another_slot_evidence() {
    let request = premium_interpretation_request();
    let mut response = premium_response_from_request(&request);
    response["best_slots"][0]["evidence_keys"] =
        request["slots"][11]["required_evidence_keys"].clone();
    let err = validate_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_EVIDENCE_MISMATCH");
}

#[test]
fn horoscope_premium_rejects_repeated_watch_slot_reasons() {
    let request = premium_interpretation_request();
    let mut response = premium_response_from_request(&request);
    response["watch_slots"] = request["watch_slots"].clone();
    for slot in response["watch_slots"].as_array_mut().unwrap() {
        slot["reason"] =
            serde_json::json!("La tension du signal principal invite à ralentir les réponses.");
    }

    let err = validate_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PREMIUM_REPETITIVE_SLOT_REASON"
    );
}

#[test]
fn horoscope_premium_rejects_public_slot_codes() {
    let request = premium_interpretation_request();
    let mut response = premium_response_from_request(&request);
    response["timeline"][0]["text"] = serde_json::json!("Le slot_00_02 ne doit pas sortir.");
    let err = validate_response_evidence(&request, &response).unwrap_err();
    assert_eq!(err.detail().message, "HOROSCOPE_PUBLIC_SLOT_CODE_LEAK");
}

#[test]
fn horoscope_premium_rejects_missing_local_chart() {
    let mut calculation = premium_calculation();
    calculation["slots"][0]
        .as_object_mut()
        .unwrap()
        .remove("local_chart");
    let err = score_calculation(&calculation).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PREMIUM_LOCAL_CHART_MISSING"
    );
}

#[test]
fn horoscope_premium_rejects_malformed_local_chart_houses() {
    let mut calculation = premium_calculation();
    calculation["slots"][0]["local_chart"]["houses"] = serde_json::json!([]);
    let err = score_calculation(&calculation).unwrap_err();
    assert_eq!(
        err.detail().message,
        "HOROSCOPE_PREMIUM_LOCAL_CHART_MISSING"
    );
}

#[test]
fn horoscope_calculation_response_schema_accepts_premium_12_slots() {
    let schema = calculator_response_schema();
    assert_eq!(schema["properties"]["slots"]["maxItems"], 12);
    assert_eq!(
        schema["allOf"][2]["then"]["properties"]["slots"]["maxItems"],
        12
    );
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
fn horoscope_premium_real_local_calculation_never_uses_fake_fallback() {
    let calculation = premium_calculation();
    for slot in calculation["slots"].as_array().unwrap() {
        let source = slot["transits_to_natal"][0]["source"].as_str().unwrap();
        assert_eq!(source, "test");
        assert_ne!(source, "real_calculator");
    }
}

#[test]
fn horoscope_response_golden_passes_schema_and_evidence_guard() {
    let request = interpretation_request();
    let response = golden_response();
    validate_response_evidence(&request, &response).unwrap();
}

#[test]
fn horoscope_basic_daily_public_watch_points_are_humanized() {
    let request = interpretation_request();
    let response = golden_response();
    validate_response_evidence(&request, &response).unwrap();
    let public_watch_points = response["slots"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|slot| slot["watch_point"].as_str())
        .collect::<Vec<_>>();

    assert_eq!(public_watch_points.len(), 3);
    assert!(public_watch_points
        .iter()
        .all(|watch_point| !watch_point.contains("avoid_")));
}

#[test]
fn horoscope_basic_daily_rejects_internal_watch_point_codes() {
    let request = interpretation_request();
    let mut response = golden_response();
    response["slots"][0]["watch_point"] = serde_json::json!("avoid_opening_too_many_topics");

    let err = validate_response_evidence(&request, &response).unwrap_err();
    assert_eq!(
        err.detail().code,
        astral_llm_domain::GenerationErrorCode::PostSafetyValidationFailed
    );
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
fn horoscope_response_schema_accepts_premium_shape() {
    let request = premium_interpretation_request();
    let response = premium_response_from_request(&request);
    validate_horoscope_response_schema(&response).unwrap();
}

#[test]
fn horoscope_response_schema_rejects_premium_without_timeline() {
    let request = premium_interpretation_request();
    let mut response = premium_response_from_request(&request);
    response.as_object_mut().unwrap().remove("timeline");
    assert!(validate_horoscope_response_schema(&response).is_err());
}

#[test]
fn horoscope_response_schema_rejects_premium_with_less_than_12_timeline_slots() {
    let request = premium_interpretation_request();
    let mut response = premium_response_from_request(&request);
    response["timeline"].as_array_mut().unwrap().pop();
    assert!(validate_horoscope_response_schema(&response).is_err());
}

#[test]
fn horoscope_response_schema_rejects_free_with_premium_timeline() {
    let request = premium_interpretation_request();
    let mut response = free_golden_response();
    response["timeline"] = premium_response_from_request(&request)["timeline"].clone();
    assert!(validate_horoscope_response_schema(&response).is_err());
}

#[test]
fn horoscope_response_schema_rejects_basic_with_premium_shape() {
    let request = premium_interpretation_request();
    let mut response = premium_response_from_request(&request);
    response["service_code"] = serde_json::json!(HOROSCOPE_SERVICE_CODE);
    assert!(validate_horoscope_response_schema(&response).is_err());
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
    assert!(service_has_v1_orchestrator(&horoscope_premium_service()));
    assert!(service_has_v1_orchestrator(&horoscope_free_period_service()));
    assert!(service_has_v1_orchestrator(&horoscope_period_service()));
    assert!(service_has_v1_orchestrator(
        &horoscope_premium_period_service()
    ));
}

#[test]
fn horoscope_basic_free_non_regression_after_premium_routing() {
    assert!(service_has_v1_orchestrator(&horoscope_service()));
    assert!(service_has_v1_orchestrator(&horoscope_free_service()));
    let public = validate_public_request(&public_payload()).unwrap();
    assert_eq!(
        build_calculation_request(&public).unwrap()["slots"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        build_calculation_request_for_service(HOROSCOPE_FREE_DAILY_SERVICE_CODE, &public).unwrap()
            ["slots"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn horoscope_basic_free_non_regression_after_premium_validators() {
    let basic_request = interpretation_request();
    let basic_response = golden_response();
    validate_response_evidence(&basic_request, &basic_response).unwrap();

    let free_request = free_interpretation_request();
    let free_response = free_golden_response();
    validate_response_evidence(&free_request, &free_response).unwrap();
}
