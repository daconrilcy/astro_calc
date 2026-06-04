use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    OpenAi,
    Anthropic,
    Mistral,
    Fake,
    Custom(String),
}

impl ProviderKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
            Self::Mistral => "mistral",
            Self::Fake => "fake",
            Self::Custom(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StructuredOutputMode {
    JsonSchemaStrict,
    JsonObjectOnly,
    ToolSchema,
    PromptOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffort {
    /// Valeur API OpenAI `none` (frontier gpt-5.4+). Ne pas confondre avec l'absence de parametre.
    None,
    /// OpenAI gpt-5-mini : effort `minimal` pour sous-taches.
    Minimal,
    Low,
    Medium,
    High,
}

impl ReasoningEffort {
    pub fn parse_api_value(raw: &str) -> Option<Self> {
        match raw.trim().to_lowercase().as_str() {
            "none" => Some(Self::None),
            "minimal" => Some(Self::Minimal),
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            "xhigh" => Some(Self::High),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProviderCapabilities {
    pub structured_output: StructuredOutputMode,
    pub supports_reasoning_effort: bool,
    pub supports_streaming: bool,
    pub supports_native_safety_prompt: bool,
    pub supports_prompt_cache: bool,
    pub max_input_tokens: Option<u32>,
    pub max_output_tokens: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SafetyMode {
    PlatformRulesOnly,
    PlatformAndNative,
}
