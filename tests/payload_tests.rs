mod common;

use chrono::{TimeZone, Utc};
use serde_json::json;

use astral_calculator::domain::*;
use astral_calculator::features::payload as payload_mod;
use common::natal_catalog::test_catalog;

fn build_basic_payload(
    chart_calculation_id: i32,
    input: &NatalChartInput,
    positions: &[ObjectPositionFact],
    signals: &[InterpretationSignalRow],
) -> BasicPayload {
    payload_mod::build_basic_payload(
        chart_calculation_id,
        input,
        positions,
        signals,
        &test_catalog(),
    )
}

fn build_basic_payload_with_rulership(
    chart_calculation_id: i32,
    input: &NatalChartInput,
    positions: &[ObjectPositionFact],
    signals: &[InterpretationSignalRow],
    domicile_rulers: &[DomicileRulerReference],
) -> BasicPayload {
    payload_mod::build_basic_payload_with_rulership(
        chart_calculation_id,
        input,
        positions,
        signals,
        domicile_rulers,
        &test_catalog(),
    )
}

fn build_basic_payload_with_references(
    chart_calculation_id: i32,
    input: &NatalChartInput,
    positions: &[ObjectPositionFact],
    signals: &[InterpretationSignalRow],
    domicile_rulers: &[DomicileRulerReference],
    house_axes: &[HouseAxisReference],
) -> BasicPayload {
    payload_mod::build_basic_payload_with_references(
        chart_calculation_id,
        input,
        positions,
        signals,
        domicile_rulers,
        house_axes,
        &test_catalog(),
    )
}

fn build_basic_payload_with_all_references(
    chart_calculation_id: i32,
    input: &NatalChartInput,
    positions: &[ObjectPositionFact],
    signals: &[InterpretationSignalRow],
    domicile_rulers: &[DomicileRulerReference],
    house_axes: &[HouseAxisReference],
    lunar_phases: &[LunarPhaseReference],
) -> BasicPayload {
    payload_mod::build_basic_payload_with_all_references(
        chart_calculation_id,
        input,
        positions,
        signals,
        domicile_rulers,
        house_axes,
        lunar_phases,
        &test_catalog(),
    )
}

fn build_basic_payload_with_accidental_references(
    chart_calculation_id: i32,
    input: &NatalChartInput,
    positions: &[ObjectPositionFact],
    signals: &[InterpretationSignalRow],
    domicile_rulers: &[DomicileRulerReference],
    house_axes: &[HouseAxisReference],
    lunar_phases: &[LunarPhaseReference],
    accidental_conditions: &[AccidentalDignityConditionReference],
    sect_affinities: &[ObjectSectAffinityReference],
) -> BasicPayload {
    payload_mod::build_basic_payload_with_accidental_references(
        chart_calculation_id,
        input,
        positions,
        signals,
        domicile_rulers,
        house_axes,
        lunar_phases,
        accidental_conditions,
        sect_affinities,
        &test_catalog(),
    )
}

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
        client_idempotency_key: None,
    }
}

fn reason_codes(reasons: &[BasicProjectionReason]) -> Vec<&str> {
    reasons
        .iter()
        .map(|reason| reason.reason_code.as_str())
        .collect()
}

fn with_signal_scoring(mut position: ObjectPositionFact) -> ObjectPositionFact {
    let scoring = match position.object_code.as_str() {
        "sun" | "moon" => json!({
            "position_priority_base": 100.0,
            "angle_priority_base": null,
            "source_weight": 1.0
        }),
        "mercury" | "venus" | "mars" => json!({
            "position_priority_base": 85.0,
            "angle_priority_base": null,
            "source_weight": 0.75
        }),
        "jupiter" | "saturn" => json!({
            "position_priority_base": 75.0,
            "angle_priority_base": null,
            "source_weight": 0.6
        }),
        "ascendant" => json!({
            "position_priority_base": 99.0,
            "angle_priority_base": 99.0,
            "source_weight": 1.0
        }),
        "descendant" | "ic" => json!({
            "position_priority_base": 68.0,
            "angle_priority_base": 68.0,
            "source_weight": 0.4
        }),
        "mc" => json!({
            "position_priority_base": 82.0,
            "angle_priority_base": 82.0,
            "source_weight": 0.8
        }),
        _ => json!({
            "position_priority_base": 60.0,
            "angle_priority_base": null,
            "source_weight": 0.35
        }),
    };

    let modality_delta = position
        .facts_json
        .as_ref()
        .and_then(|facts| facts.get("house_modality"))
        .and_then(|modality| modality.get("code"))
        .and_then(|code| code.as_str())
        .map(|code| match code {
            "angular" => 2.0,
            "succedent" => 0.75,
            "cadent" => -0.75,
            _ => 0.0,
        });

    let facts = position.facts_json.get_or_insert_with(|| json!({}));
    if let Some(root) = facts.as_object_mut() {
        let object_context = root
            .entry("object_context".to_string())
            .or_insert_with(|| json!({}));
        if let Some(object_context) = object_context.as_object_mut() {
            object_context.insert("signal_scoring".to_string(), scoring);
        }
        if let Some(delta) = modality_delta {
            if let Some(modality) = root
                .get_mut("house_modality")
                .and_then(|modality| modality.as_object_mut())
            {
                modality.insert("priority_delta".to_string(), json!(delta));
            }
        }
    }

    position
}

