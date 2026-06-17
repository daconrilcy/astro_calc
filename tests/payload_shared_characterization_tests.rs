use chrono::{TimeZone, Utc};
use serde_json::json;

use astral_calculator::domain::{
    HouseAxisReference, LunarPhaseReference, NatalChartInput, ObjectPositionFact,
};
use astral_calculator::domain::HouseReference;
use astral_calculator::features::payload::{build_basic_payload, build_basic_payload_with_all_references};
use astral_calculator::runtime::validate_house_axis_references;

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

#[allow(clippy::too_many_arguments)]
fn angle_position(
    chart_object_id: i32,
    object_code: &str,
    object_name: &str,
    angle_point_code: &str,
    opposite_angle_code: &str,
    axis: &str,
    longitude_deg: f64,
    house_number: i32,
) -> ObjectPositionFact {
    ObjectPositionFact {
        chart_object_id,
        object_code: object_code.to_string(),
        object_name: object_name.to_string(),
        zodiacal_reference_system_id: 1,
        coordinate_reference_system_id: 1,
        sign_id: 1,
        sign_code: "aries".to_string(),
        sign_name: "Aries".to_string(),
        house_id: Some(house_number),
        house_number: Some(house_number),
        house_name: Some(format!("House {house_number}")),
        motion_state_id: None,
        horizon_position_id: Some(1),
        longitude_deg,
        latitude_deg: None,
        apparent_speed_deg_per_day: None,
        altitude_deg: None,
        is_visible: None,
        facts_json: Some(json!({
            "object_context": {
                "role": "angle",
                "role_label": "Angle"
            },
            "angle_context": {
                "angle_point_code": angle_point_code,
                "opposite_angle_code": opposite_angle_code,
                "axis": axis,
                "full_name": object_name,
                "associated_house_number": house_number,
                "chart_object_sort_order": chart_object_id
            },
            "visibility_context": {
                "horizon_position": if house_number >= 7 { "above_horizon" } else { "below_horizon" },
                "source": "angle_context"
            }
        })),
    }
}

fn mobile_position(
    chart_object_id: i32,
    object_code: &str,
    object_name: &str,
    house_number: i32,
    altitude_deg: f64,
) -> ObjectPositionFact {
    ObjectPositionFact {
        chart_object_id,
        object_code: object_code.to_string(),
        object_name: object_name.to_string(),
        zodiacal_reference_system_id: 1,
        coordinate_reference_system_id: 1,
        sign_id: 3,
        sign_code: "gemini".to_string(),
        sign_name: "Gemini".to_string(),
        house_id: Some(house_number),
        house_number: Some(house_number),
        house_name: Some(format!("House {house_number}")),
        motion_state_id: Some(1),
        horizon_position_id: Some(if altitude_deg > 0.0 { 1 } else { 2 }),
        longitude_deg: 80.0 + chart_object_id as f64,
        latitude_deg: None,
        apparent_speed_deg_per_day: Some(1.0),
        altitude_deg: Some(altitude_deg),
        is_visible: Some(altitude_deg >= 0.0),
        facts_json: Some(json!({
            "sign_context": {
                "element": "air",
                "modality": "mutable",
                "polarity": "yang"
            },
            "house_context": {
                "theme_code": "beliefs",
                "house_number": house_number
            },
            "house_modality": {
                "code": if [1, 4, 7, 10].contains(&house_number) { "angular" } else { "cadent" }
            },
            "object_context": {
                "role": "planet",
                "role_label": "Planet",
                "is_luminary": object_code == "sun",
                "signal_scoring": {
                    "source_weight": 1.0
                }
            },
            "motion_context": {
                "motion_state": "direct"
            }
        })),
    }
}

fn positions() -> Vec<ObjectPositionFact> {
    vec![
        mobile_position(1, "sun", "Sun", 9, 12.0),
        mobile_position(2, "moon", "Moon", 3, -4.0),
        angle_position(
            3,
            "ascendant",
            "Ascendant",
            "ascendant",
            "descendant",
            "horizontal",
            15.0,
            1,
        ),
        angle_position(
            4,
            "descendant",
            "Descendant",
            "descendant",
            "ascendant",
            "horizontal",
            195.0,
            7,
        ),
        angle_position(5, "mc", "Midheaven", "mc", "ic", "vertical", 100.0, 10),
        angle_position(6, "ic", "IC", "ic", "mc", "vertical", 280.0, 4),
    ]
}

fn angle_signal_row() -> astral_calculator::domain::InterpretationSignalRow {
    astral_calculator::domain::InterpretationSignalRow {
        id: 1,
        signal_key: "angle:ascendant:sign:aries".to_string(),
        theme_code: Some("identity".to_string()),
        title: "Ascendant in Aries".to_string(),
        summary: Some("Kept".to_string()),
        priority_score: 80.0,
        confidence_score: Some(0.9),
        payload_json: Some(json!({
            "interpretive_hint": "Active style",
            "semantic_tags": ["identity"],
            "source_weight": 1.0,
            "aggregation_group": "angles",
            "evidence": {
                "fact_type": "chart_angle",
                "angle_code": "ascendant",
                "opposite_angle_code": "descendant",
                "opposite_angle_object_code": "descendant"
            }
        })),
    }
}

