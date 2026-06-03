use chrono::{TimeZone, Utc};
use serde_json::json;

use rust_sqlx_connection_test::domain::{
    BasicAngleFact, BasicCalculationReliability, BasicChartContext, BasicChartEmphasis,
    BasicDignity, BasicDominantHouse, BasicDominantObject, BasicDominantSign,
    BasicHemisphereEmphasis, BasicHouseAxisEmphasis, BasicHouseAxisScore, BasicLunarPhaseContext,
    BasicObjectPosition, BasicPayload, BasicPayloadContract, BasicReadingPlanItem,
    BasicRulerContext, BasicRulerSource, BasicRulershipContext, BasicSecondarySlotCandidate,
    BasicSectContext, BasicSignal, CalculationReferenceData, HouseAxisReference,
    LunarPhaseReference,
};
use rust_sqlx_connection_test::models::{
    AnglePointReference, ChartObject, DomicileRulerReference, HouseReference, SignReference,
};
use rust_sqlx_connection_test::repositories::parse_existing_basic_payload_value;
use rust_sqlx_connection_test::runtime::{
    has_current_rulership_references, is_current_basic_payload, validate_calculation_references,
    validate_chart_object_signal_profiles, validate_house_axis_references,
    validate_lunar_phase_references,
};

fn current_payload() -> BasicPayload {
    BasicPayload {
        product_code: "basic".to_string(),
        chart_calculation_id: 1,
        reference_version_id: 1,
        subject_label: None,
        birth_datetime_utc: Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap(),
        chart_context: BasicChartContext {
            chart_type: "natal".to_string(),
            zodiacal_reference_system_id: 1,
            coordinate_reference_system_id: 1,
            house_system_id: 1,
            reference_version_id: 1,
            payload_contract: BasicPayloadContract {
                contract_version: "natal_structured_v12".to_string(),
                calculation_scope: "full_natal".to_string(),
                interpretation_scope: "structured_interpretation".to_string(),
                projection_depth: "rich".to_string(),
            },
            calculation_reliability: BasicCalculationReliability {
                birth_time_precision_required: true,
                house_system_sensitive: true,
            },
            sect: BasicSectContext {
                chart_sect: Some("day".to_string()),
                sun_horizon_position: Some("above_horizon".to_string()),
                source: Some("calculated_altitude".to_string()),
            },
            hemisphere_emphasis: BasicHemisphereEmphasis {
                count_scope: "mobile_chart_objects_only".to_string(),
                above_horizon_count: 2,
                below_horizon_count: 0,
                on_horizon_count: 0,
                interpretive_hint: Some(
                    "The chart has a stronger visible or outward emphasis.".to_string(),
                ),
            },
        },
        positions: vec![
            BasicObjectPosition {
                object_code: "sun".to_string(),
                object_name: "Sun".to_string(),
                longitude_deg: 84.0,
                sign_id: 3,
                sign_code: "gemini".to_string(),
                sign_name: "Gemini".to_string(),
                house_id: Some(9),
                house_number: Some(9),
                house_name: Some("Beliefs".to_string()),
                motion_state_id: Some(1),
                sign_context: Some(json!({
                    "element": "air",
                    "modality": "mutable",
                    "polarity": "yang",
                    "keywords": ["communication"]
                })),
                house_context: Some(json!({
                    "theme_code": "beliefs"
                })),
                house_modality: Some(json!({
                    "code": "cadent",
                    "accidental_strength": "weak_or_background",
                    "interpretation_weight": "lower_for_external_manifestation"
                })),
                object_context: Some(json!({
                    "role": "luminary",
                    "nature": ["luminary"],
                    "is_luminary": true
                })),
                motion_context: Some(json!({
                    "motion_state": "direct",
                    "label": "Direct",
                    "motion_family": "forward"
                })),
                dignity_context: json!([]),
                visibility_context: json!({
                    "horizon_position_id": 1,
                    "horizon_position": "above_horizon",
                    "altitude_deg": 12.5,
                    "is_visible": true,
                    "source": "calculated_altitude"
                }),
            },
            BasicObjectPosition {
                object_code: "moon".to_string(),
                object_name: "Moon".to_string(),
                longitude_deg: 144.0,
                sign_id: 5,
                sign_code: "leo".to_string(),
                sign_name: "Leo".to_string(),
                house_id: Some(11),
                house_number: Some(11),
                house_name: Some("Community".to_string()),
                motion_state_id: Some(1),
                sign_context: Some(json!({
                    "element": "fire",
                    "modality": "fixed",
                    "polarity": "yang"
                })),
                house_context: Some(json!({
                    "theme_code": "community"
                })),
                house_modality: Some(json!({
                    "code": "succedent",
                    "accidental_strength": "medium",
                    "interpretation_weight": "medium"
                })),
                object_context: Some(json!({
                    "role": "luminary",
                    "nature": ["luminary"],
                    "is_luminary": true
                })),
                motion_context: Some(json!({
                    "motion_state": "direct",
                    "label": "Direct",
                    "motion_family": "forward"
                })),
                dignity_context: json!([]),
                visibility_context: json!({
                    "horizon_position_id": 1,
                    "horizon_position": "above_horizon",
                    "altitude_deg": 14.0,
                    "is_visible": true,
                    "source": "calculated_altitude"
                }),
            },
        ],
        angles: vec![
            angle_fact(
                "ascendant",
                "Ascendant",
                "horizontal",
                "descendant",
                84.0,
                1,
            ),
            angle_fact(
                "descendant",
                "Descendant",
                "horizontal",
                "ascendant",
                264.0,
                7,
            ),
            angle_fact("mc", "Midheaven", "vertical", "ic", 10.0, 10),
            angle_fact("ic", "Imum Coeli", "vertical", "mc", 190.0, 4),
        ],
        dignities: Vec::new(),
        chart_emphasis: BasicChartEmphasis {
            dominant_signs: vec![BasicDominantSign {
                sign_code: "gemini".to_string(),
                score: 0.2174,
                reasons: vec!["sun_in_sign".to_string()],
            }],
            dominant_houses: vec![BasicDominantHouse {
                house_number: 9,
                theme_code: "beliefs".to_string(),
                score: 0.2174,
                reasons: vec!["sun_in_house".to_string()],
            }],
            dominant_objects: vec![BasicDominantObject {
                object_code: "sun".to_string(),
                score: 0.4167,
                reasons: vec!["placement".to_string()],
            }],
        },
        rulership_context: BasicRulershipContext {
            ascendant_ruler: Some(test_ruler_context(
                "angle:ascendant:ruler",
                "angle",
                "ascendant",
                "gemini",
                "mercury",
                "identity_ruler",
            )),
            mc_ruler: Some(test_ruler_context(
                "angle:mc:ruler",
                "angle",
                "mc",
                "gemini",
                "mercury",
                "public_direction_ruler",
            )),
            ..BasicRulershipContext::default()
        },
        house_axis_emphasis: vec![BasicHouseAxisEmphasis {
            axis_code: "local_distant".to_string(),
            houses: vec![3, 9],
            theme_codes: vec!["communication".to_string(), "beliefs".to_string()],
            house_scores: vec![
                BasicHouseAxisScore {
                    house_number: 3,
                    theme_code: "communication".to_string(),
                    score: 0.1,
                    reasons: vec!["communication_theme".to_string()],
                },
                BasicHouseAxisScore {
                    house_number: 9,
                    theme_code: "beliefs".to_string(),
                    score: 0.45,
                    reasons: vec![
                        "dominant_house".to_string(),
                        "sun_in_house".to_string(),
                        "beliefs_theme".to_string(),
                    ],
                },
            ],
            primary_house: 9,
            secondary_house: 3,
            axis_score: 0.485,
            polarity_balance: "secondary_house_dominant".to_string(),
            source_signal_keys: vec!["object_position:sun".to_string()],
            source_context_keys: Vec::new(),
            reasons: vec![
                "dominant_house".to_string(),
                "sun_in_house".to_string(),
                "beliefs_theme".to_string(),
            ],
            interpretive_hint:
                "Local and Distant is activated mainly through house 9 (beliefs), with house 3 (communication) present as a secondary counterpoint."
                    .to_string(),
        }],
        lunar_phase_context: Some(BasicLunarPhaseContext {
            phase_code: "waxing_crescent".to_string(),
            phase_label: "Waxing Crescent".to_string(),
            cycle_family: "waxing".to_string(),
            sun_object_code: "sun".to_string(),
            moon_object_code: "moon".to_string(),
            sun_longitude_deg: 84.0,
            moon_longitude_deg: 144.0,
            sun_moon_angle_deg: 60.0,
            phase_angle_range_deg: vec![22.5, 67.5],
            exact_phase_anchor_deg: 45.0,
            distance_to_exact_phase_deg: 15.0,
            phase_progress_ratio: 0.8333,
            is_major_lunar_phase: false,
            related_signal_keys: vec!["object_position:sun".to_string()],
            related_reading_slots: vec!["core_identity".to_string()],
            semantic_tags: vec![
                "lunar_phase".to_string(),
                "sun_moon_cycle".to_string(),
                "waxing".to_string(),
                "waxing_crescent".to_string(),
            ],
            interpretive_hint:
                "The Sun-Moon cycle is in a waxing crescent phase, indicating a structured waxing relationship between solar identity and lunar needs."
                    .to_string(),
        }),
        signals: vec![
            BasicSignal {
                signal_key: "object_position:sun".to_string(),
                theme_code: Some("beliefs".to_string()),
                title: "Sun in Gemini, house 9".to_string(),
                summary: Some("summary".to_string()),
                priority_score: 100.0,
                confidence_score: Some(0.95),
                interpretive_hint: Some("hint".to_string()),
                semantic_tags: vec!["placement".to_string()],
                source_weight: Some(1.0),
                aggregation_group: Some("gemini:house_9".to_string()),
                aspect_context: None,
                evidence: Some(json!({
                    "fact_type": "object_position",
                    "essential_dignities": [],
                    "placement_context": {
                        "sign_context": {
                            "element": "air",
                            "modality": "mutable",
                            "polarity": "yang"
                        },
                        "house_context": {"theme_code": "beliefs"},
                        "house_modality": {"code": "cadent"},
                        "object_context": {"role": "luminary"},
                        "motion_context": {"motion_state": "direct"}
                        ,"visibility_context": {
                            "horizon_position_id": 1,
                            "horizon_position": "above_horizon",
                            "altitude_deg": 12.5,
                            "is_visible": true,
                            "source": "calculated_altitude"
                        }
                    }
                })),
            },
            BasicSignal {
                signal_key: "angle:ascendant:sign:gemini".to_string(),
                theme_code: Some("identity".to_string()),
                title: "Ascendant in Gemini".to_string(),
                summary: Some("summary".to_string()),
                priority_score: 99.0,
                confidence_score: Some(0.95),
                interpretive_hint: Some("hint".to_string()),
                semantic_tags: vec!["angle".to_string(), "ascendant".to_string()],
                source_weight: Some(1.0),
                aggregation_group: Some("angle:ascendant:gemini".to_string()),
                aspect_context: None,
                evidence: Some(json!({
                    "fact_type": "chart_angle",
                    "angle_code": "ascendant",
                    "opposite_angle_code": "dsc",
                    "opposite_angle_object_code": "descendant",
                    "sign_code": "gemini"
                })),
            },
        ],
        reading_plan: vec![BasicReadingPlanItem {
            slot: "core_identity".to_string(),
            title: "Core identity markers".to_string(),
            source_signal_keys: vec!["object_position:sun".to_string()],
            primary_signal_keys: vec!["object_position:sun".to_string()],
            secondary_slot_candidates: Vec::new(),
        }],
    }
}

