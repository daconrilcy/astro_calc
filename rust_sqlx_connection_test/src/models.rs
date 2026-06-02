use chrono::{DateTime, Utc};
use serde_json::Value;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChartObject {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub swe_id: Option<i32>,
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
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct HouseReference {
    pub id: i32,
    pub number: i32,
    pub name: String,
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
    pub(crate) orb_deg: f64,
    pub(crate) phase_state: String,
    pub(crate) is_applying: bool,
    pub(crate) is_exact: bool,
    pub(crate) strength_score: Option<f64>,
    pub(crate) calculation_notes_json: Option<Value>,
}
