use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::provider::{ProviderKind, ReasoningEffort, StructuredOutputMode};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct ProviderModelRef {
    pub provider: ProviderKind,
    pub model: String,
}

impl ProviderModelRef {
    pub fn new(provider: ProviderKind, model: impl Into<String>) -> Self {
        Self {
            provider,
            model: model.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StructuredOutputAdapterKind {
    OpenAiResponsesTextFormat,
    AnthropicOutputConfigFormat,
    MistralResponseFormatJsonSchema,
    MistralResponseFormatJsonObject,
    PromptOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModelCapability {
    pub provider: ProviderKind,
    pub model: String,
    pub supports_json_schema_strict: bool,
    pub supports_json_object: bool,
    pub supports_reasoning_effort: bool,
    pub supports_streaming: bool,
    pub supports_native_safety_prompt: bool,
    pub max_input_tokens: u32,
    pub max_output_tokens: u32,
    pub structured_output_mode: StructuredOutputMode,
    pub structured_output_adapter: StructuredOutputAdapterKind,
    pub storage_disable_supported: bool,
    pub is_active: bool,
}

impl ModelCapability {
    pub fn ref_key(&self) -> ProviderModelRef {
        ProviderModelRef::new(self.provider.clone(), self.model.clone())
    }

    pub fn satisfies_strict_schema(&self) -> bool {
        self.supports_json_schema_strict
            && self.structured_output_mode == StructuredOutputMode::JsonSchemaStrict
    }

    pub fn allows_reasoning(&self, effort: ReasoningEffort) -> bool {
        if effort == ReasoningEffort::None {
            return true;
        }
        self.supports_reasoning_effort
    }

    pub fn to_provider_capabilities(&self) -> crate::provider::ProviderCapabilities {
        crate::provider::ProviderCapabilities {
            structured_output: self.structured_output_mode,
            supports_reasoning_effort: self.supports_reasoning_effort,
            supports_streaming: self.supports_streaming,
            supports_native_safety_prompt: self.supports_native_safety_prompt,
            supports_prompt_cache: false,
            max_input_tokens: Some(self.max_input_tokens),
            max_output_tokens: Some(self.max_output_tokens),
        }
    }
}