fn position() -> ObjectPositionFact {
    with_signal_scoring(ObjectPositionFact {
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
    })
}

fn saturn_capricorn_position() -> ObjectPositionFact {
    with_signal_scoring(ObjectPositionFact {
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
    })
}

fn capricorn_house_2_position(
    chart_object_id: i32,
    object_code: &str,
    object_name: &str,
) -> ObjectPositionFact {
    with_signal_scoring(ObjectPositionFact {
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
    })
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
    with_signal_scoring(ObjectPositionFact {
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
    })
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
    let payload_json = serde_json::to_value(&payload).expect("payload should serialize");
    assert!(payload_json.get("llm_handoff_contract").is_none());
    assert!(payload_json.get("drafting_plan").is_none());
    assert!(payload_json.get("lunar_phase_context").is_none());
    assert!(payload_json["signals"][0].get("writing_guidance").is_none());
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
    assert_eq!(payload.chart_context.chart_type, "natal");
    assert_eq!(
        payload.chart_context.payload_contract.contract_version,
        "natal_structured_v11"
    );
    assert_eq!(
        payload.chart_context.hemisphere_emphasis.count_scope,
        "mobile_chart_objects_only"
    );
    assert_eq!(
        payload.chart_context.sect.chart_sect.as_deref(),
        Some("day")
    );
    assert_eq!(
        payload.chart_context.sect.sun_horizon_position.as_deref(),
        Some("above_horizon")
    );
    assert_eq!(
        payload.positions[0]
            .visibility_context
            .get("horizon_position")
            .and_then(|value| value.as_str()),
        Some("above_horizon")
    );
}

#[test]
fn chart_context_uses_calculated_altitude_for_sun_sect_boundary() {
    let mut sun = position();
    sun.altitude_deg = Some(0.0);
    sun.is_visible = Some(true);

    let payload = build_basic_payload(42, &input(), &[sun], &[]);

    assert_eq!(
        payload.chart_context.sect.chart_sect.as_deref(),
        Some("all")
    );
    assert_eq!(
        payload.chart_context.sect.sun_horizon_position.as_deref(),
        Some("on_horizon")
    );
    assert_eq!(
        payload.chart_context.sect.source.as_deref(),
        Some("calculated_altitude")
    );
    assert_eq!(
        payload.positions[0]
            .visibility_context
            .get("horizon_position")
            .and_then(|value| value.as_str()),
        Some("on_horizon")
    );
    assert_eq!(
        payload.positions[0]
            .visibility_context
            .get("source")
            .and_then(|value| value.as_str()),
        Some("calculated_altitude")
    );
}

