mod common;

use common::json_db::{
    major_aspect_definitions_from_json_db_seed, major_aspect_family_expected_count_from_json_db_seed,
    major_aspect_family_max_default_orb_deg_from_json_db_seed,
};
use astral_calculator::aspects::{canonical_aspect_orb_deg, detect_aspects};
use astral_calculator::domain::ObjectPositionFact;
use astral_calculator::models::AspectDefinition;
use astral_calculator::runtime::validate_aspect_definitions;
use serde_json::json;

fn single_aspect_definition(code: &str, angle: f64, orb: f64) -> Vec<AspectDefinition> {
    vec![AspectDefinition {
        id: 1,
        code: code.to_string(),
        name: code.to_string(),
        angle,
        family: "major".to_string(),
        default_orb_deg: Some(orb),
        max_default_orb_deg: major_aspect_family_max_default_orb_deg_from_json_db_seed(),
    }]
}

fn validate_major_aspects(aspects: &[AspectDefinition]) -> Result<(), astral_calculator::runtime::RuntimeError> {
    validate_aspect_definitions(
        aspects,
        8.0,
        major_aspect_family_expected_count_from_json_db_seed(),
        major_aspect_family_max_default_orb_deg_from_json_db_seed(),
    )
}

fn detect_between(longitude_left: f64, longitude_right: f64, aspects: &[AspectDefinition]) -> Vec<astral_calculator::domain::AspectFact> {
    detect_aspects(
        &[
            position(1, longitude_left, 0.0),
            position(2, longitude_right, 0.0),
        ],
        aspects,
    )
}

fn orb_limit_from_fact(
    fact: &astral_calculator::domain::AspectFact,
) -> Option<f64> {
    fact.calculation_notes_json
        .as_ref()
        .and_then(|notes| notes.get("orb_limit_deg"))
        .and_then(|value| value.as_f64())
}

fn position(id: i32, longitude_deg: f64, speed: f64) -> ObjectPositionFact {
    ObjectPositionFact {
        chart_object_id: id,
        object_code: format!("object_{id}"),
        object_name: format!("Object {id}"),
        zodiacal_reference_system_id: 1,
        coordinate_reference_system_id: 1,
        sign_id: 1,
        sign_code: "aries".to_string(),
        sign_name: "Aries".to_string(),
        house_id: None,
        house_number: None,
        house_name: None,
        motion_state_id: None,
        horizon_position_id: None,
        longitude_deg,
        latitude_deg: None,
        apparent_speed_deg_per_day: Some(speed),
        altitude_deg: None,
        is_visible: None,
        facts_json: None,
    }
}

fn angle_position(
    id: i32,
    object_code: &str,
    angle_point_code: &str,
    longitude_deg: f64,
    axis: &str,
    opposite_angle_code: &str,
) -> ObjectPositionFact {
    ObjectPositionFact {
        chart_object_id: id,
        object_code: object_code.to_string(),
        object_name: object_code.to_string(),
        zodiacal_reference_system_id: 1,
        coordinate_reference_system_id: 1,
        sign_id: 1,
        sign_code: "aries".to_string(),
        sign_name: "Aries".to_string(),
        house_id: None,
        house_number: None,
        house_name: None,
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
                "axis": axis,
                "opposite_angle_code": opposite_angle_code
            }
        })),
    }
}

#[test]
fn json_db_seed_major_aspects_match_runtime_validation() {
    let aspects = major_aspect_definitions_from_json_db_seed();
    assert_eq!(aspects.len(), 5);
    assert!(validate_major_aspects(&aspects).is_ok());
}

#[test]
fn aspect_phase_uses_relative_speed() {
    let aspects = vec![AspectDefinition {
        id: 1,
        code: "conjunction".to_string(),
        name: "Conjunction".to_string(),
        angle: 0.0,
        family: "major".to_string(),
        default_orb_deg: Some(8.0),
        max_default_orb_deg: major_aspect_family_max_default_orb_deg_from_json_db_seed(),
    }];
    let applying = detect_aspects(
        &[position(1, 0.0, 1.0), position(2, 2.0, 0.0)],
        &aspects,
    );
    assert_eq!(applying[0].phase_state, "applying");
    assert!(applying[0].is_applying);

    let separating = detect_aspects(
        &[position(1, 0.0, -1.0), position(2, 2.0, 0.0)],
        &aspects,
    );
    assert_eq!(separating[0].phase_state, "separating");
    assert!(!separating[0].is_applying);
}

