use chrono::{TimeZone, Utc};
use serde_json::json;

use rust_sqlx_connection_test::domain::*;
use rust_sqlx_connection_test::payload::*;

fn input() -> NatalChartInput {
    NatalChartInput {
        subject_label: None,
        birth_datetime_utc: Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap(),
        latitude_deg: 48.8566,
        longitude_deg: 2.3522,
        altitude_m: None,
        reference_version_id: 1,
        calculation_profile_id: None,
        zodiacal_reference_system_id: 1,
        coordinate_reference_system_id: 1,
        house_system_id: 1,
        product_code: Some("basic".to_string()),
    }
}

fn position() -> ObjectPositionFact {
    ObjectPositionFact {
        chart_object_id: 1,
        object_code: "sun".to_string(),
        object_name: "Sun".to_string(),
        zodiacal_reference_system_id: 1,
        coordinate_reference_system_id: 1,
        sign_id: 3,
        sign_code: "gemini".to_string(),
        sign_name: "Gemini".to_string(),
        house_id: Some(9),
        house_number: Some(9),
        house_name: Some("Beliefs".to_string()),
        motion_state_id: Some(1),
        horizon_position_id: None,
        longitude_deg: 84.0,
        latitude_deg: None,
        apparent_speed_deg_per_day: Some(1.0),
        altitude_deg: None,
        is_visible: None,
        facts_json: Some(json!({
            "sign_context": {
                "element": "air",
                "modality": "mutable",
                "polarity": "yang",
                "keywords": ["communication"]
            },
            "house_context": {"theme_code": "beliefs"},
            "house_modality": {
                "code": "cadent",
                "accidental_strength": "weak_or_background",
                "interpretation_weight": "lower_for_external_manifestation"
            },
            "object_context": {
                "role": "luminary",
                "nature": ["luminary"],
                "is_luminary": true
            },
            "motion_context": {
                "motion_state": "direct",
                "label": "Direct",
                "motion_family": "forward"
            }
        })),
    }
}

fn saturn_capricorn_position() -> ObjectPositionFact {
    ObjectPositionFact {
        chart_object_id: 7,
        object_code: "saturn".to_string(),
        object_name: "Saturn".to_string(),
        zodiacal_reference_system_id: 1,
        coordinate_reference_system_id: 1,
        sign_id: 10,
        sign_code: "capricorn".to_string(),
        sign_name: "Capricorn".to_string(),
        house_id: Some(2),
        house_number: Some(2),
        house_name: Some("Resources".to_string()),
        motion_state_id: Some(1),
        horizon_position_id: None,
        longitude_deg: 276.0,
        latitude_deg: None,
        apparent_speed_deg_per_day: Some(0.05),
        altitude_deg: None,
        is_visible: None,
        facts_json: Some(json!({
            "sign_context": {
                "element": "earth",
                "modality": "cardinal",
                "polarity": "yin"
            },
            "house_context": {"theme_code": "resources"},
            "house_modality": {
                "code": "succedent"
            },
            "object_context": {
                "role": "planet"
            },
            "motion_context": {
                "motion_state": "direct"
            }
        })),
    }
}

fn capricorn_house_2_position(
    chart_object_id: i32,
    object_code: &str,
    object_name: &str,
) -> ObjectPositionFact {
    ObjectPositionFact {
        chart_object_id,
        object_code: object_code.to_string(),
        object_name: object_name.to_string(),
        zodiacal_reference_system_id: 1,
        coordinate_reference_system_id: 1,
        sign_id: 10,
        sign_code: "capricorn".to_string(),
        sign_name: "Capricorn".to_string(),
        house_id: Some(2),
        house_number: Some(2),
        house_name: Some("Resources".to_string()),
        motion_state_id: Some(1),
        horizon_position_id: None,
        longitude_deg: 270.0 + chart_object_id as f64,
        latitude_deg: None,
        apparent_speed_deg_per_day: Some(1.0),
        altitude_deg: None,
        is_visible: None,
        facts_json: Some(json!({
            "sign_context": {
                "element": "earth",
                "modality": "cardinal",
                "polarity": "yin"
            },
            "house_context": {"theme_code": "resources"},
            "house_modality": {"code": "succedent"},
            "object_context": {"role": "planet"},
            "motion_context": {"motion_state": "direct"}
        })),
    }
}

fn angle_position(
    id: i32,
    object_code: &str,
    object_name: &str,
    angle_point_code: &str,
    opposite_angle_code: &str,
    axis: &str,
    longitude_deg: f64,
) -> ObjectPositionFact {
    ObjectPositionFact {
        chart_object_id: id,
        object_code: object_code.to_string(),
        object_name: object_name.to_string(),
        zodiacal_reference_system_id: 1,
        coordinate_reference_system_id: 1,
        sign_id: 1,
        sign_code: "aries".to_string(),
        sign_name: "Aries".to_string(),
        house_id: Some(1),
        house_number: Some(1),
        house_name: Some("Self".to_string()),
        motion_state_id: None,
        horizon_position_id: None,
        longitude_deg,
        latitude_deg: None,
        apparent_speed_deg_per_day: None,
        altitude_deg: None,
        is_visible: None,
        facts_json: Some(json!({
            "angle_context": {
                "angle_point_code": angle_point_code,
                "full_name": object_name,
                "axis": axis,
                "opposite_angle_code": opposite_angle_code,
                "associated_house_number": 1,
                "chart_object_sort_order": id
            },
            "house_context": {"theme_code": "identity"}
        })),
    }
}

