use astral_calculator::domain::{ObjectPositionFact, PositionFactContext};
use serde_json::json;

fn sample_position(facts_json: Option<serde_json::Value>) -> ObjectPositionFact {
    ObjectPositionFact {
        chart_object_id: 1,
        object_code: "sun".to_string(),
        object_name: "Sun".to_string(),
        zodiacal_reference_system_id: 1,
        coordinate_reference_system_id: 1,
        sign_id: 1,
        sign_code: "aries".to_string(),
        sign_name: "Aries".to_string(),
        house_id: Some(1),
        house_number: Some(1),
        house_name: Some("House I".to_string()),
        motion_state_id: None,
        horizon_position_id: Some(1),
        longitude_deg: 10.0,
        latitude_deg: None,
        apparent_speed_deg_per_day: None,
        altitude_deg: Some(5.0),
        is_visible: Some(true),
        facts_json,
    }
}

#[test]
fn context_is_absent_when_facts_json_is_missing() {
    assert_eq!(sample_position(None).context(), None);
}

#[test]
fn context_parses_angle_context_when_present() {
    let position = sample_position(Some(json!({
        "angle_context": {
            "angle_point_code": "asc",
            "axis": "horizon",
            "associated_house_number": 1
        }
    })));

    let context = position.context().expect("typed context");
    let angle = context.angle_context.expect("angle context");
    assert_eq!(angle.angle_point_code.as_deref(), Some("asc"));
    assert_eq!(angle.axis.as_deref(), Some("horizon"));
    assert_eq!(angle.associated_house_number, Some(1));
}

#[test]
fn context_parses_object_role_angle() {
    let position = sample_position(Some(json!({
        "object_context": {
            "role": "angle",
            "role_label": "Angle"
        }
    })));

    let context = position.context().expect("typed context");
    let object = context.object_context.expect("object context");
    assert_eq!(object.role.as_deref(), Some("angle"));
    assert_eq!(object.role_label.as_deref(), Some("Angle"));
}

#[test]
fn unknown_json_is_preserved_even_if_typed_context_is_absent() {
    let facts_json = json!({ "unknown_block": { "x": 1 } });
    let position = sample_position(Some(facts_json.clone()));

    assert_eq!(position.context(), None);
    assert_eq!(position.facts_json, Some(facts_json));
}

#[test]
fn context_can_be_built_directly_from_json() {
    let context = PositionFactContext::from_facts_json(Some(&json!({
        "object_context": { "role": "planet" }
    })))
    .expect("typed context");

    assert_eq!(
        context
            .object_context
            .and_then(|object| object.role)
            .as_deref(),
        Some("planet")
    );
}
