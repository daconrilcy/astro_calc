use chrono::{TimeZone, Utc};
use serde_json::json;

use rust_sqlx_connection_test::domain::{
    BasicAngleFact, BasicChartEmphasis, BasicDignity, BasicDominantHouse, BasicDominantObject,
    BasicDominantSign, BasicDraftingPlanItem, BasicEmphasisRefs, BasicLlmHandoffContract,
    BasicObjectPosition, BasicPayload, BasicReadingPlanItem, BasicSecondarySlotCandidate,
    BasicSignal, CalculationReferenceData,
};
use rust_sqlx_connection_test::models::{AnglePointReference, HouseReference, SignReference};
use rust_sqlx_connection_test::runtime::{
    is_current_basic_payload, validate_calculation_references,
};

fn current_payload() -> BasicPayload {
    BasicPayload {
            product_code: "basic".to_string(),
            chart_calculation_id: 1,
            reference_version_id: 1,
            subject_label: None,
            birth_datetime_utc: Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap(),
            llm_handoff_contract: Some(BasicLlmHandoffContract {
                contract_version: "basic_natal_structured_v7".to_string(),
                payload_language_code: "en".to_string(),
                target_language_policy: "provided_by_llm_service".to_string(),
                audience_level: "beginner".to_string(),
                tone: "clear, warm, non fatalistic".to_string(),
                must_use: vec![
                    "chart_emphasis".to_string(),
                    "dignities".to_string(),
                    "angles".to_string(),
                    "signals".to_string(),
                    "reading_plan".to_string(),
                    "drafting_plan".to_string(),
                ],
                must_not: vec![
                    "invent facts not present in source signals".to_string(),
                    "mention technical IDs".to_string(),
                    "list placements mechanically".to_string(),
                    "translate technical keys such as signal_key, theme_code, semantic_tags, slot, or aggregation_group".to_string(),
                    "expose raw evidence unless explicitly requested".to_string(),
                    "treat chart_emphasis as a standalone section instead of weighting context".to_string(),
                    "make deterministic or fatalistic predictions".to_string(),
                ],
                output_format: "structured_sections".to_string(),
            }),
            positions: vec![BasicObjectPosition {
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
            }],
            angles: vec![
                angle_fact("ascendant", "Ascendant", "horizontal", "descendant", 84.0, 1),
                angle_fact("descendant", "Descendant", "horizontal", "ascendant", 264.0, 7),
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
                    writing_guidance: Some("guidance".to_string()),
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
                    writing_guidance: Some("guidance".to_string()),
                    aspect_context: None,
                    evidence: Some(json!({
                        "fact_type": "chart_angle",
                        "angle_code": "ascendant",
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
            drafting_plan: vec![BasicDraftingPlanItem {
                slot: "core_identity".to_string(),
                section_title: "Core chart markers".to_string(),
                source_signal_keys: vec!["object_position:sun".to_string()],
                primary_signal_keys: vec!["object_position:sun".to_string()],
                secondary_slot_candidates: Vec::new(),
                emphasis_refs: BasicEmphasisRefs {
                    dominant_signs: vec!["gemini".to_string()],
                    dominant_houses: vec![9],
                    dominant_objects: vec!["sun".to_string()],
                },
                writing_objective: "Explain the central markers.".to_string(),
                max_words: 110,
                avoid: vec![
                    "use technical IDs".to_string(),
                    "turn chart_emphasis into a standalone section".to_string(),
                ],
            }],
        }
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

#[test]
fn current_payload_requires_canonical_llm_handoff_contract() {
    let mut payload = current_payload();
    payload
        .llm_handoff_contract
        .as_mut()
        .expect("llm handoff contract")
        .payload_language_code = "fr".to_string();

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_requires_llm_handoff_contract() {
    let mut payload = current_payload();
    payload.llm_handoff_contract = None;

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_requires_signals() {
    let mut payload = current_payload();
    payload.signals.clear();

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
        writing_guidance: Some(
            "Use the aspect as a relationship between two chart factors.".to_string(),
        ),
        aspect_context: Some(json!({
            "aspect_family": "major",
            "primary_valence": null,
            "intensity_modifier": null,
            "secondary_effect": null,
            "dynamic_quality": "contextual",
            "phase_state": "separating",
            "writing_guidance": "Use the aspect as a relationship between two chart factors."
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
        "is_intensity_modifier": true,
        "writing_guidance": "Treat amplifying as an intensity modifier."
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
        writing_guidance: Some("Present as a polarity to balance.".to_string()),
        aspect_context: Some(json!({
            "aspect_family": "major",
            "primary_valence": "polarizing",
            "intensity_modifier": null,
            "secondary_effect": null,
            "dynamic_quality": "tension",
            "phase_state": "exact",
            "valence_family": "tonal",
            "is_tonal_valence": true,
            "is_intensity_modifier": false,
            "writing_guidance": "Present as a polarity to balance."
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
            "motion_context": {"motion_state": "direct"}
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
            "motion_context": {"motion_state": "direct"}
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
        writing_guidance: Some("guidance".to_string()),
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
        writing_guidance: Some("guidance".to_string()),
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
fn current_payload_requires_drafting_plan() {
    let mut payload = current_payload();
    payload.drafting_plan.clear();

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_misaligned_emphasis_refs() {
    let mut payload = current_payload();
    payload.drafting_plan[0]
        .emphasis_refs
        .dominant_signs
        .clear();

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_drafting_plan_without_chart_emphasis_guardrail() {
    let mut payload = current_payload();
    payload.drafting_plan[0]
        .avoid
        .retain(|rule| rule != "turn chart_emphasis into a standalone section");

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
    payload.drafting_plan.push(BasicDraftingPlanItem {
        slot: "dominant_cluster".to_string(),
        section_title: "Dominant cluster".to_string(),
        source_signal_keys: vec!["object_position:sun".to_string()],
        primary_signal_keys: vec!["object_position:sun".to_string()],
        secondary_slot_candidates: Vec::new(),
        emphasis_refs: BasicEmphasisRefs::default(),
        writing_objective: "Explain the repeated primary signal.".to_string(),
        max_words: 120,
        avoid: vec![
            "repeat".to_string(),
            "turn chart_emphasis into a standalone section".to_string(),
        ],
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
        writing_guidance: Some("guidance".to_string()),
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
        .push(candidate.clone());
    payload.drafting_plan[0]
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
    payload.drafting_plan[0].slot = "custom_slot".to_string();

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
        writing_guidance: Some("guidance".to_string()),
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
    payload.drafting_plan.insert(
        0,
        BasicDraftingPlanItem {
            slot: "expression_style".to_string(),
            section_title: "Expression and action style".to_string(),
            source_signal_keys: vec!["object_position:mercury".to_string()],
            primary_signal_keys: vec!["object_position:mercury".to_string()],
            secondary_slot_candidates: Vec::new(),
            emphasis_refs: BasicEmphasisRefs::default(),
            writing_objective: "Show how the person thinks and acts.".to_string(),
            max_words: 110,
            avoid: vec![
                "use technical IDs".to_string(),
                "turn chart_emphasis into a standalone section".to_string(),
            ],
        },
    );

    assert!(!is_current_basic_payload(&payload));
}

#[test]
fn current_payload_rejects_drafting_plan_with_missing_signal_key() {
    let mut payload = current_payload();
    payload.drafting_plan[0]
        .source_signal_keys
        .push("object_position:moon".to_string());

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
                interpretation_weight: Some("high".to_string()),
            })
            .collect(),
        motion_states: vec![rust_sqlx_connection_test::models::MotionStateReference {
            id: 1,
            code: "direct".to_string(),
            label: "Direct".to_string(),
            motion_family: "forward".to_string(),
        }],
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
