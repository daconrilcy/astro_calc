//! Module astral_calculator\src\engine\request.rs du moteur astral_calculator.

use serde::{Deserialize, Serialize};

pub const REQUEST_CONTRACT_VERSION: &str = "astro_engine_request_v1";
pub const RESPONSE_CONTRACT_VERSION: &str = "astro_engine_response_v1";
pub const LLM_PROJECTION_CONTRACT_VERSION: &str = "llm_projection_natal_v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Structure AstroEngineRequest.
pub struct AstroEngineRequest {
    pub request_contract_version: String,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    pub calculation: EngineCalculationRequest,
    pub birth: EngineBirthRequest,
    pub projection: EngineProjectionRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Structure EngineCalculationRequest.
pub struct EngineCalculationRequest {
    #[serde(rename = "type")]
    pub calculation_type: String,
    #[serde(default = "default_tropical")]
    pub zodiacal_reference_system: String,
    #[serde(default = "default_geocentric")]
    pub coordinate_reference_system: String,
    #[serde(default = "default_placidus")]
    pub house_system: String,
}

/// Fonction default_tropical.
fn default_tropical() -> String {
    "tropical".to_string()
}

/// Fonction default_geocentric.
fn default_geocentric() -> String {
    "geocentric".to_string()
}

/// Fonction default_placidus.
fn default_placidus() -> String {
    "placidus".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Structure EngineBirthRequest.
pub struct EngineBirthRequest {
    pub date: String,
    pub time: String,
    pub timezone: String,
    pub location: EngineBirthLocation,
    #[serde(default)]
    pub time_precision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Structure EngineBirthLocation.
pub struct EngineBirthLocation {
    #[serde(default)]
    pub label: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default)]
    pub country_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Structure EngineProjectionRequest.
pub struct EngineProjectionRequest {
    #[serde(default)]
    pub contract_version: Option<String>,
    pub level: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
}