#[test]
fn existing_payload_parser_treats_legacy_endpoint_shape_as_stale() {
    let mut payload = serde_json::to_value(current_payload()).unwrap();
    payload["rulership_context"]["final_dispositors"] = json!([
        {
            "object_codes": ["jupiter", "moon"],
            "source_objects": ["jupiter", "mars", "moon", "pluto"]
        }
    ]);

    let parsed = parse_existing_basic_payload_value(payload).unwrap();

    assert!(parsed.is_none());
}

#[test]
fn persisted_payload_reuse_rejects_stale_rulership_reference_sources() {
    let payload = current_payload();
    let current_rulers = vec![domicile_ruler_reference("gemini", "venus", 4)];

    assert!(!has_current_rulership_references(&payload, &current_rulers));
}

#[test]
fn persisted_payload_reuse_accepts_matching_rulership_reference_sources() {
    let payload = current_payload();
    let current_rulers = vec![domicile_ruler_reference("gemini", "mercury", 3)];

    assert!(has_current_rulership_references(&payload, &current_rulers));
}

#[test]
fn persisted_payload_reuse_rejects_stale_rulership_reference_weight() {
    let payload = current_payload();
    let mut current_rulers = vec![domicile_ruler_reference("gemini", "mercury", 3)];
    current_rulers[0].weight = 0.5;

    assert!(!has_current_rulership_references(&payload, &current_rulers));
}

