use astral_calculator::domain::ObjectPositionFact;
use astral_calculator::features::natal::catalog::test_catalog;
use astral_calculator::features::natal::dignities::*;

fn position(object_code: &str, sign_code: &str) -> ObjectPositionFact {
    ObjectPositionFact {
        chart_object_id: 1,
        object_code: object_code.to_string(),
        object_name: object_code.to_string(),
        zodiacal_reference_system_id: 1,
        coordinate_reference_system_id: 1,
        sign_id: 1,
        sign_code: sign_code.to_string(),
        sign_name: sign_code.to_string(),
        house_id: None,
        house_number: None,
        house_name: None,
        motion_state_id: None,
        horizon_position_id: None,
        longitude_deg: 0.0,
        latitude_deg: None,
        apparent_speed_deg_per_day: None,
        altitude_deg: None,
        is_visible: None,
        facts_json: None,
    }
}

#[test]
fn detects_requested_major_dignities() {
    let catalog = test_catalog();
    assert_eq!(
        essential_dignity_for_position(&position("saturn", "capricorn"), &catalog)
            .expect("saturn dignity")
            .dignity_type,
        "domicile"
    );
    assert_eq!(
        essential_dignity_for_position(&position("jupiter", "cancer"), &catalog)
            .expect("jupiter dignity")
            .dignity_type,
        "exaltation"
    );
}

#[test]
fn preserves_double_mercury_dignities() {
    let catalog = test_catalog();
    let virgo = essential_dignities_for_position(&position("mercury", "virgo"), &catalog);
    let pisces = essential_dignities_for_position(&position("mercury", "pisces"), &catalog);

    assert_eq!(virgo.len(), 2);
    assert!(virgo.iter().any(|d| d.dignity_type == "domicile"));
    assert!(virgo.iter().any(|d| d.dignity_type == "exaltation"));
    assert_eq!(pisces.len(), 2);
    assert!(pisces.iter().any(|d| d.dignity_type == "detriment"));
    assert!(pisces.iter().any(|d| d.dignity_type == "fall"));
}
