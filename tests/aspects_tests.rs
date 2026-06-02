use rust_sqlx_connection_test::aspects::detect_aspects;
use rust_sqlx_connection_test::domain::ObjectPositionFact;
use rust_sqlx_connection_test::models::AspectDefinition;

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

#[test]
fn aspect_phase_uses_relative_speed() {
    let aspects = vec![AspectDefinition {
        id: 1,
        code: "conjunction".to_string(),
        name: "Conjunction".to_string(),
        angle: 0.0,
    }];

    let applying = detect_aspects(&[position(1, 0.0, 1.0), position(2, 2.0, 0.0)], &aspects);
    assert_eq!(applying[0].phase_state, "applying");
    assert!(applying[0].is_applying);

    let separating = detect_aspects(&[position(1, 0.0, -1.0), position(2, 2.0, 0.0)], &aspects);
    assert_eq!(separating[0].phase_state, "separating");
    assert!(!separating[0].is_applying);
}