fn angle_fact(
    angle_code: &str,
    angle_name: &str,
    axis: &str,
    opposite_angle_code: &str,
    longitude_deg: f64,
    house_number: i32,
) -> BasicAngleFact {
    BasicAngleFact {
        angle_code: angle_code.to_string(),
        angle_name: angle_name.to_string(),
        axis: axis.to_string(),
        opposite_angle_code: opposite_angle_code.to_string(),
        longitude_deg,
        sign_id: 3,
        sign_code: "gemini".to_string(),
        sign_name: "Gemini".to_string(),
        house_id: Some(house_number),
        house_number,
        house_name: Some(format!("House {house_number}")),
    }
}

fn domicile_ruler_reference(
    sign_code: &str,
    object_code: &str,
    chart_object_id: i32,
) -> DomicileRulerReference {
    DomicileRulerReference {
        reference_version_id: Some(1),
        astral_system_id: 1,
        astral_system_code: "traditional".to_string(),
        sign_id: 3,
        sign_code: sign_code.to_string(),
        sign_name: sign_code.to_string(),
        chart_object_id,
        object_code: object_code.to_string(),
        object_name: object_code.to_string(),
        dignity_type: "domicile".to_string(),
        weight: 1.0,
        is_primary: true,
    }
}

fn test_ruler_context(
    context_key: &str,
    source_kind: &str,
    source_code: &str,
    sign_code: &str,
    ruler_object_code: &str,
    interpretive_role: &str,
) -> BasicRulerContext {
    BasicRulerContext {
        context_key: context_key.to_string(),
        source_kind: source_kind.to_string(),
        source_code: source_code.to_string(),
        sign_code: sign_code.to_string(),
        ruler_object_codes: vec![ruler_object_code.to_string()],
        ruler_object_code: ruler_object_code.to_string(),
        ruler_position_signal_key: None,
        ruler_house_number: None,
        ruler_sign_code: None,
        interpretive_role: interpretive_role.to_string(),
        strength_context: Vec::new(),
        ruler_sources: vec![BasicRulerSource {
            object_code: ruler_object_code.to_string(),
            reference_version_id: Some(1),
            astral_system_id: 1,
            astral_system_code: "traditional".to_string(),
            dignity_type: "domicile".to_string(),
            weight: 1.0,
            is_primary: true,
        }],
        interpretive_hint: "Reference-derived ruler context.".to_string(),
    }
}

