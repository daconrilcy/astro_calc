//! Module astral_calculator\src\features\natal\catalog.rs du moteur astral_calculator.

use std::collections::HashMap;

use crate::domain::{
    AccidentalConditionTrigger, AccidentalDignityConditionReference, AccidentalPolarityBand,
    AccidentalScoringParams, BasicProductScoringProfile, EssentialDignityRuleReference,
    ObjectSectAffinityReference, ProjectionLabelDefinition, ProjectionReasonDefinition,
};

#[derive(Debug, Clone)]
/// Structure BasicPayloadCatalog.
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
/// Structure EssentialDignityScoringWeight.
pub struct EssentialDignityScoringWeight {
    pub priority_delta: f64,
    pub signal_weight_delta: f64,
    pub signal_worthy_min_strength: f64,
    pub emphasis_weight: f64,
}

impl BasicPayloadCatalog {
    /// Fonction build.
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

    /// Fonction essential_rules_for.
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

    /// Fonction dignity_scoring_weight.
    pub fn dignity_scoring_weight(
        &self,
        dignity_type: &str,
    ) -> Option<&EssentialDignityScoringWeight> {
        self.dignity_weight_by_type.get(dignity_type)
    }

    /// Fonction condition_code_for_house_modality.
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

    /// Fonction condition_code_for_motion_state.
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

    /// Fonction condition_code_for_horizon_position.
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