#[test]
fn chart_context_prefers_calculated_altitude_over_legacy_visibility_context() {
    let mut sun = position();
    sun.altitude_deg = Some(12.5);
    sun.is_visible = Some(true);
    sun.facts_json
        .as_mut()
        .and_then(|facts| facts.as_object_mut())
        .expect("facts object")
        .insert(
            "visibility_context".to_string(),
            json!({
                "horizon_position": "below_horizon",
                "source": "house_hemisphere_projection"
            }),
        );

    let payload = build_basic_payload(42, &input(), &[sun], &[]);

    assert_eq!(
        payload.chart_context.sect.chart_sect.as_deref(),
        Some("day")
    );
    assert_eq!(
        payload.chart_context.sect.source.as_deref(),
        Some("calculated_altitude")
    );
    assert_eq!(
        payload.positions[0]
            .visibility_context
            .get("horizon_position")
            .and_then(|value| value.as_str()),
        Some("above_horizon")
    );
    assert_eq!(
        payload.positions[0]
            .visibility_context
            .get("source")
            .and_then(|value| value.as_str()),
        Some("calculated_altitude")
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

    let payload_json = serde_json::to_value(&payload).expect("payload should serialize");
    assert!(payload_json.get("drafting_plan").is_none());
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
    assert!(dominant_sign.reason_details.iter().any(|reason| {
        reason.reason_code == "object_in_sign" && reason.object_code.as_deref() == Some("sun")
    }));
    assert!(dominant_sign.reason_details.iter().any(|reason| {
        reason.reason_code == "essential_dignity"
            && reason.object_code.as_deref() == Some("saturn")
            && reason.dignity_type.as_deref() == Some("domicile")
    }));
    assert!(reason_codes(&dominant_sign.reason_details).contains(&"sign_house_cluster"));
    assert!(reason_codes(&dominant_sign.reason_details).contains(&"multiple_objects"));

    let dominant_house = payload
        .chart_emphasis
        .dominant_houses
        .first()
        .expect("expected dominant house");
    assert_eq!(dominant_house.house_number, 2);
    assert_eq!(dominant_house.theme_code, "resources");
    assert!(dominant_house.reason_details.iter().any(|reason| {
        reason.reason_code == "object_in_house" && reason.object_code.as_deref() == Some("sun")
    }));
    assert!(reason_codes(&dominant_house.reason_details).contains(&"cluster"));

    let saturn = payload
        .chart_emphasis
        .dominant_objects
        .iter()
        .find(|entry| entry.object_code == "saturn")
        .expect("expected saturn emphasis");
    assert!(saturn.score > 0.0);
    assert!(saturn.reason_details.iter().any(|reason| {
        reason.reason_code == "essential_dignity"
            && reason.object_code.as_deref() == Some("saturn")
            && reason.dignity_type.as_deref() == Some("domicile")
    }));
    assert!(reason_codes(&saturn.reason_details).contains(&"cluster_participant"));
    assert!(saturn.reason_details.iter().any(|reason| {
        reason.reason_code == "sign_emphasis" && reason.sign_code.as_deref() == Some("capricorn")
    }));
    assert!(reason_codes(&saturn.reason_details).contains(&"strong_aspect_participant"));
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
        with_signal_scoring(ObjectPositionFact {
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
        }),
        with_signal_scoring(ObjectPositionFact {
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
        }),
    ];

    let payload = build_basic_payload(42, &input(), &positions, &signals);

    assert!(payload
        .chart_emphasis
        .dominant_objects
        .iter()
        .any(|entry| entry.object_code == "mercury"));
    assert!(!payload
        .chart_emphasis
        .dominant_objects
        .iter()
        .any(|entry| entry.object_code == "moon"
            && reason_codes(&entry.reason_details) == vec!["placement"]));
}

