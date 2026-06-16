#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicPayload {
    pub product_code: String,
    pub chart_calculation_id: i32,
    pub reference_version_id: i32,
    pub subject_label: Option<String>,
    pub birth_datetime_utc: DateTime<Utc>,
    #[serde(default)]
    pub chart_context: BasicChartContext,
    pub positions: Vec<BasicObjectPosition>,
    #[serde(default)]
    pub angles: Vec<BasicAngleFact>,
    #[serde(default)]
    pub dignities: Vec<BasicDignity>,
    #[serde(default)]
    pub chart_emphasis: BasicChartEmphasis,
    #[serde(default)]
    pub rulership_context: BasicRulershipContext,
    #[serde(default)]
    pub house_axis_emphasis: Vec<BasicHouseAxisEmphasis>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lunar_phase_context: Option<BasicLunarPhaseContext>,
    #[serde(default)]
    pub accidental_dignities: Vec<BasicAccidentalDignityEvaluation>,
    pub signals: Vec<BasicSignal>,
    #[serde(default)]
    pub reading_plan: Vec<BasicReadingPlanItem>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BasicChartContext {
    pub chart_type: String,
    pub zodiacal_reference_system_id: i32,
    pub coordinate_reference_system_id: i32,
    pub house_system_id: i32,
    pub reference_version_id: i32,
    pub payload_contract: BasicPayloadContract,
    pub calculation_reliability: BasicCalculationReliability,
    pub sect: BasicSectContext,
    pub hemisphere_emphasis: BasicHemisphereEmphasis,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accidental_scoring: Option<BasicAccidentalScoringSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_scoring: Option<BasicProductScoringSnapshot>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BasicPayloadContract {
    pub contract_version: String,
    pub calculation_scope: String,
    pub interpretation_scope: String,
    pub projection_depth: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BasicCalculationReliability {
    pub birth_time_precision_required: bool,
    pub house_system_sensitive: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BasicSectContext {
    pub chart_sect: Option<String>,
    pub sun_horizon_position: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BasicHemisphereEmphasis {
    #[serde(default)]
    pub count_scope: String,
    pub above_horizon_count: i32,
    pub below_horizon_count: i32,
    pub on_horizon_count: i32,
    pub interpretive_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicAngleFact {
    pub angle_code: String,
    pub angle_name: String,
    pub axis: String,
    pub opposite_angle_code: String,
    pub longitude_deg: f64,
    pub sign_id: i32,
    pub sign_code: String,
    pub sign_name: String,
    pub house_id: Option<i32>,
    pub house_number: i32,
    pub house_name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BasicChartEmphasis {
    #[serde(default)]
    pub dominant_signs: Vec<BasicDominantSign>,
    #[serde(default)]
    pub dominant_houses: Vec<BasicDominantHouse>,
    #[serde(default)]
    pub dominant_objects: Vec<BasicDominantObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicDominantSign {
    pub sign_code: String,
    pub score: f64,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicDominantHouse {
    pub house_number: i32,
    pub theme_code: String,
    pub score: f64,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicDominantObject {
    pub object_code: String,
    pub score: f64,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicDignity {
    pub object_code: String,
    pub object_name: String,
    pub sign_id: i32,
    pub sign_code: String,
    pub sign_name: String,
    pub dignity_type: String,
    pub dignity_label: String,
    pub polarity: String,
    pub strength_score: f64,
    pub signal_key: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BasicRulershipContext {
    #[serde(default)]
    pub ascendant_ruler: Option<BasicRulerContext>,
    #[serde(default)]
    pub mc_ruler: Option<BasicRulerContext>,
    #[serde(default)]
    pub descendant_ruler: Option<BasicRulerContext>,
    #[serde(default)]
    pub dominant_house_rulers: Vec<BasicRulerContext>,
    #[serde(default)]
    pub dominant_sign_rulers: Vec<BasicRulerContext>,
    #[serde(default)]
    pub dispositor_links: Vec<BasicDispositorLink>,
    #[serde(default)]
    pub rulership_chains: Vec<BasicRulershipChain>,
    #[serde(default)]
    pub final_dispositors: Vec<BasicFinalDispositor>,
    #[serde(default)]
    pub mutual_receptions: Vec<BasicMutualReception>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicRulerContext {
    pub context_key: String,
    pub source_kind: String,
    pub source_code: String,
    pub sign_code: String,
    #[serde(default)]
    pub ruler_object_codes: Vec<String>,
    pub ruler_object_code: String,
    pub ruler_position_signal_key: Option<String>,
    pub ruler_house_number: Option<i32>,
    pub ruler_sign_code: Option<String>,
    pub interpretive_role: String,
    #[serde(default)]
    pub strength_context: Vec<String>,
    #[serde(default)]
    pub ruler_sources: Vec<BasicRulerSource>,
    pub interpretive_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicRulerSource {
    pub object_code: String,
    pub reference_version_id: Option<i32>,
    pub astral_system_id: i32,
    pub astral_system_code: String,
    pub dignity_type: String,
    pub weight: f64,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicDispositorLink {
    pub object_code: String,
    pub object_sign_code: String,
    pub dispositor_object_code: String,
    pub dispositor_signal_key: String,
    #[serde(default)]
    pub ruler_sources: Vec<BasicRulerSource>,
    pub interpretive_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicRulershipChain {
    pub object_code: String,
    #[serde(default)]
    pub chain: Vec<String>,
    pub termination: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicFinalDispositor {
    pub object_code: String,
    #[serde(default)]
    pub source_objects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicMutualReception {
    #[serde(default)]
    pub object_codes: Vec<String>,
    #[serde(default)]
    pub source_objects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicHouseAxisEmphasis {
    pub axis_code: String,
    #[serde(default)]
    pub houses: Vec<i32>,
    #[serde(default)]
    pub theme_codes: Vec<String>,
    #[serde(default)]
    pub house_scores: Vec<BasicHouseAxisScore>,
    pub primary_house: i32,
    pub secondary_house: i32,
    pub axis_score: f64,
    pub polarity_balance: String,
    #[serde(default)]
    pub source_signal_keys: Vec<String>,
    #[serde(default)]
    pub source_context_keys: Vec<String>,
    #[serde(default)]
    pub reasons: Vec<String>,
    pub interpretive_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicHouseAxisScore {
    pub house_number: i32,
    pub theme_code: String,
    pub score: f64,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicLunarPhaseContext {
    pub phase_code: String,
    pub phase_label: String,
    pub cycle_family: String,
    pub sun_object_code: String,
    pub moon_object_code: String,
    pub sun_longitude_deg: f64,
    pub moon_longitude_deg: f64,
    pub sun_moon_angle_deg: f64,
    #[serde(default)]
    pub phase_angle_range_deg: Vec<f64>,
    pub exact_phase_anchor_deg: f64,
    pub distance_to_exact_phase_deg: f64,
    pub phase_progress_ratio: f64,
    pub is_major_lunar_phase: bool,
    #[serde(default)]
    pub related_signal_keys: Vec<String>,
    #[serde(default)]
    pub related_reading_slots: Vec<String>,
    #[serde(default)]
    pub semantic_tags: Vec<String>,
    pub interpretive_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicAccidentalDignityEvaluation {
    pub object_code: String,
    pub object_name: String,
    pub overall_score: f64,
    pub overall_polarity: String,
    pub expression_quality: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_signal_key: Option<String>,
    pub conditions: Vec<BasicAccidentalDignityCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicAccidentalDignityCondition {
    pub condition_code: String,
    pub condition_family: String,
    pub polarity: String,
    pub strength_score: f64,
    pub score_delta: f64,
    pub source: serde_json::Value,
    pub interpretive_hint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BasicAccidentalDignityContextSummary {
    pub condition_code: String,
    pub condition_family: String,
    pub polarity: String,
    pub strength_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicObjectPosition {
    pub object_code: String,
    pub object_name: String,
    pub longitude_deg: f64,
    pub sign_id: i32,
    #[serde(default)]
    pub sign_code: String,
    #[serde(default)]
    pub sign_name: String,
    pub house_id: Option<i32>,
    pub house_number: Option<i32>,
    pub house_name: Option<String>,
    pub motion_state_id: Option<i32>,
    #[serde(default)]
    pub sign_context: Option<Value>,
    #[serde(default)]
    pub house_context: Option<Value>,
    #[serde(default)]
    pub house_modality: Option<Value>,
    #[serde(default)]
    pub object_context: Option<Value>,
    #[serde(default)]
    pub motion_context: Option<Value>,
    #[serde(default)]
    pub dignity_context: Value,
    #[serde(default)]
    pub visibility_context: Value,
    #[serde(default)]
    pub accidental_dignity_context: Vec<BasicAccidentalDignityContextSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicSignal {
    pub signal_key: String,
    pub theme_code: Option<String>,
    pub title: String,
    pub summary: Option<String>,
    pub priority_score: f64,
    pub confidence_score: Option<f64>,
    pub interpretive_hint: Option<String>,
    #[serde(default)]
    pub semantic_tags: Vec<String>,
    pub source_weight: Option<f64>,
    pub aggregation_group: Option<String>,
    #[serde(default)]
    pub aspect_context: Option<Value>,
    pub evidence: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicReadingPlanItem {
    pub slot: String,
    pub title: String,
    #[serde(default)]
    pub source_signal_keys: Vec<String>,
    #[serde(default)]
    pub primary_signal_keys: Vec<String>,
    #[serde(default)]
    pub secondary_slot_candidates: Vec<BasicSecondarySlotCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BasicSecondarySlotCandidate {
    pub signal_key: String,
    pub primary_slot: String,
    pub candidate_slot: String,
}
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{BasicAccidentalScoringSnapshot, BasicProductScoringSnapshot};
