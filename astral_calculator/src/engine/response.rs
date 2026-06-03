use serde::{Deserialize, Serialize};

use crate::domain::BasicPayload;
use crate::llm_projection::LlmProjectionNatalV1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstroEngineResponse {
    pub response_contract_version: String,
    pub request_echo: EngineRequestEcho,
    pub calculation_result: EngineCalculationResult,
    pub audit_payload: EngineAuditPayload,
    pub llm_payload: LlmProjectionNatalV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngineRequestEcho {
    pub calculation_type: String,
    pub birth_datetime_local: String,
    pub birth_timezone: String,
    pub birth_datetime_utc: String,
    pub location: EngineEchoLocation,
    pub projection_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngineEchoLocation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngineCalculationResult {
    pub status: String,
    pub chart_calculation_id: i32,
    pub engine_version: String,
    pub ephemeris_version: String,
    pub raw_payload_contract_version: String,
    pub llm_projection_contract_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineAuditPayload {
    pub contract_version: String,
    pub payload: BasicPayload,
}