#[test]
fn chart_emphasis_keeps_cluster_scope_distinct_from_other_objects() {
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
        with_signal_scoring(ObjectPositionFact {
            object_code: "sun".to_string(),
            object_name: "Sun".to_string(),
            ..position()
        }),
        with_signal_scoring(ObjectPositionFact {
            chart_object_id: 3,
            object_code: "mercury".to_string(),
            object_name: "Mercury".to_string(),
            longitude_deg: 70.0,
            ..position()
        }),
        with_signal_scoring(ObjectPositionFact {
            chart_object_id: 6,
            object_code: "jupiter".to_string(),
            object_name: "Jupiter".to_string(),
            longitude_deg: 80.0,
            ..position()
        }),
        with_signal_scoring(ObjectPositionFact {
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
        }),
    ];

    let payload = build_basic_payload(42, &input(), &positions, &signals);
    assert!(payload
        .chart_emphasis
        .dominant_objects
        .iter()
        .any(|entry| entry.object_code == "mars"));
    assert!(payload
        .reading_plan
        .iter()
        .any(|item| item.slot == "dominant_cluster"));
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
    assert_eq!(
        reason_codes(&dominant_object.reason_details),
        vec!["placement"]
    );
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
fn basic_payload_exposes_rulership_context_from_reference_rules() {
    let mut ascendant = angle_position(
        11,
        "ascendant",
        "Ascendant",
        "ascendant",
        "descendant",
        "horizontal",
        222.0,
    );
    ascendant.sign_id = 8;
    ascendant.sign_code = "scorpio".to_string();
    ascendant.sign_name = "Scorpio".to_string();

    let mut mc = angle_position(12, "mc", "Midheaven", "mc", "ic", "vertical", 125.0);
    mc.sign_id = 5;
    mc.sign_code = "leo".to_string();
    mc.sign_name = "Leo".to_string();
    mc.house_number = Some(10);

    let mut descendant = angle_position(
        13,
        "descendant",
        "Descendant",
        "descendant",
        "ascendant",
        "horizontal",
        42.0,
    );
    descendant.sign_id = 2;
    descendant.sign_code = "taurus".to_string();
    descendant.sign_name = "Taurus".to_string();
    descendant.house_number = Some(7);

    let mut venus = capricorn_house_2_position(3, "venus", "Venus");
    venus.sign_id = 11;
    venus.sign_code = "aquarius".to_string();
    venus.sign_name = "Aquarius".to_string();
    venus.house_id = Some(3);
    venus.house_number = Some(3);
    venus.house_name = Some("Communication".to_string());

    let mut mars = capricorn_house_2_position(5, "mars", "Mars");
    mars.sign_id = 9;
    mars.sign_code = "sagittarius".to_string();
    mars.sign_name = "Sagittarius".to_string();
    mars.house_id = Some(1);
    mars.house_number = Some(1);
    mars.house_name = Some("Self".to_string());

    let sun = capricorn_house_2_position(1, "sun", "Sun");
    let signals = vec![
        placement_signal_row(1, "object_position:mars", "mars"),
        placement_signal_row(2, "object_position:sun", "sun"),
        placement_signal_row(4, "object_position:venus", "venus"),
        InterpretationSignalRow {
            id: 3,
            signal_key: "angle:mc:sign:leo".to_string(),
            theme_code: Some("public_direction".to_string()),
            title: "MC in Leo".to_string(),
            summary: Some("summary".to_string()),
            priority_score: 82.0,
            confidence_score: Some(0.95),
            payload_json: Some(json!({
                "interpretive_hint": "hint",
                "semantic_tags": ["angle", "mc", "leo"],
                "source_weight": 0.8,
                "aggregation_group": "angle:mc:leo",
                "evidence": {
                    "fact_type": "chart_angle",
                    "angle_code": "mc",
                    "opposite_angle_code": "ic",
                    "opposite_angle_object_code": "ic",
                    "sign_code": "leo"
                }
            })),
        },
    ];
    let rulers = vec![
        domicile_ruler(8, "scorpio", "Scorpio", 5, "mars", "Mars"),
        modern_domicile_ruler(8, "scorpio", "Scorpio", 10, "pluto", "Pluto"),
        domicile_ruler(5, "leo", "Leo", 1, "sun", "Sun"),
        domicile_ruler(2, "taurus", "Taurus", 3, "venus", "Venus"),
        domicile_ruler(10, "capricorn", "Capricorn", 7, "saturn", "Saturn"),
        domicile_ruler(9, "sagittarius", "Sagittarius", 6, "jupiter", "Jupiter"),
    ];

    let payload = build_basic_payload_with_rulership(
        42,
        &input(),
        &[ascendant, mc, descendant, mars, sun, venus],
        &signals,
        &rulers,
    );

    let ascendant_ruler = payload
        .rulership_context
        .ascendant_ruler
        .as_ref()
        .expect("ascendant ruler");
    assert_eq!(ascendant_ruler.sign_code, "scorpio");
    assert_eq!(ascendant_ruler.ruler_object_codes, vec!["mars", "pluto"]);
    assert_eq!(ascendant_ruler.ruler_object_code, "mars");
    assert_eq!(ascendant_ruler.ruler_house_number, Some(1));
    assert_eq!(
        ascendant_ruler.ruler_position_signal_key.as_deref(),
        Some("object_position:mars")
    );

    let mc_ruler = payload
        .rulership_context
        .mc_ruler
        .as_ref()
        .expect("mc ruler");
    assert_eq!(mc_ruler.sign_code, "leo");
    assert_eq!(mc_ruler.ruler_object_codes, vec!["sun"]);
    assert_eq!(mc_ruler.ruler_object_code, "sun");
    assert_eq!(mc_ruler.ruler_house_number, Some(2));

    let descendant_ruler = payload
        .rulership_context
        .descendant_ruler
        .as_ref()
        .expect("descendant ruler");
    assert_eq!(descendant_ruler.sign_code, "taurus");
    assert_eq!(descendant_ruler.ruler_object_codes, vec!["venus"]);
    assert_eq!(descendant_ruler.ruler_object_code, "venus");
    assert_eq!(descendant_ruler.ruler_house_number, Some(3));
    assert_eq!(
        descendant_ruler.ruler_position_signal_key.as_deref(),
        Some("object_position:venus")
    );
    assert!(payload
        .rulership_context
        .dispositor_links
        .iter()
        .any(|link| link.object_code == "sun"
            && link.object_sign_code == "capricorn"
            && link.dispositor_object_code == "saturn"));
    assert!(!payload
        .rulership_context
        .dispositor_links
        .iter()
        .any(|link| matches!(link.object_code.as_str(), "ascendant" | "mc")));
    assert!(payload
        .reading_plan
        .iter()
        .any(|item| item.slot == "core_identity"));
    assert!(payload
        .reading_plan
        .iter()
        .any(|item| item.slot == "background_factors"));
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

    assert!(!background_plan.secondary_slot_candidates.is_empty());
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
    let payload_json = serde_json::to_value(&payload).expect("payload should serialize");
    assert!(payload_json.get("drafting_plan").is_none());
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
        "aspect_context": {
            "aspect_family": "major",
            "primary_valence": "polarizing",
            "dynamic_quality": "tension",
            "phase_state": "exact",
            "is_structural_axis": true,
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
                    .reason_details
                    .iter()
                    .any(|reason| reason.reason_code == "strong_aspect_participant")
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

#[allow(clippy::too_many_arguments)]
fn object_in_house(
    id: i32,
    object_code: &str,
    object_name: &str,
    sign_code: &str,
    sign_name: &str,
    house_number: i32,
    theme_code: &str,
    is_luminary: bool,
) -> ObjectPositionFact {
    with_signal_scoring(ObjectPositionFact {
        chart_object_id: id,
        object_code: object_code.to_string(),
        object_name: object_name.to_string(),
        zodiacal_reference_system_id: 1,
        coordinate_reference_system_id: 1,
        sign_id: house_number,
        sign_code: sign_code.to_string(),
        sign_name: sign_name.to_string(),
        house_id: Some(house_number),
        house_number: Some(house_number),
        house_name: Some(format!("House {house_number}")),
        motion_state_id: Some(1),
        horizon_position_id: None,
        longitude_deg: (house_number * 20) as f64,
        latitude_deg: None,
        apparent_speed_deg_per_day: Some(1.0),
        altitude_deg: None,
        is_visible: None,
        facts_json: Some(json!({
            "sign_context": {
                "element": "water",
                "modality": "fixed",
                "polarity": "yin"
            },
            "house_context": {"theme_code": theme_code},
            "house_modality": {"code": "angular"},
            "object_context": {
                "role": if is_luminary { "luminary" } else { "planet" },
                "is_luminary": is_luminary
            },
            "motion_context": {"motion_state": "direct"}
        })),
    })
}

fn canonical_house_axes() -> Vec<HouseAxisReference> {
    vec![
        house_axis(
            "self_relationship",
            1,
            7,
            "identity",
            "relationships",
            "Self and Relationship",
        ),
        house_axis(
            "resources_sharing",
            2,
            8,
            "resources",
            "shared_resources",
            "Resources and Sharing",
        ),
        house_axis(
            "local_distant",
            3,
            9,
            "communication",
            "beliefs",
            "Local and Distant",
        ),
        house_axis(
            "private_public",
            4,
            10,
            "roots",
            "career",
            "Private and Public",
        ),
        house_axis(
            "creation_collective",
            5,
            11,
            "creativity",
            "community",
            "Creation and Collective",
        ),
        house_axis(
            "control_surrender",
            6,
            12,
            "work_health",
            "inner_world",
            "Control and Surrender",
        ),
    ]
}

fn house_axis(
    axis_code: &str,
    house_a_number: i32,
    house_b_number: i32,
    theme_a_code: &str,
    theme_b_code: &str,
    label: &str,
) -> HouseAxisReference {
    HouseAxisReference {
        axis_code: axis_code.to_string(),
        house_a_number,
        house_b_number,
        theme_a_code: theme_a_code.to_string(),
        theme_b_code: theme_b_code.to_string(),
        label: label.to_string(),
        description: format!("{label} description"),
    }
}

fn cluster_house_2_signal() -> InterpretationSignalRow {
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
            "source_weight": 2.0,
            "aggregation_group": "capricorn_house_2_cluster",
            "evidence": {
                "fact_type": "position_cluster",
                "cluster_type": "sign_house",
                "sign_code": "capricorn",
                "house_number": 2,
                "house_theme_code": "resources",
                "source_signals": ["object_position:sun", "object_position:saturn"],
                "source_objects": ["sun", "saturn"]
            }
        })),
    }
}