#[test]
fn basic_payload_exposes_semantic_signal_fields() {
    let signal = InterpretationSignalRow {
        id: 1,
        signal_key: "object_position:sun".to_string(),
        theme_code: Some("beliefs".to_string()),
        title: "Sun in Gemini, house 9".to_string(),
        summary: Some("summary".to_string()),
        priority_score: 100.0,
        confidence_score: Some(0.95),
        payload_json: Some(json!({
            "interpretive_hint": "hint",
            "semantic_tags": ["placement", "gemini", "beliefs"],
            "source_weight": 1.0,
            "aggregation_group": "gemini:house_9",
            "writing_guidance": "guidance",
            "evidence": {"fact_type": "object_position"}
        })),
    };

    let payload = build_basic_payload(42, &input(), &[position()], &[signal]);
    let basic_signal = &payload.signals[0];

    assert_eq!(basic_signal.theme_code.as_deref(), Some("beliefs"));
    assert_eq!(basic_signal.interpretive_hint.as_deref(), Some("hint"));
    assert_eq!(
        basic_signal.semantic_tags,
        vec!["placement", "gemini", "beliefs"]
    );
    assert_eq!(basic_signal.source_weight, Some(1.0));
    assert_eq!(
        basic_signal.aggregation_group.as_deref(),
        Some("gemini:house_9")
    );
    assert_eq!(basic_signal.writing_guidance.as_deref(), Some("guidance"));
    assert_eq!(
        basic_signal
            .evidence
            .as_ref()
            .and_then(|value| value.get("fact_type"))
            .and_then(|value| value.as_str()),
        Some("object_position")
    );
    assert_eq!(payload.reading_plan.len(), 1);
    assert_eq!(payload.reading_plan[0].slot, "core_identity");
    assert_eq!(
        payload.reading_plan[0].source_signal_keys,
        vec!["object_position:sun"]
    );
    assert_eq!(payload.drafting_plan.len(), 1);
    assert_eq!(payload.drafting_plan[0].slot, "core_identity");
    assert_eq!(
        payload.drafting_plan[0].source_signal_keys,
        payload.reading_plan[0].source_signal_keys
    );
    assert_eq!(payload.drafting_plan[0].max_words, 110);
    assert_eq!(
        payload.drafting_plan[0].emphasis_refs.dominant_signs,
        vec!["gemini"]
    );
    assert_eq!(
        payload.drafting_plan[0].emphasis_refs.dominant_houses,
        vec![9]
    );
    assert_eq!(
        payload.drafting_plan[0].emphasis_refs.dominant_objects,
        vec!["sun"]
    );
    assert_eq!(
        payload
            .llm_handoff_contract
            .as_ref()
            .expect("llm handoff contract")
            .contract_version,
        "basic_natal_structured_v9"
    );
    let contract = payload
        .llm_handoff_contract
        .as_ref()
        .expect("llm handoff contract");
    assert!(contract.must_use.contains(&"chart_emphasis".to_string()));
    assert!(contract.must_not.contains(
        &"treat chart_emphasis as a standalone section instead of weighting context".to_string()
    ));
    assert!(contract.must_use.contains(&"dignities".to_string()));
    assert!(contract.must_use.contains(&"angles".to_string()));
    assert_eq!(contract.payload_language_code, "en");
    assert_eq!(contract.target_language_policy, "provided_by_llm_service");
    assert!(contract.must_use.contains(&"signals".to_string()));
    assert_eq!(
        payload.positions[0]
            .sign_context
            .as_ref()
            .and_then(|context| context.get("element"))
            .and_then(|value| value.as_str()),
        Some("air")
    );
    assert_eq!(
        payload.positions[0]
            .motion_context
            .as_ref()
            .and_then(|context| context.get("motion_state"))
            .and_then(|value| value.as_str()),
        Some("direct")
    );
    assert_eq!(
        payload.positions[0]
            .dignity_context
            .as_array()
            .map(Vec::len),
        Some(0)
    );
}

#[test]
fn basic_payload_resolves_angle_opposites_to_object_codes() {
    let positions = vec![
        angle_position(
            11,
            "ascendant",
            "Ascendant",
            "asc",
            "dsc",
            "horizontal",
            15.0,
        ),
        angle_position(
            12,
            "descendant",
            "Descendant",
            "dsc",
            "asc",
            "horizontal",
            195.0,
        ),
    ];

    let payload = build_basic_payload(42, &input(), &positions, &[]);
    let ascendant = payload
        .angles
        .iter()
        .find(|angle| angle.angle_code == "ascendant")
        .expect("ascendant angle");
    let descendant = payload
        .angles
        .iter()
        .find(|angle| angle.angle_code == "descendant")
        .expect("descendant angle");

    assert_eq!(ascendant.opposite_angle_code, "descendant");
    assert_eq!(descendant.opposite_angle_code, "ascendant");
}

