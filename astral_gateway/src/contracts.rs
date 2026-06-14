use astral_contracts::{
    BirthInputCommon, QualityMetadataCommon, RequestContextCommon, ResponseMetadataCommon,
};
use astral_llm_domain::GenerateReadingResponse;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NatalReadingRequestV2 {
    pub context: RequestContextCommon,
    pub birth: BirthInputCommon,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NatalReadingResponseV2 {
    pub metadata: ResponseMetadataCommon,
    pub quality: QualityMetadataCommon,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calculation: Option<Value>,
    pub reading: GenerateReadingResponse,
}
