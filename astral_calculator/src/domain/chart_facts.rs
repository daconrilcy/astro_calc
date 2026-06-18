//! Module astral_calculator\src\domain\chart_facts.rs du moteur astral_calculator.

#[derive(Debug, Clone)]
/// Structure ObjectPositionFact.
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
/// Structure HouseCuspFact.
pub struct HouseCuspFact {
    pub house_id: i32,
    pub house_number: i32,
    pub sign_id: i32,
    pub longitude_deg: f64,
}

#[derive(Debug, Clone)]
/// Structure AspectFact.
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
    pub calculation_notes_json: Option<Value>,
}

#[derive(Debug, Clone)]
/// Structure CalculatedChartFacts.
pub struct CalculatedChartFacts {
    pub positions: Vec<ObjectPositionFact>,
    pub house_cusps: Vec<HouseCuspFact>,
    pub aspects: Vec<AspectFact>,
}

#[derive(Debug, Clone)]
/// Structure InterpretationSignalDraft.
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

use serde_json::Value;