#[test]
fn basic_payload_builds_reading_plan_with_cluster_sources() {
    let signals = vec![
        InterpretationSignalRow {
            id: 1,
            signal_key: "cluster:capricorn:house_2".to_string(),
            theme_code: Some("resources".to_string()),
            title: "Strong concentration in Capricorn, house 2".to_string(),
            summary: Some("summary".to_string()),
            priority_score: 99.0,
            confidence_score: Some(0.9),
            payload_json: Some(json!({
                "interpretive_hint": "hint",
                "semantic_tags": ["cluster", "capricorn", "house_2", "resources", "structure", "responsibility"],
                "source_weight": 2.0,
                "aggregation_group": "capricorn_house_2_cluster",
                "writing_guidance": "guidance",
                "evidence": {
                    "fact_type": "position_cluster",
                    "sign_name": "Capricorn",
                    "house_name": "Resources",
                    "source_signals": [
                        "object_position:sun",
                        "object_position:saturn"
                    ]
                }
            })),
        },
        InterpretationSignalRow {
            id: 2,
            signal_key: "object_position:sun".to_string(),
            theme_code: Some("resources".to_string()),
            title: "Sun in Capricorn, house 2".to_string(),
            summary: Some("summary".to_string()),
            priority_score: 100.0,
            confidence_score: Some(0.95),
            payload_json: Some(json!({
                "interpretive_hint": "hint",
                "semantic_tags": ["placement", "sun"],
                "source_weight": 1.0,
                "aggregation_group": "capricorn:house_2",
                "writing_guidance": "guidance",
                "evidence": {"fact_type": "object_position", "object_code": "sun"}
            })),
        },
        InterpretationSignalRow {
            id: 3,
            signal_key: "aspect:sun:neptune:conjunction".to_string(),
            theme_code: Some("aspect".to_string()),
            title: "Sun conjunction Neptune".to_string(),
            summary: Some("summary".to_string()),
            priority_score: 78.0,
            confidence_score: Some(0.85),
            payload_json: Some(json!({
                "interpretive_hint": "hint",
                "semantic_tags": ["aspect", "conjunction"],
                "source_weight": 1.35,
                "aggregation_group": "aspect:conjunction",
                "writing_guidance": "guidance",
                "evidence": {"fact_type": "aspect"}
            })),
        },
    ];

    let payload = build_basic_payload(
        42,
        &input(),
        &[capricorn_house_2_position(1, "sun", "Sun")],
        &signals,
    );
    let cluster_plan = payload
        .reading_plan
        .iter()
        .find(|item| item.slot == "dominant_cluster")
        .expect("expected dominant cluster plan item");

    assert_eq!(
        cluster_plan.source_signal_keys,
        vec!["cluster:capricorn:house_2"]
    );
    assert_eq!(
        cluster_plan.primary_signal_keys,
        vec!["cluster:capricorn:house_2"]
    );
    assert!(cluster_plan
        .secondary_slot_candidates
        .iter()
        .any(|candidate| {
            candidate.signal_key == "object_position:sun"
                && candidate.primary_slot == "core_identity"
                && candidate.candidate_slot == "dominant_cluster"
        }));
    assert!(payload
        .reading_plan
        .iter()
        .any(|item| item.slot == "main_tension_or_support"));

    let cluster_drafting = payload
        .drafting_plan
        .iter()
        .find(|item| item.slot == "dominant_cluster")
        .expect("expected dominant cluster drafting item");
    assert_eq!(
        cluster_drafting.source_signal_keys,
        cluster_plan.source_signal_keys
    );
    assert_eq!(
        cluster_drafting.primary_signal_keys,
        cluster_plan.primary_signal_keys
    );
    assert_eq!(
        cluster_drafting.secondary_slot_candidates,
        cluster_plan.secondary_slot_candidates
    );
    assert_eq!(
        cluster_drafting.section_title,
        "A Capricorn dominant theme around Resources"
    );
    assert_eq!(
        cluster_drafting.emphasis_refs.dominant_signs,
        vec!["capricorn"]
    );
    assert_eq!(cluster_drafting.emphasis_refs.dominant_houses, vec![2]);
    assert!(cluster_drafting
        .emphasis_refs
        .dominant_objects
        .contains(&"sun".to_string()));
    let core_drafting = payload
        .drafting_plan
        .iter()
        .find(|item| item.slot == "core_identity")
        .expect("expected core identity drafting item");
    assert!(core_drafting.emphasis_refs.dominant_signs.is_empty());
    assert!(core_drafting.emphasis_refs.dominant_houses.is_empty());
    assert!(core_drafting.emphasis_refs.dominant_objects.is_empty());
    assert!(cluster_drafting
        .avoid
        .contains(&"repeat each placement one by one".to_string()));
}

