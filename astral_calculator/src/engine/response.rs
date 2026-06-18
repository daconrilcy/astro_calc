//! Module astral_calculator\src\engine\response.rs du moteur astral_calculator.

use serde::{Deserialize, Serialize};

use crate::domain::BasicPayload;
use crate::engine::projection::LlmProjectionNatalV1;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Structure AstroEngineResponse.
pub struct AstroEngineResponse {
    pub response_contract_version: String,
    pub request_echo: EngineRequestEcho,
    pub calculation_result: EngineCalculationResult,
    pub audit_payload: EngineAuditPayload,
    pub llm_payload: LlmProjectionNatalV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Structure EngineRequestEcho.
pub struct EngineRequestEcho {
    pub calculation_type: String,
    pub birth_datetime_local: String,
    pub birth_timezone: String,
    pub birth_datetime_utc: String,
    pub location: EngineEchoLocation,
    pub projection_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Structure EngineEchoLocation.
pub struct EngineEchoLocation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Structure EngineCalculationResult.
pub struct EngineCalculationResult {
    pub status: String,
    pub chart_calculation_id: i32,
    pub engine_version: String,
    pub ephemeris_version: String,
    pub raw_payload_contract_version: String,
    pub llm_projection_contract_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Structure EngineAuditPayload.
pub struct EngineAuditPayload {
    pub contract_version: String,
    pub payload: BasicPayload,
}