#[test]
fn v11_contains_house_axis_emphasis_from_reference_axes() {
    let positions = vec![
        angle_position(
            11,
            "ascendant",
            "Ascendant",
            "asc",
            "dsc",
            "horizontal",
            215.0,
        ),
        object_in_house(
            5, "mars", "Mars", "scorpio", "Scorpio", 1, "identity", false,
        ),
        object_in_house(
            10, "pluto", "Pluto", "scorpio", "Scorpio", 1, "identity", false,
        ),
        object_in_house(
            1,
            "sun",
            "Sun",
            "capricorn",
            "Capricorn",
            2,
            "resources",
            true,
        ),
        object_in_house(
            7,
            "saturn",
            "Saturn",
            "capricorn",
            "Capricorn",
            2,
            "resources",
            false,
        ),
        object_in_house(
            6,
            "jupiter",
            "Jupiter",
            "cancer",
            "Cancer",
            8,
            "shared_resources",
            false,
        ),
    ];
    let signals = vec![
        cluster_house_2_signal(),
        placement_signal_row(2, "object_position:mars", "mars"),
        InterpretationSignalRow {
            id: 3,
            signal_key: "angle:ascendant:sign:scorpio".to_string(),
            theme_code: Some("identity".to_string()),
            title: "Ascendant in Scorpio".to_string(),
            summary: Some("summary".to_string()),
            priority_score: 99.0,
            confidence_score: Some(0.95),
            payload_json: Some(json!({
                "interpretive_hint": "hint",
                "semantic_tags": ["angle", "ascendant"],
                "source_weight": 1.0,
                "aggregation_group": "angle:ascendant:scorpio",
                "evidence": {"fact_type": "chart_angle", "angle_code": "ascendant"}
            })),
        },
        dignity_signal_row(4, "dignity:saturn:domicile:capricorn", "saturn"),
        placement_signal_row(5, "object_position:jupiter", "jupiter"),
        dignity_signal_row(6, "dignity:jupiter:exaltation:cancer", "jupiter"),
    ];

    let payload = build_basic_payload_with_references(
        42,
        &input(),
        &positions,
        &signals,
        &[],
        &canonical_house_axes(),
    );

    assert!(!payload.house_axis_emphasis.is_empty());
    assert!(payload.house_axis_emphasis.len() <= 3);
    assert!(payload
        .house_axis_emphasis
        .windows(2)
        .all(|pair| pair[0].axis_score >= pair[1].axis_score));
}

