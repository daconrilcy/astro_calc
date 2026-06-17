use serde::{Deserialize, Serialize};

pub type AnglePointReference = crate::infra::db::models::AnglePointReference;
pub type AspectDefinition = crate::infra::db::models::AspectDefinition;
pub type ChartObject = crate::infra::db::models::ChartObject;
pub type DomicileRulerReference = crate::infra::db::models::DomicileRulerReference;
pub type HorizonPositionReference = crate::infra::db::models::HorizonPositionReference;
pub type HouseReference = crate::infra::db::models::HouseReference;
pub type HouseSystem = crate::infra::db::models::HouseSystem;
pub type InterpretationSignalRow = crate::infra::db::models::InterpretationSignalRow;
pub type MotionStateReference = crate::infra::db::models::MotionStateReference;
pub type SignReference = crate::infra::db::models::SignReference;

pub struct CalculationReferenceData {
    pub signs: Vec<SignReference>,
    pub houses: Vec<HouseReference>,
    pub motion_states: Vec<MotionStateReference>,
    pub horizon_positions: Vec<HorizonPositionReference>,
    pub angle_points: Vec<AnglePointReference>,
}

#[derive(Debug, Clone)]
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
pub struct ObjectSectAffinityReference {
    pub object_code: String,
    pub sect_affinity_code: String,
    pub is_variable: bool,
    pub description: String,
}

#[derive(Debug, Clone)]
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
pub struct AccidentalConditionTrigger {
    pub trigger_family: String,
    pub source_code: Option<String>,
    pub angle_object_code: Option<String>,
    pub condition_code: String,
}

#[derive(Debug, Clone)]
pub struct AccidentalScoringParams {
    pub code: String,
    pub overall_score_baseline: f64,
    pub overall_score_min: f64,
    pub overall_score_max: f64,
    pub angle_proximity_max_orb_deg: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccidentalPolarityBand {
    pub polarity_code: String,
    pub expression_quality_code: String,
    pub min_score: f64,
    pub max_score: f64,
    pub sort_order: i32,
}

#[derive(Debug, Clone)]
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

