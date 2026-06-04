use std::collections::{HashMap, HashSet};

use astral_llm_domain::{
    model_capability::{ModelCapability, StructuredOutputAdapterKind},
    model_usage_tier::ModelRouteContext,
    provider::{ProviderKind, StructuredOutputMode},
    GenerationError, GenerationErrorCode, ReasoningEffort,
};

pub struct ModelCapabilityRegistry {
    models: HashMap<String, ModelCapability>,
    active_providers: HashSet<String>,
    enforce_provider_catalog: bool,
}

impl ModelCapabilityRegistry {
    /// Catalogue minimal pour tests locaux sans base (fake uniquement).
    pub fn bootstrap() -> Self {
        let mut registry = Self::empty();
        registry.register(fake_capability());
        registry
    }

    /// Fallback dev hors base : inclut les modeles historiques pour tests unitaires.
    pub fn bootstrap_dev_fallback() -> Self {
        let mut registry = Self::bootstrap();
        for cap in dev_fallback_capabilities() {
            registry.register(cap);
        }
        registry
    }

    pub fn from_db_catalog(
        active_provider_codes: Vec<String>,
        db_models: Vec<ModelCapability>,
    ) -> Self {
        let mut registry = Self::bootstrap();
        registry.enforce_provider_catalog = !active_provider_codes.is_empty();
        registry.active_providers = active_provider_codes
            .into_iter()
            .map(|c| c.trim().to_lowercase())
            .collect();
        for cap in db_models {
            registry.register(cap);
        }
        registry
    }

    fn empty() -> Self {
        Self {
            models: HashMap::new(),
            active_providers: HashSet::new(),
            enforce_provider_catalog: false,
        }
    }

    pub fn register(&mut self, capability: ModelCapability) {
        let key = Self::key(&capability.provider, &capability.model);
        self.models.insert(key, capability);
    }

    pub fn active_provider_codes(&self) -> Vec<String> {
        let mut codes: Vec<_> = self.active_providers.iter().cloned().collect();
        codes.sort();
        codes
    }

    pub fn get(&self, provider: &ProviderKind, model: &str) -> Option<&ModelCapability> {
        self.models.get(&Self::key(provider, model))
    }

    pub fn require(
        &self,
        provider: &ProviderKind,
        model: &str,
    ) -> Result<&ModelCapability, GenerationError> {
        if self.enforce_provider_catalog && !self.provider_is_active(provider) {
            return Err(GenerationError::with_details(
                GenerationErrorCode::UnsupportedProvider,
                format!("provider is not in the active catalog: {}", provider.as_str()),
                serde_json::json!({ "provider": provider.as_str() }),
            ));
        }

        self.get(provider, model).ok_or_else(|| {
            GenerationError::with_details(
                GenerationErrorCode::UnsupportedCapability,
                format!("unknown or inactive model: {provider:?}/{model}"),
                serde_json::json!({ "provider": provider.as_str(), "model": model }),
            )
        })
    }

    pub fn validate_engine_in_catalog(
        &self,
        provider: &ProviderKind,
        model: &str,
    ) -> Result<(), GenerationError> {
        self.validate_engine_for_context(ModelRouteContext::PrimaryReading, provider, model, false)
    }

    pub fn validate_engine_for_context(
        &self,
        context: ModelRouteContext,
        provider: &ProviderKind,
        model: &str,
        allow_oracle_benchmark: bool,
    ) -> Result<(), GenerationError> {
        if model.trim().is_empty() {
            return Err(GenerationError::new(
                GenerationErrorCode::InvalidInput,
                "resolved engine.model is empty",
            ));
        }
        if matches!(provider, ProviderKind::Custom(_)) {
            return Err(GenerationError::new(
                GenerationErrorCode::UnsupportedProvider,
                "custom providers are not supported",
            ));
        }
        let cap = self.require(provider, model)?;
        if !cap.is_active {
            return Err(GenerationError::with_details(
                GenerationErrorCode::UnsupportedCapability,
                "model is not active in the provider catalog",
                serde_json::json!({
                    "provider": provider.as_str(),
                    "model": model
                }),
            ));
        }

        let effective_context = if allow_oracle_benchmark && context == ModelRouteContext::PrimaryReading
        {
            ModelRouteContext::OracleBenchmark
        } else {
            context
        };

        if !cap.tier_policy.allows(effective_context) {
            return Err(GenerationError::with_details(
                GenerationErrorCode::UnsupportedCapability,
                "model usage tier does not allow this generation context",
                serde_json::json!({
                    "provider": provider.as_str(),
                    "model": model,
                    "usage_tier_code": cap.usage_tier_code,
                    "context": format!("{effective_context:?}"),
                }),
            ));
        }
        Ok(())
    }