#[test]
fn current_payload_requires_signals() {
    let mut payload = current_payload();
    payload.signals.clear();

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_mismatched_chart_context_reference_version() {
    let mut payload = current_payload();
    payload.chart_context.reference_version_id = 2;

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_mismatched_sun_sect_context() {
    let mut payload = current_payload();
    payload.chart_context.sect.chart_sect = Some("night".to_string());

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_mismatched_sun_sect_source() {
    let mut payload = current_payload();
    payload.chart_context.sect.source = Some("house_hemisphere_projection".to_string());

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_mismatched_hemisphere_counts() {
    let mut payload = current_payload();
    payload
        .chart_context
        .hemisphere_emphasis
        .above_horizon_count = 0;

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_non_angle_without_calculated_altitude() {
    let mut payload = current_payload();
    payload.positions[0].visibility_context["altitude_deg"] = serde_json::Value::Null;

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_non_angle_visibility_source_without_calculated_altitude() {
    let mut payload = current_payload();
    payload.positions[0].visibility_context["source"] = json!("house_hemisphere_projection");

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_non_angle_horizon_position_inconsistent_with_altitude() {
    let mut payload = current_payload();
    payload.positions[0].visibility_context["horizon_position"] = json!("below_horizon");

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_position_without_horizon_reference_id() {
    let mut payload = current_payload();
    payload.positions[0].visibility_context["horizon_position_id"] = serde_json::Value::Null;

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_unsorted_chart_emphasis() {
    let mut payload = current_payload();
    payload
        .chart_emphasis
        .dominant_objects
        .push(BasicDominantObject {
            object_code: "moon".to_string(),
            score: 0.9,
            reasons: vec!["moon_in_sign".to_string()],
        });

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_weak_secondary_chart_emphasis() {
    let mut payload = current_payload();
    payload
        .chart_emphasis
        .dominant_signs
        .push(BasicDominantSign {
            sign_code: "taurus".to_string(),
            score: 0.2,
            reasons: vec!["mars_in_sign".to_string()],
        });

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_placement_only_secondary_dominant_object() {
    let mut payload = current_payload();
    payload.chart_emphasis.dominant_objects[0].score = 0.8;
    payload
        .chart_emphasis
        .dominant_objects
        .push(BasicDominantObject {
            object_code: "moon".to_string(),
            score: 0.6,
            reasons: vec!["placement".to_string()],
        });

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_empty_semantic_contract_fields() {
    let mut payload = current_payload();
    payload.signals[0].interpretive_hint = Some(" ".to_string());

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_aspect_context_without_reference_effect() {
    let mut payload = current_payload();
    payload.signals.push(BasicSignal {
        signal_key: "aspect:sun:mercury:conjunction".to_string(),
        theme_code: Some("aspect".to_string()),
        title: "Sun conjunction Mercury".to_string(),
        summary: Some("summary".to_string()),
        priority_score: 70.0,
        confidence_score: Some(0.85),
        interpretive_hint: Some("Sun and Mercury are connected by a conjunction.".to_string()),
        semantic_tags: vec![
            "aspect".to_string(),
            "conjunction".to_string(),
            "contextual".to_string(),
        ],
        source_weight: Some(1.75),
        aggregation_group: Some("aspect:conjunction".to_string()),
        aspect_context: Some(json!({
            "aspect_family": "major",
            "primary_valence": null,
            "intensity_modifier": null,
            "secondary_effect": null,
            "dynamic_quality": "contextual",
            "phase_state": "separating",
            "valence_family": "neutral",
            "is_tonal_valence": false,
            "is_intensity_modifier": false
        })),
        evidence: Some(json!({
            "fact_type": "aspect",
            "aspect_code": "conjunction",
            "strength_score": 0.875
        })),
    });

    assert!(!is_current_basic_payload(&payload));

    payload
        .signals
        .last_mut()
        .expect("aspect signal")
        .aspect_context = Some(json!({
        "aspect_family": "major",
        "primary_valence": null,
        "intensity_modifier": "amplifying",
        "secondary_effect": null,
        "dynamic_quality": "intensification",
        "phase_state": "separating",
        "valence_family": "intensity",
        "is_tonal_valence": false,
        "is_intensity_modifier": true
    }));

    assert!(!is_current_basic_payload(&payload));

    payload
            .signals
            .last_mut()
            .expect("aspect signal")
            .interpretive_hint = Some(
            "Read this conjunction as an amplifying contact between Sun and Mercury, with attention to the separating phase."
                .to_string(),
        );

    assert!(is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_legacy_unflagged_structural_axis_aspect() {
    let mut payload = current_payload();
    payload.signals.push(BasicSignal {
        signal_key: "aspect:ascendant:descendant:opposition".to_string(),
        theme_code: Some("aspect".to_string()),
        title: "Ascendant opposition Descendant".to_string(),
        summary: Some("summary".to_string()),
        priority_score: 80.0,
        confidence_score: Some(0.85),
        interpretive_hint: Some(
            "Read this opposition as a polarity to balance between Ascendant and Descendant, with attention to the exact phase."
                .to_string(),
        ),
        semantic_tags: vec![
            "aspect".to_string(),
            "opposition".to_string(),
            "tension".to_string(),
        ],
        source_weight: Some(2.0),
        aggregation_group: Some("aspect:opposition".to_string()),
        aspect_context: Some(json!({
            "aspect_family": "major",
            "primary_valence": "polarizing",
            "intensity_modifier": null,
            "secondary_effect": null,
            "dynamic_quality": "tension",
            "phase_state": "exact",
            "valence_family": "tonal",
            "is_tonal_valence": true,
            "is_intensity_modifier": false
        })),
        evidence: Some(json!({
            "fact_type": "aspect",
            "source_object_code": "ascendant",
            "target_object_code": "descendant",
            "aspect_code": "opposition",
            "aspect_name": "Opposition",
            "strength_score": 1.0
        })),
    });

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_angle_signal_without_opposite_object_code() {
    let mut payload = current_payload();
    payload.signals[1].evidence = Some(json!({
        "fact_type": "chart_angle",
        "angle_code": "ascendant",
        "opposite_angle_code": "dsc",
        "sign_code": "gemini"
    }));

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_angle_signal_without_short_opposite_code() {
    let mut payload = current_payload();
    payload.signals[1].evidence = Some(json!({
        "fact_type": "chart_angle",
        "angle_code": "ascendant",
        "opposite_angle_object_code": "descendant",
        "sign_code": "gemini"
    }));

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_angle_signal_with_mismatched_opposite_object_code() {
    let mut payload = current_payload();
    payload.signals[1].evidence = Some(json!({
        "fact_type": "chart_angle",
        "angle_code": "ascendant",
        "opposite_angle_object_code": "dsc",
        "sign_code": "gemini"
    }));

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_missing_canonical_top_level_angle() {
    let mut payload = current_payload();
    payload.angles[1].angle_code = "vertex".to_string();

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_mismatched_top_level_angle_opposite() {
    let mut payload = current_payload();
    payload.angles[0].opposite_angle_code = "mc".to_string();
    payload.signals[1].evidence = Some(json!({
        "fact_type": "chart_angle",
        "angle_code": "ascendant",
        "opposite_angle_code": "dsc",
        "opposite_angle_object_code": "mc",
        "sign_code": "gemini"
    }));

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_mismatched_top_level_angle_axis() {
    let mut payload = current_payload();
    payload.angles[0].axis = "vertical".to_string();

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_angle_to_angle_aspect() {
    let mut payload = current_payload();
    payload.signals.push(BasicSignal {
        signal_key: "aspect:descendant:ic:square".to_string(),
        theme_code: Some("aspect".to_string()),
        title: "Descendant square IC".to_string(),
        summary: Some("summary".to_string()),
        priority_score: 80.0,
        confidence_score: Some(0.85),
        interpretive_hint: Some(
            "Read this square as friction between Descendant and IC, with attention to the exact phase."
                .to_string(),
        ),
        semantic_tags: vec![
            "aspect".to_string(),
            "square".to_string(),
            "tension".to_string(),
        ],
        source_weight: Some(2.0),
        aggregation_group: Some("aspect:square".to_string()),
        aspect_context: Some(json!({
            "aspect_family": "major",
            "primary_valence": "challenging",
            "intensity_modifier": null,
            "secondary_effect": null,
            "dynamic_quality": "friction",
            "phase_state": "exact",
            "valence_family": "tonal",
            "is_tonal_valence": true,
            "is_intensity_modifier": false
        })),
        evidence: Some(json!({
            "fact_type": "aspect",
            "source_object_code": "descendant",
            "target_object_code": "ic",
            "aspect_code": "square",
            "aspect_name": "Square",
            "strength_score": 0.9
        })),
    });

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_incomplete_placement_context() {
    let mut payload = current_payload();
    payload.positions[0].sign_context = Some(json!({
        "element": "air",
        "modality": "mutable"
    }));

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_non_array_dignity_context() {
    let mut payload = current_payload();
    payload.positions[0].dignity_context = serde_json::Value::Null;

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_incomplete_signal_placement_context() {
    let mut payload = current_payload();
    payload.signals[0].evidence = Some(json!({
        "fact_type": "object_position",
        "essential_dignities": [],
        "placement_context": {
            "sign_context": {
                "element": "air",
                "modality": "mutable"
            },
            "house_modality": {"code": "cadent"},
            "object_context": {"role": "luminary"},
            "motion_context": {"motion_state": "direct"}
        }
    }));

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_signal_placement_visibility_without_calculated_altitude() {
    let mut payload = current_payload();
    payload.signals[0].evidence = Some(json!({
        "fact_type": "object_position",
        "essential_dignities": [],
        "placement_context": {
            "sign_context": {
                "element": "air",
                "modality": "mutable",
                "polarity": "yang"
            },
            "house_context": {"theme_code": "beliefs"},
            "house_modality": {"code": "cadent"},
            "object_context": {"role": "luminary"},
            "motion_context": {"motion_state": "direct"},
            "visibility_context": {
                "horizon_position_id": 1,
                "horizon_position": "above_horizon",
                "altitude_deg": null,
                "is_visible": null,
                "source": "calculated_altitude"
            }
        }
    }));

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_signal_placement_visibility_inconsistent_with_altitude() {
    let mut payload = current_payload();
    payload.signals[0].evidence = Some(json!({
        "fact_type": "object_position",
        "essential_dignities": [],
        "placement_context": {
            "sign_context": {
                "element": "air",
                "modality": "mutable",
                "polarity": "yang"
            },
            "house_context": {"theme_code": "beliefs"},
            "house_modality": {"code": "cadent"},
            "object_context": {"role": "luminary"},
            "motion_context": {"motion_state": "direct"},
            "visibility_context": {
                "horizon_position_id": 1,
                "horizon_position": "below_horizon",
                "altitude_deg": 12.5,
                "is_visible": true,
                "source": "calculated_altitude"
            }
        }
    }));

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_placement_signal_without_dignity_array() {
    let mut payload = current_payload();
    payload.signals[0].evidence = Some(json!({
        "fact_type": "object_position",
        "placement_context": {
            "sign_context": {
                "element": "air",
                "modality": "mutable",
                "polarity": "yang"
            },
            "house_context": {"theme_code": "beliefs"},
            "house_modality": {"code": "cadent"},
            "object_context": {"role": "luminary"},
            "motion_context": {"motion_state": "direct"},
            "visibility_context": {
                "horizon_position_id": 1,
                "horizon_position": "above_horizon",
                "altitude_deg": 12.5,
                "is_visible": true,
                "source": "calculated_altitude"
            }
        }
    }));

    assert!(!is_current_basic_payload(&payload));

    payload.signals[0].evidence = Some(json!({
        "fact_type": "object_position",
        "essential_dignities": [],
        "placement_context": {
            "sign_context": {
                "element": "air",
                "modality": "mutable",
                "polarity": "yang"
            },
            "house_context": {"theme_code": "beliefs"},
            "house_modality": {"code": "cadent"},
            "object_context": {"role": "luminary"},
            "motion_context": {"motion_state": "direct"},
            "visibility_context": {
                "horizon_position_id": 1,
                "horizon_position": "above_horizon",
                "altitude_deg": 12.5,
                "is_visible": true,
                "source": "calculated_altitude"
            }
        }
    }));

    assert!(is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_dignity_signal_without_structured_dignity() {
    let mut payload = current_payload();
    payload.signals.push(BasicSignal {
        signal_key: "dignity:saturn:domicile:capricorn".to_string(),
        theme_code: Some("functional_strength".to_string()),
        title: "Saturn strongly placed in Capricorn".to_string(),
        summary: Some("summary".to_string()),
        priority_score: 88.0,
        confidence_score: Some(0.95),
        interpretive_hint: Some("hint".to_string()),
        semantic_tags: vec!["dignity".to_string(), "saturn".to_string()],
        source_weight: Some(0.75),
        aggregation_group: Some("dignity:saturn".to_string()),
        aspect_context: None,
        evidence: Some(json!({
            "fact_type": "essential_dignity",
            "chart_object": "saturn",
            "sign_code": "capricorn",
            "dignity_type": "domicile"
        })),
    });

    assert!(!is_current_basic_payload(&payload));

    payload.dignities.push(BasicDignity {
        object_code: "saturn".to_string(),
        object_name: "Saturn".to_string(),
        sign_id: 10,
        sign_code: "capricorn".to_string(),
        sign_name: "Capricorn".to_string(),
        dignity_type: "domicile".to_string(),
        dignity_label: "Domicile".to_string(),
        polarity: "dignity".to_string(),
        strength_score: 1.0,
        signal_key: Some("dignity:saturn:domicile:capricorn".to_string()),
    });

    assert!(is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_dignity_signal_mismatched_with_structured_dignity() {
    let mut payload = current_payload();
    payload.signals.push(BasicSignal {
        signal_key: "dignity:saturn:domicile:capricorn".to_string(),
        theme_code: Some("functional_strength".to_string()),
        title: "Saturn strongly placed in Capricorn".to_string(),
        summary: Some("summary".to_string()),
        priority_score: 88.0,
        confidence_score: Some(0.95),
        interpretive_hint: Some("hint".to_string()),
        semantic_tags: vec!["dignity".to_string(), "saturn".to_string()],
        source_weight: Some(0.75),
        aggregation_group: Some("dignity:saturn".to_string()),
        aspect_context: None,
        evidence: Some(json!({
            "fact_type": "essential_dignity",
            "chart_object": "jupiter",
            "sign_code": "cancer",
            "dignity_type": "exaltation"
        })),
    });
    payload.dignities.push(BasicDignity {
        object_code: "saturn".to_string(),
        object_name: "Saturn".to_string(),
        sign_id: 10,
        sign_code: "capricorn".to_string(),
        sign_name: "Capricorn".to_string(),
        dignity_type: "domicile".to_string(),
        dignity_label: "Domicile".to_string(),
        polarity: "dignity".to_string(),
        strength_score: 1.0,
        signal_key: Some("dignity:saturn:domicile:capricorn".to_string()),
    });

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_requires_reading_plan() {
    let mut payload = current_payload();
    payload.reading_plan.clear();

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_reading_plan_with_missing_signal_key() {
    let mut payload = current_payload();
    payload.reading_plan[0]
        .source_signal_keys
        .push("object_position:moon".to_string());

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_repeated_primary_source_signal() {
    let mut payload = current_payload();
    payload.reading_plan.push(BasicReadingPlanItem {
        slot: "dominant_cluster".to_string(),
        title: "Dominant cluster".to_string(),
        source_signal_keys: vec!["object_position:sun".to_string()],
        primary_signal_keys: vec!["object_position:sun".to_string()],
        secondary_slot_candidates: Vec::new(),
    });

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_secondary_candidate_without_primary_source() {
    let mut payload = current_payload();
    payload.signals.push(BasicSignal {
        signal_key: "object_position:moon".to_string(),
        theme_code: Some("emotional_style".to_string()),
        title: "Moon in Pisces, house 4".to_string(),
        summary: Some("summary".to_string()),
        priority_score: 90.0,
        confidence_score: Some(0.95),
        interpretive_hint: Some("hint".to_string()),
        semantic_tags: vec!["placement".to_string()],
        source_weight: Some(0.75),
        aggregation_group: Some("pisces:house_4".to_string()),
        aspect_context: None,
        evidence: Some(json!({
            "fact_type": "object_position",
            "object_code": "moon",
            "placement_context": {
                "sign_context": {
                    "element": "water",
                    "modality": "mutable",
                    "polarity": "yin"
                },
                "house_modality": {"code": "angular"},
                "object_context": {"role": "luminary"},
                "motion_context": {"motion_state": "direct"}
            },
            "essential_dignities": []
        })),
    });

    let candidate = BasicSecondarySlotCandidate {
        signal_key: "object_position:moon".to_string(),
        primary_slot: "dominant_cluster".to_string(),
        candidate_slot: "core_identity".to_string(),
    };
    payload.reading_plan[0]
        .secondary_slot_candidates
        .push(candidate);

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_duplicate_reading_plan_slots() {
    let mut payload = current_payload();
    payload.reading_plan.push(BasicReadingPlanItem {
        slot: "core_identity".to_string(),
        title: "Duplicate".to_string(),
        source_signal_keys: vec!["object_position:sun".to_string()],
        primary_signal_keys: vec!["object_position:sun".to_string()],
        secondary_slot_candidates: Vec::new(),
    });

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_unknown_reading_plan_slot() {
    let mut payload = current_payload();
    payload.reading_plan[0].slot = "custom_slot".to_string();

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_out_of_order_reading_plan_slots() {
    let mut payload = current_payload();
    payload.signals.push(BasicSignal {
        signal_key: "object_position:mercury".to_string(),
        theme_code: Some("communication".to_string()),
        title: "Mercury in Gemini, house 9".to_string(),
        summary: Some("summary".to_string()),
        priority_score: 85.0,
        confidence_score: Some(0.95),
        interpretive_hint: Some("hint".to_string()),
        semantic_tags: vec!["placement".to_string()],
        source_weight: Some(0.75),
        aggregation_group: Some("gemini:house_9".to_string()),
        aspect_context: None,
        evidence: Some(json!({"fact_type": "object_position"})),
    });
    payload.reading_plan.insert(
        0,
        BasicReadingPlanItem {
            slot: "expression_style".to_string(),
            title: "Expression style".to_string(),
            source_signal_keys: vec!["object_position:mercury".to_string()],
            primary_signal_keys: vec!["object_position:mercury".to_string()],
            secondary_slot_candidates: Vec::new(),
        },
    );
    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_old_opposition_hint_template() {
    let mut payload = current_payload();
    payload.signals[0].interpretive_hint = Some(
            "Jupiter and Uranus are connected by a opposition, so their functions should be read together."
                .to_string(),
        );

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn reference_validation_requires_twelve_signs() {
    let mut references = reference_data();
    references.signs.pop();

    assert!(validate_calculation_references(&references).is_err());
}

#[test]
fn reference_validation_rejects_duplicate_house_numbers() {
    let mut references = reference_data();
    references.houses[1].number = 1;

    assert!(validate_calculation_references(&references).is_err());
}

#[test]
fn reference_validation_requires_house_modality_priority_delta() {
    let mut references = reference_data();
    references.houses[0].modality_priority_delta = None;

    assert!(validate_calculation_references(&references).is_err());
}

#[test]
fn reference_validation_requires_horizon_positions() {
    let mut references = reference_data();
    references
        .horizon_positions
        .retain(|position| position.code != "on_horizon");

    assert!(validate_calculation_references(&references).is_err());
}

#[test]
fn chart_object_validation_requires_signal_profile() {
    let mut objects = chart_objects();
    objects[0].source_weight = None;

    assert!(validate_chart_object_signal_profiles(&objects).is_err());
}

#[test]
fn chart_object_validation_requires_angle_priority_for_angles() {
    let mut objects = chart_objects();
    objects[1].angle_priority_base = None;

    assert!(validate_chart_object_signal_profiles(&objects).is_err());
}

#[test]
fn house_axis_reference_validation_requires_six_canonical_axes() {
    let mut axes = house_axis_references();
    axes.pop();

    assert!(validate_house_axis_references(&axes).is_err());
}

#[test]
fn house_axis_reference_validation_rejects_mismatched_theme_codes() {
    let mut axes = house_axis_references();
    axes[0].theme_a_code = "resources".to_string();

    assert!(validate_house_axis_references(&axes).is_err());
}

#[test]
fn house_axis_reference_validation_accepts_canonical_axes() {
    let axes = house_axis_references();

    assert!(validate_house_axis_references(&axes).is_ok());
}

#[test]
fn lunar_phase_reference_validation_requires_eight_phases() {
    let mut phases = lunar_phase_references();
    phases.pop();

    assert!(validate_lunar_phase_references(&phases).is_err());
}

#[test]
fn lunar_phase_reference_validation_rejects_mismatched_range() {
    let mut phases = lunar_phase_references();
    phases[1].range_end_deg = 68.0;

    assert!(validate_lunar_phase_references(&phases).is_err());
}

#[test]
fn lunar_phase_reference_validation_rejects_cycle_gap() {
    let mut phases = lunar_phase_references();
    phases[2].range_start_deg = 68.5;
    phases[2].range_end_deg = 113.5;
    phases[2].exact_anchor_deg = 91.0;

    assert!(validate_lunar_phase_references(&phases).is_err());
}

#[test]
fn lunar_phase_reference_validation_accepts_canonical_phases() {
    let phases = lunar_phase_references();

    assert!(validate_lunar_phase_references(&phases).is_ok());
}

fn chart_objects() -> Vec<ChartObject> {
    vec![
        ChartObject {
            id: 1,
            code: "sun".to_string(),
            name: "Sun".to_string(),
            swe_id: Some(0),
            role_code: Some("luminary".to_string()),
            role_label: Some("Luminary".to_string()),
            is_luminary: Some(true),
            is_planet_symbolic: Some(false),
            is_visible_to_naked_eye: Some(true),
            nature_codes: Some(json!(["luminary"])),
            position_priority_base: Some(100.0),
            angle_priority_base: None,
            source_weight: Some(1.0),
        },
        ChartObject {
            id: 11,
            code: "ascendant".to_string(),
            name: "Ascendant".to_string(),
            swe_id: None,
            role_code: Some("angle".to_string()),
            role_label: Some("Angle".to_string()),
            is_luminary: Some(false),
            is_planet_symbolic: Some(false),
            is_visible_to_naked_eye: Some(false),
            nature_codes: Some(json!(["angle"])),
            position_priority_base: Some(99.0),
            angle_priority_base: Some(99.0),
            source_weight: Some(1.0),
        },
    ]
}

fn house_axis_references() -> Vec<HouseAxisReference> {
    vec![
        house_axis_reference(
            "self_relationship",
            1,
            7,
            "identity",
            "relationships",
            "Self and Relationship",
        ),
        house_axis_reference(
            "resources_sharing",
            2,
            8,
            "resources",
            "shared_resources",
            "Resources and Sharing",
        ),
        house_axis_reference(
            "local_distant",
            3,
            9,
            "communication",
            "beliefs",
            "Local and Distant",
        ),
        house_axis_reference(
            "private_public",
            4,
            10,
            "roots",
            "career",
            "Private and Public",
        ),
        house_axis_reference(
            "creation_collective",
            5,
            11,
            "creativity",
            "community",
            "Creation and Collective",
        ),
        house_axis_reference(
            "control_surrender",
            6,
            12,
            "work_health",
            "inner_world",
            "Control and Surrender",
        ),
    ]
}

fn house_axis_reference(
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

fn lunar_phase_references() -> Vec<LunarPhaseReference> {
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

fn reference_data() -> CalculationReferenceData {
    CalculationReferenceData {
        signs: (1..=12)
            .map(|id| SignReference {
                id,
                code: format!("sign_{id}"),
                name: format!("Sign {id}"),
                element_code: Some("earth".to_string()),
                element_label: Some("Earth".to_string()),
                modality_code: Some("cardinal".to_string()),
                modality_name: Some("Cardinal".to_string()),
                polarity_code: Some("yin".to_string()),
                polarity_name: Some("Yin".to_string()),
                keywords_json: Some(json!(["structure"])),
                shadow_keywords_json: None,
            })
            .collect(),
        houses: (1..=12)
            .map(|number| HouseReference {
                id: number + 100,
                number,
                name: format!("House {number}"),
                theme_code: format!("house_{number}_theme"),
                modality_code: Some("angular".to_string()),
                modality_label: Some("Angular".to_string()),
                accidental_strength: Some("strong".to_string()),
                modality_priority_delta: Some(2.0),
                interpretation_weight: Some("high".to_string()),
            })
            .collect(),
        motion_states: vec![rust_sqlx_connection_test::models::MotionStateReference {
            id: 1,
            code: "direct".to_string(),
            label: "Direct".to_string(),
            motion_family: "forward".to_string(),
        }],
        horizon_positions: vec![
            rust_sqlx_connection_test::models::HorizonPositionReference {
                id: 1,
                code: "above_horizon".to_string(),
                label: "Above horizon".to_string(),
            },
            rust_sqlx_connection_test::models::HorizonPositionReference {
                id: 2,
                code: "below_horizon".to_string(),
                label: "Below horizon".to_string(),
            },
            rust_sqlx_connection_test::models::HorizonPositionReference {
                id: 3,
                code: "on_horizon".to_string(),
                label: "On horizon".to_string(),
            },
        ],
        angle_points: vec![
            angle_reference(
                1,
                "asc",
                "ASC",
                "Ascendant",
                "horizontal",
                Some("dsc"),
                1,
                11,
            ),
            angle_reference(
                2,
                "dsc",
                "DSC",
                "Descendant",
                "horizontal",
                Some("asc"),
                7,
                12,
            ),
            angle_reference(3, "mc", "MC", "Midheaven", "vertical", Some("ic"), 10, 13),
            angle_reference(4, "ic", "IC", "Imum Coeli", "vertical", Some("mc"), 4, 14),
        ],
    }
}

#[allow(clippy::too_many_arguments)]
fn angle_reference(
    id: i32,
    code: &str,
    short_label: &str,
    full_name: &str,
    axis: &str,
    opposite_angle_code: Option<&str>,
    associated_house: i32,
    chart_object_id: i32,
) -> AnglePointReference {
    AnglePointReference {
        id,
        code: code.to_string(),
        short_label: short_label.to_string(),
        full_name: full_name.to_string(),
        axis: axis.to_string(),
        opposite_angle_code: opposite_angle_code.map(ToString::to_string),
        associated_house,
        description: format!("{full_name} description"),
        chart_object_id,
        chart_object_code: full_name.to_ascii_lowercase().replace(' ', "_"),
        chart_object_name: full_name.to_string(),
        chart_object_sort_order: chart_object_id,
    }
}