#[test]
fn basic_payload_exposes_chart_emphasis_summary() {
    let signals = vec![
        InterpretationSignalRow {
            id: 1,
            signal_key: "cluster:capricorn:house_2".to_string(),
            theme_code: Some("resources".to_string()),
            title: "Strong concentration in Capricorn, house 2".to_string(),
            summary: Some("summary".to_string()),
            priority_score: 99.0,
            confidence_score: Some(0.9),
            payload_json: Some(json!({
                "interpretive_hint": "hint",
                "semantic_tags": ["cluster", "capricorn", "house_2", "resources"],
                "source_weight": 2.35,
                "aggregation_group": "capricorn_house_2_cluster",
                "writing_guidance": "guidance",
                "evidence": {
                    "fact_type": "position_cluster",
                    "cluster_type": "sign_house",
                    "sign_code": "capricorn",
                    "sign_name": "Capricorn",
                    "house_number": 2,
                    "house_name": "Resources",
                    "source_signals": [
                        "object_position:sun",
                        "object_position:saturn",
                        "object_position:mars"
                    ],
                    "source_objects": ["sun", "saturn", "mars"]
                }
            })),
        },
        placement_signal_row(2, "object_position:sun", "sun"),
        placement_signal_row(3, "object_position:saturn", "saturn"),
        dignity_signal_row(4, "dignity:saturn:domicile:capricorn", "saturn"),
        aspect_signal(5, "aspect:sun:saturn:trine", "trine", 0.82),
    ];
    let positions = vec![
        capricorn_house_2_position(1, "sun", "Sun"),
        capricorn_house_2_position(7, "saturn", "Saturn"),
        capricorn_house_2_position(5, "mars", "Mars"),
    ];

    let payload = build_basic_payload(42, &input(), &positions, &signals);

    let dominant_sign = payload
        .chart_emphasis
        .dominant_signs
        .first()
        .expect("expected dominant sign");
    assert_eq!(dominant_sign.sign_code, "capricorn");
    assert!(dominant_sign.score >= 0.85);
    assert!(dominant_sign.score < 1.0);
    assert!(dominant_sign.reasons.contains(&"sun_in_sign".to_string()));
    assert!(dominant_sign
        .reasons
        .contains(&"saturn_domicile".to_string()));
    assert!(dominant_sign
        .reasons
        .contains(&"sign_house_cluster".to_string()));
    assert!(dominant_sign
        .reasons
        .contains(&"multiple_objects".to_string()));

    let dominant_house = payload
        .chart_emphasis
        .dominant_houses
        .first()
        .expect("expected dominant house");
    assert_eq!(dominant_house.house_number, 2);
    assert_eq!(dominant_house.theme_code, "resources");
    assert!(dominant_house.reasons.contains(&"sun_in_house".to_string()));
    assert!(dominant_house.reasons.contains(&"cluster".to_string()));

    let saturn = payload
        .chart_emphasis
        .dominant_objects
        .iter()
        .find(|entry| entry.object_code == "saturn")
        .expect("expected saturn emphasis");
    assert!(saturn.score > 0.0);
    assert!(saturn.reasons.contains(&"domicile".to_string()));
    assert!(saturn.reasons.contains(&"cluster_participant".to_string()));
    assert!(saturn.reasons.contains(&"capricorn_emphasis".to_string()));
    assert!(saturn
        .reasons
        .contains(&"strong_aspect_participant".to_string()));
}

#[test]
fn chart_emphasis_omits_placement_only_objects_when_stronger_evidence_exists() {
    let signals = vec![
        placement_signal_row(1, "object_position:sun", "sun"),
        placement_signal_row(2, "object_position:moon", "moon"),
        placement_signal_row(3, "object_position:mercury", "mercury"),
        dignity_signal_row(4, "dignity:mercury:domicile:gemini", "mercury"),
    ];
    let positions = vec![
        position(),
        ObjectPositionFact {
            chart_object_id: 2,
            object_code: "moon".to_string(),
            object_name: "Moon".to_string(),
            zodiacal_reference_system_id: 1,
            coordinate_reference_system_id: 1,
            sign_id: 7,
            sign_code: "libra".to_string(),
            sign_name: "Libra".to_string(),
            house_id: Some(1),
            house_number: Some(1),
            house_name: Some("Self".to_string()),
            motion_state_id: Some(1),
            horizon_position_id: None,
            longitude_deg: 180.0,
            latitude_deg: None,
            apparent_speed_deg_per_day: Some(12.0),
            altitude_deg: None,
            is_visible: None,
            facts_json: Some(json!({
                "sign_context": {"element": "air", "modality": "cardinal", "polarity": "yang"},
                "house_modality": {"code": "angular"},
                "object_context": {"role": "luminary"},
                "motion_context": {"motion_state": "direct"}
            })),
        },
        ObjectPositionFact {
            chart_object_id: 3,
            object_code: "mercury".to_string(),
            object_name: "Mercury".to_string(),
            zodiacal_reference_system_id: 1,
            coordinate_reference_system_id: 1,
            sign_id: 3,
            sign_code: "gemini".to_string(),
            sign_name: "Gemini".to_string(),
            house_id: Some(9),
            house_number: Some(9),
            house_name: Some("Beliefs".to_string()),
            motion_state_id: Some(1),
            horizon_position_id: None,
            longitude_deg: 70.0,
            latitude_deg: None,
            apparent_speed_deg_per_day: Some(1.0),
            altitude_deg: None,
            is_visible: None,
            facts_json: Some(json!({
                "sign_context": {"element": "air", "modality": "mutable", "polarity": "yang"},
                "house_modality": {"code": "cadent"},
                "object_context": {"role": "planet"},
                "motion_context": {"motion_state": "direct"}
            })),
        },
    ];

    let payload = build_basic_payload(42, &input(), &positions, &signals);

    assert!(payload
        .chart_emphasis
        .dominant_objects
        .iter()
        .any(|entry| entry.object_code == "mercury"));
    assert!(
        !payload
            .chart_emphasis
            .dominant_objects
            .iter()
            .any(|entry| entry.object_code == "moon"
                && entry.reasons == vec!["placement".to_string()])
    );
}

