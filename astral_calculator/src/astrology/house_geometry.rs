//! Géométrie des maisons astrologiques calculée à partir de faits métier.

use crate::astrology::angles::{arc_contains, normalize_degrees};
use crate::domain::HouseCuspFact;

/// Localise la maison contenant une longitude à partir d'un jeu complet de
/// cuspides ordonnées.
pub fn house_number_from_cusps(longitude_deg: f64, cusps: &[HouseCuspFact]) -> Option<i32> {
    if cusps.len() != 12 {
        return None;
    }

    let longitude = normalize_degrees(longitude_deg);
    for index in 0..12 {
        let start = normalize_degrees(cusps[index].longitude_deg);
        let end = normalize_degrees(cusps[(index + 1) % 12].longitude_deg);
        if arc_contains(start, end, longitude) {
            return Some(cusps[index].house_number);
        }
    }

    None
}
