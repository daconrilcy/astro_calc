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
fn context_parses_sign_house_and_motion_blocks() {
    let context = PositionFactContext::from_facts_json(Some(&json!({
        "sign_context": {
            "element": "air",
            "polarity": "yang"
        },
        "house_context": {
            "theme_code": "identity"
        },
        "motion_context": {
            "motion_state": "direct",
            "label": "Direct"
        }
    })))
    .expect("typed context");

    assert_eq!(
        context
            .sign_context
            .as_ref()
            .and_then(|sign| sign.element.as_deref()),
        Some("air")
    );
    assert_eq!(
        context
            .house_context
            .as_ref()
            .and_then(|house| house.theme_code.as_deref()),
        Some("identity")
    );
    assert_eq!(
        context
            .motion_context
            .as_ref()
            .and_then(|motion| motion.motion_state.as_deref()),
        Some("direct")
    );
}

#[test]
fn context_round_trips_back_to_facts_json() {
    let context = PositionFactContext::from_facts_json(Some(&json!({
        "sign_context": {
            "element": "water",
            "element_label": "Water",
            "polarity": "yin"
        },
        "visibility_context": {
            "horizon_position": "above_horizon",
            "source": "calculated_altitude"
        },
        "angle_context": {
            "angle_point_code": "asc",
            "associated_house_number": 1
        }
    })))
    .expect("typed context");

    let facts = context.to_facts_json();
    assert_eq!(
        facts
            .get("sign_context")
            .and_then(|value| value.get("element"))
            .and_then(|value| value.as_str()),
        Some("water")
    );
    assert_eq!(
        facts
            .get("visibility_context")
            .and_then(|value| value.get("source"))
            .and_then(|value| value.as_str()),
        Some("calculated_altitude")
    );
    assert!(facts.get("angle_context").is_some());
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
        "object_context": { "role": "planet" },
        "visibility_context": {
            "horizon_position_id": 1,
            "horizon_position": "above_horizon",
            "altitude_deg": 5.0,
            "is_visible": true,
            "source": "calculated_altitude"
        }
    })))
    .expect("typed context");

    assert_eq!(
        context
            .object_context
            .and_then(|object| object.role)
            .as_deref(),
        Some("planet")
    );
    assert_eq!(
        context
            .visibility_context
            .and_then(|visibility| visibility.horizon_position),
        Some("above_horizon".to_string())
    );
}

#[test]
fn object_position_exposes_typed_visibility_context() {
    let position = sample_position(Some(json!({
        "visibility_context": {
            "horizon_position_id": 2,
            "horizon_position": "below_horizon",
            "altitude_deg": -4.5,
            "is_visible": false,
            "source": "legacy_payload"
        }
    })));

    let visibility = position.visibility_context().expect("visibility context");
    assert_eq!(visibility.horizon_position_id, Some(2));
    assert_eq!(
        visibility.horizon_position.as_deref(),
        Some("below_horizon")
    );
    assert_eq!(visibility.is_visible, Some(false));
    assert_eq!(visibility.source.as_deref(), Some("legacy_payload"));
}
