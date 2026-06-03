use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use crate::models::{
    AnglePointReference, AspectDefinition, ChartObject, DomicileRulerReference,
    HorizonPositionReference, HouseReference, HouseSystem, InterpretationSignalRow,
    MotionStateReference, SignReference,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatalChartInput {
    pub subject_label: Option<String>,
    pub birth_datetime_utc: DateTime<Utc>,
    pub latitude_deg: f64,
    pub longitude_deg: f64,
    pub altitude_m: Option<f64>,
    pub reference_version_id: i32,
    pub calculation_profile_id: Option<i32>,
    pub zodiacal_reference_system_id: i32,
    pub coordinate_reference_system_id: i32,
    pub house_system_id: i32,
    pub product_code: Option<String>,
}

impl NatalChartInput {
    pub fn product_code(&self) -> &str {
        self.product_code.as_deref().unwrap_or("basic")
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeOptions {
    pub engine_version: String,
    pub ephemeris_version: String,
    pub stale_after_seconds: i32,
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            ephemeris_version: "se-2026a".to_string(),
            stale_after_seconds: 900,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CalculationReferenceData {
    pub signs: Vec<SignReference>,
    pub houses: Vec<HouseReference>,
    pub motion_states: Vec<MotionStateReference>,
    pub horizon_positions: Vec<HorizonPositionReference>,
    pub angle_points: Vec<AnglePointReference>,
}

#[derive(Debug, Clone)]
pub struct ObjectPositionFact {
    pub chart_object_id: i32,
    pub object_code: String,
    pub object_name: String,
    pub zodiacal_reference_system_id: i32,
    pub coordinate_reference_system_id: i32,
    pub sign_id: i32,
    pub sign_code: String,
    pub sign_name: String,
    pub house_id: Option<i32>,
    pub house_number: Option<i32>,
    pub house_name: Option<String>,
    pub motion_state_id: Option<i32>,
    pub horizon_position_id: Option<i32>,
    pub longitude_deg: f64,
    pub latitude_deg: Option<f64>,
    pub apparent_speed_deg_per_day: Option<f64>,
    pub altitude_deg: Option<f64>,
    pub is_visible: Option<bool>,
    pub facts_json: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct HouseCuspFact {
    pub house_id: i32,
    pub house_number: i32,
    pub sign_id: i32,
    pub longitude_deg: f64,
}

#[derive(Debug, Clone)]
pub struct AspectFact {
    pub source_chart_object_id: i32,
    pub source_object_code: String,
    pub source_object_name: String,
    pub target_chart_object_id: i32,
    pub target_object_code: String,
    pub target_object_name: String,
    pub aspect_id: i32,
    pub aspect_code: String,
    pub aspect_name: String,
    pub aspect_family: String,
    pub orb_deg: f64,
    pub phase_state: String,
    pub is_applying: bool,
    pub is_exact: bool,
    pub strength_score: Option<f64>,
    pub primary_valence: Option<String>,
    pub intensity_modifier: Option<String>,
    pub secondary_effect: Option<String>,
    pub valence_family: Option<String>,
    pub valence_is_tonal: Option<bool>,
    pub valence_is_intensity_modifier: Option<bool>,
    pub valence_writing_guidance: Option<String>,
    pub calculation_notes_json: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct CalculatedChartFacts {
    pub positions: Vec<ObjectPositionFact>,
    pub house_cusps: Vec<HouseCuspFact>,
    pub aspects: Vec<AspectFact>,
}

#[derive(Debug, Clone)]
pub struct InterpretationSignalDraft {
    pub signal_key: String,
    pub signal_type_id: Option<i32>,
    pub theme_code: Option<String>,
    pub title: String,
    pub summary: Option<String>,
    pub priority_score: f64,
    pub confidence_score: Option<f64>,
    pub suppression_state: String,
    pub payload_json: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicPayload {
    pub product_code: String,
    pub chart_calculation_id: i32,
    pub reference_version_id: i32,
    pub subject_label: Option<String>,
    pub birth_datetime_utc: DateTime<Utc>,
    #[serde(default)]
    pub llm_handoff_contract: Option<BasicLlmHandoffContract>,
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
    pub signals: Vec<BasicSignal>,
    #[serde(default)]
    pub reading_plan: Vec<BasicReadingPlanItem>,
    #[serde(default)]
    pub drafting_plan: Vec<BasicDraftingPlanItem>,
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
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BasicPayloadContract {
    pub contract_version: String,
    pub calculation_scope: String,
    pub interpretation_scope: String,
    pub projection_depth: String,
    pub writing_contract: String,
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
    pub dominant_house_rulers: Vec<BasicRulerContext>,
    #[serde(default)]
    pub dominant_sign_rulers: Vec<BasicRulerContext>,
    #[serde(default)]
    pub dispositor_links: Vec<BasicDispositorLink>,
    #[serde(default)]
    pub rulership_chains: Vec<BasicRulershipChain>,
    #[serde(default)]
    pub final_dispositors: Vec<BasicFinalDispositor>,
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
    pub disposition_type: String,
    #[serde(default)]
    pub source_objects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicLlmHandoffContract {
    pub contract_version: String,
    pub payload_language_code: String,
    pub target_language_policy: String,
    pub audience_level: String,
    pub tone: String,
    #[serde(default)]
    pub must_use: Vec<String>,
    #[serde(default)]
    pub must_not: Vec<String>,
    pub output_format: String,
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
    pub writing_guidance: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicDraftingPlanItem {
    pub slot: String,
    pub section_title: String,
    #[serde(default)]
    pub source_signal_keys: Vec<String>,
    #[serde(default)]
    pub primary_signal_keys: Vec<String>,
    #[serde(default)]
    pub secondary_slot_candidates: Vec<BasicSecondarySlotCandidate>,
    #[serde(default)]
    pub emphasis_refs: BasicEmphasisRefs,
    #[serde(default)]
    pub context_refs: BasicContextRefs,
    pub writing_objective: String,
    pub max_words: u16,
    #[serde(default)]
    pub avoid: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BasicEmphasisRefs {
    #[serde(default)]
    pub dominant_signs: Vec<String>,
    #[serde(default)]
    pub dominant_houses: Vec<i32>,
    #[serde(default)]
    pub dominant_objects: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BasicContextRefs {
    #[serde(default)]
    pub chart_context: Vec<String>,
    #[serde(default)]
    pub rulership_context: Vec<String>,
}
