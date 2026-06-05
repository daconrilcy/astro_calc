use astral_calculator::simplified::{
    build_response, build_uncertainty_window, dedupe_preserve_order, sample_points_utc,
    validate_and_resolve, AmbiguousSignFactResponse, AstroSimplifiedNatalRequest,
    CalculationScope, CollectedSignFacts, InputPrecisionLevel, LimitationCode,
    ReliabilityLevel, SignFactResponse, SimplifiedCatalog, SimplifiedLocationRequest,
    SimplifiedPolicy, RELIABILITY_AMBIGUOUS, RELIABILITY_STABLE,
};
use chrono::{Duration, TimeZone, Utc};
use serde_json::json;

fn test_catalog() -> SimplifiedCatalog {
    SimplifiedCatalog {
        policy: SimplifiedPolicy {
            code: "default_v1".into(),
            reference_time_utc: "12:00:00".into(),
            date_only_uncertainty_mode: "world_civil_date_window".into(),
            uncertainty_sampling_minutes: 60,
            default_timezone_strategy: "explicit_iana_only".into(),
            cusp_warning_orb_deg: 1.0,
            stable_fact_strategy: "sampled_window_unique_signs".into(),
        },
        limitation_codes: vec![
            LimitationCode {
                code: "birth_time_missing".into(),
                severity: "blocking".into(),
                affected_features_json: json!(["ascendant", "houses"]),
            },
            LimitationCode {
                code: "location_provided_without_usable_timezone".into(),
                severity: "info".into(),
                affected_features_json: json!(["local_day_window"]),
            },
            LimitationCode {
                code: "location_missing_for_ascendant_and_houses".into(),
                severity: "info".into(),
                affected_features_json: json!(["ascendant", "houses"]),
            },
        ],
        reliability_levels: vec![
            ReliabilityLevel {
                code: "calculated_from_declared_datetime".into(),
                allows_interpretive_affirmation: true,
            },
            ReliabilityLevel {
                code: "stable_across_uncertainty_window".into(),
                allows_interpretive_affirmation: true,
            },
            ReliabilityLevel {
                code: "ambiguous_across_uncertainty_window".into(),
                allows_interpretive_affirmation: false,
            },
            ReliabilityLevel {
                code: "reference_based".into(),
                allows_interpretive_affirmation: false,
            },
        ],
        calculation_scopes: vec![
            CalculationScope {
                code: "stable_birth_date_profile".into(),
                min_input_precision_code: "date_only".into(),
                supports_angles: false,
                supports_houses: false,
                supports_aspects: false,
                supports_object_sign_facts: true,
                supports_ambiguous_facts: true,
            },
            CalculationScope {
                code: "planetary_positions".into(),
                min_input_precision_code: "datetime_without_location".into(),
                supports_angles: false,
                supports_houses: false,
                supports_aspects: true,
                supports_object_sign_facts: true,
                supports_ambiguous_facts: false,
            },
            CalculationScope {
                code: "angular_chart".into(),
                min_input_precision_code: "complete_birth_data".into(),
                supports_angles: true,
                supports_houses: true,
                supports_aspects: true,
                supports_object_sign_facts: true,
                supports_ambiguous_facts: false,
            },
        ],
        input_precision_levels: vec![
            InputPrecisionLevel { code: "date_only".into() },
            InputPrecisionLevel {
                code: "date_with_location_without_timezone".into(),
            },
            InputPrecisionLevel {
                code: "date_with_timezone_without_time".into(),
            },
            InputPrecisionLevel {
                code: "date_with_location_and_timezone_without_time".into(),
            },
            InputPrecisionLevel {
                code: "datetime_without_location".into(),
            },
            InputPrecisionLevel {
                code: "complete_birth_data".into(),
            },
        ],
    }
}

fn base_request() -> AstroSimplifiedNatalRequest {
    serde_json::from_value(json!({
        "request_contract_version": "astro_simplified_natal_request_v1",
        "birth": { "date": "1990-03-21" }
    }))
    .expect("request")
}

