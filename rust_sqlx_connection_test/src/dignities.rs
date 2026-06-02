use serde::{Deserialize, Serialize};

use crate::domain::ObjectPositionFact;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EssentialDignityFact {
    pub chart_object_id: i32,
    pub object_code: String,
    pub object_name: String,
    pub sign_id: i32,
    pub sign_code: String,
    pub sign_name: String,
    pub dignity_type: String,
    pub dignity_label: String,
    pub polarity: String,
    pub strength_score: f64,
    pub is_major: bool,
}

pub fn essential_dignity_for_position(
    position: &ObjectPositionFact,
) -> Option<EssentialDignityFact> {
    essential_dignities_for_position(position)
        .into_iter()
        .next()
}

pub fn essential_dignities_for_position(
    position: &ObjectPositionFact,
) -> Vec<EssentialDignityFact> {
    dignity_rules(&position.object_code, &position.sign_code)
        .into_iter()
        .map(|rule| EssentialDignityFact {
            chart_object_id: position.chart_object_id,
            object_code: position.object_code.clone(),
            object_name: position.object_name.clone(),
            sign_id: position.sign_id,
            sign_code: position.sign_code.clone(),
            sign_name: position.sign_name.clone(),
            dignity_type: rule.dignity_type.to_string(),
            dignity_label: rule.dignity_label.to_string(),
            polarity: rule.polarity.to_string(),
            strength_score: rule.strength_score,
            is_major: true,
        })
        .collect()
}

pub fn essential_dignities_for_positions(
    positions: &[ObjectPositionFact],
) -> Vec<EssentialDignityFact> {
    positions
        .iter()
        .flat_map(essential_dignities_for_position)
        .collect()
}

pub fn dignity_priority_delta(dignity: &EssentialDignityFact) -> f64 {
    match dignity.dignity_type.as_str() {
        "domicile" => 8.0,
        "exaltation" => 6.0,
        "detriment" => 4.0,
        "fall" => 3.0,
        _ => 0.0,
    }
}

pub fn dignity_source_weight_delta(dignity: &EssentialDignityFact) -> f64 {
    match dignity.dignity_type.as_str() {
        "domicile" | "exaltation" => 0.15,
        "detriment" | "fall" => 0.1,
        _ => 0.0,
    }
}

pub fn dignity_is_signal_worthy(dignity: &EssentialDignityFact) -> bool {
    dignity.is_major && dignity.strength_score >= 0.7
}

pub fn dignity_priority_delta_for_position(position: &ObjectPositionFact) -> f64 {
    essential_dignities_for_position(position)
        .iter()
        .map(dignity_priority_delta)
        .sum::<f64>()
        .min(9.0)
}

pub fn dignity_source_weight_delta_for_position(position: &ObjectPositionFact) -> f64 {
    essential_dignities_for_position(position)
        .iter()
        .map(dignity_source_weight_delta)
        .sum::<f64>()
        .min(0.2)
}

struct DignityRule {
    dignity_type: &'static str,
    dignity_label: &'static str,
    polarity: &'static str,
    strength_score: f64,
}

