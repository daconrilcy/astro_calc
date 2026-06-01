use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    pub language_id: Option<i32>,
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
    pub orb_deg: f64,
    pub phase_state: String,
    pub is_applying: bool,
    pub is_exact: bool,
    pub strength_score: Option<f64>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicPayload {
    pub product_code: String,
    pub chart_calculation_id: i32,
    pub reference_version_id: i32,
    pub subject_label: Option<String>,
    pub birth_datetime_utc: DateTime<Utc>,
    pub positions: Vec<BasicObjectPosition>,
    pub signals: Vec<BasicSignal>,
    #[serde(default)]
    pub reading_plan: Vec<BasicReadingPlanItem>,
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
    pub evidence: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicReadingPlanItem {
    pub slot: String,
    pub title: String,
    #[serde(default)]
    pub source_signal_keys: Vec<String>,
}
