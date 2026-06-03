use astral_llm_domain::{
    provider::{ReasoningEffort, SafetyMode},
    ProviderKind,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptRole {
    System,
    Developer,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMessage {
    pub role: PromptRole,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct GenerationMetadata {
    pub run_id: String,
    pub request_id: Option<String>,
    pub product_code: String,
    pub chapter_code: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProviderGenerationRequest {
    pub model: String,
    pub messages: Vec<PromptMessage>,
    pub structured_schema: Option<serde_json::Value>,
    pub reasoning_effort: Option<ReasoningEffort>,
    pub temperature: Option<f32>,
    pub max_output_tokens: Option<u32>,
    pub safety_mode: SafetyMode,
    pub timeout: Duration,
    pub metadata: GenerationMetadata,
}

#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct ProviderGenerationResponse {
    pub raw_text: String,
    pub parsed_json: Option<serde_json::Value>,
    pub usage: Option<TokenUsage>,
    pub provider_metadata: serde_json::Value,
    pub model_used: String,
    pub provider_kind: ProviderKind,
}

#[derive(Debug, thiserror::Error)]
pub enum LlmProviderError {
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("Timeout")]
    Timeout,
    #[error("Rate limited")]
    RateLimited,
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Configuration: {0}")]
    Config(String),
}

impl LlmProviderError {
    pub fn is_transient(&self) -> bool {
        match self {
            Self::Timeout | Self::RateLimited => true,
            Self::Http(msg) => is_transient_status(msg),
            Self::Api(msg) => is_transient_status(msg),
            _ => false,
        }
    }
}

fn is_transient_status(msg: &str) -> bool {
    ["429 ", "502 ", "503 ", "504 ", "429:", "502:", "503:", "504:"]
        .iter()
        .any(|code| msg.starts_with(code))
}