#[test]
fn shared_angle_pair_extraction_keeps_axis_aspects_out_of_payload() {
    let positions = positions();
    let signals = vec![
        astral_calculator::domain::InterpretationSignalRow {
            id: 10,
            signal_key: "aspect:descendant:ascendant:opposition".to_string(),
            theme_code: Some("relationships".to_string()),
            title: "Axis opposition".to_string(),
            summary: Some("Should be filtered".to_string()),
            priority_score: 80.0,
            confidence_score: Some(0.9),
            payload_json: Some(json!({
                "aspect_context": {
                    "aspect_family": "major",
                    "primary_valence": "polarizing",
                    "intensity_modifier": "high",
                    "secondary_effect": "contrast",
                    "dynamic_quality": "tension",
                    "phase_state": "applying",
                    "valence_family": "dynamic",
                    "is_tonal_valence": true,
                    "is_intensity_modifier": true
                },
                "evidence": {
                    "aspect_code": "opposition",
                    "source_object_code": "descendant",
                    "target_object_code": "ascendant"
                }
            })),
        },
        astral_calculator::domain::InterpretationSignalRow {
            id: 11,
            ..angle_signal_row()
        },
    ];

    let payload = build_basic_payload(7, &input(), &positions, &signals);
    assert_eq!(payload.signals.len(), 1);
    assert_eq!(payload.signals[0].signal_key, "angle:ascendant:sign:aries");
}

#[test]
fn shared_visibility_mapping_keeps_payload_runtime_compatible() {
    let signal = angle_signal_row();
    let payload = build_basic_payload(7, &input(), &positions(), &[signal]);
    assert_eq!(
        payload.chart_context.sect.chart_sect.as_deref(),
        Some("day")
    );
    assert_eq!(
        payload.chart_context.sect.sun_horizon_position.as_deref(),
        Some("above_horizon")
    );
}

#[test]
fn shared_house_axis_canonical_mapping_builds_expected_axis() {
    let references = vec![
        (
            "self_relationship",
            1,
            7,
            "identity",
            "relationships",
            "Self and Relationship",
        ),
        (
            "resources_sharing",
            2,
            8,
            "resources",
            "shared_resources",
            "Resources and Sharing",
        ),
        (
            "local_distant",
            3,
            9,
            "communication",
            "beliefs",
            "Local and Distant",
        ),
        (
            "private_public",
            4,
            10,
            "roots",
            "career",
            "Private and Public",
        ),
        (
            "creation_collective",
            5,
            11,
            "creativity",
            "community",
            "Creation and Collective",
        ),
        (
            "control_surrender",
            6,
            12,
            "work_health",
            "inner_world",
            "Control and Surrender",
        ),
    ]
    .into_iter()
    .map(
        |(axis_code, house_a_number, house_b_number, theme_a_code, theme_b_code, label)| {
            HouseAxisReference {
                axis_code: axis_code.to_string(),
                house_a_number,
                house_b_number,
                theme_a_code: theme_a_code.to_string(),
                theme_b_code: theme_b_code.to_string(),
                label: label.to_string(),
                description: format!("{label} axis"),
            }
        },
    )
    .collect::<Vec<_>>();

    let houses = vec![
        (1, "identity"),
        (2, "resources"),
        (3, "communication"),
        (4, "roots"),
        (5, "creativity"),
        (6, "work_health"),
        (7, "relationships"),
        (8, "shared_resources"),
        (9, "beliefs"),
        (10, "career"),
        (11, "community"),
        (12, "inner_world"),
    ]
    .into_iter()
    .map(|(number, theme_code)| HouseReference {
        id: number,
        number,
        name: format!("House {number}"),
        theme_code: theme_code.to_string(),
        modality_code: None,
        modality_label: None,
        accidental_strength: None,
        modality_priority_delta: Some(0.0),
        interpretation_weight: None,
    })
    .collect::<Vec<_>>();

    assert!(validate_house_axis_references(&references, &houses).is_ok());
}

#[test]
fn shared_lunar_phase_wraparound_mapping_builds_expected_context() {
    let mut positions = positions();
    positions[0].longitude_deg = 350.0;
    positions[1].longitude_deg = 10.0;

    let payload = build_basic_payload_with_all_references(
        9,
        &input(),
        &positions,
        &[angle_signal_row()],
        &[],
        &[],
        &[
            LunarPhaseReference {
                phase_code: "new_moon".to_string(),
                label: "New Moon".to_string(),
                description: "Wrap-around conjunction phase".to_string(),
                cycle_family: "conjunction".to_string(),
                range_start_deg: 337.5,
                range_end_deg: 22.5,
                exact_anchor_deg: 0.0,
                is_major_lunar_phase: true,
            },
            LunarPhaseReference {
                phase_code: "waxing_crescent".to_string(),
                label: "Waxing Crescent".to_string(),
                description: "Post-conjunction waxing phase".to_string(),
                cycle_family: "waxing".to_string(),
                range_start_deg: 22.5,
                range_end_deg: 67.5,
                exact_anchor_deg: 45.0,
                is_major_lunar_phase: false,
            },
        ],
    );

    let lunar_phase = payload
        .lunar_phase_context
        .expect("expected lunar phase context");
    assert_eq!(lunar_phase.phase_code, "new_moon");
    assert!((lunar_phase.sun_moon_angle_deg - 20.0).abs() <= 0.0001);
    assert!(lunar_phase
        .related_signal_keys
        .iter()
        .all(|key| ["object_position:sun", "object_position:moon"].contains(&key.as_str())));
}
