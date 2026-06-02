use rust_sqlx_connection_test::facts::{arc_contains, zodiac_slot_for_longitude};

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