    /// Fonction angle_proximity_triggers.
    pub fn angle_proximity_triggers(&self) -> &[AccidentalConditionTrigger] {
        self.triggers_by_family
            .get("angle_proximity")
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Fonction sect_condition_code.
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

    /// Fonction overall_polarity_for_score.
    pub fn overall_polarity_for_score(&self, score: f64) -> (String, String) {
        overall_polarity_for_score_with_bands(score, &self.accidental_polarity_bands)
    }

    /// Fonction valid_accidental_condition_codes.
    pub fn valid_accidental_condition_codes(
        definitions: &[AccidentalDignityConditionReference],
    ) -> Vec<&str> {
        definitions
            .iter()
            .map(|definition| definition.condition_code.as_str())
            .collect()
    }

    /// Fonction projection_reason_definition.
    pub fn projection_reason_definition(
        &self,
        reason_code: &str,
    ) -> Option<&ProjectionReasonDefinition> {
        self.projection_reason_by_code.get(reason_code)
    }

    /// Fonction projection_label_definition.
    pub fn projection_label_definition(
        &self,
        label_family: &str,
        label_code: &str,
    ) -> Option<&ProjectionLabelDefinition> {
        self.projection_label_by_family_code
            .get(&(label_family.to_string(), label_code.to_string()))
    }
}

/// Fonction test_essential_dignity_rules.
fn test_essential_dignity_rules() -> Vec<EssentialDignityRuleReference> {
    use crate::domain::EssentialDignityRuleReference;

    /// Fonction rule.
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

fn test_projection_reason_definitions() -> Vec<ProjectionReasonDefinition> {
    fn definition(
        reason_code: &str,
        reason_family: &str,
        label_template_en: &str,
        requires_object: bool,
        requires_dignity_type: bool,
        requires_sign_code: bool,
        requires_house_number: bool,
        requires_theme_code: bool,
        requires_angle_code: bool,
        requires_signal_key: bool,
        requires_context_key: bool,
        sort_order: i32,
    ) -> ProjectionReasonDefinition {
        ProjectionReasonDefinition {
            reason_code: reason_code.to_string(),
            reason_family: reason_family.to_string(),
            label_template_en: label_template_en.to_string(),
            requires_object,
            requires_dignity_type,
            requires_sign_code,
            requires_house_number,
            requires_theme_code,
            requires_angle_code,
            requires_signal_key,
            requires_context_key,
            is_active: true,
            sort_order,
        }
    }

    vec![
        definition(
            "object_in_sign",
            "placement",
            "{object} in sign",
            true,
            false,
            true,
            false,
            false,
            false,
            false,
            false,
            10,
        ),
        definition(
            "object_in_house",
            "placement",
            "{object} in house",
            true,
            false,
            false,
            true,
            true,
            false,
            false,
            false,
            20,
        ),
        definition(
            "placement",
            "placement",
            "Strong placement",
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            30,
        ),
        definition(
            "multiple_objects",
            "cluster",
            "Several chart factors are concentrated in the same sign and house",
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            40,
        ),
        definition(
            "sign_house_cluster",
            "cluster",
            "Several chart factors are concentrated in the same sign and house",
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            50,
        ),
        definition(
            "cluster",
            "cluster",
            "Dominant house cluster",
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            60,
        ),
        definition(
            "cluster_participant",
            "cluster",
            "Participant in dominant theme",
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            70,
        ),
        definition(
            "essential_dignity",
            "dignity",
            "{object} in {dignity}",
            true,
            true,
            false,
            false,
            false,
            false,
            false,
            false,
            80,
        ),
        definition(
            "sign_emphasis",
            "sign",
            "{sign} emphasis",
            false,
            false,
            true,
            false,
            false,
            false,
            false,
            false,
            90,
        ),
        definition(
            "strong_aspect_participant",
            "aspect",
            "Involved in a strong major aspect",
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            100,
        ),
        definition(
            "accidental_context",
            "context",
            "Reinforced or modified by accidental conditions",
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            110,
        ),
        definition(
            "dominant_house",
            "axis",
            "Dominant house emphasis",
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            120,
        ),
        definition(
            "luminary_in_house",
            "axis",
            "{object} highlights this house",
            true,
            false,
            false,
            true,
            true,
            false,
            false,
            false,
            130,
        ),
        definition(
            "angle_in_house",
            "axis",
            "{angle} emphasizes this house",
            false,
            false,
            false,
            true,
            true,
            true,
            false,
            false,
            140,
        ),
        definition(
            "active_signal",
            "axis",
            "Active chart signal",
            false,
            false,
            false,
            false,
            false,
            false,
            true,
            false,
            150,
        ),
        definition(
            "rulership_context",
            "axis",
            "Supported by rulership links",
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            true,
            160,
        ),
        definition(
            "theme_emphasis",
            "axis",
            "{theme} theme emphasized",
            false,
            false,
            false,
            false,
            true,
            false,
            false,
            false,
            170,
        ),
        definition(
            "cross_axis_aspect",
            "axis",
            "A major aspect connects both sides of this house axis",
            false,
            false,
            false,
            false,
            false,
            false,
            true,
            false,
            180,
        ),
    ]
}

fn test_projection_label_definitions() -> Vec<ProjectionLabelDefinition> {
    fn definition(
        label_family: &str,
        label_code: &str,
        label_template_en: &str,
        sort_order: i32,
    ) -> ProjectionLabelDefinition {
        ProjectionLabelDefinition {
            label_family: label_family.to_string(),
            label_code: label_code.to_string(),
            label_template_en: label_template_en.to_string(),
            is_active: true,
            sort_order,
        }
    }

    vec![
        definition("angle_display", "ascendant", "Ascendant", 10),
        definition("angle_display", "descendant", "Descendant", 20),
        definition("angle_display", "mc", "The Midheaven", 30),
        definition("angle_display", "ic", "The IC", 40),
        definition(
            "axis_balance",
            "primary_house_dominant",
            "Mainly house {primary_house}",
            10,
        ),
        definition(
            "axis_balance",
            "secondary_house_dominant",
            "Mainly house {secondary_house}",
            20,
        ),
        definition(
            "axis_balance",
            "balanced_axis",
            "Balanced houses {primary_house} and {secondary_house}",
            30,
        ),
        definition("chart_sect", "day", "Day chart", 10),
        definition("chart_sect", "night", "Night chart", 20),
        definition(
            "condition_variant",
            "sect_affinity_match_day",
            "Day sect match",
            10,
        ),
        definition(
            "condition_variant",
            "sect_affinity_match_night",
            "Night sect match",
            20,
        ),
        definition(
            "condition_variant",
            "sect_affinity_match_default",
            "Sect match",
            30,
        ),
        definition(
            "dignity_meaning",
            "domicile",
            "Strong functional expression",
            10,
        ),
        definition("dignity_meaning", "exaltation", "Constructive emphasis", 20),
        definition(
            "dignity_meaning",
            "detriment",
            "Challenged functional expression",
            30,
        ),
        definition("dignity_meaning", "fall", "Weakened expression", 40),
        definition("dignity_meaning", "default", "Notable dignity context", 50),
        definition("dynamic_quality", "tension", "Tension", 10),
        definition("dynamic_quality", "flow", "Flow", 20),
        definition("dynamic_quality", "adjustment", "Adjustment", 30),
        definition("dynamic_quality", "symbolic", "Symbolic", 40),
        definition("dynamic_quality", "integration", "Integration", 50),
        definition("dynamic_quality", "intensification", "Intensification", 60),
        definition("dynamic_quality", "contextual", "Contextual", 70),
        definition("hemisphere_area", "below_horizon", "Below horizon", 10),
        definition("hemisphere_area", "above_horizon", "Above horizon", 20),
        definition("hemisphere_area", "balanced", "Balanced hemispheres", 30),
        definition("phase", "separating", "Separating", 10),
        definition("phase", "applying", "Applying", 20),
        definition("phase", "exact", "Exact", 30),
        definition("motion_display", "direct", "Direct motion", 10),
        definition("motion_display", "retrograde", "Retrograde motion", 20),
        definition("motion_display", "stationary", "Stationary motion", 30),
        definition("reading_slot", "core_identity", "Core identity", 10),
        definition("reading_slot", "dominant_cluster", "Dominant theme", 20),
        definition(
            "reading_slot",
            "main_tension_or_support",
            "Main dynamic",
            30,
        ),
        definition("reading_slot", "expression_style", "Expression style", 40),
        definition(
            "reading_slot",
            "background_factors",
            "Background factors",
            50,
        ),
        definition("valence", "polarizing", "Polarizing", 10),
        definition("valence", "supportive", "Supportive", 20),
        definition("valence", "harmonious", "Harmonious", 30),
        definition("valence", "dynamic_challenging", "Dynamic challenging", 40),
        definition("valence", "minor_friction", "Minor friction", 50),
        definition("valence", "indirect_tension", "Indirect tension", 60),
        definition("valence", "adjustment", "Adjustment", 70),
        definition("valence", "subtle_adjustment", "Subtle adjustment", 80),
        definition("valence", "creative", "Creative", 90),
        definition("valence", "refined_creative", "Creative", 100),
        definition("valence", "creative_ordering", "Creative", 110),
        definition("valence", "symbolic_fated", "Symbolic", 120),
        definition("valence", "spiritual_integration", "Integrating", 130),
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
            payload_contract_version: "natal_structured_v14".to_string(),
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
        test_projection_reason_definitions(),
        test_projection_label_definitions(),
    )
}

const POLARITY_BAND_SCORE_TOLERANCE: f64 = 0.000_001;

/// Fonction accidental_polarity_bands_are_valid.
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

/// Fonction overall_polarity_for_score_with_bands.
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