#[test]
fn structural_angle_axes_are_not_detected_as_aspects() {
    let aspects = vec![AspectDefinition {
        id: 1,
        code: "opposition".to_string(),
        name: "Opposition".to_string(),
        angle: 180.0,
        family: "major".to_string(),
        default_orb_deg: Some(8.0),
        max_default_orb_deg: major_aspect_family_max_default_orb_deg_from_json_db_seed(),
    }];

    let detected = detect_aspects(
        &[
            angle_position(100, "ascendant", "asc", 15.0, "horizontal", "dsc"),
            angle_position(101, "descendant", "dsc", 195.0, "horizontal", "asc"),
            position(1, 15.0, 1.0),
            position(2, 195.0, 0.0),
        ],
        &aspects,
    );

    assert!(!detected.iter().any(|aspect| {
        aspect.source_object_code == "ascendant" && aspect.target_object_code == "descendant"
    }));
    assert!(detected
        .iter()
        .any(|aspect| aspect.source_object_code == "object_1"
            && aspect.target_object_code == "object_2"));
}

#[test]
fn detect_aspects_uses_per_aspect_orb_not_product_fallback() {
    let aspects = vec![
        AspectDefinition {
            id: 1,
            code: "conjunction".to_string(),
            name: "Conjunction".to_string(),
            angle: 0.0,
            family: "major".to_string(),
            default_orb_deg: Some(8.0),
            max_default_orb_deg: major_aspect_family_max_default_orb_deg_from_json_db_seed(),
        },
        AspectDefinition {
            id: 2,
            code: "sextile".to_string(),
            name: "Sextile".to_string(),
            angle: 60.0,
            family: "major".to_string(),
            default_orb_deg: Some(1.0),
            max_default_orb_deg: major_aspect_family_max_default_orb_deg_from_json_db_seed(),
        },
    ];

    let near_conjunction =
        detect_aspects(&[position(1, 0.0, 0.0), position(2, 3.0, 0.0)], &aspects);
    assert!(near_conjunction
        .iter()
        .any(|aspect| aspect.aspect_code == "conjunction"));

    let near_sextile =
        detect_aspects(&[position(1, 0.0, 0.0), position(2, 58.0, 0.0)], &aspects);
    assert!(!near_sextile.iter().any(|aspect| aspect.aspect_code == "sextile"));
}

#[test]
fn canonical_major_aspect_orbs_match_json_db_seed() {
    for aspect in major_aspect_definitions_from_json_db_seed() {
        let code = aspect.code.as_str();
        let angle = aspect.angle;
        let orb = aspect
            .default_orb_deg
            .expect("json_db major aspect must define default_orb_deg");
        let aspect = &single_aspect_definition(code, angle, orb)[0];
        assert_eq!(canonical_aspect_orb_deg(aspect), Some(orb));

        let inside_right = angle - (orb - 0.01);
        let inside = detect_between(0.0, inside_right, &single_aspect_definition(&code, angle, orb));
        let inside_fact = inside
            .iter()
            .find(|fact| fact.aspect_code == code)
            .unwrap_or_else(|| panic!("expected {code} inside {orb}° orb"));
        assert_eq!(orb_limit_from_fact(inside_fact), Some(orb));

        let outside_right = angle - (orb + 0.01);
        let outside =
            detect_between(0.0, outside_right, &single_aspect_definition(&code, angle, orb));
        assert!(
            !outside.iter().any(|fact| fact.aspect_code == code),
            "{code} should be outside {orb}° orb at separation {:.2}",
            (outside_right - angle).abs()
        );
    }
}

#[test]
fn detect_aspects_applies_each_canonical_orb_with_full_major_set() {
    let aspects = major_aspect_definitions_from_json_db_seed();

    let conjunction = detect_between(0.0, 7.5, &aspects);
    assert!(conjunction.iter().any(|f| f.aspect_code == "conjunction"));
    assert!(!conjunction.iter().any(|f| f.aspect_code == "sextile"));

    let sextile = detect_between(0.0, 54.0, &aspects);
    assert!(sextile.iter().any(|f| f.aspect_code == "sextile"));

    let square = detect_between(0.0, 84.0, &aspects);
    assert!(square.iter().any(|f| f.aspect_code == "square"));

    let trine = detect_between(0.0, 114.0, &aspects);
    assert!(trine.iter().any(|f| f.aspect_code == "trine"));

    let opposition = detect_between(0.0, 172.0, &aspects);
    assert!(opposition.iter().any(|f| f.aspect_code == "opposition"));
}

#[test]
fn canonical_aspect_orb_deg_rejects_orb_above_family_max() {
    let max = major_aspect_family_max_default_orb_deg_from_json_db_seed();
    let aspect = AspectDefinition {
        id: 1,
        code: "conjunction".to_string(),
        name: "Conjunction".to_string(),
        angle: 0.0,
        family: "major".to_string(),
        default_orb_deg: Some(max + 1.0),
        max_default_orb_deg: max,
    };
    assert_eq!(canonical_aspect_orb_deg(&aspect), None);
    assert!(
        detect_aspects(&[position(1, 0.0, 0.0), position(2, 3.0, 0.0)], &[aspect]).is_empty()
    );
}