#[test]
fn drafting_emphasis_refs_scope_objects_to_the_receiving_slot() {
    let signals = vec![
        InterpretationSignalRow {
            id: 1,
            signal_key: "cluster:gemini:house_9".to_string(),
            theme_code: Some("beliefs".to_string()),
            title: "Strong concentration in Gemini, house 9".to_string(),
            summary: Some("summary".to_string()),
            priority_score: 99.0,
            confidence_score: Some(0.9),
            payload_json: Some(json!({
                "interpretive_hint": "hint",
                "semantic_tags": ["cluster", "gemini", "house_9"],
                "source_weight": 2.35,
                "aggregation_group": "gemini_house_9_cluster",
                "writing_guidance": "guidance",
                "evidence": {
                    "fact_type": "position_cluster",
                    "cluster_type": "sign_house",
                    "sign_code": "gemini",
                    "sign_name": "Gemini",
                    "house_number": 9,
                    "house_name": "Beliefs",
                    "source_signals": [
                        "object_position:sun",
                        "object_position:mercury",
                        "object_position:jupiter"
                    ],
                    "source_objects": ["sun", "mercury", "jupiter"]
                }
            })),
        },
        placement_signal_row(2, "object_position:sun", "sun"),
        placement_signal_row(3, "object_position:mercury", "mercury"),
        placement_signal_row(4, "object_position:jupiter", "jupiter"),
        placement_signal_row(5, "object_position:mars", "mars"),
        dignity_signal_row(6, "dignity:mercury:domicile:gemini", "mercury"),
        dignity_signal_row(7, "dignity:mars:detriment:taurus", "mars"),
    ];
    let positions = vec![
        ObjectPositionFact {
            object_code: "sun".to_string(),
            object_name: "Sun".to_string(),
            ..position()
        },
        ObjectPositionFact {
            chart_object_id: 3,
            object_code: "mercury".to_string(),
            object_name: "Mercury".to_string(),
            longitude_deg: 70.0,
            ..position()
        },
        ObjectPositionFact {
            chart_object_id: 6,
            object_code: "jupiter".to_string(),
            object_name: "Jupiter".to_string(),
            longitude_deg: 80.0,
            ..position()
        },
        ObjectPositionFact {
            chart_object_id: 5,
            object_code: "mars".to_string(),
            object_name: "Mars".to_string(),
            sign_id: 2,
            sign_code: "taurus".to_string(),
            sign_name: "Taurus".to_string(),
            house_id: Some(8),
            house_number: Some(8),
            house_name: Some("Transformation".to_string()),
            longitude_deg: 45.0,
            ..position()
        },
    ];

    let payload = build_basic_payload(42, &input(), &positions, &signals);
    let cluster_drafting = payload
        .drafting_plan
        .iter()
        .find(|item| item.slot == "dominant_cluster")
        .expect("expected dominant cluster drafting item");

    assert!(payload
        .chart_emphasis
        .dominant_objects
        .iter()
        .any(|entry| entry.object_code == "mars"));
    assert!(!cluster_drafting
        .emphasis_refs
        .dominant_objects
        .contains(&"mars".to_string()));
}

#[test]
fn chart_emphasis_scores_do_not_overstate_weak_distributions() {
    let signals = vec![placement_signal_row(1, "object_position:sun", "sun")];
    let payload = build_basic_payload(42, &input(), &[position()], &signals);

    let dominant_sign = payload
        .chart_emphasis
        .dominant_signs
        .first()
        .expect("expected fallback dominant sign");
    let dominant_house = payload
        .chart_emphasis
        .dominant_houses
        .first()
        .expect("expected fallback dominant house");
    let dominant_object = payload
        .chart_emphasis
        .dominant_objects
        .first()
        .expect("expected fallback dominant object");

    assert_eq!(dominant_sign.sign_code, "gemini");
    assert_eq!(dominant_house.house_number, 9);
    assert_eq!(dominant_object.object_code, "sun");
    assert!(dominant_sign.score < 0.35);
    assert!(dominant_house.score < 0.35);
    assert!(dominant_object.score < 0.5);
    assert_eq!(dominant_object.reasons, vec!["placement"]);
}

#[test]
fn basic_payload_exposes_structured_dignities() {
    let signal = InterpretationSignalRow {
        id: 1,
        signal_key: "dignity:saturn:domicile:capricorn".to_string(),
        theme_code: Some("functional_strength".to_string()),
        title: "Saturn strongly placed in Capricorn".to_string(),
        summary: Some("summary".to_string()),
        priority_score: 88.0,
        confidence_score: Some(0.95),
        payload_json: Some(json!({
            "interpretive_hint": "hint",
            "semantic_tags": ["dignity", "saturn", "capricorn", "domicile"],
            "source_weight": 0.75,
            "aggregation_group": "dignity:saturn",
            "writing_guidance": "guidance",
            "evidence": {
                "fact_type": "essential_dignity",
                "chart_object": "saturn",
                "sign_code": "capricorn",
                "dignity_type": "domicile"
            }
        })),
    };

    let position = saturn_capricorn_position();
    let payload = build_basic_payload(42, &input(), &[position], &[signal]);

    assert_eq!(payload.dignities.len(), 1);
    assert_eq!(payload.dignities[0].object_code, "saturn");
    assert_eq!(payload.dignities[0].dignity_type, "domicile");
    assert_eq!(
        payload.dignities[0].signal_key.as_deref(),
        Some("dignity:saturn:domicile:capricorn")
    );
    assert_eq!(
        payload.positions[0]
            .dignity_context
            .as_array()
            .and_then(|context| context.first())
            .and_then(|context| context.get("dignity_type"))
            .and_then(|value| value.as_str()),
        Some("domicile")
    );
}