#[test]
fn resources_and_identity_axes_are_detected_with_existing_signal_sources() {
    let positions = vec![
        angle_position(
            11,
            "ascendant",
            "Ascendant",
            "asc",
            "dsc",
            "horizontal",
            215.0,
        ),
        object_in_house(
            5, "mars", "Mars", "scorpio", "Scorpio", 1, "identity", false,
        ),
        object_in_house(
            10, "pluto", "Pluto", "scorpio", "Scorpio", 1, "identity", false,
        ),
        object_in_house(
            1,
            "sun",
            "Sun",
            "capricorn",
            "Capricorn",
            2,
            "resources",
            true,
        ),
        object_in_house(
            7,
            "saturn",
            "Saturn",
            "capricorn",
            "Capricorn",
            2,
            "resources",
            false,
        ),
        object_in_house(
            6,
            "jupiter",
            "Jupiter",
            "cancer",
            "Cancer",
            8,
            "shared_resources",
            false,
        ),
    ];
    let signals = vec![
        cluster_house_2_signal(),
        placement_signal_row(2, "object_position:mars", "mars"),
        InterpretationSignalRow {
            id: 3,
            signal_key: "angle:ascendant:sign:scorpio".to_string(),
            theme_code: Some("identity".to_string()),
            title: "Ascendant in Scorpio".to_string(),
            summary: Some("summary".to_string()),
            priority_score: 99.0,
            confidence_score: Some(0.95),
            payload_json: Some(json!({
                "interpretive_hint": "hint",
                "semantic_tags": ["angle", "ascendant"],
                "source_weight": 1.0,
                "aggregation_group": "angle:ascendant:scorpio",
                "evidence": {"fact_type": "chart_angle", "angle_code": "ascendant"}
            })),
        },
        dignity_signal_row(4, "dignity:saturn:domicile:capricorn", "saturn"),
        placement_signal_row(5, "object_position:jupiter", "jupiter"),
        dignity_signal_row(6, "dignity:jupiter:exaltation:cancer", "jupiter"),
    ];

    let payload = build_basic_payload_with_references(
        42,
        &input(),
        &positions,
        &signals,
        &[],
        &canonical_house_axes(),
    );
    let resources = payload
        .house_axis_emphasis
        .iter()
        .find(|axis| axis.axis_code == "resources_sharing")
        .expect("resources axis");
    let identity = payload
        .house_axis_emphasis
        .iter()
        .find(|axis| axis.axis_code == "self_relationship")
        .expect("identity axis");
    let signal_keys: std::collections::HashSet<_> = payload
        .signals
        .iter()
        .map(|signal| signal.signal_key.as_str())
        .collect();

    assert_eq!(resources.primary_house, 2);
    assert_eq!(identity.primary_house, 1);
    assert!(resources
        .source_signal_keys
        .contains(&"cluster:capricorn:house_2".to_string()));
    assert!(identity
        .source_signal_keys
        .contains(&"angle:ascendant:sign:scorpio".to_string()));
    for axis in &payload.house_axis_emphasis {
        assert_eq!(axis.houses[0] + 6, axis.houses[1]);
        for source in &axis.source_signal_keys {
            assert!(signal_keys.contains(source.as_str()));
        }
    }
}

#[test]
fn v13_requires_sect_affinities_to_emit_accidental_block() {
    let mut sun = position();
    sun.longitude_deg = 281.4543;
    let mut moon = position();
    moon.chart_object_id = 2;
    moon.object_code = "moon".to_string();
    moon.object_name = "Moon".to_string();
    moon.longitude_deg = 341.7642;

    let payload = build_basic_payload_with_accidental_references(
        42,
        &input(),
        &[sun, moon],
        &[
            placement_signal_row(1, "object_position:sun", "sun"),
            placement_signal_row(2, "object_position:moon", "moon"),
        ],
        &[],
        &[],
        &canonical_lunar_phases(),
        &canonical_accidental_conditions(),
        &[],
    );

    assert_eq!(
        payload.chart_context.payload_contract.contract_version,
        "natal_structured_v12"
    );
    assert!(payload.accidental_dignities.is_empty());
}

