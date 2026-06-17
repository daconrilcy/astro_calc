use astral_calculator::domain::*;
use astral_calculator::features::signals::*;
use serde_json::json;

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

fn capricorn_house_2_position(id: i32, object_code: &str, object_name: &str) -> ObjectPositionFact {
    with_signal_scoring(ObjectPositionFact {
        chart_object_id: id,
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
        motion_state_id: None,
        horizon_position_id: None,
        longitude_deg: 270.0 + f64::from(id),
        latitude_deg: None,
        apparent_speed_deg_per_day: Some(1.0),
        altitude_deg: None,
        is_visible: None,
        facts_json: Some(json!({
            "house_context": {"theme_code": "resources"},
            "house_modality": {"code": "succedent"}
        })),
    })
}

fn position(
    id: i32,
    object_code: &str,
    object_name: &str,
    sign_code: &str,
    sign_name: &str,
    house_number: i32,
) -> ObjectPositionFact {
    with_signal_scoring(ObjectPositionFact {
        chart_object_id: id,
        object_code: object_code.to_string(),
        object_name: object_name.to_string(),
        zodiacal_reference_system_id: 1,
        coordinate_reference_system_id: 1,
        sign_id: id,
        sign_code: sign_code.to_string(),
        sign_name: sign_name.to_string(),
        house_id: Some(house_number),
        house_number: Some(house_number),
        house_name: Some(format!("House {house_number}")),
        motion_state_id: None,
        horizon_position_id: None,
        longitude_deg: f64::from(id) * 30.0,
        latitude_deg: None,
        apparent_speed_deg_per_day: Some(1.0),
        altitude_deg: None,
        is_visible: None,
        facts_json: Some(json!({
            "house_context": {"theme_code": format!("house_{house_number}_theme")},
            "house_modality": {"code": house_modality_code(house_number)}
        })),
    })
}

fn house_modality_code(house_number: i32) -> &'static str {
    match house_number {
        1 | 4 | 7 | 10 => "angular",
        2 | 5 | 8 | 11 => "succedent",
        _ => "cadent",
    }
}

