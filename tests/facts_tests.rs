use astral_calculator::shared::astro_math::{
    arc_contains, shortest_angular_distance, zodiac_slot_for_longitude,
};

#[test]
fn sign_handles_wraparound() {
    assert_eq!(zodiac_slot_for_longitude(0.0), 1);
    assert_eq!(zodiac_slot_for_longitude(359.9), 12);
    assert_eq!(zodiac_slot_for_longitude(-1.0), 12);
}

#[test]
fn arc_contains_handles_zero_crossing() {
    assert!(arc_contains(350.0, 10.0, 2.0));
    assert!(arc_contains(350.0, 10.0, 355.0));
    assert!(!arc_contains(350.0, 10.0, 180.0));
}

#[test]
fn shortest_angular_distance_handles_wraparound() {
    assert_eq!(shortest_angular_distance(10.0, 40.0), 30.0);
    assert_eq!(shortest_angular_distance(350.0, 10.0), 20.0);
    assert_eq!(shortest_angular_distance(-10.0, 10.0), 20.0);
}
