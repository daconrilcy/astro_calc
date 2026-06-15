use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::model_usage_tier::ModelUsageTierPolicy;
use crate::provider::{ProviderKind, ReasoningEffort, StructuredOutputMode};
use crate::token_usage::TokenPricing;

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
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub api_model_id: Option<String>,
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
    pub supports_temperature: bool,
    /// Plancher de `max_output_tokens` quand le modele consomme des tokens de raisonnement (canonique en base).
    #[serde(default)]
    pub reasoning_output_reserve_min: Option<u32>,
    /// Effort par defaut (`llm_provider_models.reasoning_effort_*`), valeurs API OpenAI.
    #[serde(default)]
    pub reasoning_effort_subtask: Option<ReasoningEffort>,
    #[serde(default)]
    pub reasoning_effort_primary: Option<ReasoningEffort>,
    #[serde(default)]
    pub reasoning_effort_oracle: Option<ReasoningEffort>,
    /// Code tier canonique (`llm_model_usage_tiers.tier_code`), ex. `production_candidate`.
    #[serde(default)]
    pub usage_tier_code: Option<String>,
    #[serde(default = "ModelUsageTierPolicy::unrestricted")]
    pub tier_policy: ModelUsageTierPolicy,
    #[serde(default)]
    pub input_price_usd_per_mtok: Option<f64>,
    #[serde(default)]
    pub output_price_usd_per_mtok: Option<f64>,
    #[serde(default)]
    pub cache_read_price_usd_per_mtok: Option<f64>,
    #[serde(default)]
    pub cache_write_price_usd_per_mtok: Option<f64>,
    #[serde(default)]
    pub reasoning_price_usd_per_mtok: Option<f64>,
    #[serde(default)]
    pub pricing_currency: Option<String>,
    #[serde(default)]
    pub pricing_source: Option<String>,
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
        if matches!(effort, ReasoningEffort::None) {
            return true;
        }
        self.supports_reasoning_effort
    }

    /// Reserve minimale de sortie pour laisser place au raisonnement + message (0 si non applicable).
    pub fn reasoning_output_reserve(&self) -> u32 {
        if !self.supports_reasoning_effort {
            return 0;
        }
        self.reasoning_output_reserve_min
            .filter(|&n| n > 0)
            .unwrap_or(0)
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

    pub fn token_pricing(&self) -> TokenPricing {
        TokenPricing {
            currency: self
                .pricing_currency
                .clone()
                .unwrap_or_else(|| "USD".to_string()),
            input_price_usd_per_mtok: self.input_price_usd_per_mtok,
            output_price_usd_per_mtok: self.output_price_usd_per_mtok,
            cache_read_price_usd_per_mtok: self.cache_read_price_usd_per_mtok,
            cache_write_price_usd_per_mtok: self.cache_write_price_usd_per_mtok,
            reasoning_price_usd_per_mtok: self.reasoning_price_usd_per_mtok,
        }
    }
}
