use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const SIMPLIFIED_RESPONSE_CONTRACT_VERSION: &str = "astro_simplified_natal_response_v1";
pub const SIMPLIFIED_PAYLOAD_CONTRACT: &str = "natal_simplified_structured_v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstroSimplifiedNatalResponse {
    pub response_contract_version: String,
    pub input_precision: InputPrecisionResponse,
    pub computed_scope: String,
    pub limitations: Vec<LimitationResponse>,
    pub facts: Vec<SignFactResponse>,
    pub ambiguous_facts: Vec<AmbiguousSignFactResponse>,
    pub excluded_features: Vec<String>,
    #[serde(default)]
    pub cusp_warnings: Vec<CuspWarningResponse>,
    pub simplified_payload: SimplifiedPayloadEnvelope,
    pub llm_payload: LlmPayloadControls,
    pub reading_hint: ReadingHintResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputPrecisionResponse {
    pub level: String,
    pub date_provided: bool,
    pub time_provided: bool,
    pub timezone_provided: bool,
    pub location_provided: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitationResponse {
    pub code: String,
    pub severity: String,
    pub affects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignFactResponse {
    pub object_code: String,
    pub fact_type: String,
    pub sign_code: String,
    pub reliability: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude_deg: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmbiguousSignFactResponse {
    pub object_code: String,
    pub fact_type: String,
    pub possible_sign_codes: Vec<String>,
    pub reliability: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuspWarningResponse {
    pub object_code: String,
    pub message_code: String,
    pub orb_deg: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimplifiedPayloadEnvelope {
    pub payload_contract: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmPayloadControls {
    pub profile_code: String,
    pub allowed_fact_codes: Vec<String>,
    pub allowed_astro_basis_fact_ids: Vec<String>,
    pub blocked_interpretation_fact_codes: Vec<String>,
    pub excluded_feature_codes: Vec<String>,
    pub profile_excluded_feature_codes: Vec<String>,
    pub allowed_limitation_mentions: Vec<String>,
    /// Agrégat dérivé (prompt / doc) — non consommé directement par SafetyGuard.
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "forbidden_topics",
        rename = "forbidden_interpretation_topics"
    )]
    pub forbidden_interpretation_topics: Option<Vec<String>>,
    /// Miroir déprécié de `forbidden_interpretation_topics` (compat lecture clients legacy).
    #[serde(
        skip_serializing_if = "Option::is_none",
        skip_deserializing,
        rename = "forbidden_topics"
    )]
    pub forbidden_topics: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingHintResponse {
    pub recommended_profile_code: String,
    pub reading_completeness: String,
}

pub const RECOMMENDED_SIMPLIFIED_PROFILE_CODE: &str = "natal_simplified";
pub const READING_COMPLETENESS_V1: &str = "partial";