#[test]
fn resolve_input_precision_matrix() {
    let catalog = test_catalog();

    let date_only = validate_and_resolve(&base_request(), &catalog).expect("date_only");
    assert_eq!(date_only.input_precision_level, "date_only");
    assert_eq!(date_only.computed_scope, "stable_birth_date_profile");

    let mut with_tz = base_request();
    with_tz.birth.timezone = Some("Europe/Paris".into());
    let resolved = validate_and_resolve(&with_tz, &catalog).expect("date+tz");
    assert_eq!(resolved.input_precision_level, "date_with_timezone_without_time");

    let mut with_loc_tz = with_tz.clone();
    with_loc_tz.birth.location = Some(SimplifiedLocationRequest {
        latitude: 48.8566,
        longitude: 2.3522,
        label: None,
    });
    let resolved = validate_and_resolve(&with_loc_tz, &catalog).expect("date+loc+tz");
    assert_eq!(
        resolved.input_precision_level,
        "date_with_location_and_timezone_without_time"
    );
    assert_eq!(resolved.computed_scope, "stable_birth_date_profile");
}

#[test]
fn resolve_rejects_time_without_timezone() {
    let catalog = test_catalog();
    let mut request = base_request();
    request.birth.time = Some("14:30:00".into());
    let err = validate_and_resolve(&request, &catalog).expect_err("must fail");
    assert!(err.to_string().contains("timezone"));
}

#[test]
fn resolve_rejects_invalid_coordinates() {
    let catalog = test_catalog();
    let mut request = base_request();
    request.birth.location = Some(SimplifiedLocationRequest {
        latitude: 91.0,
        longitude: 0.0,
        label: None,
    });
    let err = validate_and_resolve(&request, &catalog).expect_err("latitude");
    assert!(err.to_string().contains("latitude"));
}

#[test]
fn uncertainty_world_window_is_about_50_hours() {
    let catalog = test_catalog();
    let resolved = validate_and_resolve(&base_request(), &catalog).expect("resolved");
    let (start, end) = build_uncertainty_window(&resolved, &catalog).expect("window");
    let hours = (end - start).num_hours();
    assert!((48..=52).contains(&hours), "expected ~50h, got {hours}h");
}

#[test]
fn uncertainty_local_day_is_24_hours() {
    let catalog = test_catalog();
    let mut request = base_request();
    request.birth.timezone = Some("Europe/Paris".into());
    let resolved = validate_and_resolve(&request, &catalog).expect("resolved");
    let (start, end) = build_uncertainty_window(&resolved, &catalog).expect("window");
    let hours = (end - start).num_hours();
    assert!((23..=24).contains(&hours), "expected ~24h, got {hours}h");
}

#[test]
fn location_without_timezone_uses_same_window_as_date_only() {
    let catalog = test_catalog();
    let date_only = validate_and_resolve(&base_request(), &catalog).expect("date");
    let mut with_loc = base_request();
    with_loc.birth.location = Some(SimplifiedLocationRequest {
        latitude: 48.8566,
        longitude: 2.3522,
        label: None,
    });
    let with_loc = validate_and_resolve(&with_loc, &catalog).expect("loc");
    let (s1, e1) = build_uncertainty_window(&date_only, &catalog).expect("w1");
    let (s2, e2) = build_uncertainty_window(&with_loc, &catalog).expect("w2");
    assert_eq!(s1, s2);
    assert_eq!(e1, e2);
}

#[test]
fn sampling_includes_start_and_end_and_60_min_steps() {
    let start = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
    let end = start + Duration::hours(3);
    let points = sample_points_utc(start, end, 60);
    assert_eq!(points.first(), Some(&start));
    assert_eq!(points.last(), Some(&end));
    assert!(points.len() >= 4);
}

#[test]
fn dedupe_preserves_observation_order() {
    let input = vec![
        "aries".into(),
        "taurus".into(),
        "aries".into(),
        "gemini".into(),
        "taurus".into(),
    ];
    assert_eq!(
        dedupe_preserve_order(&input),
        vec!["aries", "taurus", "gemini"]
    );
}

