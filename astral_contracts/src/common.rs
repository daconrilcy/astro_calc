use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProductTier {
    Free,
    Basic,
    Premium,
}

impl ProductTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Basic => "basic",
            Self::Premium => "premium",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct RequestContextCommon {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    pub target_language_code: String,
    pub audience_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LocationCommon {
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BirthInputCommon {
    pub date: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<LocationCommon>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ChartReferenceCommon {
    pub chart_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chart_calculation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PeriodContextCommon {
    pub timezone: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResponseMetadataCommon {
    pub product_code: String,
    pub tier: ProductTier,
    pub variant: String,
    pub contract_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct QualityMetadataCommon {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calculator_contract_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_contract_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reading_completeness: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ErrorResponseCommon {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}
