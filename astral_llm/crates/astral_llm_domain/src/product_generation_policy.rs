use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::model_capability::ProviderModelRef;
use crate::output_contract::GenerationMode;
use crate::provider::{ProviderKind, ReasoningEffort};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProductGenerationPolicy {
    pub product_code: String,
    pub allowed_providers: Vec<ProviderKind>,
    pub allowed_models: Vec<ProviderModelRef>,
    pub max_domains: u8,
    pub max_chapters: u8,
    pub max_output_tokens: u32,
    pub max_reasoning_effort: ReasoningEffort,
    pub allow_chapter_orchestrated: bool,
    /// References astro_basis valides minimum par chapitre (0 = pas de quota).
    pub min_astro_basis_refs_per_chapter: u8,
    /// References interpretatives minimum par chapitre (0 = pas de quota).
    /// Premium exige au moins 1 fact non-domain_score.
    pub min_interpretive_astro_basis_refs_per_chapter: u8,
    /// Moteur par defaut si `engine.provider` / `engine.model` absents (charge depuis `llm_product_default_engine`).
    pub default_provider: Option<ProviderKind>,
    pub default_model: Option<String>,
    /// SummarySynthesizer si `engine.model` absent — voir `config/llm_product_models.conf`.
    pub economic_model: Option<String>,
}

impl ProductGenerationPolicy {
    /// Politique produit minimale ; les caps effectifs viennent du profil d'interpretation.
    pub fn bootstrap_natal_prompter() -> Self {
        Self {
            product_code: crate::interpretation_profile::NATAL_PROMPTER_PRODUCT.into(),
            allowed_providers: vec![
                ProviderKind::Fake,
                ProviderKind::OpenAi,
                ProviderKind::Anthropic,
                ProviderKind::Mistral,
            ],
            allowed_models: vec![],
            max_domains: 12,
            max_chapters: 12,
            max_output_tokens: 16_000,
            max_reasoning_effort: ReasoningEffort::High,
            allow_chapter_orchestrated: true,
            min_astro_basis_refs_per_chapter: 0,
            min_interpretive_astro_basis_refs_per_chapter: 0,
            default_provider: None,
            default_model: None,
            economic_model: None,
        }
    }

    /// Liste vide = aucune restriction explicite (typique des lignes chargees depuis la DB).
    pub fn allows_provider(&self, provider: &ProviderKind) -> bool {
        if self.allowed_providers.is_empty() {
            return true;
        }
        self.allowed_providers.contains(provider)
    }

    pub fn allows_model(&self, provider: &ProviderKind, model: &str) -> bool {
        if self.allowed_models.is_empty() {
            return self.allows_provider(provider);
        }
        self.allowed_models
            .iter()
            .any(|m| m.provider == *provider && m.model.eq_ignore_ascii_case(model))
    }

    pub fn allows_mode(&self, mode: &GenerationMode) -> bool {
        match mode {
            GenerationMode::ChapterOrchestrated => self.allow_chapter_orchestrated,
            GenerationMode::SinglePass => true,
        }
    }

    pub fn caps_reasoning(&self, requested: Option<ReasoningEffort>) -> ReasoningEffort {
        let requested = requested.unwrap_or(ReasoningEffort::None);
        if reasoning_rank(requested) > reasoning_rank(self.max_reasoning_effort) {
            self.max_reasoning_effort
        } else {
            requested
        }
    }
}
fn reasoning_rank(effort: ReasoningEffort) -> u8 {
    match effort {
        ReasoningEffort::None => 0,
        ReasoningEffort::Minimal => 1,
        ReasoningEffort::Low => 2,
        ReasoningEffort::Medium => 3,
        ReasoningEffort::High => 4,
    }
}
