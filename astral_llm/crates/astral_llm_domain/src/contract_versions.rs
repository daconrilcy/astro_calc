use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GenerationRunContractVersions {
    pub astro_contract_version: String,
    pub output_schema_version: String,
    pub prompt_family: String,
    pub prompt_version: String,
    pub safety_policy_version: String,
    pub provider_capability_version: String,
}

impl GenerationRunContractVersions {
    pub const SAFETY_POLICY_VERSION: &'static str = "astrology_safety_v1";
    pub const PROVIDER_CAPABILITY_VERSION: &'static str = "provider_models_2026_06";

    pub fn new(
        astro_contract_version: impl Into<String>,
        output_schema_version: impl Into<String>,
        prompt_family: impl Into<String>,
        prompt_version: impl Into<String>,
    ) -> Self {
        Self {
            astro_contract_version: astro_contract_version.into(),
            output_schema_version: output_schema_version.into(),
            prompt_family: prompt_family.into(),
            prompt_version: prompt_version.into(),
            safety_policy_version: Self::SAFETY_POLICY_VERSION.into(),
            provider_capability_version: Self::PROVIDER_CAPABILITY_VERSION.into(),
        }
    }
}
