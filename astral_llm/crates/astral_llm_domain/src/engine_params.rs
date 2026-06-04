use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::provider::{ProviderKind, ReasoningEffort};

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct EngineParams {
    /// Si absent, le service utilise `ASTRAL_LLM_DEFAULT_PROVIDER` (OpenAI par defaut).
    #[serde(default)]
    pub provider: Option<ProviderKind>,
    /// Si absent ou vide : `llm_product_default_engine` pour le produit, sinon `ASTRAL_LLM_DEFAULT_MODEL`.
    #[serde(default)]
    pub model: Option<String>,
    /// SummarySynthesizer uniquement ; sinon `economic_model` produit si `model` absent.
    #[serde(default)]
    pub summary_model: Option<String>,
    pub reasoning_effort: Option<ReasoningEffort>,
    pub temperature: Option<f32>,
    pub max_output_tokens: Option<u32>,
    pub domain_count: Option<u8>,
    #[serde(default = "default_allow_fallback")]
    pub allow_fallback: bool,
    pub timeout_ms: Option<u64>,
    /// Autorise un modele `oracle_only` (ex. gpt-5.5-pro) sur un run benchmark dedie.
    #[serde(default)]
    pub allow_oracle_benchmark: bool,
}

fn default_allow_fallback() -> bool {
    true
}
