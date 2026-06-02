use chrono::{DateTime, Utc};
use serde_json::Value;

#[derive(Debug, Clone, sqlx::FromRow)]
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
pub struct AspectDefinition {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub angle: f64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct HouseSystem {
    pub id: i32,
    pub code: String,
    pub calculation_engine_code: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
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
pub struct MotionStateReference {
    pub id: i32,
    pub code: String,
    pub label: String,
    pub motion_family: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
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
    pub(crate) valence_writing_guidance: Option<String>,
    pub(crate) calculation_notes_json: Option<Value>,
}
