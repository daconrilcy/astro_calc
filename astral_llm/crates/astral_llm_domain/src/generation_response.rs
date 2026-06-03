use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::errors::GenerationErrorDetail;
use crate::output_contract::GenerationMode;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum GenerateReadingResponse {
    Success(StructuredReadingResponse),
    SafetyRejected(SafetyRejectedResponse),
    Failed(GenerationFailedResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StructuredReadingResponse {
    pub run_id: String,
    pub reading: NatalReadingResponse,
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
pub struct SafetyRejectedResponse {
    pub run_id: String,
    pub violations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GenerationFailedResponse {
    pub run_id: String,
    pub error: GenerationErrorDetail,
}