#[test]
fn reading_plan_uses_active_dignity_signals() {
    let signals = vec![
        InterpretationSignalRow {
            id: 1,
            signal_key: "cluster:capricorn:house_2".to_string(),
            theme_code: Some("resources".to_string()),
            title: "Strong concentration in Capricorn, house 2".to_string(),
            summary: Some("summary".to_string()),
            priority_score: 99.0,
            confidence_score: Some(0.9),
            payload_json: Some(json!({
                "interpretive_hint": "hint",
                "semantic_tags": ["cluster", "capricorn", "house_2"],
                "source_weight": 2.0,
                "aggregation_group": "capricorn_house_2_cluster",
                "writing_guidance": "guidance",
                "evidence": {
                    "fact_type": "position_cluster",
                    "sign_name": "Capricorn",
                    "house_name": "Resources",
                    "source_signals": ["object_position:sun", "object_position:saturn"],
                    "source_objects": ["sun", "saturn"]
                }
            })),
        },
        InterpretationSignalRow {
            id: 2,
            signal_key: "object_position:sun".to_string(),
            theme_code: Some("resources".to_string()),
            title: "Sun in Capricorn, house 2".to_string(),
            summary: Some("summary".to_string()),
            priority_score: 100.0,
            confidence_score: Some(0.95),
            payload_json: Some(json!({
                "interpretive_hint": "hint",
                "semantic_tags": ["placement", "sun"],
                "source_weight": 1.0,
                "aggregation_group": "capricorn:house_2",
                "writing_guidance": "guidance",
                "evidence": {"fact_type": "object_position", "object_code": "sun"}
            })),
        },
        dignity_signal_row(3, "dignity:saturn:domicile:capricorn", "saturn"),
        InterpretationSignalRow {
            id: 4,
            signal_key: "object_position:jupiter".to_string(),
            theme_code: Some("shared_resources".to_string()),
            title: "Jupiter in Cancer, house 8".to_string(),
            summary: Some("summary".to_string()),
            priority_score: 81.75,
            confidence_score: Some(0.95),
            payload_json: Some(json!({
                "interpretive_hint": "hint",
                "semantic_tags": ["placement", "jupiter"],
                "source_weight": 0.75,
                "aggregation_group": "cancer:house_8",
                "writing_guidance": "guidance",
                "evidence": {"fact_type": "object_position", "object_code": "jupiter"}
            })),
        },
        dignity_signal_row(5, "dignity:jupiter:exaltation:cancer", "jupiter"),
    ];

    let payload = build_basic_payload(42, &input(), &[position()], &signals);
    let cluster_plan = payload
        .reading_plan
        .iter()
        .find(|item| item.slot == "dominant_cluster")
        .expect("expected cluster plan");
    let background_plan = payload
        .reading_plan
        .iter()
        .find(|item| item.slot == "background_factors")
        .expect("expected background plan");

    assert!(cluster_plan
        .source_signal_keys
        .contains(&"dignity:saturn:domicile:capricorn".to_string()));
    assert!(!background_plan
        .source_signal_keys
        .contains(&"dignity:saturn:domicile:capricorn".to_string()));
    assert!(background_plan
        .secondary_slot_candidates
        .iter()
        .any(|candidate| {
            candidate.signal_key == "dignity:saturn:domicile:capricorn"
                && candidate.primary_slot == "dominant_cluster"
                && candidate.candidate_slot == "background_factors"
        }));
    assert!(background_plan
        .source_signal_keys
        .contains(&"dignity:jupiter:exaltation:cancer".to_string()));

    let background_drafting = payload
        .drafting_plan
        .iter()
        .find(|item| item.slot == "background_factors")
        .expect("expected background drafting plan");
    assert_eq!(
        background_drafting.secondary_slot_candidates,
        background_plan.secondary_slot_candidates
    );
}

#[test]
fn reading_plan_drops_slots_that_only_have_secondary_candidates() {
    let signals = vec![
        InterpretationSignalRow {
            id: 1,
            signal_key: "cluster:capricorn:house_2".to_string(),
            theme_code: Some("resources".to_string()),
            title: "Strong concentration in Capricorn, house 2".to_string(),
            summary: Some("summary".to_string()),
            priority_score: 99.0,
            confidence_score: Some(0.9),
            payload_json: Some(json!({
                "interpretive_hint": "hint",
                "semantic_tags": ["cluster", "capricorn", "house_2"],
                "source_weight": 2.0,
                "aggregation_group": "capricorn_house_2_cluster",
                "writing_guidance": "guidance",
                "evidence": {
                    "fact_type": "position_cluster",
                    "sign_name": "Capricorn",
                    "house_name": "Resources",
                    "source_signals": ["object_position:saturn"],
                    "source_objects": ["saturn"]
                }
            })),
        },
        dignity_signal_row(2, "dignity:saturn:domicile:capricorn", "saturn"),
    ];

    let payload = build_basic_payload(42, &input(), &[saturn_capricorn_position()], &signals);

    assert!(payload
        .reading_plan
        .iter()
        .any(|item| item.slot == "dominant_cluster"));
    assert!(!payload
        .reading_plan
        .iter()
        .any(|item| item.slot == "background_factors"));
    assert!(!payload
        .drafting_plan
        .iter()
        .any(|item| item.slot == "background_factors"));
}

