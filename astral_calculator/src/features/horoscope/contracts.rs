//! Module astral_calculator\src\features\horoscope\contracts.rs du moteur astral_calculator.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Structure HoroscopeCalculationRequest.
pub struct HoroscopeCalculationRequest {
    pub contract_version: String,
    pub service_code: String,
    pub period: HoroscopePeriod,
    pub chart_calculation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<HoroscopeLocation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot_profile_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub house_system_code: Option<String>,
    #[serde(default)]
    pub calculation_features: Vec<String>,
    pub slots: Vec<HoroscopeCalculationSlotRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Structure HoroscopeLocation.
pub struct HoroscopeLocation {
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Structure HoroscopePeriod.
pub struct HoroscopePeriod {
    pub date: String,
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Structure HoroscopeCalculationSlotRequest.
pub struct HoroscopeCalculationSlotRequest {
    pub slot_code: String,
    pub start_local_time: String,
    pub end_local_time: String,
    pub reference_local_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Structure HoroscopeCalculationResponse.
pub struct HoroscopeCalculationResponse {
    pub contract_version: String,
    pub service_code: String,
    pub period: HoroscopePeriod,
    pub slots: Vec<HoroscopeCalculationSlot>,
    pub calculation_warnings: Vec<String>,
    pub evidence_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Structure HoroscopeCalculationSlot.
pub struct HoroscopeCalculationSlot {
    pub slot_code: String,
    pub reference_local_time: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_datetime_utc: Option<String>,
    pub sky_snapshot: serde_json::Value,
    pub moon_context: serde_json::Value,
    pub transits_to_natal: Vec<HoroscopeTransitFact>,
    pub current_sky_aspects: Vec<serde_json::Value>,
    pub natal_house_activations: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_chart: Option<serde_json::Value>,
    #[serde(default)]
    pub local_house_placements: Vec<serde_json::Value>,
    #[serde(default)]
    pub angle_activations: Vec<serde_json::Value>,
    #[serde(default)]
    pub calculation_warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Structure HoroscopeTransitFact.
pub struct HoroscopeTransitFact {
    pub evidence_key: String,
    pub fact_type: String,
    pub source: String,
    pub transiting_object: String,
    pub natal_target: Option<String>,
    pub aspect: Option<String>,
    pub orb_deg: Option<f64>,
    pub natal_house: Option<i32>,
}

#[derive(Debug, Clone)]
/// Mapping DB d'un signal horoscope vers un theme.
pub struct HoroscopeSignalThemeMapping {
    pub match_object: String,
    pub match_aspect: Option<String>,
    pub match_natal_target: Option<String>,
    pub theme_code: String,
}

#[derive(Debug, Clone)]
/// Objet transitant supporte par le produit horoscope.
pub struct HoroscopeSupportedObject {
    pub object_code: String,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Structure HoroscopePeriodCalculationRequest.
pub struct HoroscopePeriodCalculationRequest {
    pub contract_version: String,
    pub service_code: String,
    pub chart_calculation_id: String,
    pub period_resolution: serde_json::Value,
    pub scan_plan: HoroscopeScanPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Structure HoroscopeScanPlan.
pub struct HoroscopeScanPlan {
    pub scan_profile_code: String,
    pub granularity: String,
    pub snapshot_count: i32,
    pub snapshots: Vec<HoroscopeSnapshotRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Structure HoroscopeSnapshotRequest.
pub struct HoroscopeSnapshotRequest {
    pub snapshot_key: String,
    pub date: String,
    pub reference_time_local: String,
    pub reference_datetime_local: String,
    pub reference_datetime_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Structure HoroscopePeriodCalculationResponse.
pub struct HoroscopePeriodCalculationResponse {
    pub contract_version: String,
    pub service_code: String,
    pub period_resolution: serde_json::Value,
    pub scan_plan: HoroscopeScanPlan,
    pub snapshots: Vec<HoroscopePeriodSnapshot>,
    pub calculation_warnings: Vec<String>,
    pub evidence_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Structure HoroscopePeriodSnapshot.
pub struct HoroscopePeriodSnapshot {
    pub snapshot_key: String,
    pub date: String,
    pub reference_datetime_utc: String,
    pub sky_snapshot: serde_json::Value,
    pub moon_context: serde_json::Value,
    pub transits_to_natal: Vec<HoroscopeTransitFact>,
    pub current_sky_aspects: Vec<serde_json::Value>,
    pub natal_house_activations: Vec<serde_json::Value>,
    #[serde(default)]
    pub calculation_warnings: Vec<String>,
}
