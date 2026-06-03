use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChapterGenerationStatus {
    Pending,
    Generated,
    SchemaInvalid,
    SafetyRejected,
    AstroBasisInvalid,
    Repaired,
    Failed,
}

impl ChapterGenerationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Generated => "generated",
            Self::SchemaInvalid => "schema_invalid",
            Self::SafetyRejected => "safety_rejected",
            Self::AstroBasisInvalid => "astro_basis_invalid",
            Self::Repaired => "repaired",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadingPlanChapter {
    pub code: String,
    pub title: String,
    pub min_words: u32,
    pub target_words: u32,
    pub max_words: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadingPlan {
    pub product_code: String,
    pub domain_count: u8,
    pub selected_domains: Vec<String>,
    pub chapters: Vec<ReadingPlanChapter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationStepRecord {
    pub step_type: String,
    pub chapter_code: Option<String>,
    pub provider: String,
    pub model: String,
    pub status: ChapterGenerationStatus,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub latency_ms: Option<u32>,
    pub error_code: Option<String>,
}
