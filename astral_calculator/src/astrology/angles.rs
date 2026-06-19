//! Primitives d'angles zodiacaux réutilisables.

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

/// Teste l'appartenance à un arc, y compris lorsqu'il traverse `0° Bélier`.
pub fn arc_contains(start: f64, end: f64, value: f64) -> bool {
    if start <= end {
        value >= start && value < end
    } else {
        value >= start || value < end
    }
}