#[test]
fn v13_contains_accidental_dignities_from_reference_definitions() {
    let mut sun = position();
    sun.longitude_deg = 281.4543;
    let mut moon = position();
    moon.chart_object_id = 2;
    moon.object_code = "moon".to_string();
    moon.object_name = "Moon".to_string();
    moon.longitude_deg = 341.7642;

    let payload = build_basic_payload_with_accidental_references(
        42,
        &input(),
        &[sun, moon],
        &[placement_signal_row(1, "object_position:sun", "sun")],
        &[],
        &[],
        &canonical_lunar_phases(),
        &canonical_accidental_conditions(),
        &canonical_sect_affinities(),
    );

    assert!(!payload.accidental_dignities.is_empty());
    assert!(payload
        .positions
        .iter()
        .all(|position| position.object_code == "moon"
            || !position.accidental_dignity_context.is_empty()));
}

#[test]
fn v13_contains_lunar_phase_context_from_reference_phases() {
    let mut sun = position();
    sun.longitude_deg = 281.4543;
    let mut moon = position();
    moon.chart_object_id = 2;
    moon.object_code = "moon".to_string();
    moon.object_name = "Moon".to_string();
    moon.longitude_deg = 341.7642;
    moon.sign_id = 12;
    moon.sign_code = "pisces".to_string();
    moon.sign_name = "Pisces".to_string();
    moon.house_id = Some(4);
    moon.house_number = Some(4);
    moon.house_name = Some("Home".to_string());

    let payload = build_basic_payload_with_accidental_references(
        42,
        &input(),
        &[sun, moon],
        &[
            placement_signal_row(1, "object_position:sun", "sun"),
            placement_signal_row(2, "object_position:moon", "moon"),
        ],
        &[],
        &[],
        &canonical_lunar_phases(),
        &canonical_accidental_conditions(),
        &canonical_sect_affinities(),
    );
    let phase = payload
        .lunar_phase_context
        .expect("expected lunar phase context");

    assert_eq!(
        payload.chart_context.payload_contract.contract_version,
        "natal_structured_v14"
    );
    assert_eq!(phase.phase_code, "waxing_crescent");
    assert_eq!(phase.cycle_family, "waxing");
    assert!((phase.sun_moon_angle_deg - 60.3099).abs() <= 0.0001);
    assert_eq!(
        phase.related_signal_keys,
        vec!["object_position:sun", "object_position:moon"]
    );
    assert_eq!(phase.related_reading_slots, vec!["core_identity"]);
}

