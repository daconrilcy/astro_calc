use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResponseContract {
    pub output_schema_version: String,
    pub generation_mode: GenerationMode,
    #[serde(default)]
    pub format: OutputFormat,
    #[serde(default)]
    pub chapters: Vec<ChapterContract>,
    pub global_max_tokens: Option<u32>,
    pub include_astro_sources: bool,
    pub include_legal_disclaimer: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum GenerationMode {
    #[default]
    SinglePass,
    ChapterOrchestrated,
}

impl GenerationMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SinglePass => "single_pass",
            Self::ChapterOrchestrated => "chapter_orchestrated",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    #[default]
    StructuredJson,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ChapterContract {
    pub code: String,
    pub title: String,
    pub min_words: Option<u32>,
    pub max_words: Option<u32>,
    pub target_tokens: Option<u32>,
    #[serde(default)]
    pub required_fields: Vec<String>,
}