#[test]
fn reading_plan_object_limits_do_not_count_dignity_sources() {
    let signals = vec![
        placement_signal_row(1, "object_position:mercury", "mercury"),
        dignity_signal_row(2, "dignity:mercury:domicile:virgo", "mercury"),
        dignity_signal_row(3, "dignity:mercury:exaltation:virgo", "mercury"),
        placement_signal_row(4, "object_position:venus", "venus"),
        placement_signal_row(5, "object_position:mars", "mars"),
    ];

    let payload = build_basic_payload(42, &input(), &[position()], &signals);
    let expression_plan = payload
        .reading_plan
        .iter()
        .find(|item| item.slot == "expression_style")
        .expect("expected expression style plan");

    assert!(expression_plan
        .source_signal_keys
        .contains(&"object_position:mercury".to_string()));
    assert!(expression_plan
        .source_signal_keys
        .contains(&"dignity:mercury:domicile:virgo".to_string()));
    assert!(expression_plan
        .source_signal_keys
        .contains(&"dignity:mercury:exaltation:virgo".to_string()));
    assert!(expression_plan
        .source_signal_keys
        .contains(&"object_position:venus".to_string()));
    assert!(expression_plan
        .source_signal_keys
        .contains(&"object_position:mars".to_string()));
}

#[test]
fn main_dynamic_aspects_include_strong_tension_when_available() {
    let signals = vec![
        aspect_signal(1, "aspect:moon:neptune:sextile", "sextile", 0.95),
        aspect_signal(2, "aspect:sun:moon:sextile", "sextile", 0.93),
        aspect_signal(3, "aspect:sun:neptune:conjunction", "conjunction", 0.9),
        aspect_signal(4, "aspect:moon:mars:square", "square", 0.88),
    ];

    let payload = build_basic_payload(42, &input(), &[position()], &signals);
    let aspect_plan = payload
        .reading_plan
        .iter()
        .find(|item| item.slot == "main_tension_or_support")
        .expect("expected aspect plan");

    assert_eq!(aspect_plan.source_signal_keys.len(), 3);
    assert!(aspect_plan
        .source_signal_keys
        .contains(&"aspect:moon:mars:square".to_string()));
}

#[test]
fn main_dynamic_aspects_balance_support_and_tension_by_valence() {
    let signals = vec![
        aspect_signal(1, "aspect:sun:neptune:conjunction", "conjunction", 0.99),
        aspect_signal(2, "aspect:moon:pluto:conjunction", "conjunction", 0.98),
        aspect_signal(3, "aspect:mars:saturn:conjunction", "conjunction", 0.97),
        aspect_signal(4, "aspect:moon:mars:square", "square", 0.86),
        aspect_signal(5, "aspect:venus:jupiter:sextile", "sextile", 0.84),
    ];

    let payload = build_basic_payload(42, &input(), &[position()], &signals);
    let aspect_plan = payload
        .reading_plan
        .iter()
        .find(|item| item.slot == "main_tension_or_support")
        .expect("expected aspect plan");

    assert_eq!(aspect_plan.source_signal_keys.len(), 3);
    assert!(aspect_plan
        .source_signal_keys
        .contains(&"aspect:moon:mars:square".to_string()));
    assert!(aspect_plan
        .source_signal_keys
        .contains(&"aspect:venus:jupiter:sextile".to_string()));
}

#[test]
fn structural_axis_aspects_are_excluded_from_payload_planning_and_emphasis() {
    let mut structural_axis = aspect_signal(
        1,
        "aspect:ascendant:descendant:opposition",
        "opposition",
        1.0,
    );
    structural_axis.payload_json = Some(json!({
        "interpretive_hint": "hint",
        "semantic_tags": ["aspect", "opposition", "axis"],
        "source_weight": 2.0,
        "aggregation_group": "aspect:opposition",
        "writing_guidance": "Use as background axis context, not as a main tension aspect.",
        "aspect_context": {
            "aspect_family": "major",
            "primary_valence": "polarizing",
            "dynamic_quality": "tension",
            "phase_state": "exact",
            "is_structural_axis": true,
            "writing_guidance": "guidance"
        },
        "evidence": {
            "fact_type": "aspect",
            "source_object_code": "ascendant",
            "target_object_code": "descendant",
            "aspect_code": "opposition",
            "aspect_name": "opposition",
            "strength_score": 1.0,
            "is_structural_axis": true
        }
    }));
    let square = aspect_signal(2, "aspect:moon:mars:square", "square", 0.88);

    let payload = build_basic_payload(42, &input(), &[position()], &[structural_axis, square]);
    let aspect_plan = payload
        .reading_plan
        .iter()
        .find(|item| item.slot == "main_tension_or_support")
        .expect("expected aspect plan");

    assert_eq!(
        aspect_plan.source_signal_keys,
        vec!["aspect:moon:mars:square"]
    );
    assert!(
        !payload.chart_emphasis.dominant_objects.iter().any(|entry| {
            entry.object_code == "ascendant"
                && entry
                    .reasons
                    .contains(&"strong_aspect_participant".to_string())
        })
    );
}

