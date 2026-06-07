use std::collections::HashMap;

use crate::domain::{
    AccidentalConditionTrigger, AccidentalDignityConditionReference, AccidentalPolarityBand,
    AccidentalScoringParams, BasicProductScoringProfile, EssentialDignityRuleReference,
    ObjectSectAffinityReference,
};

#[derive(Debug, Clone)]
pub struct BasicPayloadCatalog {
    pub product_scoring: BasicProductScoringProfile,
    pub essential_dignity_rules: Vec<EssentialDignityRuleReference>,
    pub accidental_triggers: Vec<AccidentalConditionTrigger>,
    pub accidental_scoring: AccidentalScoringParams,
    pub accidental_polarity_bands: Vec<AccidentalPolarityBand>,
    essential_by_object_sign: HashMap<(String, String), Vec<EssentialDignityRuleReference>>,
    triggers_by_family: HashMap<String, Vec<AccidentalConditionTrigger>>,
    dignity_weight_by_type: HashMap<String, EssentialDignityScoringWeight>,
}

#[derive(Debug, Clone)]
pub struct EssentialDignityScoringWeight {
    pub priority_delta: f64,
    pub signal_weight_delta: f64,
    pub signal_worthy_min_strength: f64,
    pub emphasis_weight: f64,
}

impl BasicPayloadCatalog {
    pub fn build(
        product_scoring: BasicProductScoringProfile,
        essential_dignity_rules: Vec<EssentialDignityRuleReference>,
        accidental_triggers: Vec<AccidentalConditionTrigger>,
        accidental_scoring: AccidentalScoringParams,
        accidental_polarity_bands: Vec<AccidentalPolarityBand>,
    ) -> Self {
        let mut essential_by_object_sign = HashMap::new();
        let mut dignity_weight_by_type = HashMap::new();
        for rule in &essential_dignity_rules {
            essential_by_object_sign
                .entry((rule.object_code.clone(), rule.sign_code.clone()))
                .or_insert_with(Vec::new)
                .push(rule.clone());
            dignity_weight_by_type
                .entry(rule.dignity_type.clone())
                .or_insert_with(|| EssentialDignityScoringWeight {
                    priority_delta: rule.priority_delta,
                    signal_weight_delta: rule.signal_weight_delta,
                    signal_worthy_min_strength: rule.signal_worthy_min_strength,
                    emphasis_weight: rule.emphasis_weight,
                });
        }

        let mut triggers_by_family = HashMap::new();
        for trigger in &accidental_triggers {
            triggers_by_family
                .entry(trigger.trigger_family.clone())
                .or_insert_with(Vec::new)
                .push(trigger.clone());
        }

        Self {
            product_scoring,
            essential_dignity_rules,
            accidental_triggers,
            accidental_scoring,
            accidental_polarity_bands,
            essential_by_object_sign,
            triggers_by_family,
            dignity_weight_by_type,
        }
    }