fn dignity_rules(object_code: &str, sign_code: &str) -> Vec<DignityRule> {
    let mut rules = Vec::new();

    match (object_code, sign_code) {
        ("sun", "leo") => rules.push(positive("domicile", "Domicile", 1.0)),
        ("moon", "cancer") => rules.push(positive("domicile", "Domicile", 1.0)),
        ("mercury", "gemini" | "virgo") => rules.push(positive("domicile", "Domicile", 1.0)),
        ("venus", "taurus" | "libra") => rules.push(positive("domicile", "Domicile", 1.0)),
        ("mars", "aries" | "scorpio") => rules.push(positive("domicile", "Domicile", 1.0)),
        ("jupiter", "sagittarius" | "pisces") => rules.push(positive("domicile", "Domicile", 1.0)),
        ("saturn", "capricorn" | "aquarius") => rules.push(positive("domicile", "Domicile", 1.0)),
        _ => {}
    }

    match (object_code, sign_code) {
        ("sun", "aquarius") => rules.push(negative("detriment", "Detriment", 0.85)),
        ("moon", "capricorn") => rules.push(negative("detriment", "Detriment", 0.85)),
        ("mercury", "sagittarius" | "pisces") => {
            rules.push(negative("detriment", "Detriment", 0.85))
        }
        ("venus", "aries" | "scorpio") => rules.push(negative("detriment", "Detriment", 0.85)),
        ("mars", "taurus" | "libra") => rules.push(negative("detriment", "Detriment", 0.85)),
        ("jupiter", "gemini" | "virgo") => rules.push(negative("detriment", "Detriment", 0.85)),
        ("saturn", "cancer" | "leo") => rules.push(negative("detriment", "Detriment", 0.85)),
        _ => {}
    }

    match (object_code, sign_code) {
        ("sun", "aries") => rules.push(positive("exaltation", "Exaltation", 0.9)),
        ("moon", "taurus") => rules.push(positive("exaltation", "Exaltation", 0.9)),
        ("mercury", "virgo") => rules.push(positive("exaltation", "Exaltation", 0.9)),
        ("venus", "pisces") => rules.push(positive("exaltation", "Exaltation", 0.9)),
        ("mars", "capricorn") => rules.push(positive("exaltation", "Exaltation", 0.9)),
        ("jupiter", "cancer") => rules.push(positive("exaltation", "Exaltation", 0.9)),
        ("saturn", "libra") => rules.push(positive("exaltation", "Exaltation", 0.9)),
        _ => {}
    }

    match (object_code, sign_code) {
        ("sun", "libra") => rules.push(negative("fall", "Fall", 0.75)),
        ("moon", "scorpio") => rules.push(negative("fall", "Fall", 0.75)),
        ("mercury", "pisces") => rules.push(negative("fall", "Fall", 0.75)),
        ("venus", "virgo") => rules.push(negative("fall", "Fall", 0.75)),
        ("mars", "cancer") => rules.push(negative("fall", "Fall", 0.75)),
        ("jupiter", "capricorn") => rules.push(negative("fall", "Fall", 0.75)),
        ("saturn", "aries") => rules.push(negative("fall", "Fall", 0.75)),
        _ => {}
    }

    rules
}

fn positive(
    dignity_type: &'static str,
    dignity_label: &'static str,
    strength_score: f64,
) -> DignityRule {
    DignityRule {
        dignity_type,
        dignity_label,
        polarity: "dignity",
        strength_score,
    }
}

fn negative(
    dignity_type: &'static str,
    dignity_label: &'static str,
    strength_score: f64,
) -> DignityRule {
    DignityRule {
        dignity_type,
        dignity_label,
        polarity: "debility",
        strength_score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(
            essential_dignity_for_position(&position("saturn", "capricorn"))
                .expect("saturn dignity")
                .dignity_type,
            "domicile"
        );
        assert_eq!(
            essential_dignity_for_position(&position("jupiter", "cancer"))
                .expect("jupiter dignity")
                .dignity_type,
            "exaltation"
        );
    }

    #[test]
    fn preserves_double_mercury_dignities() {
        let virgo = essential_dignities_for_position(&position("mercury", "virgo"));
        let pisces = essential_dignities_for_position(&position("mercury", "pisces"));

        assert_eq!(virgo.len(), 2);
        assert!(virgo.iter().any(|d| d.dignity_type == "domicile"));
        assert!(virgo.iter().any(|d| d.dignity_type == "exaltation"));
        assert_eq!(pisces.len(), 2);
        assert!(pisces.iter().any(|d| d.dignity_type == "detriment"));
        assert!(pisces.iter().any(|d| d.dignity_type == "fall"));
    }
}