#[test]
fn lunar_phase_angle_rounding_stays_inside_zodiac_range() {
    let mut sun = position();
    sun.longitude_deg = 10.0;
    let mut moon = position();
    moon.chart_object_id = 2;
    moon.object_code = "moon".to_string();
    moon.object_name = "Moon".to_string();
    moon.longitude_deg = 9.99996;

    let payload = build_basic_payload_with_all_references(
        42,
        &input(),
        &[sun, moon],
        &[
            placement_signal_row(1, "object_position:sun", "sun"),
            placement_signal_row(2, "object_position:moon", "moon"),
        ],
        &[],
        &[],
        &canonical_lunar_phases(),
    );
    let phase = payload
        .lunar_phase_context
        .expect("expected lunar phase context");

    assert_eq!(phase.sun_moon_angle_deg, 0.0);
    assert_eq!(phase.phase_code, "new_moon");
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
            "aspect_context": {
                "aspect_family": "major",
                "primary_valence": primary_valence_for_test(aspect_code),
                "intensity_modifier": intensity_modifier_for_test(aspect_code),
                "secondary_effect": null,
                "dynamic_quality": dynamic_quality_for_test(aspect_code),
                "phase_state": "applying",
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

fn canonical_accidental_conditions() -> Vec<AccidentalDignityConditionReference> {
    vec![
        accidental_condition_ref("angular_house", "house_modality", "dignity", 0.75, 0.25),
        accidental_condition_ref(
            "succedent_house",
            "house_modality",
            "contextual",
            0.45,
            0.05,
        ),
        accidental_condition_ref("cadent_house", "house_modality", "debility", 0.35, -0.12),
        accidental_condition_ref("near_ascendant", "angle_proximity", "dignity", 0.82, 0.22),
        accidental_condition_ref("near_descendant", "angle_proximity", "dignity", 0.82, 0.22),
        accidental_condition_ref("near_mc", "angle_proximity", "dignity", 0.82, 0.22),
        accidental_condition_ref("near_ic", "angle_proximity", "dignity", 0.82, 0.22),
        accidental_condition_ref("retrograde_motion", "motion", "debility", 0.45, -0.1),
        accidental_condition_ref("stationary_motion", "motion", "intensifier", 0.7, 0.1),
        accidental_condition_ref("above_horizon", "horizon", "contextual", 0.45, 0.05),
        accidental_condition_ref("below_horizon", "horizon", "contextual", 0.35, 0.0),
        accidental_condition_ref("on_horizon", "horizon", "dignity", 0.75, 0.2),
        accidental_condition_ref("sect_affinity_match", "sect", "dignity", 0.45, 0.08),
        accidental_condition_ref("sect_affinity_mismatch", "sect", "debility", 0.35, -0.06),
        accidental_condition_ref(
            "sect_affinity_variable_unresolved",
            "sect",
            "contextual",
            0.2,
            0.0,
        ),
    ]
}

fn accidental_condition_ref(
    condition_code: &str,
    condition_family: &str,
    polarity: &str,
    strength_score: f64,
    score_delta: f64,
) -> AccidentalDignityConditionReference {
    AccidentalDignityConditionReference {
        condition_code: condition_code.to_string(),
        condition_family: condition_family.to_string(),
        label: condition_code.to_string(),
        polarity: polarity.to_string(),
        strength_score,
        score_delta,
        description: format!("{condition_code} description"),
    }
}

fn canonical_sect_affinities() -> Vec<ObjectSectAffinityReference> {
    vec![
        sect_affinity_ref("sun", "day", false),
        sect_affinity_ref("jupiter", "day", false),
        sect_affinity_ref("saturn", "day", false),
        sect_affinity_ref("moon", "night", false),
        sect_affinity_ref("venus", "night", false),
        sect_affinity_ref("mars", "night", false),
        sect_affinity_ref("mercury", "variable", true),
    ]
}

fn sect_affinity_ref(
    object_code: &str,
    sect_affinity_code: &str,
    is_variable: bool,
) -> ObjectSectAffinityReference {
    ObjectSectAffinityReference {
        object_code: object_code.to_string(),
        sect_affinity_code: sect_affinity_code.to_string(),
        is_variable,
        description: format!("{object_code} sect affinity"),
    }
}

fn canonical_lunar_phases() -> Vec<LunarPhaseReference> {
    vec![
        lunar_phase(
            "new_moon",
            "New Moon",
            "conjunction",
            337.5,
            22.5,
            0.0,
            true,
        ),
        lunar_phase(
            "waxing_crescent",
            "Waxing Crescent",
            "waxing",
            22.5,
            67.5,
            45.0,
            false,
        ),
        lunar_phase(
            "first_quarter",
            "First Quarter",
            "waxing",
            67.5,
            112.5,
            90.0,
            true,
        ),
        lunar_phase(
            "waxing_gibbous",
            "Waxing Gibbous",
            "waxing",
            112.5,
            157.5,
            135.0,
            false,
        ),
        lunar_phase(
            "full_moon",
            "Full Moon",
            "opposition",
            157.5,
            202.5,
            180.0,
            true,
        ),
        lunar_phase(
            "waning_gibbous",
            "Waning Gibbous",
            "waning",
            202.5,
            247.5,
            225.0,
            false,
        ),
        lunar_phase(
            "last_quarter",
            "Last Quarter",
            "waning",
            247.5,
            292.5,
            270.0,
            true,
        ),
        lunar_phase(
            "waning_crescent",
            "Waning Crescent",
            "waning",
            292.5,
            337.5,
            315.0,
            false,
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn lunar_phase(
    phase_code: &str,
    label: &str,
    cycle_family: &str,
    range_start_deg: f64,
    range_end_deg: f64,
    exact_anchor_deg: f64,
    is_major_lunar_phase: bool,
) -> LunarPhaseReference {
    LunarPhaseReference {
        phase_code: phase_code.to_string(),
        label: label.to_string(),
        cycle_family: cycle_family.to_string(),
        range_start_deg,
        range_end_deg,
        exact_anchor_deg,
        is_major_lunar_phase,
        description: format!("{label} description"),
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
            "evidence": {
                "fact_type": "object_position",
                "object_code": object_code
            }
        })),
    }
}

fn domicile_ruler(
    sign_id: i32,
    sign_code: &str,
    sign_name: &str,
    chart_object_id: i32,
    object_code: &str,
    object_name: &str,
) -> DomicileRulerReference {
    DomicileRulerReference {
        reference_version_id: Some(1),
        astral_system_id: 1,
        astral_system_code: "traditional".to_string(),
        sign_id,
        sign_code: sign_code.to_string(),
        sign_name: sign_name.to_string(),
        chart_object_id,
        object_code: object_code.to_string(),
        object_name: object_name.to_string(),
        dignity_type: "domicile".to_string(),
        weight: 1.0,
        is_primary: true,
    }
}

fn modern_domicile_ruler(
    sign_id: i32,
    sign_code: &str,
    sign_name: &str,
    chart_object_id: i32,
    object_code: &str,
    object_name: &str,
) -> DomicileRulerReference {
    DomicileRulerReference {
        astral_system_id: 2,
        astral_system_code: "modern".to_string(),
        ..domicile_ruler(
            sign_id,
            sign_code,
            sign_name,
            chart_object_id,
            object_code,
            object_name,
        )
    }
}
