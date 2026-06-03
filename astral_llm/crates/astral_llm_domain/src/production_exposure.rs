use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProductionExposureMode {
    /// Reseau restreint (bind local, persistence optionnelle).
    Internal,
    /// Service expose (API key clients externes) : audit et idempotence obligatoires.
    Public,
}

impl ProductionExposureMode {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_lowercase().as_str() {
            "public" => Self::Public,
            _ => Self::Internal,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Internal => "internal",
            Self::Public => "public",
        }
    }

    pub fn is_public(&self) -> bool {
        matches!(self, Self::Public)
    }
}
