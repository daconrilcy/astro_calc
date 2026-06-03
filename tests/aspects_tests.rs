use rust_sqlx_connection_test::aspects::detect_aspects;
use rust_sqlx_connection_test::catalog::test_catalog;
use rust_sqlx_connection_test::domain::ObjectPositionFact;
use rust_sqlx_connection_test::models::AspectDefinition;
use serde_json::json;

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
fn aspect_phase_uses_relative_speed() {
    let aspects = vec![AspectDefinition {
        id: 1,
        code: "conjunction".to_string(),
        name: "Conjunction".to_string(),
        angle: 0.0,
        default_orb_deg: Some(8.0),
    }];
    let default_orb = test_catalog().product_scoring.default_major_orb_deg;

    let applying = detect_aspects(
        &[position(1, 0.0, 1.0), position(2, 2.0, 0.0)],
        &aspects,
        default_orb,
    );
    assert_eq!(applying[0].phase_state, "applying");
    assert!(applying[0].is_applying);

    let separating = detect_aspects(
        &[position(1, 0.0, -1.0), position(2, 2.0, 0.0)],
        &aspects,
        default_orb,
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
        default_orb_deg: Some(8.0),
    }];

    let detected = detect_aspects(
        &[
            angle_position(100, "ascendant", "asc", 15.0, "horizontal", "dsc"),
            angle_position(101, "descendant", "dsc", 195.0, "horizontal", "asc"),
            position(1, 15.0, 1.0),
            position(2, 195.0, 0.0),
        ],
        &aspects,
        test_catalog().product_scoring.default_major_orb_deg,
    );

    assert!(!detected.iter().any(|aspect| {
        aspect.source_object_code == "ascendant" && aspect.target_object_code == "descendant"
    }));
    assert!(detected
        .iter()
        .any(|aspect| aspect.source_object_code == "object_1"
            && aspect.target_object_code == "object_2"));
}