    pub fn essential_rules_for(
        &self,
        object_code: &str,
        sign_code: &str,
    ) -> &[EssentialDignityRuleReference] {
        self.essential_by_object_sign
            .get(&(object_code.to_string(), sign_code.to_string()))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn dignity_scoring_weight(
        &self,
        dignity_type: &str,
    ) -> Option<&EssentialDignityScoringWeight> {
        self.dignity_weight_by_type.get(dignity_type)
    }

    pub fn condition_code_for_house_modality(&self, modality_code: &str) -> Option<&str> {
        self.triggers_by_family
            .get("house_modality")
            .and_then(|triggers| {
                triggers
                    .iter()
                    .find(|trigger| trigger.source_code.as_deref() == Some(modality_code))
            })
            .map(|trigger| trigger.condition_code.as_str())
    }

    pub fn condition_code_for_motion_state(&self, motion_state: &str) -> Option<&str> {
        self.triggers_by_family
            .get("motion_state")
            .and_then(|triggers| {
                triggers
                    .iter()
                    .find(|trigger| trigger.source_code.as_deref() == Some(motion_state))
            })
            .map(|trigger| trigger.condition_code.as_str())
    }

    pub fn condition_code_for_horizon_position(&self, horizon_position: &str) -> Option<&str> {
        self.triggers_by_family
            .get("horizon_position")
            .and_then(|triggers| {
                triggers
                    .iter()
                    .find(|trigger| trigger.source_code.as_deref() == Some(horizon_position))
            })
            .map(|trigger| trigger.condition_code.as_str())
    }

    pub fn angle_proximity_triggers(&self) -> &[AccidentalConditionTrigger] {
        self.triggers_by_family
            .get("angle_proximity")
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn sect_condition_code(
        &self,
        chart_sect: &str,
        affinity: &ObjectSectAffinityReference,
    ) -> Option<&str> {
        let family = if affinity.is_variable {
            "sect_variable"
        } else if affinity.sect_affinity_code == chart_sect {
            "sect_match"
        } else {
            "sect_mismatch"
        };
        self.triggers_by_family
            .get(family)
            .and_then(|triggers| triggers.first())
            .map(|trigger| trigger.condition_code.as_str())
    }

    pub fn overall_polarity_for_score(&self, score: f64) -> (String, String) {
        overall_polarity_for_score_with_bands(score, &self.accidental_polarity_bands)
    }

    pub fn valid_accidental_condition_codes(
        definitions: &[AccidentalDignityConditionReference],
    ) -> Vec<&str> {
        definitions
            .iter()
            .map(|definition| definition.condition_code.as_str())
            .collect()
    }
}

fn test_essential_dignity_rules() -> Vec<EssentialDignityRuleReference> {
    use crate::domain::EssentialDignityRuleReference;

    fn rule(
        object_code: &str,
        sign_code: &str,
        dignity_type: &str,
        label: &str,
        polarity: &str,
        strength: f64,
        priority: f64,
    ) -> EssentialDignityRuleReference {
        EssentialDignityRuleReference {
            object_code: object_code.to_string(),
            sign_code: sign_code.to_string(),
            dignity_type: dignity_type.to_string(),
            dignity_label: label.to_string(),
            polarity: polarity.to_string(),
            strength_score: strength,
            priority_delta: priority,
            signal_weight_delta: if polarity == "dignity" { 0.15 } else { 0.1 },
            signal_worthy_min_strength: 0.7,
            emphasis_weight: match dignity_type {
                "domicile" => 0.65,
                "exaltation" => 0.55,
                "detriment" => 0.45,
                "fall" => 0.35,
                _ => 0.25,
            },
        }
    }

    vec![
        rule("sun", "leo", "domicile", "Domicile", "dignity", 1.0, 8.0),
        rule(
            "moon", "cancer", "domicile", "Domicile", "dignity", 1.0, 8.0,
        ),
        rule(
            "mercury", "gemini", "domicile", "Domicile", "dignity", 1.0, 8.0,
        ),
        rule(
            "mercury", "virgo", "domicile", "Domicile", "dignity", 1.0, 8.0,
        ),
        rule(
            "mercury",
            "virgo",
            "exaltation",
            "Exaltation",
            "dignity",
            0.9,
            6.0,
        ),
        rule(
            "mercury",
            "sagittarius",
            "detriment",
            "Detriment",
            "debility",
            0.85,
            4.0,
        ),
        rule(
            "mercury",
            "pisces",
            "detriment",
            "Detriment",
            "debility",
            0.85,
            4.0,
        ),
        rule("mercury", "pisces", "fall", "Fall", "debility", 0.75, 3.0),
        rule(
            "venus", "taurus", "domicile", "Domicile", "dignity", 1.0, 8.0,
        ),
        rule(
            "venus", "libra", "domicile", "Domicile", "dignity", 1.0, 8.0,
        ),
        rule("mars", "aries", "domicile", "Domicile", "dignity", 1.0, 8.0),
        rule(
            "mars", "scorpio", "domicile", "Domicile", "dignity", 1.0, 8.0,
        ),
        rule(
            "mars",
            "taurus",
            "detriment",
            "Detriment",
            "debility",
            0.85,
            4.0,
        ),
        rule(
            "jupiter",
            "sagittarius",
            "domicile",
            "Domicile",
            "dignity",
            1.0,
            8.0,
        ),
        rule(
            "jupiter", "pisces", "domicile", "Domicile", "dignity", 1.0, 8.0,
        ),
        rule(
            "jupiter",
            "cancer",
            "exaltation",
            "Exaltation",
            "dignity",
            0.9,
            6.0,
        ),
        rule(
            "saturn",
            "capricorn",
            "domicile",
            "Domicile",
            "dignity",
            1.0,
            8.0,
        ),
        rule(
            "saturn", "aquarius", "domicile", "Domicile", "dignity", 1.0, 8.0,
        ),
    ]
}

/// Catalogue minimal pour les tests unitaires et les builders sans connexion DB.
pub fn test_catalog() -> BasicPayloadCatalog {
    use crate::domain::{
        AccidentalConditionTrigger, AccidentalPolarityBand, AccidentalScoringParams,
        BasicProductScoringProfile,
    };

    BasicPayloadCatalog::build(
        BasicProductScoringProfile {
            product_code: "basic".to_string(),
            payload_contract_version: "natal_structured_v13".to_string(),
            essential_dignity_score_profile_id: 5,
            accidental_scoring_params_id: 1,
            default_major_orb_deg: 8.0,
            sign_emphasis_full_score: 4.6,
            house_emphasis_full_score: 4.6,
            object_emphasis_full_score: 2.4,
            sign_house_emphasis_min_score: 0.35,
            object_emphasis_min_score: 0.5,
            house_axis_full_score: 2.5,
            axis_min_score: 0.35,
            axis_secondary_weight: 0.35,
            axis_polarity_dominance_delta: 0.2,
            axis_balanced_min_score: 0.35,
            max_dominant_signs: 3,
            max_dominant_houses: 3,
            max_dominant_objects: 5,
            max_active_signals: 12,
            aspect_min_strength: 0.4,
            max_house_axis_emphasis: 3,
        },
        test_essential_dignity_rules(),
        vec![
            AccidentalConditionTrigger {
                trigger_family: "house_modality".to_string(),
                source_code: Some("angular".to_string()),
                angle_object_code: None,
                condition_code: "angular_house".to_string(),
            },
            AccidentalConditionTrigger {
                trigger_family: "angle_proximity".to_string(),
                source_code: None,
                angle_object_code: Some("ascendant".to_string()),
                condition_code: "near_ascendant".to_string(),
            },
            AccidentalConditionTrigger {
                trigger_family: "sect_match".to_string(),
                source_code: None,
                angle_object_code: None,
                condition_code: "sect_affinity_match".to_string(),
            },
        ],
        AccidentalScoringParams {
            code: "basic_mvp".to_string(),
            overall_score_baseline: 0.5,
            overall_score_min: 0.0,
            overall_score_max: 1.0,
            angle_proximity_max_orb_deg: 10.0,
        },
        vec![
            AccidentalPolarityBand {
                polarity_code: "strongly_weakened".to_string(),
                expression_quality_code: "strongly_constrained_expression".to_string(),
                min_score: 0.0,
                max_score: 0.3,
                sort_order: 1,
            },
            AccidentalPolarityBand {
                polarity_code: "weakened".to_string(),
                expression_quality_code: "constrained_expression".to_string(),
                min_score: 0.3,
                max_score: 0.45,
                sort_order: 2,
            },
            AccidentalPolarityBand {
                polarity_code: "mixed_or_contextual".to_string(),
                expression_quality_code: "mixed_or_contextual_expression".to_string(),
                min_score: 0.45,
                max_score: 0.7,
                sort_order: 3,
            },
            AccidentalPolarityBand {
                polarity_code: "fortified".to_string(),
                expression_quality_code: "strong_external_manifestation".to_string(),
                min_score: 0.7,
                max_score: 1.0,
                sort_order: 4,
            },
        ],
    )
}

const POLARITY_BAND_SCORE_TOLERANCE: f64 = 0.000_001;

pub fn accidental_polarity_bands_are_valid(bands: &[AccidentalPolarityBand]) -> bool {
    if bands.is_empty() {
        return false;
    }

    let mut sorted: Vec<_> = bands.to_vec();
    sorted.sort_by_key(|band| band.sort_order);

    let mut seen_polarities = std::collections::HashSet::new();
    for band in &sorted {
        if band.polarity_code.trim().is_empty()
            || band.expression_quality_code.trim().is_empty()
            || band.min_score >= band.max_score
            || !seen_polarities.insert(band.polarity_code.as_str())
        {
            return false;
        }
    }

    if sorted.first().is_none_or(|band| band.min_score != 0.0)
        || sorted.last().is_none_or(|band| band.max_score != 1.0)
    {
        return false;
    }

    sorted.windows(2).all(|window| {
        (window[0].max_score - window[1].min_score).abs() <= POLARITY_BAND_SCORE_TOLERANCE
    })
}

pub fn overall_polarity_for_score_with_bands(
    score: f64,
    bands: &[AccidentalPolarityBand],
) -> (String, String) {
    let mut bands: Vec<AccidentalPolarityBand> = bands.to_vec();
    bands.sort_by(|left, right| {
        left.sort_order.cmp(&right.sort_order).then_with(|| {
            left.min_score
                .partial_cmp(&right.min_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });
    for (index, band) in bands.iter().enumerate() {
        let is_last = index + 1 == bands.len();
        if score >= band.min_score && (is_last || score < band.max_score) {
            return (
                band.polarity_code.clone(),
                band.expression_quality_code.clone(),
            );
        }
    }
    let Some(last) = bands.last() else {
        return (String::new(), String::new());
    };
    (
        last.polarity_code.clone(),
        last.expression_quality_code.clone(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::AccidentalPolarityBand;

    #[test]
    fn polarity_bands_must_cover_zero_to_one_without_gaps() {
        let valid = test_catalog().accidental_polarity_bands;
        assert!(accidental_polarity_bands_are_valid(&valid));

        let mut gapped = valid.clone();
        gapped[1].min_score = 0.5;
        assert!(!accidental_polarity_bands_are_valid(&gapped));
    }

    #[test]
    fn overall_polarity_respects_upper_bound_of_last_band() {
        let bands = test_catalog().accidental_polarity_bands;
        let (polarity, _) = overall_polarity_for_score_with_bands(1.0, &bands);
        assert_eq!(polarity, "fortified");
    }

    #[test]
    fn empty_polarity_bands_are_invalid() {
        assert!(!accidental_polarity_bands_are_valid(&[]));
        assert!(!accidental_polarity_bands_are_valid(&[
            AccidentalPolarityBand {
                polarity_code: "fortified".to_string(),
                expression_quality_code: "strong_external_manifestation".to_string(),
                min_score: 0.2,
                max_score: 1.0,
                sort_order: 1,
            }
        ]));
    }
}
