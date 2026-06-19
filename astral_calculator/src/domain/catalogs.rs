use std::collections::HashMap;

use serde::Deserialize;

use crate::domain::{
    AccidentalConditionTrigger, AccidentalDignityConditionReference, AccidentalPolarityBand,
    AccidentalScoringParams, BasicProductScoringProfile, EssentialDignityRuleReference,
    ObjectSectAffinityReference, ProjectionLabelDefinition, ProjectionReasonDefinition,
};

#[derive(Debug, Clone)]
pub struct BasicPayloadCatalog {
    pub product_scoring: BasicProductScoringProfile,
    pub essential_dignity_rules: Vec<EssentialDignityRuleReference>,
    pub accidental_triggers: Vec<AccidentalConditionTrigger>,
    pub accidental_scoring: AccidentalScoringParams,
    pub accidental_polarity_bands: Vec<AccidentalPolarityBand>,
    pub projection_reason_definitions: Vec<ProjectionReasonDefinition>,
    pub projection_label_definitions: Vec<ProjectionLabelDefinition>,
    essential_by_object_sign: HashMap<(String, String), Vec<EssentialDignityRuleReference>>,
    triggers_by_family: HashMap<String, Vec<AccidentalConditionTrigger>>,
    dignity_weight_by_type: HashMap<String, EssentialDignityScoringWeight>,
    projection_reason_by_code: HashMap<String, ProjectionReasonDefinition>,
    projection_label_by_family_code: HashMap<(String, String), ProjectionLabelDefinition>,
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
        projection_reason_definitions: Vec<ProjectionReasonDefinition>,
        projection_label_definitions: Vec<ProjectionLabelDefinition>,
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

        let projection_reason_by_code = projection_reason_definitions
            .iter()
            .cloned()
            .map(|definition| (definition.reason_code.clone(), definition))
            .collect();
        let projection_label_by_family_code = projection_label_definitions
            .iter()
            .cloned()
            .map(|definition| {
                (
                    (
                        definition.label_family.clone(),
                        definition.label_code.clone(),
                    ),
                    definition,
                )
            })
            .collect();

        Self {
            product_scoring,
            essential_dignity_rules,
            accidental_triggers,
            accidental_scoring,
            accidental_polarity_bands,
            projection_reason_definitions,
            projection_label_definitions,
            essential_by_object_sign,
            triggers_by_family,
            dignity_weight_by_type,
            projection_reason_by_code,
            projection_label_by_family_code,
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

    pub fn projection_reason_definition(
        &self,
        reason_code: &str,
    ) -> Option<&ProjectionReasonDefinition> {
        self.projection_reason_by_code.get(reason_code)
    }

    pub fn projection_label_definition(
        &self,
        label_family: &str,
        label_code: &str,
    ) -> Option<&ProjectionLabelDefinition> {
        self.projection_label_by_family_code
            .get(&(label_family.to_string(), label_code.to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct SimplifiedCatalog {
    pub policy: SimplifiedPolicy,
    pub limitation_codes: Vec<LimitationCode>,
    pub reliability_levels: Vec<ReliabilityLevel>,
    pub calculation_scopes: Vec<CalculationScope>,
    pub input_precision_levels: Vec<InputPrecisionLevel>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SimplifiedPolicy {
    pub code: String,
    pub reference_time_utc: String,
    pub date_only_uncertainty_mode: String,
    pub uncertainty_sampling_minutes: i32,
    pub default_timezone_strategy: String,
    pub cusp_warning_orb_deg: f64,
    pub stable_fact_strategy: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LimitationCode {
    pub code: String,
    pub severity: String,
    pub affected_features_json: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReliabilityLevel {
    pub code: String,
    pub allows_interpretive_affirmation: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CalculationScope {
    pub code: String,
    pub min_input_precision_code: String,
    pub supports_angles: bool,
    pub supports_houses: bool,
    pub supports_aspects: bool,
    pub supports_object_sign_facts: bool,
    pub supports_ambiguous_facts: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InputPrecisionLevel {
    pub code: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProfileFeatureExclusion {
    pub profile_code: String,
    pub computed_scope_code: Option<String>,
    pub feature_code: String,
    pub exclusion_kind: String,
    pub sort_order: i32,
}

impl SimplifiedCatalog {
    pub fn limitation(&self, code: &str) -> Option<&LimitationCode> {
        self.limitation_codes
            .iter()
            .find(|entry| entry.code == code)
    }

    pub fn allows_interpretive_affirmation(&self, reliability: &str) -> bool {
        self.reliability_levels
            .iter()
            .find(|level| level.code == reliability)
            .is_some_and(|level| level.allows_interpretive_affirmation)
    }

    pub fn scope(&self, code: &str) -> Option<&CalculationScope> {
        self.calculation_scopes
            .iter()
            .find(|entry| entry.code == code)
    }

    pub fn input_precision(&self, code: &str) -> Option<&InputPrecisionLevel> {
        self.input_precision_levels
            .iter()
            .find(|entry| entry.code == code)
    }

    pub fn affected_features(limitation: &LimitationCode) -> Vec<String> {
        limitation
            .affected_features_json
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn profile_feature_exclusions_for(
        exclusions: &[ProfileFeatureExclusion],
        profile_code: &str,
        computed_scope: &str,
    ) -> Vec<String> {
        let mut out = Vec::new();
        for row in exclusions {
            if row.profile_code != profile_code {
                continue;
            }
            let scope_matches = row
                .computed_scope_code
                .as_ref()
                .map(|scope| scope == computed_scope)
                .unwrap_or(true);
            if scope_matches && !out.iter().any(|existing| existing == &row.feature_code) {
                out.push(row.feature_code.clone());
            }
        }
        out
    }
}

#[derive(Debug, Clone)]
pub struct HoroscopeSignalThemeMapping {
    pub match_object: String,
    pub match_aspect: Option<String>,
    pub match_natal_target: Option<String>,
    pub theme_code: String,
}

#[derive(Debug, Clone)]
pub struct HoroscopeSupportedObject {
    pub object_code: String,
    pub weight: f64,
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
