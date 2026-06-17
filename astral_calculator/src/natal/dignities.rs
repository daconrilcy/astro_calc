use serde::{Deserialize, Serialize};

use crate::natal::catalog::{BasicPayloadCatalog, EssentialDignityScoringWeight};
use crate::domain::{EssentialDignityRuleReference, ObjectPositionFact};

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
    catalog: &BasicPayloadCatalog,
) -> Option<EssentialDignityFact> {
    essential_dignities_for_position(position, catalog)
        .into_iter()
        .next()
}

pub fn essential_dignities_for_position(
    position: &ObjectPositionFact,
    catalog: &BasicPayloadCatalog,
) -> Vec<EssentialDignityFact> {
    catalog
        .essential_rules_for(&position.object_code, &position.sign_code)
        .iter()
        .map(|rule| dignity_fact_from_rule(position, rule))
        .collect()
}

pub fn essential_dignities_for_positions(
    positions: &[ObjectPositionFact],
    catalog: &BasicPayloadCatalog,
) -> Vec<EssentialDignityFact> {
    positions
        .iter()
        .flat_map(|position| essential_dignities_for_position(position, catalog))
        .collect()
}

pub fn dignity_priority_delta(
    dignity: &EssentialDignityFact,
    catalog: &BasicPayloadCatalog,
) -> f64 {
    catalog
        .dignity_scoring_weight(&dignity.dignity_type)
        .map(|weight| weight.priority_delta)
        .unwrap_or(0.0)
}

pub fn dignity_source_weight_delta(
    dignity: &EssentialDignityFact,
    catalog: &BasicPayloadCatalog,
) -> f64 {
    catalog
        .dignity_scoring_weight(&dignity.dignity_type)
        .map(|weight| weight.signal_weight_delta)
        .unwrap_or(0.0)
}

pub fn dignity_is_signal_worthy(
    dignity: &EssentialDignityFact,
    catalog: &BasicPayloadCatalog,
) -> bool {
    dignity.is_major
        && dignity.strength_score
            >= catalog
                .dignity_scoring_weight(&dignity.dignity_type)
                .map(|weight| weight.signal_worthy_min_strength)
                .unwrap_or(0.7)
}

pub fn dignity_priority_delta_for_position(
    position: &ObjectPositionFact,
    catalog: &BasicPayloadCatalog,
) -> f64 {
    essential_dignities_for_position(position, catalog)
        .iter()
        .map(|dignity| dignity_priority_delta(dignity, catalog))
        .sum::<f64>()
        .min(9.0)
}

pub fn dignity_source_weight_delta_for_position(
    position: &ObjectPositionFact,
    catalog: &BasicPayloadCatalog,
) -> f64 {
    essential_dignities_for_position(position, catalog)
        .iter()
        .map(|dignity| dignity_source_weight_delta(dignity, catalog))
        .sum::<f64>()
        .min(0.2)
}

pub fn dignity_emphasis_weight(dignity_type: &str, catalog: &BasicPayloadCatalog) -> f64 {
    catalog
        .dignity_scoring_weight(dignity_type)
        .map(|weight: &EssentialDignityScoringWeight| weight.emphasis_weight)
        .unwrap_or(0.25)
}

fn dignity_fact_from_rule(
    position: &ObjectPositionFact,
    rule: &EssentialDignityRuleReference,
) -> EssentialDignityFact {
    EssentialDignityFact {
        chart_object_id: position.chart_object_id,
        object_code: position.object_code.clone(),
        object_name: position.object_name.clone(),
        sign_id: position.sign_id,
        sign_code: position.sign_code.clone(),
        sign_name: position.sign_name.clone(),
        dignity_type: rule.dignity_type.clone(),
        dignity_label: rule.dignity_label.clone(),
        polarity: rule.polarity.clone(),
        strength_score: rule.strength_score,
        is_major: true,
    }
}
