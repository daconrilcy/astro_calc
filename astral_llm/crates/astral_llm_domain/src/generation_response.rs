use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::errors::GenerationErrorDetail;
use crate::output_contract::GenerationMode;
use crate::PublicTokenUsage;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum GenerateReadingResponse {
    Success {
        run_id: String,
        reading: NatalReadingResponse,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        token_usage: Option<PublicTokenUsage>,
    },
    SafetyRejected {
        run_id: String,
        error: SafetyErrorBody,
        violations: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        token_usage: Option<PublicTokenUsage>,
    },
    Failed {
        run_id: String,
        error: GenerationErrorDetail,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        token_usage: Option<PublicTokenUsage>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StructuredReadingResponse {
    pub run_id: String,
    pub reading: NatalReadingResponse,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_usage: Option<PublicTokenUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NatalReadingResponse {
    pub schema_version: String,
    pub language: String,
    pub reading_type: String,
    pub summary: ReadingSummary,
    pub chapters: Vec<ReadingChapter>,
    pub legal: LegalBlock,
    pub quality: QualityMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadingSummary {
    pub title: String,
    pub short_text: String,
}

/// Schema minimal envoye au LLM pour la synthese finale (mode chapitre).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SummaryProviderResponse {
    pub title: String,
    pub short_text: String,
}

/// Schema minimal envoye au LLM en mode chapitre (sans champs serveur `quality`).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ChapterProviderResponse {
    pub code: String,
    pub title: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub astro_basis: Vec<AstroBasisItem>,
    pub confidence: ConfidenceLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadingChapter {
    pub code: String,
    pub title: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub astro_basis: Vec<AstroBasisItem>,
    pub confidence: ConfidenceLevel,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub safety_flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AstroBasisItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fact_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub factor: String,
    pub interpretive_role: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LegalBlock {
    pub disclaimer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QualityMetadata {
    pub used_provider: String,
    pub used_model: String,
    pub generation_mode: GenerationMode,
    pub prompt_family: String,
    pub prompt_version: String,
    pub astro_contract_version: String,
    pub fallback_used: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SafetyErrorBody {
    pub code: String,
    pub category: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SafetyRejectedResponse {
    pub run_id: String,
    pub status: String,
    pub error: SafetyErrorBody,
    pub violations: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_usage: Option<PublicTokenUsage>,
}

impl SafetyRejectedResponse {
    pub fn new(
        run_id: impl Into<String>,
        category: impl Into<String>,
        message: impl Into<String>,
        rule_id: Option<String>,
        violations: Vec<String>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            status: "safety_rejected".into(),
            error: SafetyErrorBody {
                code: "SAFETY_POLICY_VIOLATION".into(),
                category: category.into(),
                message: message.into(),
                rule_id,
            },
            violations,
            token_usage: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GenerationFailedResponse {
    pub run_id: String,
    pub error: GenerationErrorDetail,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_usage: Option<PublicTokenUsage>,
}

impl GenerateReadingResponse {
    pub fn token_usage(&self) -> Option<&PublicTokenUsage> {
        match self {
            Self::Success { token_usage, .. }
            | Self::SafetyRejected { token_usage, .. }
            | Self::Failed { token_usage, .. } => token_usage.as_ref(),
        }
    }

    pub fn run_id(&self) -> &str {
        match self {
            Self::Success { run_id, .. }
            | Self::SafetyRejected { run_id, .. }
            | Self::Failed { run_id, .. } => run_id,
        }
    }
}