    pub fn validate_request_capabilities(
        &self,
        provider: &ProviderKind,
        model: &str,
        reasoning_effort: Option<ReasoningEffort>,
        require_strict_schema: bool,
    ) -> Result<(), GenerationError> {
        self.validate_engine_for_context(ModelRouteContext::PrimaryReading, provider, model, false)?;
        let cap = self.require(provider, model)?;
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
            .filter(|m| m.is_active && self.provider_is_active(&m.provider))
            .collect()
    }

    pub fn default_model_for_provider(&self, provider: &ProviderKind) -> Option<String> {
        self.models
            .values()
            .find(|m| {
                m.is_active
                    && m.provider == *provider
                    && self.provider_is_active(provider)
            })
            .map(|m| m.model.clone())
    }

    fn provider_is_active(&self, provider: &ProviderKind) -> bool {
        if !self.enforce_provider_catalog {
            return true;
        }
        self.active_providers.contains(provider.as_str())
    }

    fn key(provider: &ProviderKind, model: &str) -> String {
        format!("{}:{}", provider.as_str(), model.to_lowercase())
    }
}

fn fake_capability() -> ModelCapability {
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
        supports_temperature: true,
        reasoning_output_reserve_min: Some(4096),
        reasoning_effort_subtask: None,
        reasoning_effort_primary: None,
        reasoning_effort_oracle: None,
        usage_tier_code: None,
        tier_policy: astral_llm_domain::ModelUsageTierPolicy::unrestricted(),
    }
}

