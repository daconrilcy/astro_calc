use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PrivacyPolicy {
    pub allow_external_provider: bool,
    pub allow_cross_provider_fallback: bool,
    pub redact_birth_data_before_llm: bool,
    pub disable_provider_storage: bool,
    pub max_payload_retention_days: Option<u32>,
}

impl Default for PrivacyPolicy {
    fn default() -> Self {
        Self {
            allow_external_provider: true,
            allow_cross_provider_fallback: false,
            redact_birth_data_before_llm: true,
            disable_provider_storage: true,
            max_payload_retention_days: Some(90),
        }
    }
}

impl PrivacyPolicy {
    pub fn for_env(is_production: bool) -> Self {
        let mut policy = Self::default();
        if is_production {
            policy.allow_cross_provider_fallback = false;
            policy.disable_provider_storage = true;
        }
        policy
    }
}
