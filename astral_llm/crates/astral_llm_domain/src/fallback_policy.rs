use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::model_capability::ProviderModelRef;
use crate::provider::ProviderKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FallbackReason {
    Timeout,
    RateLimited,
    ProviderUnavailable,
    InvalidJsonAfterRepair,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NonFallbackReason {
    SafetyRejected,
    InvalidInput,
    UnsupportedCapability,
    PolicyViolation,
    PostSafetyValidationFailed,
}

impl NonFallbackReason {
    pub fn from_error_code(code: &crate::GenerationErrorCode) -> Option<Self> {
        use crate::GenerationErrorCode;
        match code {
            GenerationErrorCode::SafetyRejected
            | GenerationErrorCode::PostSafetyValidationFailed => Some(Self::SafetyRejected),
            GenerationErrorCode::InvalidInput => Some(Self::InvalidInput),
            GenerationErrorCode::UnsupportedCapability => Some(Self::UnsupportedCapability),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FallbackPolicy {
    pub enabled: bool,
    pub chain: Vec<ProviderKind>,
    pub fallback_on: Vec<FallbackReason>,
    pub require_same_structured_output_level: bool,
    pub allow_cross_vendor_data_transfer: bool,
    pub max_retries_per_provider: u8,
}

impl Default for FallbackPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            chain: Vec::new(),
            fallback_on: vec![
                FallbackReason::Timeout,
                FallbackReason::RateLimited,
                FallbackReason::ProviderUnavailable,
            ],
            require_same_structured_output_level: true,
            allow_cross_vendor_data_transfer: false,
            max_retries_per_provider: 1,
        }
    }
}

impl FallbackPolicy {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }

    pub fn allows_fallback_for(&self, reason: FallbackReason) -> bool {
        self.enabled && self.fallback_on.contains(&reason)
    }

    pub fn candidate_chain(
        &self,
        requested: &ProviderKind,
        allow_fallback: bool,
    ) -> Vec<ProviderKind> {
        let mut chain = vec![requested.clone()];
        if allow_fallback && self.enabled {
            for kind in &self.chain {
                if *kind != *requested && !chain.contains(kind) {
                    chain.push(kind.clone());
                }
            }
        }
        chain
    }

    pub fn model_chain(&self, requested: &ProviderModelRef) -> Vec<ProviderModelRef> {
        let mut chain = vec![requested.clone()];
        if self.enabled {
            for kind in &self.chain {
                if kind != &requested.provider {
                    chain.push(ProviderModelRef::new(kind.clone(), requested.model.clone()));
                }
            }
        }
        chain
    }
}
