use std::collections::HashMap;

use astral_llm_domain::{
    model_capability::{ModelCapability, StructuredOutputAdapterKind},
    provider::{ProviderKind, StructuredOutputMode},
    GenerationError, GenerationErrorCode, ReasoningEffort,
};

pub struct ModelCapabilityRegistry {
    models: HashMap<String, ModelCapability>,
}

impl ModelCapabilityRegistry {
    pub fn bootstrap() -> Self {
        let mut registry = Self {
            models: HashMap::new(),
        };
        for cap in bootstrap_capabilities() {
            registry.register(cap);
        }
        registry
    }

    pub fn merge_db_models(mut self, db_models: Vec<ModelCapability>) -> Self {
        for cap in db_models {
            self.register(cap);
        }
        self
    }

    pub fn register(&mut self, capability: ModelCapability) {
        let key = Self::key(&capability.provider, &capability.model);
        self.models.insert(key, capability);
    }

    pub fn get(&self, provider: &ProviderKind, model: &str) -> Option<&ModelCapability> {
        self.models.get(&Self::key(provider, model))
    }

    pub fn require(
        &self,
        provider: &ProviderKind,
        model: &str,
    ) -> Result<&ModelCapability, GenerationError> {
        self.get(provider, model).ok_or_else(|| {
            GenerationError::with_details(
                GenerationErrorCode::UnsupportedCapability,
                format!("unknown or inactive model: {provider:?}/{model}"),
                serde_json::json!({ "provider": provider.as_str(), "model": model }),
            )
        })
    }

    pub fn validate_request_capabilities(
        &self,
        provider: &ProviderKind,
        model: &str,
        reasoning_effort: Option<ReasoningEffort>,
        require_strict_schema: bool,
    ) -> Result<(), GenerationError> {
        let cap = self.require(provider, model)?;
        if !cap.is_active {
            return Err(GenerationError::new(
                GenerationErrorCode::UnsupportedCapability,
                "model is not active",
            ));
        }
        if let Some(effort) = reasoning_effort {
            if !cap.allows_reasoning(effort) {
                return Err(GenerationError::with_details(
                    GenerationErrorCode::UnsupportedCapability,
                    "model does not support requested reasoning_effort",
                    serde_json::json!({ "model": model, "reasoning_effort": format!("{effort:?}") }),
                ));
            }
        }
        if require_strict_schema && !cap.satisfies_strict_schema() {
            return Err(GenerationError::with_details(
                GenerationErrorCode::UnsupportedCapability,
                "model does not support strict JSON schema output",
                serde_json::json!({ "model": model }),
            ));
        }
        Ok(())
    }

    pub fn fallback_compatible(
        &self,
        required: &ModelCapability,
        candidate_provider: &ProviderKind,
        candidate_model: &str,
        require_same_structured: bool,
    ) -> bool {
        let Some(candidate) = self.get(candidate_provider, candidate_model) else {
            return false;
        };
        if !candidate.is_active {
            return false;
        }
        if require_same_structured
            && required.satisfies_strict_schema()
            && !candidate.satisfies_strict_schema()
        {
            return false;
        }
        true
    }

    pub fn list_active(&self) -> Vec<&ModelCapability> {
        self.models
            .values()
            .filter(|m| m.is_active)
            .collect()
    }

    pub fn default_model_for_provider(&self, provider: &ProviderKind) -> Option<String> {
        self.models
            .values()
            .find(|m| m.is_active && m.provider == *provider)
            .map(|m| m.model.clone())
    }

    fn key(provider: &ProviderKind, model: &str) -> String {
        format!("{}:{}", provider.as_str(), model.to_lowercase())
    }
}

fn bootstrap_capabilities() -> Vec<ModelCapability> {
    vec![
        ModelCapability {
            provider: ProviderKind::Fake,
            model: "fake-model".into(),
            supports_json_schema_strict: true,
            supports_json_object: true,
            supports_reasoning_effort: true,
            supports_streaming: false,
            supports_native_safety_prompt: false,
            max_input_tokens: 128_000,
            max_output_tokens: 16_384,
            structured_output_mode: StructuredOutputMode::JsonSchemaStrict,
            structured_output_adapter: StructuredOutputAdapterKind::PromptOnly,
            storage_disable_supported: true,
            is_active: true,
        },
        ModelCapability {
            provider: ProviderKind::OpenAi,
            model: "gpt-4.1".into(),
            supports_json_schema_strict: true,
            supports_json_object: true,
            supports_reasoning_effort: false,
            supports_streaming: true,
            supports_native_safety_prompt: false,
            max_input_tokens: 128_000,
            max_output_tokens: 16_384,
            structured_output_mode: StructuredOutputMode::JsonSchemaStrict,
            structured_output_adapter: StructuredOutputAdapterKind::OpenAiResponsesTextFormat,
            storage_disable_supported: true,
            is_active: true,
        },
        ModelCapability {
            provider: ProviderKind::OpenAi,
            model: "gpt-4o-mini".into(),
            supports_json_schema_strict: true,
            supports_json_object: true,
            supports_reasoning_effort: false,
            supports_streaming: true,
            supports_native_safety_prompt: false,
            max_input_tokens: 128_000,
            max_output_tokens: 16_384,
            structured_output_mode: StructuredOutputMode::JsonSchemaStrict,
            structured_output_adapter: StructuredOutputAdapterKind::OpenAiResponsesTextFormat,
            storage_disable_supported: true,
            is_active: true,
        },
        ModelCapability {
            provider: ProviderKind::Anthropic,
            model: "claude-sonnet-4-20250514".into(),
            supports_json_schema_strict: true,
            supports_json_object: true,
            supports_reasoning_effort: false,
            supports_streaming: true,
            supports_native_safety_prompt: true,
            max_input_tokens: 200_000,
            max_output_tokens: 8_192,
            structured_output_mode: StructuredOutputMode::JsonSchemaStrict,
            structured_output_adapter: StructuredOutputAdapterKind::AnthropicOutputConfigFormat,
            storage_disable_supported: false,
            is_active: true,
        },
        ModelCapability {
            provider: ProviderKind::Mistral,
            model: "mistral-large-latest".into(),
            supports_json_schema_strict: true,
            supports_json_object: true,
            supports_reasoning_effort: false,
            supports_streaming: true,
            supports_native_safety_prompt: false,
            max_input_tokens: 128_000,
            max_output_tokens: 8_192,
            structured_output_mode: StructuredOutputMode::JsonSchemaStrict,
            structured_output_adapter: StructuredOutputAdapterKind::MistralResponseFormatJsonSchema,
            storage_disable_supported: false,
            is_active: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_reasoning_on_gpt4o_mini() {
        let registry = ModelCapabilityRegistry::bootstrap();
        let err = registry.validate_request_capabilities(
            &ProviderKind::OpenAi,
            "gpt-4o-mini",
            Some(ReasoningEffort::High),
            true,
        );
        assert!(err.is_err());
    }

    #[test]
    fn rejects_reasoning_on_gpt41() {
        let registry = ModelCapabilityRegistry::bootstrap();
        let err = registry.validate_request_capabilities(
            &ProviderKind::OpenAi,
            "gpt-4.1",
            Some(ReasoningEffort::Medium),
            true,
        );
        assert!(err.is_err());
    }
}
