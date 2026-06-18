//! Module astral_calculator\src\domain\references.rs du moteur astral_calculator.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone)]
/// Structure ChartObject.
pub struct ChartObject {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub swe_id: Option<i32>,
    pub role_code: Option<String>,
    pub role_label: Option<String>,
    pub is_luminary: Option<bool>,
    pub is_planet_symbolic: Option<bool>,
    pub is_visible_to_naked_eye: Option<bool>,
    pub nature_codes: Option<Value>,
    pub position_priority_base: Option<f64>,
    pub angle_priority_base: Option<f64>,
    pub source_weight: Option<f64>,
}

#[derive(Debug, Clone)]
/// Structure AspectDefinition.
pub struct AspectDefinition {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub angle: f64,
    pub family: String,
    pub default_orb_deg: Option<f64>,
    pub max_default_orb_deg: f64,
}

#[derive(Debug, Clone)]
/// Structure HouseSystem.
pub struct HouseSystem {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub calculation_engine_code: String,
}

#[derive(Debug, Clone)]
/// Structure SignReference.
pub struct SignReference {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub element_code: Option<String>,
    pub element_label: Option<String>,
    pub modality_code: Option<String>,
    pub modality_name: Option<String>,
    pub polarity_code: Option<String>,
    pub polarity_name: Option<String>,
    pub keywords_json: Option<Value>,
    pub shadow_keywords_json: Option<Value>,
}

#[derive(Debug, Clone)]
/// Structure HouseReference.
pub struct HouseReference {
    pub id: i32,
    pub number: i32,
    pub name: String,
    pub theme_code: String,
    pub modality_code: Option<String>,
    pub modality_label: Option<String>,
    pub accidental_strength: Option<String>,
    pub modality_priority_delta: Option<f64>,
    pub interpretation_weight: Option<String>,
}

#[derive(Debug, Clone)]
/// Structure MotionStateReference.
pub struct MotionStateReference {
    pub id: i32,
    pub code: String,
    pub label: String,
    pub motion_family: String,
}

#[derive(Debug, Clone)]
/// Structure HorizonPositionReference.
pub struct HorizonPositionReference {
    pub id: i32,
    pub code: String,
    pub label: String,
}

#[derive(Debug, Clone)]
/// Structure AnglePointReference.
pub struct AnglePointReference {
    pub id: i32,
    pub code: String,
    pub short_label: String,
    pub full_name: String,
    pub axis: String,
    pub opposite_angle_code: Option<String>,
    pub associated_house: i32,
    pub description: String,
    pub chart_object_id: i32,
    pub chart_object_code: String,
    pub chart_object_name: String,
    pub chart_object_sort_order: i32,
}

#[derive(Debug, Clone)]
/// Structure DomicileRulerReference.
pub struct DomicileRulerReference {
    pub reference_version_id: Option<i32>,
    pub astral_system_id: i32,
    pub astral_system_code: String,
    pub sign_id: i32,
    pub sign_code: String,
    pub sign_name: String,
    pub chart_object_id: i32,
    pub object_code: String,
    pub object_name: String,
    pub dignity_type: String,
    pub weight: f64,
    pub is_primary: bool,
}

#[derive(Debug, Clone)]
/// Structure InterpretationSignalRow.
pub struct InterpretationSignalRow {
    pub id: i32,
    pub signal_key: String,
    pub theme_code: Option<String>,
    pub title: String,
    pub summary: Option<String>,
    pub priority_score: f64,
    pub confidence_score: Option<f64>,
    pub payload_json: Option<Value>,
}

/// Structure CalculationReferenceData.
pub struct CalculationReferenceData {
    pub signs: Vec<SignReference>,
    pub houses: Vec<HouseReference>,
    pub motion_states: Vec<MotionStateReference>,
    pub horizon_positions: Vec<HorizonPositionReference>,
    pub angle_points: Vec<AnglePointReference>,
}

#[derive(Debug, Clone)]
/// Structure HouseAxisReference.
pub struct HouseAxisReference {
    pub axis_code: String,
    pub house_a_number: i32,
    pub house_b_number: i32,
    pub theme_a_code: String,
    pub theme_b_code: String,
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone)]
/// Structure LunarPhaseReference.
pub struct LunarPhaseReference {
    pub phase_code: String,
    pub label: String,
    pub cycle_family: String,
    pub range_start_deg: f64,
    pub range_end_deg: f64,
    pub exact_anchor_deg: f64,
    pub is_major_lunar_phase: bool,
    pub description: String,
}

#[derive(Debug, Clone)]
/// Structure AccidentalDignityConditionReference.
pub struct AccidentalDignityConditionReference {
    pub condition_code: String,
    pub condition_family: String,
    pub label: String,
    pub polarity: String,
    pub strength_score: f64,
    pub score_delta: f64,
    pub description: String,
}

#[derive(Debug, Clone)]
/// Structure ObjectSectAffinityReference.
pub struct ObjectSectAffinityReference {
    pub object_code: String,
    pub sect_affinity_code: String,
    pub is_variable: bool,
    pub description: String,
}

#[derive(Debug, Clone)]
/// Structure EssentialDignityRuleReference.
pub struct EssentialDignityRuleReference {
    pub object_code: String,
    pub sign_code: String,
    pub dignity_type: String,
    pub dignity_label: String,
    pub polarity: String,
    pub strength_score: f64,
    pub priority_delta: f64,
    pub signal_weight_delta: f64,
    pub signal_worthy_min_strength: f64,
    pub emphasis_weight: f64,
}

#[derive(Debug, Clone)]
/// Structure AccidentalConditionTrigger.
pub struct AccidentalConditionTrigger {
    pub trigger_family: String,
    pub source_code: Option<String>,
    pub angle_object_code: Option<String>,
    pub condition_code: String,
}

#[derive(Debug, Clone)]
/// Structure AccidentalScoringParams.
pub struct AccidentalScoringParams {
    pub code: String,
    pub overall_score_baseline: f64,
    pub overall_score_min: f64,
    pub overall_score_max: f64,
    pub angle_proximity_max_orb_deg: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Structure AccidentalPolarityBand.
pub struct AccidentalPolarityBand {
    pub polarity_code: String,
    pub expression_quality_code: String,
    pub min_score: f64,
    pub max_score: f64,
    pub sort_order: i32,
}

#[derive(Debug, Clone)]
/// Structure BasicProductScoringProfile.
pub struct BasicProductScoringProfile {
    pub product_code: String,
    pub payload_contract_version: String,
    pub essential_dignity_score_profile_id: i32,
    pub accidental_scoring_params_id: i32,
    pub default_major_orb_deg: f64,
    pub sign_emphasis_full_score: f64,
    pub house_emphasis_full_score: f64,
    pub object_emphasis_full_score: f64,
    pub sign_house_emphasis_min_score: f64,
    pub object_emphasis_min_score: f64,
    pub house_axis_full_score: f64,
    pub axis_min_score: f64,
    pub axis_secondary_weight: f64,
    pub axis_polarity_dominance_delta: f64,
    pub axis_balanced_min_score: f64,
    pub max_dominant_signs: usize,
    pub max_dominant_houses: usize,
    pub max_dominant_objects: usize,
    pub max_active_signals: usize,
    pub aspect_min_strength: f64,
    pub max_house_axis_emphasis: usize,
}