fn enriched_position() -> ObjectPositionFact {
    let mut position = position(1, "sun", "Sun", "gemini", "Gemini", 9);
    position.facts_json = Some(json!({
        "sign_context": {
            "element": "air",
            "modality": "mutable",
            "polarity": "yang",
            "keywords": ["communication", "curiosity"]
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
    }));
    with_signal_scoring(position)
}

fn retrograde_mercury_position() -> ObjectPositionFact {
    let mut position = position(3, "mercury", "Mercury", "capricorn", "Capricorn", 3);
    position.facts_json = Some(json!({
        "sign_context": {
            "element": "earth",
            "modality": "cardinal",
            "polarity": "yin"
        },
        "house_context": {"theme_code": "communication"},
        "house_modality": {
            "code": "cadent"
        },
        "object_context": {
            "role": "planet"
        },
        "motion_context": {
            "motion_state": "retrograde",
            "label": "Retrograde",
            "motion_family": "reverse"
        }
    }));
    with_signal_scoring(position)
}

fn angle_position(
    id: i32,
    object_code: &str,
    object_name: &str,
    angle_point_code: &str,
    opposite_angle_code: &str,
    axis: &str,
) -> ObjectPositionFact {
    let mut position = position(id, object_code, object_name, "aries", "Aries", 1);
    position.apparent_speed_deg_per_day = None;
    position.motion_state_id = None;
    position.facts_json = Some(json!({
        "sign_context": {
            "element": "fire",
            "modality": "cardinal",
            "polarity": "yang"
        },
        "house_context": {"theme_code": "identity"},
        "house_modality": {"code": "angular"},
        "object_context": {"role": "angle"},
        "angle_context": {
            "angle_point_id": id,
            "angle_point_code": angle_point_code,
            "short_label": angle_point_code.to_ascii_uppercase(),
            "full_name": object_name,
            "axis": axis,
            "opposite_angle_code": opposite_angle_code,
            "associated_house_number": 1,
            "chart_object_sort_order": id
        }
    }));
    position
}

fn aspect(
    source_code: &str,
    source_name: &str,
    target_code: &str,
    target_name: &str,
    aspect_code: &str,
    aspect_name: &str,
    strength_score: f64,
) -> AspectFact {
    AspectFact {
        source_chart_object_id: 1,
        source_object_code: source_code.to_string(),
        source_object_name: source_name.to_string(),
        target_chart_object_id: 2,
        target_object_code: target_code.to_string(),
        target_object_name: target_name.to_string(),
        aspect_id: 1,
        aspect_code: aspect_code.to_string(),
        aspect_name: aspect_name.to_string(),
        aspect_family: "major".to_string(),
        orb_deg: 1.0,
        phase_state: "applying".to_string(),
        is_applying: true,
        is_exact: false,
        strength_score: Some(strength_score),
        primary_valence: primary_valence_for_test(aspect_code).map(ToString::to_string),
        intensity_modifier: intensity_modifier_for_test(aspect_code).map(ToString::to_string),
        secondary_effect: None,
        valence_family: primary_valence_for_test(aspect_code)
            .map(|_| "tonal".to_string())
            .or_else(|| intensity_modifier_for_test(aspect_code).map(|_| "intensity".to_string())),
        valence_is_tonal: primary_valence_for_test(aspect_code)
            .map(|_| true)
            .or_else(|| intensity_modifier_for_test(aspect_code).map(|_| false)),
        valence_is_intensity_modifier: primary_valence_for_test(aspect_code)
            .map(|_| false)
            .or_else(|| intensity_modifier_for_test(aspect_code).map(|_| true)),
        calculation_notes_json: None,
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

trait AspectFactTestExt {
    fn with_intensity_modifier(self, intensity_modifier: &str) -> Self;
}

impl AspectFactTestExt for AspectFact {
    fn with_intensity_modifier(mut self, intensity_modifier: &str) -> Self {
        self.intensity_modifier = Some(intensity_modifier.to_string());
        self
    }
}

#[test]
fn major_dignities_create_dedicated_signals_and_enrich_placements() {
    let facts = CalculatedChartFacts {
        positions: vec![
            position(6, "jupiter", "Jupiter", "cancer", "Cancer", 8),
            position(7, "saturn", "Saturn", "capricorn", "Capricorn", 2),
        ],
        house_cusps: Vec::new(),
        aspects: Vec::new(),
    };

    let signals = aggregate_basic_signals(&facts, &astral_calculator::catalog::test_catalog());
    let saturn_dignity = signals
        .iter()
        .find(|signal| signal.signal_key == "dignity:saturn:domicile:capricorn")
        .expect("expected Saturn domicile dignity signal");
    let jupiter_dignity = signals
        .iter()
        .find(|signal| signal.signal_key == "dignity:jupiter:exaltation:cancer")
        .expect("expected Jupiter exaltation dignity signal");
    let saturn_placement = signals
        .iter()
        .find(|signal| signal.signal_key == "object_position:saturn")
        .expect("expected Saturn placement signal");

    assert_eq!(
        saturn_dignity.theme_code.as_deref(),
        Some("functional_strength")
    );
    assert_eq!(saturn_dignity.priority_score, 83.0);
    assert_eq!(jupiter_dignity.priority_score, 81.0);
    assert_eq!(
        saturn_dignity
            .payload_json
            .as_ref()
            .and_then(|payload| payload.get("evidence"))
            .and_then(|evidence| evidence.get("dignity_type"))
            .and_then(|value| value.as_str()),
        Some("domicile")
    );
    assert_eq!(
        saturn_placement
            .payload_json
            .as_ref()
            .and_then(|payload| payload.get("evidence"))
            .and_then(|evidence| evidence.get("essential_dignities"))
            .and_then(|dignities| dignities.as_array())
            .and_then(|dignities| dignities.first())
            .and_then(|dignity| dignity.get("dignity_type"))
            .and_then(|value| value.as_str()),
        Some("domicile")
    );
    assert_eq!(
        jupiter_dignity
            .payload_json
            .as_ref()
            .and_then(|payload| payload.get("interpretive_hint"))
            .and_then(|value| value.as_str()),
        Some(
            "Treat Jupiter in Cancer as an exaltation modifier for the existing placement signal."
        )
    );
}

#[test]
fn double_dignity_positions_create_all_signals_and_evidence() {
    let facts = CalculatedChartFacts {
        positions: vec![position(3, "mercury", "Mercury", "virgo", "Virgo", 6)],
        house_cusps: Vec::new(),
        aspects: Vec::new(),
    };

    let signals = aggregate_basic_signals(&facts, &astral_calculator::catalog::test_catalog());
    let placement = signals
        .iter()
        .find(|signal| signal.signal_key == "object_position:mercury")
        .expect("expected Mercury placement");

    assert!(signals
        .iter()
        .any(|signal| signal.signal_key == "dignity:mercury:domicile:virgo"));
    assert!(signals
        .iter()
        .any(|signal| signal.signal_key == "dignity:mercury:exaltation:virgo"));
    assert_eq!(
        placement
            .payload_json
            .as_ref()
            .and_then(|payload| payload.get("evidence"))
            .and_then(|evidence| evidence.get("essential_dignities"))
            .and_then(|value| value.as_array())
            .map(Vec::len),
        Some(2)
    );
    assert_eq!(placement.priority_score, 93.25);
}

#[test]
fn basic_signals_include_semantic_position_cluster() {
    let facts = CalculatedChartFacts {
        positions: vec![
            capricorn_house_2_position(1, "sun", "Sun"),
            capricorn_house_2_position(6, "saturn", "Saturn"),
            capricorn_house_2_position(8, "uranus", "Uranus"),
        ],
        house_cusps: Vec::new(),
        aspects: Vec::new(),
    };

    let signals = aggregate_basic_signals(&facts, &astral_calculator::catalog::test_catalog());
    let cluster = signals
        .iter()
        .find(|signal| signal.signal_key == "cluster:capricorn:house_2")
        .expect("expected a Capricorn house 2 cluster");

    assert_eq!(cluster.theme_code.as_deref(), Some("resources"));
    assert_eq!(cluster.suppression_state, "active");
    let payload = cluster.payload_json.as_ref().expect("cluster payload");
    assert_eq!(
        payload
            .get("aggregation_group")
            .and_then(|value| value.as_str()),
        Some("capricorn_house_2_cluster")
    );
    assert!(payload
        .get("semantic_tags")
        .and_then(|value| value.as_array())
        .expect("semantic tags")
        .iter()
        .any(|value| value.as_str() == Some("responsibility")));
    assert_eq!(
        payload
            .get("evidence")
            .and_then(|value| value.get("source_signals"))
            .and_then(|value| value.as_array())
            .map(Vec::len),
        Some(3)
    );
}

#[test]
fn placement_signal_includes_contextual_evidence_and_tags() {
    let facts = CalculatedChartFacts {
        positions: vec![enriched_position()],
        house_cusps: Vec::new(),
        aspects: Vec::new(),
    };

    let signals = aggregate_basic_signals(&facts, &astral_calculator::catalog::test_catalog());
    let signal = signals
        .iter()
        .find(|signal| signal.signal_key == "object_position:sun")
        .expect("expected Sun signal");
    let payload = signal.payload_json.as_ref().expect("signal payload");
    let tags = payload
        .get("semantic_tags")
        .and_then(|value| value.as_array())
        .expect("semantic tags");

    assert!(tags.iter().any(|tag| tag.as_str() == Some("air")));
    assert!(tags.iter().any(|tag| tag.as_str() == Some("mutable")));
    assert!(tags.iter().any(|tag| tag.as_str() == Some("cadent")));
    assert_eq!(
        payload
            .get("evidence")
            .and_then(|evidence| evidence.get("placement_context"))
            .and_then(|context| context.get("motion_context"))
            .and_then(|motion| motion.get("motion_state"))
            .and_then(|value| value.as_str()),
        Some("direct")
    );
    assert!(payload
        .get("evidence")
        .and_then(|evidence| evidence.get("placement_context"))
        .and_then(|context| context.get("visibility_context"))
        .is_some_and(|value| value.is_object()));
}

#[test]
fn retrograde_placements_get_specific_interpretive_context() {
    let facts = CalculatedChartFacts {
        positions: vec![retrograde_mercury_position()],
        house_cusps: Vec::new(),
        aspects: Vec::new(),
    };

    let signals = aggregate_basic_signals(&facts, &astral_calculator::catalog::test_catalog());
    let signal = signals
        .iter()
        .find(|signal| signal.signal_key == "object_position:mercury")
        .expect("expected Mercury signal");
    let payload = signal.payload_json.as_ref().expect("signal payload");

    assert!(signal
        .summary
        .as_deref()
        .expect("summary")
        .contains("retrograde motion"));
    assert!(payload
        .get("interpretive_hint")
        .and_then(|value| value.as_str())
        .expect("hint")
        .contains("internal processing"));
    assert!(payload.get("writing_guidance").is_none());
}

#[test]
fn basic_cluster_merges_secondary_source_signals() {
    let facts = CalculatedChartFacts {
        positions: vec![
            capricorn_house_2_position(1, "sun", "Sun"),
            capricorn_house_2_position(6, "saturn", "Saturn"),
            capricorn_house_2_position(8, "uranus", "Uranus"),
        ],
        house_cusps: Vec::new(),
        aspects: Vec::new(),
    };

    let signals = aggregate_basic_signals(&facts, &astral_calculator::catalog::test_catalog());
    let sun = signals
        .iter()
        .find(|signal| signal.signal_key == "object_position:sun")
        .expect("expected Sun signal");
    let saturn = signals
        .iter()
        .find(|signal| signal.signal_key == "object_position:saturn")
        .expect("expected Saturn signal");

    assert_eq!(sun.suppression_state, "active");
    assert_eq!(saturn.suppression_state, "merged");
    assert_eq!(
        saturn
            .payload_json
            .as_ref()
            .and_then(|payload| payload.get("editorial_state"))
            .and_then(|state| state.get("cluster_signal_key"))
            .and_then(|value| value.as_str()),
        Some("cluster:capricorn:house_2")
    );
}

#[test]
fn basic_cluster_dedup_refills_without_reactivating_weak_aspects() {
    let facts = CalculatedChartFacts {
        positions: vec![
            capricorn_house_2_position(1, "sun", "Sun"),
            position(2, "moon", "Moon", "cancer", "Cancer", 4),
            position(3, "mercury", "Mercury", "gemini", "Gemini", 3),
            position(4, "venus", "Venus", "taurus", "Taurus", 5),
            position(5, "mars", "Mars", "aries", "Aries", 1),
            position(6, "jupiter", "Jupiter", "sagittarius", "Sagittarius", 9),
            capricorn_house_2_position(7, "saturn", "Saturn"),
            capricorn_house_2_position(8, "uranus", "Uranus"),
            capricorn_house_2_position(9, "neptune", "Neptune"),
            position(10, "pluto", "Pluto", "scorpio", "Scorpio", 8),
            position(11, "north_node", "North Node", "aquarius", "Aquarius", 11),
        ],
        house_cusps: Vec::new(),
        aspects: vec![
            aspect("sun", "Sun", "moon", "Moon", "trine", "Trine", 0.99),
            aspect(
                "mercury", "Mercury", "venus", "Venus", "sextile", "Sextile", 0.98,
            ),
            aspect(
                "mars", "Mars", "jupiter", "Jupiter", "square", "Square", 0.97,
            ),
            aspect(
                "saturn",
                "Saturn",
                "pluto",
                "Pluto",
                "opposition",
                "Opposition",
                0.2,
            ),
        ],
    };

    let signals = aggregate_basic_signals(&facts, &astral_calculator::catalog::test_catalog());
    let active_count = signals
        .iter()
        .filter(|signal| signal.suppression_state == "active")
        .count();
    let weak_aspect = signals
        .iter()
        .find(|signal| signal.signal_key == "aspect:saturn:pluto:opposition")
        .expect("expected weak aspect signal");
    let jupiter_dignity = signals
        .iter()
        .find(|signal| signal.signal_key == "dignity:jupiter:domicile:sagittarius")
        .expect("expected Jupiter dignity signal");

    assert_eq!(active_count, BASIC_MAX_ACTIVE_SIGNALS);
    assert_eq!(weak_aspect.suppression_state, "suppressed");
    assert_eq!(jupiter_dignity.suppression_state, "active");
}

#[test]
fn basic_filter_preserves_one_strong_tension_aspect() {
    let facts = CalculatedChartFacts {
        positions: vec![
            capricorn_house_2_position(1, "sun", "Sun"),
            position(2, "moon", "Moon", "cancer", "Cancer", 4),
            position(3, "mercury", "Mercury", "gemini", "Gemini", 3),
            position(4, "venus", "Venus", "taurus", "Taurus", 5),
            position(5, "mars", "Mars", "aries", "Aries", 1),
            position(6, "jupiter", "Jupiter", "sagittarius", "Sagittarius", 9),
            capricorn_house_2_position(7, "saturn", "Saturn"),
            capricorn_house_2_position(8, "uranus", "Uranus"),
            capricorn_house_2_position(9, "neptune", "Neptune"),
            position(10, "pluto", "Pluto", "scorpio", "Scorpio", 8),
            position(11, "north_node", "North Node", "aquarius", "Aquarius", 11),
        ],
        house_cusps: Vec::new(),
        aspects: vec![
            aspect("sun", "Sun", "moon", "Moon", "trine", "Trine", 0.99),
            aspect(
                "mercury", "Mercury", "venus", "Venus", "sextile", "Sextile", 0.98,
            ),
            aspect(
                "sun",
                "Sun",
                "neptune",
                "Neptune",
                "conjunction",
                "Conjunction",
                0.97,
            ),
            aspect(
                "moon", "Moon", "neptune", "Neptune", "sextile", "Sextile", 0.96,
            ),
            aspect(
                "jupiter",
                "Jupiter",
                "uranus",
                "Uranus",
                "opposition",
                "Opposition",
                0.88,
            ),
        ],
    };

    let signals = aggregate_basic_signals(&facts, &astral_calculator::catalog::test_catalog());
    let active_count = signals
        .iter()
        .filter(|signal| signal.suppression_state == "active")
        .count();
    let tension = signals
        .iter()
        .find(|signal| signal.signal_key == "aspect:jupiter:uranus:opposition")
        .expect("expected strong opposition");

    assert_eq!(active_count, BASIC_MAX_ACTIVE_SIGNALS);
    assert_eq!(tension.suppression_state, "active");
}

#[test]
fn structural_axis_does_not_block_non_structural_tension_preservation() {
    let facts = CalculatedChartFacts {
        positions: vec![
            capricorn_house_2_position(1, "sun", "Sun"),
            position(2, "moon", "Moon", "cancer", "Cancer", 4),
            position(3, "mercury", "Mercury", "gemini", "Gemini", 3),
            position(4, "venus", "Venus", "taurus", "Taurus", 5),
            position(5, "mars", "Mars", "aries", "Aries", 1),
            position(6, "jupiter", "Jupiter", "sagittarius", "Sagittarius", 9),
            capricorn_house_2_position(7, "saturn", "Saturn"),
            capricorn_house_2_position(8, "uranus", "Uranus"),
            capricorn_house_2_position(9, "neptune", "Neptune"),
            position(10, "pluto", "Pluto", "scorpio", "Scorpio", 8),
            angle_position(11, "ascendant", "Ascendant", "asc", "dsc", "horizontal"),
            angle_position(12, "descendant", "Descendant", "dsc", "asc", "horizontal"),
            angle_position(13, "ic", "IC", "ic", "mc", "vertical"),
        ],
        house_cusps: Vec::new(),
        aspects: vec![
            aspect(
                "ascendant",
                "Ascendant",
                "descendant",
                "Descendant",
                "opposition",
                "Opposition",
                1.0,
            ),
            aspect(
                "descendant",
                "Descendant",
                "ic",
                "IC",
                "square",
                "Square",
                0.995,
            ),
            aspect("sun", "Sun", "moon", "Moon", "trine", "Trine", 0.99),
            aspect(
                "mercury", "Mercury", "venus", "Venus", "sextile", "Sextile", 0.98,
            ),
            aspect(
                "sun",
                "Sun",
                "neptune",
                "Neptune",
                "conjunction",
                "Conjunction",
                0.97,
            ),
            aspect(
                "moon", "Moon", "neptune", "Neptune", "sextile", "Sextile", 0.96,
            ),
            aspect(
                "jupiter",
                "Jupiter",
                "uranus",
                "Uranus",
                "opposition",
                "Opposition",
                0.88,
            ),
        ],
    };

    let signals = aggregate_basic_signals(&facts, &astral_calculator::catalog::test_catalog());
    let structural_axis = signals
        .iter()
        .find(|signal| signal.signal_key == "aspect:ascendant:descendant:opposition");
    let tension = signals
        .iter()
        .find(|signal| signal.signal_key == "aspect:jupiter:uranus:opposition")
        .expect("expected non-structural strong opposition");
    let angle_to_angle = signals
        .iter()
        .find(|signal| signal.signal_key == "aspect:descendant:ic:square")
        .expect("expected angle-to-angle aspect signal");

    assert!(structural_axis.is_none());
    assert_eq!(tension.suppression_state, "active");
    assert_ne!(angle_to_angle.suppression_state, "active");
}

#[test]
fn angle_signal_evidence_exposes_opposite_angle_object_code() {
    let facts = CalculatedChartFacts {
        positions: vec![
            angle_position(11, "ascendant", "Ascendant", "asc", "dsc", "horizontal"),
            angle_position(12, "descendant", "Descendant", "dsc", "asc", "horizontal"),
        ],
        house_cusps: Vec::new(),
        aspects: Vec::new(),
    };

    let signals = aggregate_basic_signals(&facts, &astral_calculator::catalog::test_catalog());
    let ascendant = signals
        .iter()
        .find(|signal| signal.signal_key == "angle:ascendant:sign:aries")
        .expect("expected ascendant signal");
    let evidence = ascendant
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get("evidence"))
        .expect("expected angle evidence");
    let angle_context = ascendant
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get("angle_context"))
        .expect("expected angle context");

    assert_eq!(
        evidence
            .get("opposite_angle_code")
            .and_then(|value| value.as_str()),
        Some("dsc")
    );
    assert_eq!(
        evidence
            .get("opposite_angle_object_code")
            .and_then(|value| value.as_str()),
        Some("descendant")
    );
    assert_eq!(
        angle_context
            .get("opposite_angle_object_code")
            .and_then(|value| value.as_str()),
        Some("descendant")
    );
}

#[test]
fn angle_signal_uses_angle_context_even_without_angle_point_id() {
    let mut ascendant = angle_position(11, "ascendant", "Ascendant", "asc", "dsc", "horizontal");
    if let Some(facts) = ascendant.facts_json.as_mut() {
        facts
            .get_mut("angle_context")
            .and_then(|context| context.as_object_mut())
            .expect("angle context")
            .remove("angle_point_id");
    }
    let facts = CalculatedChartFacts {
        positions: vec![
            ascendant,
            angle_position(12, "descendant", "Descendant", "dsc", "asc", "horizontal"),
        ],
        house_cusps: Vec::new(),
        aspects: Vec::new(),
    };

    let signals = aggregate_basic_signals(&facts, &astral_calculator::catalog::test_catalog());

    assert!(signals
        .iter()
        .any(|signal| signal.signal_key == "angle:ascendant:sign:aries"));
    assert!(!signals
        .iter()
        .any(|signal| signal.signal_key == "object_position:ascendant"));
}

#[test]
fn structural_axis_aspects_do_not_create_basic_aspect_signals() {
    let mut structural_axis = aspect(
        "ascendant",
        "Ascendant",
        "descendant",
        "Descendant",
        "opposition",
        "Opposition",
        1.0,
    );
    structural_axis.calculation_notes_json = Some(json!({
        "is_structural_axis": true
    }));

    let facts = CalculatedChartFacts {
        positions: Vec::new(),
        house_cusps: Vec::new(),
        aspects: vec![
            structural_axis,
            aspect("moon", "Moon", "mars", "Mars", "square", "Square", 0.9),
        ],
    };

    let signals = aggregate_basic_signals(&facts, &astral_calculator::catalog::test_catalog());

    assert!(!signals
        .iter()
        .any(|signal| signal.signal_key == "aspect:ascendant:descendant:opposition"));
    assert!(signals
        .iter()
        .any(|signal| signal.signal_key == "aspect:moon:mars:square"));
}

#[test]
fn indefinite_article_handles_opposition() {
    assert_eq!(indefinite_article("opposition"), "an");
    assert_eq!(indefinite_article("exaltation"), "an");
    assert_eq!(indefinite_article("square"), "a");
}

#[test]
fn aspect_hint_uses_interpretive_quality() {
    let facts = CalculatedChartFacts {
        positions: Vec::new(),
        house_cusps: Vec::new(),
        aspects: vec![AspectFact {
            source_chart_object_id: 6,
            source_object_code: "jupiter".to_string(),
            source_object_name: "Jupiter".to_string(),
            target_chart_object_id: 8,
            target_object_code: "uranus".to_string(),
            target_object_name: "Uranus".to_string(),
            aspect_id: 5,
            aspect_code: "opposition".to_string(),
            aspect_name: "Opposition".to_string(),
            aspect_family: "major".to_string(),
            orb_deg: 0.7586,
            phase_state: "separating".to_string(),
            is_applying: false,
            is_exact: false,
            strength_score: Some(0.9052),
            primary_valence: Some("polarizing".to_string()),
            intensity_modifier: None,
            secondary_effect: None,
            valence_family: Some("tonal".to_string()),
            valence_is_tonal: Some(true),
            valence_is_intensity_modifier: Some(false),
            calculation_notes_json: None,
        }],
    };

    let signals = aggregate_basic_signals(&facts, &astral_calculator::catalog::test_catalog());
    let payload = signals[0].payload_json.as_ref().expect("aspect payload");

    assert_eq!(
            signals[0].summary.as_deref(),
            Some("Jupiter and Uranus form an opposition with 0.76 degrees of orb; the phase is separating.")
        );
    assert_eq!(
            payload
                .get("interpretive_hint")
                .and_then(|value| value.as_str()),
            Some("Read this opposition as a polarity to balance between Jupiter and Uranus, with attention to the separating phase.")
        );
}

#[test]
fn aspect_signals_include_interpretive_context_and_valence_tags() {
    let facts = CalculatedChartFacts {
        positions: Vec::new(),
        house_cusps: Vec::new(),
        aspects: vec![
            aspect(
                "venus", "Venus", "jupiter", "Jupiter", "sextile", "Sextile", 0.9,
            ),
            aspect("venus", "Venus", "pluto", "Pluto", "trine", "Trine", 0.89)
                .with_intensity_modifier("amplifying"),
            aspect("moon", "Moon", "mars", "Mars", "square", "Square", 0.88),
            aspect(
                "sun",
                "Sun",
                "neptune",
                "Neptune",
                "conjunction",
                "Conjunction",
                0.86,
            ),
        ],
    };

    let signals = aggregate_basic_signals(&facts, &astral_calculator::catalog::test_catalog());
    let sextile = aspect_payload(&signals, "aspect:venus:jupiter:sextile");
    let amplified_trine = aspect_payload(&signals, "aspect:venus:pluto:trine");
    let square = aspect_payload(&signals, "aspect:moon:mars:square");
    let conjunction = aspect_payload(&signals, "aspect:sun:neptune:conjunction");

    assert_eq!(
        sextile
            .get("aspect_context")
            .and_then(|context| context.get("primary_valence"))
            .and_then(|value| value.as_str()),
        Some("supportive")
    );
    assert_eq!(
            sextile
                .get("interpretive_hint")
                .and_then(|value| value.as_str()),
            Some("Read this sextile as a supportive flow between Venus and Jupiter, with attention to the applying phase.")
        );
    assert!(sextile
        .get("semantic_tags")
        .and_then(|value| value.as_array())
        .expect("semantic tags")
        .iter()
        .any(|tag| tag.as_str() == Some("flow")));
    assert_eq!(
            amplified_trine
                .get("interpretive_hint")
                .and_then(|value| value.as_str()),
            Some("Read this trine as a natural flow with extra emphasis between Venus and Pluto, with attention to the applying phase.")
        );
    assert_eq!(
        square
            .get("aspect_context")
            .and_then(|context| context.get("primary_valence"))
            .and_then(|value| value.as_str()),
        Some("dynamic_challenging")
    );
    assert_eq!(
            square
                .get("interpretive_hint")
                .and_then(|value| value.as_str()),
            Some("Read this square as an active tension between Moon and Mars, with attention to the applying phase.")
        );
    assert!(square
        .get("semantic_tags")
        .and_then(|value| value.as_array())
        .expect("semantic tags")
        .iter()
        .any(|tag| tag.as_str() == Some("tension")));
    assert_eq!(
        conjunction
            .get("aspect_context")
            .and_then(|context| context.get("primary_valence")),
        Some(&serde_json::Value::Null)
    );
    assert_eq!(
        conjunction
            .get("aspect_context")
            .and_then(|context| context.get("intensity_modifier"))
            .and_then(|value| value.as_str()),
        Some("amplifying")
    );
    assert_eq!(
            conjunction
                .get("interpretive_hint")
                .and_then(|value| value.as_str()),
            Some("Read this conjunction as an amplifying contact between Sun and Neptune, with attention to the applying phase.")
        );
    assert_eq!(
        conjunction
            .get("aspect_context")
            .and_then(|context| context.get("valence_family"))
            .and_then(|value| value.as_str()),
        Some("intensity")
    );
    assert_eq!(
        conjunction
            .get("aspect_context")
            .and_then(|context| context.get("is_intensity_modifier"))
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    assert!(conjunction.get("writing_guidance").is_none());
    assert_eq!(
        conjunction
            .get("aspect_context")
            .and_then(|context| context.get("is_tonal_valence"))
            .and_then(|value| value.as_bool()),
        Some(false)
    );
}

fn aspect_payload<'a>(
    signals: &'a [InterpretationSignalDraft],
    signal_key: &str,
) -> &'a serde_json::Value {
    signals
        .iter()
        .find(|signal| signal.signal_key == signal_key)
        .and_then(|signal| signal.payload_json.as_ref())
        .expect("aspect payload")
}