fn dev_fallback_capabilities() -> Vec<ModelCapability> {
    vec![
        ModelCapability {
            provider: ProviderKind::OpenAi,
            model: "gpt-4.1".into(),
            supports_json_schema_strict: true,
            supports_json_object: true,
            supports_reasoning_effort: false,
            supports_streaming: true,
            supports_native_safety_prompt: false,
            max_input_tokens: 1_000_000,
            max_output_tokens: 32_000,
            structured_output_mode: StructuredOutputMode::JsonSchemaStrict,
            structured_output_adapter: StructuredOutputAdapterKind::OpenAiResponsesTextFormat,
            storage_disable_supported: true,
            is_active: true,
            supports_temperature: true,
            reasoning_output_reserve_min: None,
            reasoning_effort_subtask: None,
            reasoning_effort_primary: None,
            reasoning_effort_oracle: None,
            usage_tier_code: Some("baseline".into()),
            tier_policy: astral_llm_domain::ModelUsageTierPolicy {
                allows_primary_reading: true,
                allows_subtask: true,
                allows_oracle_benchmark: false,
            },
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
            supports_temperature: true,
            reasoning_output_reserve_min: None,
            reasoning_effort_subtask: None,
            reasoning_effort_primary: None,
            reasoning_effort_oracle: None,
            usage_tier_code: None,
            tier_policy: astral_llm_domain::ModelUsageTierPolicy::unrestricted(),
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
            supports_temperature: true,
            reasoning_output_reserve_min: None,
            reasoning_effort_subtask: None,
            reasoning_effort_primary: None,
            reasoning_effort_oracle: None,
            usage_tier_code: Some("production_candidate".into()),
            tier_policy: astral_llm_domain::ModelUsageTierPolicy {
                allows_primary_reading: true,
                allows_subtask: true,
                allows_oracle_benchmark: false,
            },
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
            supports_temperature: true,
            reasoning_output_reserve_min: None,
            reasoning_effort_subtask: None,
            reasoning_effort_primary: None,
            reasoning_effort_oracle: None,
            usage_tier_code: Some("production_candidate".into()),
            tier_policy: astral_llm_domain::ModelUsageTierPolicy {
                allows_primary_reading: true,
                allows_subtask: true,
                allows_oracle_benchmark: false,
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_reasoning_on_gpt4o_mini() {
        let registry = ModelCapabilityRegistry::bootstrap_dev_fallback();
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
        let registry = ModelCapabilityRegistry::bootstrap_dev_fallback();
        let err = registry.validate_request_capabilities(
            &ProviderKind::OpenAi,
            "gpt-4.1",
            Some(ReasoningEffort::Medium),
            true,
        );
        assert!(err.is_err());
    }

    #[test]
    fn rejects_inactive_provider_when_catalog_enforced() {
        let registry = ModelCapabilityRegistry::from_db_catalog(
            vec!["fake".into()],
            vec![ModelCapability {
                provider: ProviderKind::OpenAi,
                model: "gpt-4.1".into(),
                supports_json_schema_strict: true,
                supports_json_object: true,
                supports_reasoning_effort: false,
                supports_streaming: true,
                supports_native_safety_prompt: false,
                max_input_tokens: 1_000_000,
                max_output_tokens: 32_000,
                structured_output_mode: StructuredOutputMode::JsonSchemaStrict,
                structured_output_adapter: StructuredOutputAdapterKind::OpenAiResponsesTextFormat,
                storage_disable_supported: true,
                is_active: true,
                supports_temperature: true,
                reasoning_output_reserve_min: None,
                reasoning_effort_subtask: None,
                reasoning_effort_primary: None,
                reasoning_effort_oracle: None,
                usage_tier_code: Some("baseline".into()),
                tier_policy: astral_llm_domain::ModelUsageTierPolicy {
                    allows_primary_reading: true,
                    allows_subtask: true,
                    allows_oracle_benchmark: false,
                },
            }],
        );
        assert!(registry
            .validate_engine_in_catalog(&ProviderKind::OpenAi, "gpt-4.1")
            .is_err());
    }

    #[test]
    fn rejects_inactive_model_when_catalog_enforced() {
        let mut registry = ModelCapabilityRegistry::from_db_catalog(
            vec!["openai".into()],
            vec![],
        );
        registry.register(ModelCapability {
            provider: ProviderKind::OpenAi,
            model: "gpt-5.5-pro".into(),
            supports_json_schema_strict: true,
            supports_json_object: true,
            supports_reasoning_effort: true,
            supports_streaming: false,
            supports_native_safety_prompt: false,
            max_input_tokens: 1_050_000,
            max_output_tokens: 128_000,
            structured_output_mode: StructuredOutputMode::JsonSchemaStrict,
            structured_output_adapter: StructuredOutputAdapterKind::OpenAiResponsesTextFormat,
            storage_disable_supported: true,
            is_active: false,
            supports_temperature: false,
            reasoning_output_reserve_min: Some(4096),
            reasoning_effort_subtask: Some(ReasoningEffort::None),
            reasoning_effort_primary: Some(ReasoningEffort::Low),
            reasoning_effort_oracle: Some(ReasoningEffort::Medium),
            usage_tier_code: Some("oracle_only".into()),
            tier_policy: astral_llm_domain::ModelUsageTierPolicy {
                allows_primary_reading: false,
                allows_subtask: false,
                allows_oracle_benchmark: true,
            },
        });
        assert!(registry
            .validate_engine_for_context(
                ModelRouteContext::PrimaryReading,
                &ProviderKind::OpenAi,
                "gpt-5.5-pro",
                false,
            )
            .is_err());
    }

    #[test]
    fn oracle_allowed_with_explicit_flag() {
        let mut registry = ModelCapabilityRegistry::from_db_catalog(
            vec!["openai".into()],
            vec![],
        );
        registry.register(ModelCapability {
            provider: ProviderKind::OpenAi,
            model: "gpt-5.5-pro".into(),
            supports_json_schema_strict: true,
            supports_json_object: true,
            supports_reasoning_effort: true,
            supports_streaming: false,
            supports_native_safety_prompt: false,
            max_input_tokens: 1_050_000,
            max_output_tokens: 128_000,
            structured_output_mode: StructuredOutputMode::JsonSchemaStrict,
            structured_output_adapter: StructuredOutputAdapterKind::OpenAiResponsesTextFormat,
            storage_disable_supported: true,
            is_active: true,
            supports_temperature: false,
            reasoning_output_reserve_min: Some(4096),
            reasoning_effort_subtask: Some(ReasoningEffort::None),
            reasoning_effort_primary: Some(ReasoningEffort::Low),
            reasoning_effort_oracle: Some(ReasoningEffort::Medium),
            usage_tier_code: Some("oracle_only".into()),
            tier_policy: astral_llm_domain::ModelUsageTierPolicy {
                allows_primary_reading: false,
                allows_subtask: false,
                allows_oracle_benchmark: true,
            },
        });
        assert!(registry
            .validate_engine_for_context(
                ModelRouteContext::PrimaryReading,
                &ProviderKind::OpenAi,
                "gpt-5.5-pro",
                true,
            )
            .is_ok());
    }
}
