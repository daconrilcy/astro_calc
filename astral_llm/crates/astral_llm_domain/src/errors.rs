use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GenerationErrorCode {
    InvalidInput,
    UnsupportedProvider,
    UnsupportedCapability,
    SafetyRejected,
    ProviderTimeout,
    ProviderRateLimited,
    ProviderUnavailable,
    InvalidJsonOutput,
    SchemaValidationFailed,
    PostSafetyValidationFailed,
    FallbackFailed,
    PersistenceFailed,
    ProductPolicyViolation,
    PolicyViolation,
    ReadingQualityFailed,
    PremiumEvidenceDiversityFailed,
    AstroBasisInvalid,
}

impl GenerationErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_INPUT",
            Self::UnsupportedProvider => "UNSUPPORTED_PROVIDER",
            Self::UnsupportedCapability => "UNSUPPORTED_CAPABILITY",
            Self::SafetyRejected => "SAFETY_REJECTED",
            Self::ProviderTimeout => "PROVIDER_TIMEOUT",
            Self::ProviderRateLimited => "PROVIDER_RATE_LIMITED",
            Self::ProviderUnavailable => "PROVIDER_UNAVAILABLE",
            Self::InvalidJsonOutput => "INVALID_JSON_OUTPUT",
            Self::SchemaValidationFailed => "SCHEMA_VALIDATION_FAILED",
            Self::PostSafetyValidationFailed => "POST_SAFETY_VALIDATION_FAILED",
            Self::FallbackFailed => "FALLBACK_FAILED",
            Self::PersistenceFailed => "PERSISTENCE_FAILED",
            Self::ProductPolicyViolation => "PRODUCT_POLICY_VIOLATION",
            Self::PolicyViolation => "POLICY_VIOLATION",
            Self::ReadingQualityFailed => "READING_QUALITY_FAILED",
            Self::PremiumEvidenceDiversityFailed => "PREMIUM_EVIDENCE_DIVERSITY_FAILED",
            Self::AstroBasisInvalid => "ASTRO_BASIS_INVALID",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GenerationErrorDetail {
    pub code: GenerationErrorCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Error)]
pub enum GenerationError {
    #[error("{message}")]
    Detailed {
        detail: GenerationErrorDetail,
        message: String,
    },
}

impl GenerationError {
    pub fn new(code: GenerationErrorCode, message: impl Into<String>) -> Self {
        let message = message.into();
        Self::Detailed {
            detail: GenerationErrorDetail {
                code,
                message: message.clone(),
                details: None,
            },
            message,
        }
    }

    pub fn with_details(
        code: GenerationErrorCode,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        let message = message.into();
        Self::Detailed {
            detail: GenerationErrorDetail {
                code,
                message: message.clone(),
                details: Some(details),
            },
            message,
        }
    }

    pub fn detail(&self) -> &GenerationErrorDetail {
        match self {
            Self::Detailed { detail, .. } => detail,
        }
    }
}