#[test]
fn llm_controls_block_ambiguous_and_allow_stable() {
    let catalog = test_catalog();
    let resolved = validate_and_resolve(&base_request(), &catalog).expect("resolved");
    let collected = CollectedSignFacts {
        facts: vec![SignFactResponse {
            object_code: "sun".into(),
            fact_type: "sign".into(),
            sign_code: "aries".into(),
            reliability: RELIABILITY_STABLE.to_string(),
            longitude_deg: None,
        }],
        ambiguous_facts: vec![AmbiguousSignFactResponse {
            object_code: "moon".into(),
            fact_type: "sign".into(),
            possible_sign_codes: vec!["gemini".into(), "cancer".into()],
            reliability: RELIABILITY_AMBIGUOUS.to_string(),
        }],
        cusp_warnings: vec![],
    };
    let response = build_response(&resolved, &catalog, collected, None);
    assert!(response.llm_payload.allowed_fact_codes.contains(&"sun.sign".to_string()));
    assert!(response
        .llm_payload
        .blocked_interpretation_fact_codes
        .contains(&"moon.sign".to_string()));
    assert!(response
        .llm_payload
        .allowed_limitation_mentions
        .contains(&"moon.sign".to_string()));
    let payload = response.simplified_payload.payload;
    assert!(payload["planets"]["sun"].is_object());
    assert!(payload["planets"]["moon"].is_null() || payload["planets"].get("moon").is_none());
}

#[test]
fn datetime_without_location_adds_limitation() {
    let catalog = test_catalog();
    let mut request = base_request();
    request.birth.time = Some("10:15:00".into());
    request.birth.timezone = Some("Europe/Paris".into());
    let resolved = validate_and_resolve(&request, &catalog).expect("resolved");
    assert_eq!(resolved.computed_scope, "planetary_positions");
    assert!(resolved
        .limitations
        .contains(&"location_missing_for_ascendant_and_houses".to_string()));
}

#[cfg(feature = "swisseph-engine")]
#[test]
fn moon_can_span_multiple_signs_on_world_window() {
    use astral_calculator::config::ephemeris_path_from_env;
    use astral_calculator::simplified::{build_uncertainty_window, collect_window_sign_facts};
    use astral_calculator::models::SignReference;

    let catalog = test_catalog();
    let ephemeris_path = ephemeris_path_from_env();
    if !ephemeris_path.exists() {
        eprintln!("SKIP moon_can_span_multiple_signs_on_world_window: ephemerides missing");
        return;
    }

    let signs: Vec<SignReference> = (1..=12)
        .map(|id| SignReference {
            id,
            code: format!("sign_{id}"),
            name: format!("Sign {id}"),
            element_code: None,
            element_label: None,
            modality_code: None,
            modality_name: None,
            polarity_code: None,
            polarity_name: None,
            keywords_json: None,
            shadow_keywords_json: None,
        })
        .collect();

    let moon_object = astral_calculator::models::ChartObject {
        id: 2,
        code: "moon".into(),
        name: "Moon".into(),
        swe_id: Some(1),
        role_code: None,
        role_label: None,
        is_luminary: Some(true),
        is_planet_symbolic: None,
        is_visible_to_naked_eye: None,
        nature_codes: None,
        position_priority_base: None,
        angle_priority_base: None,
        source_weight: None,
    };

    let mut found_two_or_more = false;
    for day in 1..=28 {
        let mut request = base_request();
        request.birth.date = format!("1990-01-{day:02}");
        let Ok(resolved) = validate_and_resolve(&request, &catalog) else {
            continue;
        };
        let Ok((start, end)) = build_uncertainty_window(&resolved, &catalog) else {
            continue;
        };
        let collected = collect_window_sign_facts(
            &ephemeris_path,
            &resolved,
            &catalog,
            std::slice::from_ref(&moon_object),
            &signs,
            start,
            end,
        )
        .expect("collect");
        if let Some(moon) = collected.ambiguous_facts.iter().find(|f| f.object_code == "moon") {
            if moon.possible_sign_codes.len() >= 2 {
                found_two_or_more = true;
                break;
            }
        }
    }
    assert!(
        found_two_or_more,
        "expected at least one January 1990 date with moon spanning 2+ signs in world window"
    );
}
