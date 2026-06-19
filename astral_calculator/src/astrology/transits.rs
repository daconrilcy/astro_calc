//! Primitives de transits réutilisables par les produits horoscope.

use crate::astrology::angles::shortest_angular_distance;
use crate::astrology::aspects::canonical_aspect_orb_deg;
use crate::domain::{AspectDefinition, ObjectPositionFact};

#[derive(Debug, Clone)]
/// Aspect transit-vers-natal retenu pour assemblage produit.
pub struct TransitAspectMatch {
    pub transiting_object: String,
    pub natal_target: String,
    pub aspect_code: String,
    pub orb_deg: f64,
    pub natal_house: Option<i32>,
}

/// Retourne les objets mobiles courants utilisés par les horoscopes.
pub fn is_standard_transit_object(code: &str) -> bool {
    !code.trim().is_empty()
}

/// Sélectionne une position transitante standard réelle, en privilégiant l'objet demandé.
pub fn preferred_transit_position<'a>(
    positions: Option<&'a [ObjectPositionFact]>,
    preferred_object_code: &str,
) -> Option<&'a ObjectPositionFact> {
    let positions = positions?;
    positions
        .iter()
        .find(|position| position.object_code == preferred_object_code)
        .or_else(|| {
            positions
                .iter()
                .find(|position| is_standard_transit_object(position.object_code.as_str()))
        })
}

/// Trouve le meilleur aspect majeur entre une position transitante et un thème natal.
pub fn nearest_major_transit_match(
    transit: &ObjectPositionFact,
    natal_positions: &[&ObjectPositionFact],
    max_orb_deg: f64,
    aspect_definitions: &[AspectDefinition],
) -> Option<TransitAspectMatch> {
    let mut best: Option<TransitAspectMatch> = None;
    for natal in natal_positions {
        for aspect in major_aspect_candidates(aspect_definitions) {
            let separation = shortest_angular_distance(transit.longitude_deg, natal.longitude_deg);
            let orb = (separation - aspect.angle_deg).abs();
            if orb > aspect.effective_orb_limit_deg(max_orb_deg) {
                continue;
            }
            if best
                .as_ref()
                .is_some_and(|existing| existing.orb_deg <= orb)
            {
                continue;
            }
            best = Some(TransitAspectMatch {
                transiting_object: transit.object_code.clone(),
                natal_target: format!("natal_{}", natal.object_code),
                aspect_code: aspect.code.to_string(),
                orb_deg: orb,
                natal_house: natal.house_number,
            });
        }
    }
    best
}

/// Retourne l'aspect majeur le plus proche, même hors orbe, pour un fallback contextuel.
pub fn nearest_major_aspect_name_and_orb(
    left_longitude_deg: f64,
    right_longitude_deg: f64,
    aspect_definitions: &[AspectDefinition],
) -> Option<(String, f64)> {
    let separation = shortest_angular_distance(left_longitude_deg, right_longitude_deg);
    let mut best: Option<(String, f64)> = None;
    for aspect in major_aspect_candidates(aspect_definitions) {
        let orb = (separation - aspect.angle_deg).abs();
        if best
            .as_ref()
            .is_none_or(|(_, existing_orb)| orb < *existing_orb)
        {
            best = Some((aspect.code, orb));
        }
    }
    best
}

#[derive(Debug, Clone)]
struct MajorAspectCandidate {
    code: String,
    angle_deg: f64,
    reference_orb_limit_deg: Option<f64>,
}

impl MajorAspectCandidate {
    fn effective_orb_limit_deg(&self, max_orb_deg: f64) -> f64 {
        match self.reference_orb_limit_deg {
            Some(reference_orb) if max_orb_deg > 0.0 => reference_orb.min(max_orb_deg),
            Some(reference_orb) => reference_orb,
            None => max_orb_deg,
        }
    }
}

fn major_aspect_candidates(aspect_definitions: &[AspectDefinition]) -> Vec<MajorAspectCandidate> {
    let from_reference = aspect_definitions
        .iter()
        .filter(|aspect| aspect.family == "major")
        .map(|aspect| MajorAspectCandidate {
            code: aspect.code.clone(),
            angle_deg: aspect.angle,
            reference_orb_limit_deg: canonical_aspect_orb_deg(aspect),
        })
        .collect::<Vec<_>>();

    from_reference
}
