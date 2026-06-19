//! Module astral_calculator\src\infra\db\models.rs du moteur astral_calculator.

use chrono::{DateTime, Utc};
use serde_json::Value;

#[derive(Debug, Clone, sqlx::FromRow)]
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

#[derive(Debug, Clone, sqlx::FromRow)]
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

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure MajorAspectFamilyReference.
pub struct MajorAspectFamilyReference {
    pub expected_aspect_count: i32,
    pub max_default_orb_deg: f64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure HouseSystem.
pub struct HouseSystem {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub calculation_engine_code: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure ZodiacalReferenceSystemRow.
pub struct ZodiacalReferenceSystemRow {
    pub id: i32,
    pub key: String,
    pub display_name: String,
    pub category_id: i32,
    pub description: String,
    pub requires_ayanamsha: bool,
    pub usage_note: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure CoordinateReferenceSystemRow.
pub struct CoordinateReferenceSystemRow {
    pub id: i32,
    pub key: String,
    pub display_name: String,
    pub category_id: i32,
    pub description: String,
    pub usage_note: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure HoroscopeServiceRow.
pub struct HoroscopeServiceRow {
    pub service_code: String,
    pub product_level_code: String,
    pub shortlist_profile_code: String,
    pub time_slot_profile_code: Option<String>,
    pub slot_mode: String,
    pub requires_natal_chart: bool,
    pub requires_location: bool,
    pub requires_timezone: bool,
    pub requires_inline_birth_data: bool,
    pub house_system_code: Option<String>,
    pub period_profile_code: Option<String>,
    pub detail_profile_code: Option<String>,
    pub scan_profile_code: Option<String>,
    pub detail_level: Option<String>,
    pub generation_mode: String,
    pub max_words_target: Option<i32>,
    pub max_words_hard_limit: Option<i32>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure HoroscopeTimeSlotProfileRow.
pub struct HoroscopeTimeSlotProfileRow {
    pub service_code: String,
    pub slot_code: String,
    pub start_local_time: String,
    pub end_local_time: String,
    pub reference_local_time: String,
    pub slot_label: String,
    pub is_public: bool,
    pub sort_order: i32,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
/// Structure AstralTimePeriodProfileRow.
pub struct AstralTimePeriodProfileRow {
    pub period_profile_code: String,
    pub resolution_strategy: String,
    pub duration_days: Option<i32>,
    pub week_offset: Option<i32>,
    pub included_days: Option<Value>,
    pub is_enabled: bool,
    pub sort_order: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure HoroscopeScanProfileRow.
pub struct HoroscopeScanProfileRow {
    pub scan_profile_code: String,
    pub granularity: String,
    pub reference_time_local: String,
    pub expected_snapshots_per_day: i32,
    pub is_enabled: bool,
    pub sort_order: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure HoroscopeOrbWeightBandRow.
pub struct HoroscopeOrbWeightBandRow {
    pub band_code: String,
    pub min_orb_deg: f64,
    pub max_orb_deg: f64,
    pub weight: f64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure HoroscopeSignalThemeMappingRow.
pub struct HoroscopeSignalThemeMappingRow {
    pub match_object: String,
    pub match_aspect: Option<String>,
    pub match_natal_target: Option<String>,
    pub theme_code: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure HoroscopeSupportedObjectRow.
pub struct HoroscopeSupportedObjectRow {
    pub object_code: String,
    pub weight: f64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure SimplifiedPolicyRow.
pub struct SimplifiedPolicyRow {
    pub code: String,
    pub reference_time_utc: String,
    pub date_only_uncertainty_mode: String,
    pub uncertainty_sampling_minutes: i32,
    pub default_timezone_strategy: String,
    pub cusp_warning_orb_deg: f64,
    pub stable_fact_strategy: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure LimitationCodeRow.
pub struct LimitationCodeRow {
    pub code: String,
    pub severity: String,
    pub affected_features_json: Value,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure ReliabilityLevelRow.
pub struct ReliabilityLevelRow {
    pub code: String,
    pub allows_interpretive_affirmation: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure CalculationScopeRow.
pub struct CalculationScopeRow {
    pub code: String,
    pub min_input_precision_code: String,
    pub supports_angles: bool,
    pub supports_houses: bool,
    pub supports_aspects: bool,
    pub supports_object_sign_facts: bool,
    pub supports_ambiguous_facts: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure InputPrecisionLevelRow.
pub struct InputPrecisionLevelRow {
    pub code: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure ProfileFeatureExclusionRow.
pub struct ProfileFeatureExclusionRow {
    pub profile_code: String,
    pub computed_scope_code: Option<String>,
    pub feature_code: String,
    pub exclusion_kind: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
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

#[derive(Debug, Clone, sqlx::FromRow)]
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

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure MotionStateReference.
pub struct MotionStateReference {
    pub id: i32,
    pub code: String,
    pub label: String,
    pub motion_family: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure HorizonPositionReference.
pub struct HorizonPositionReference {
    pub id: i32,
    pub code: String,
    pub label: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
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

#[derive(Debug, Clone, sqlx::FromRow)]
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

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure HouseAxisReferenceRow.
pub struct HouseAxisReferenceRow {
    pub axis_code: String,
    pub house_a_number: i32,
    pub house_b_number: i32,
    pub theme_a_code: String,
    pub theme_b_code: String,
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure LunarPhaseReferenceRow.
pub struct LunarPhaseReferenceRow {
    pub phase_code: String,
    pub label: String,
    pub cycle_family: String,
    pub range_start_deg: f64,
    pub range_end_deg: f64,
    pub exact_anchor_deg: f64,
    pub is_major_lunar_phase: bool,
    pub description: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure AccidentalDignityConditionReferenceRow.
pub struct AccidentalDignityConditionReferenceRow {
    pub condition_code: String,
    pub condition_family: String,
    pub label: String,
    pub polarity: String,
    pub strength_score: f64,
    pub score_delta: f64,
    pub description: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure ObjectSectAffinityReferenceRow.
pub struct ObjectSectAffinityReferenceRow {
    pub object_code: String,
    pub sect_affinity_code: String,
    pub is_variable: bool,
    pub description: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure EssentialDignityRuleReferenceRow.
pub struct EssentialDignityRuleReferenceRow {
    pub object_code: String,
    pub sign_code: String,
    pub dignity_type: String,
    pub dignity_label: String,
    pub polarity: String,
    pub strength_score: f64,
    pub priority_delta: Option<f64>,
    pub signal_weight_delta: Option<f64>,
    pub signal_worthy_min_strength: Option<f64>,
    pub emphasis_weight: Option<f64>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure AccidentalConditionTriggerRow.
pub struct AccidentalConditionTriggerRow {
    pub trigger_family: String,
    pub source_code: Option<String>,
    pub angle_object_code: Option<String>,
    pub condition_code: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure ProjectionReasonDefinitionRow.
pub struct ProjectionReasonDefinitionRow {
    pub reason_code: String,
    pub reason_family: String,
    pub label_template_en: String,
    pub requires_object: bool,
    pub requires_dignity_type: bool,
    pub requires_sign_code: bool,
    pub requires_house_number: bool,
    pub requires_theme_code: bool,
    pub requires_angle_code: bool,
    pub requires_signal_key: bool,
    pub requires_context_key: bool,
    pub is_active: bool,
    pub sort_order: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure ProjectionLabelDefinitionRow.
pub struct ProjectionLabelDefinitionRow {
    pub label_family: String,
    pub label_code: String,
    pub label_template_en: String,
    pub is_active: bool,
    pub sort_order: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure AccidentalScoringParamsRow.
pub struct AccidentalScoringParamsRow {
    pub code: String,
    pub overall_score_baseline: f64,
    pub overall_score_min: f64,
    pub overall_score_max: f64,
    pub angle_proximity_max_orb_deg: f64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure AccidentalPolarityBandRow.
pub struct AccidentalPolarityBandRow {
    pub polarity_code: String,
    pub expression_quality_code: String,
    pub min_score: f64,
    pub max_score: f64,
    pub sort_order: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure BasicProductScoringProfileRow.
pub struct BasicProductScoringProfileRow {
    pub product_code: String,
    pub payload_contract_version: String,
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
    pub max_dominant_signs: i32,
    pub max_dominant_houses: i32,
    pub max_dominant_objects: i32,
    pub max_active_signals: i32,
    pub aspect_min_strength: f64,
    pub max_house_axis_emphasis: i32,
    pub accidental_scoring_params_id: i32,
    pub essential_dignity_score_profile_id: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
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

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure ChartCalculationRow.
pub struct ChartCalculationRow {
    pub id: i32,
    pub status: String,
    pub execution_attempt: i32,
    pub heartbeat_at: Option<DateTime<Utc>>,
    pub stale_after_seconds: Option<i32>,
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct PersistedObjectPositionFact {
    pub(crate) chart_object_id: i32,
    pub(crate) object_code: String,
    pub(crate) object_name: String,
    pub(crate) zodiacal_reference_system_id: i32,
    pub(crate) coordinate_reference_system_id: i32,
    pub(crate) sign_id: i32,
    pub(crate) sign_code: String,
    pub(crate) sign_name: String,
    pub(crate) house_id: Option<i32>,
    pub(crate) house_number: Option<i32>,
    pub(crate) house_name: Option<String>,
    pub(crate) motion_state_id: Option<i32>,
    pub(crate) horizon_position_id: Option<i32>,
    pub(crate) longitude_deg: f64,
    pub(crate) latitude_deg: Option<f64>,
    pub(crate) apparent_speed_deg_per_day: Option<f64>,
    pub(crate) altitude_deg: Option<f64>,
    pub(crate) is_visible: Option<bool>,
    pub(crate) facts_json: Option<Value>,
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct PersistedAspectFact {
    pub(crate) source_chart_object_id: i32,
    pub(crate) source_object_code: String,
    pub(crate) source_object_name: String,
    pub(crate) target_chart_object_id: i32,
    pub(crate) target_object_code: String,
    pub(crate) target_object_name: String,
    pub(crate) aspect_id: i32,
    pub(crate) aspect_code: String,
    pub(crate) aspect_name: String,
    pub(crate) aspect_family: String,
    pub(crate) orb_deg: f64,
    pub(crate) phase_state: String,
    pub(crate) is_applying: bool,
    pub(crate) is_exact: bool,
    pub(crate) strength_score: Option<f64>,
    pub(crate) primary_valence: Option<String>,
    pub(crate) intensity_modifier: Option<String>,
    pub(crate) secondary_effect: Option<String>,
    pub(crate) valence_family: Option<String>,
    pub(crate) valence_is_tonal: Option<bool>,
    pub(crate) valence_is_intensity_modifier: Option<bool>,
    pub(crate) calculation_notes_json: Option<Value>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
/// Structure LlmProjectionProfileRow.
pub struct LlmProjectionProfileRow {
    pub id: i32,
    pub contract_version: String,
    pub level_code: String,
    pub max_keywords_per_item: i32,
    pub max_core_placements: i32,
    pub max_supporting_placements: i32,
    pub max_dominant_signs: i32,
    pub max_dominant_houses: i32,
    pub max_dominant_objects: i32,
    pub max_house_axes: i32,
    pub max_aspects: i32,
    pub include_accidental_conditions: bool,
    pub include_rulership_details: bool,
    pub include_minor_evidence: bool,
    pub include_degrees: bool,
    pub include_scores: bool,
}
