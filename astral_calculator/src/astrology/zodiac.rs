//! Helpers zodiacaux réutilisables.

use crate::astrology::angles::normalize_degrees;

/// Retourne le signe 1..=12 correspondant à une longitude tropicale.
pub fn zodiac_slot_for_longitude(longitude_deg: f64) -> i32 {
    (normalize_degrees(longitude_deg) / 30.0).floor() as i32 + 1
}

/// Déduit la maison whole-sign d'un objet à partir du signe de l'ascendant.
pub fn whole_sign_house_number(ascendant_longitude_deg: f64, body_longitude_deg: f64) -> i32 {
    let asc_sign = zodiac_slot_for_longitude(ascendant_longitude_deg);
    let body_sign = zodiac_slot_for_longitude(body_longitude_deg);
    ((body_sign - asc_sign).rem_euclid(12)) + 1
}