#[test]
fn legacy_unflagged_axis_aspects_are_excluded_when_angle_positions_define_axis() {
    let structural_axis = aspect_signal(
        1,
        "aspect:ascendant:descendant:opposition",
        "opposition",
        1.0,
    );
    let square = aspect_signal(2, "aspect:moon:mars:square", "square", 0.88);
    let positions = vec![
        angle_position(
            11,
            "ascendant",
            "Ascendant",
            "asc",
            "dsc",
            "horizontal",
            15.0,
        ),
        angle_position(
            12,
            "descendant",
            "Descendant",
            "dsc",
            "asc",
            "horizontal",
            195.0,
        ),
        position(),
    ];

    let payload = build_basic_payload(42, &input(), &positions, &[structural_axis, square]);

    assert!(!payload
        .signals
        .iter()
        .any(|signal| signal.signal_key == "aspect:ascendant:descendant:opposition"));
    let aspect_plan = payload
        .reading_plan
        .iter()
        .find(|item| item.slot == "main_tension_or_support")
        .expect("expected aspect plan");
    assert_eq!(
        aspect_plan.source_signal_keys,
        vec!["aspect:moon:mars:square"]
    );
}

#[test]
fn angle_to_angle_aspects_are_excluded_from_payload() {
    let angle_square = aspect_signal(1, "aspect:descendant:ic:square", "square", 0.99);
    let planet_square = aspect_signal(2, "aspect:moon:mars:square", "square", 0.88);
    let positions = vec![
        angle_position(
            11,
            "descendant",
            "Descendant",
            "dsc",
            "asc",
            "horizontal",
            195.0,
        ),
        angle_position(12, "ic", "IC", "ic", "mc", "vertical", 285.0),
        position(),
    ];

    let payload = build_basic_payload(42, &input(), &positions, &[angle_square, planet_square]);

    assert!(!payload
        .signals
        .iter()
        .any(|signal| signal.signal_key == "aspect:descendant:ic:square"));
    let aspect_plan = payload
        .reading_plan
        .iter()
        .find(|item| item.slot == "main_tension_or_support")
        .expect("expected aspect plan");
    assert_eq!(
        aspect_plan.source_signal_keys,
        vec!["aspect:moon:mars:square"]
    );
}

fn aspect_signal(
    id: i32,
    signal_key: &str,
    aspect_code: &str,
    strength_score: f64,
) -> InterpretationSignalRow {
    InterpretationSignalRow {
        id,
        signal_key: signal_key.to_string(),
        theme_code: Some("aspect".to_string()),
        title: format!("Aspect {aspect_code}"),
        summary: Some(format!(
            "Two chart factors form a {aspect_code} with a controlled summary."
        )),
        priority_score: strength_score * 80.0,
        confidence_score: Some(0.85),
        payload_json: Some(json!({
            "interpretive_hint": "hint",
            "semantic_tags": ["aspect", aspect_code],
            "source_weight": 1.0,
            "aggregation_group": format!("aspect:{aspect_code}"),
            "writing_guidance": "guidance",
            "aspect_context": {
                "aspect_family": "major",
                "primary_valence": primary_valence_for_test(aspect_code),
                "intensity_modifier": intensity_modifier_for_test(aspect_code),
                "secondary_effect": null,
                "dynamic_quality": dynamic_quality_for_test(aspect_code),
                "phase_state": "applying",
                "writing_guidance": "guidance"
            },
            "evidence": {
                "fact_type": "aspect",
                "aspect_code": aspect_code,
                "aspect_name": aspect_code,
                "strength_score": strength_score
            }
        })),
    }
}

fn primary_valence_for_test(aspect_code: &str) -> Option<&'static str> {
    match aspect_code {
        "sextile" => Some("supportive"),
        "square" => Some("dynamic_challenging"),
        "trine" => Some("harmonious"),
        "opposition" => Some("polarizing"),
        _ => None,
    }
}

fn intensity_modifier_for_test(aspect_code: &str) -> Option<&'static str> {
    match aspect_code {
        "conjunction" => Some("amplifying"),
        _ => None,
    }
}

fn dynamic_quality_for_test(aspect_code: &str) -> &'static str {
    match aspect_code {
        "sextile" | "trine" => "flow",
        "square" | "opposition" => "tension",
        "conjunction" => "intensification",
        _ => "contextual",
    }
}

fn dignity_signal_row(id: i32, signal_key: &str, object_code: &str) -> InterpretationSignalRow {
    InterpretationSignalRow {
        id,
        signal_key: signal_key.to_string(),
        theme_code: Some("functional_strength".to_string()),
        title: format!("{object_code} dignity"),
        summary: Some("summary".to_string()),
        priority_score: 86.0,
        confidence_score: Some(0.95),
        payload_json: Some(json!({
            "interpretive_hint": "hint",
            "semantic_tags": ["dignity", object_code],
            "source_weight": 0.75,
            "aggregation_group": format!("dignity:{object_code}"),
            "writing_guidance": "guidance",
            "evidence": {
                "fact_type": "essential_dignity",
                "chart_object": object_code
            }
        })),
    }
}

fn placement_signal_row(id: i32, signal_key: &str, object_code: &str) -> InterpretationSignalRow {
    InterpretationSignalRow {
        id,
        signal_key: signal_key.to_string(),
        theme_code: Some("object_position".to_string()),
        title: format!("{object_code} placement"),
        summary: Some("summary".to_string()),
        priority_score: 85.0,
        confidence_score: Some(0.95),
        payload_json: Some(json!({
            "interpretive_hint": "hint",
            "semantic_tags": ["placement", object_code],
            "source_weight": 0.75,
            "aggregation_group": object_code,
            "writing_guidance": "guidance",
            "evidence": {
                "fact_type": "object_position",
                "object_code": object_code
            }
        })),
    }
}
