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
}

impl ProductGenerationPolicy {
    pub fn bootstrap_basic() -> Self {
        Self {
            product_code: "natal_basic".into(),
            allowed_providers: vec![
                ProviderKind::Fake,
                ProviderKind::OpenAi,
                ProviderKind::Mistral,
                ProviderKind::Anthropic,
            ],
            allowed_models: vec![
                ProviderModelRef::new(ProviderKind::Fake, "fake-model"),
                ProviderModelRef::new(ProviderKind::OpenAi, "gpt-4.1"),
                ProviderModelRef::new(ProviderKind::OpenAi, "gpt-4o-mini"),
            ],
            max_domains: 6,
            max_chapters: 6,
            max_output_tokens: 8_000,
            max_reasoning_effort: ReasoningEffort::Medium,
            allow_chapter_orchestrated: false,
            min_astro_basis_refs_per_chapter: 0,
        }
    }

    pub fn bootstrap_premium() -> Self {
        Self {
            product_code: "natal_premium".into(),
            allowed_providers: vec![
                ProviderKind::Fake,
                ProviderKind::OpenAi,
                ProviderKind::Anthropic,
                ProviderKind::Mistral,
            ],
            allowed_models: vec![
                ProviderModelRef::new(ProviderKind::Fake, "fake-model"),
                ProviderModelRef::new(ProviderKind::OpenAi, "gpt-4.1"),
                ProviderModelRef::new(ProviderKind::Anthropic, "claude-sonnet-4-20250514"),
            ],
            max_domains: 12,
            max_chapters: 12,
            max_output_tokens: 16_000,
            max_reasoning_effort: ReasoningEffort::High,
            allow_chapter_orchestrated: true,
            min_astro_basis_refs_per_chapter: 1,
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
        self.allowed_models.iter().any(|m| {
            m.provider == *provider && m.model.eq_ignore_ascii_case(model)
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ProviderKind;

    #[test]
    fn empty_allowed_providers_means_unrestricted() {
        let policy = ProductGenerationPolicy {
            allowed_providers: vec![],
            allowed_models: vec![],
            ..ProductGenerationPolicy::bootstrap_basic()
        };
        assert!(policy.allows_provider(&ProviderKind::OpenAi));
    }
}

fn reasoning_rank(effort: ReasoningEffort) -> u8 {
    match effort {
        ReasoningEffort::None => 0,
        ReasoningEffort::Low => 1,
        ReasoningEffort::Medium => 2,
        ReasoningEffort::High => 3,
    }
}
