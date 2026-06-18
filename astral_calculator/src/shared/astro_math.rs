//! Utilitaires trigonométriques et zodiacaux communs aux calculs astrologiques.

/// Ramène un angle dans l'intervalle `[0, 360)`.
pub fn normalize_degrees(value: f64) -> f64 {
    let normalized = value % 360.0;
    if normalized < 0.0 {
        normalized + 360.0
    } else {
        normalized
    }
}

/// Calcule la séparation minimale entre deux longitudes sur le cercle zodiacal.
pub fn shortest_angular_distance(left: f64, right: f64) -> f64 {
    let diff = (normalize_degrees(left) - normalize_degrees(right)).abs();
    diff.min(360.0 - diff)
}

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

/// Teste l'appartenance à un arc, y compris lorsqu'il traverse `0° Bélier`.
pub fn arc_contains(start: f64, end: f64, value: f64) -> bool {
    if start <= end {
        value >= start && value < end
    } else {
        value >= start || value < end
    }
}
