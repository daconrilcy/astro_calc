use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    astrologer_profile::AstrologerProfile, engine_params::EngineParams,
    output_contract::ResponseContract, safety_policy::SafetyPolicyOverride,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GenerateReadingRequest {
    pub request_id: Option<String>,
    pub product_context: ProductContext,
    pub astro_result: AstroCalculationPayload,
    pub astrologer_profile: AstrologerProfile,
    pub engine: EngineParams,
    pub response_contract: ResponseContract,
    pub safety_policy: Option<SafetyPolicyOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProductContext {
    pub product_code: String,
    pub user_language: String,
    pub audience_level: AudienceLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AudienceLevel {
    Beginner,
    Intermediate,
    Expert,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AstroCalculationPayload {
    pub contract_version: String,
    pub chart_type: String,
    pub data: serde_json::Value,
}
