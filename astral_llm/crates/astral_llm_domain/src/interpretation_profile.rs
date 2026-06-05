use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::interpretive_evidence::PremiumEvidencePolicy;
use crate::output_contract::GenerationMode;
use crate::product_generation_policy::ProductGenerationPolicy;
use crate::provider::{ProviderKind, ReasoningEffort};

pub const NATAL_PROMPTER_PRODUCT: &str = "natal_prompter";

pub const PROFILE_NATAL_LIGHT: &str = "natal_light";
pub const PROFILE_NATAL_BASIC: &str = "natal_basic";
pub const PROFILE_NATAL_PREMIUM: &str = "natal_premium";
pub const PROFILE_NATAL_PREMIUM_PLUS: &str = "natal_premium_plus";
pub const SYNTHESIS_CHAPTER_CODE: &str = "synthesis";

/// Anciens `product_code` API acceptes temporairement par le shim de migration.
pub const LEGACY_PRODUCT_NATAL_PREMIUM: &str = "natal_premium";
pub const LEGACY_PRODUCT_NATAL_BASIC: &str = "natal_basic";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InterpretationProfileDocument {
    pub profile_code: String,
    pub product_code: String,
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    pub generation_mode: GenerationMode,
    pub max_domains: u8,
    pub max_chapters: u8,
    pub max_output_tokens: u32,
    pub max_reasoning_effort: ReasoningEffort,
    #[serde(default)]
    pub default_domain_count: Option<u8>,
    pub chapter_models: InterpretationChapterModels,
    #[serde(default)]
    pub chapter_types: Vec<String>,
    pub chapter_word_targets: ChapterWordTargets,
    pub quality: InterpretationQualityConfig,
    pub evidence: InterpretationEvidenceConfig,
    #[serde(default)]
    pub task_fragment: Option<String>,
}

fn default_schema_version() -> String {
    "v1".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InterpretationChapterModels {
    pub default_provider: String,
    pub default_model: String,
    #[serde(default)]
    pub summary_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ChapterWordTargets {
    pub min: u16,
    pub target: u16,
    pub max: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InterpretationQualityConfig {
    pub blocking_gate: bool,
    pub min_words_per_chapter: u16,
    pub max_repeated_trigrams: u8,
    pub min_astro_basis_refs_per_chapter: u8,
    pub min_interpretive_astro_basis_refs_per_chapter: u8,
    #[serde(default = "default_true")]
    pub require_disclaimer: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct InterpretationEvidenceConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub policy: Option<EmbeddedEvidencePolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EmbeddedEvidencePolicy {
    pub min_evidence_per_chapter: u8,
    pub min_distinct_kind_families: u8,
    pub min_non_placement_if_available: u8,
    pub max_core_overlap_ratio: f32,
    #[serde(default)]
    pub domain_score_counts_in_minimum: bool,
    pub max_core_evidence: u8,
    pub max_supporting_evidence: u8,
    pub max_nuance_evidence: u8,
    pub max_avoid_repeating: u8,
    #[serde(default = "default_max_supporting_semantic")]
    pub max_supporting_semantic_chapters: u8,
}

fn default_max_supporting_semantic() -> u8 {
    3
}

#[derive(Debug, Clone)]
pub struct InterpretationProfile {
    pub profile_code: String,
    pub product_code: String,
    pub schema_version: String,
    pub document: InterpretationProfileDocument,
}

impl InterpretationProfile {
    pub fn from_document(doc: InterpretationProfileDocument) -> Self {
        Self {
            profile_code: doc.profile_code.clone(),
            product_code: doc.product_code.clone(),
            schema_version: doc.schema_version.clone(),
            document: doc,
        }
    }

    pub fn allows_chapter_orchestration(&self) -> bool {
        matches!(
            self.document.generation_mode,
            GenerationMode::ChapterOrchestrated
        )
    }

    pub fn evidence_enabled(&self) -> bool {
        self.document.evidence.enabled
    }

    pub fn blocking_quality_gate(&self) -> bool {
        self.document.quality.blocking_gate
    }

    pub fn require_disclaimer(&self) -> bool {
        self.document.quality.require_disclaimer
    }

    pub fn default_domain_count(&self) -> u8 {
        self.document
            .default_domain_count
            .unwrap_or(self.document.max_domains.min(3))
    }

    pub fn to_product_generation_policy(&self) -> ProductGenerationPolicy {
        ProductGenerationPolicy {
            product_code: self.product_code.clone(),
            allowed_providers: vec![],
            allowed_models: vec![],
            max_domains: self.document.max_domains,
            max_chapters: self.document.max_chapters,
            max_output_tokens: self.document.max_output_tokens,
            max_reasoning_effort: self.document.max_reasoning_effort,
            allow_chapter_orchestrated: self.allows_chapter_orchestration(),
            min_astro_basis_refs_per_chapter: self.document.quality.min_astro_basis_refs_per_chapter,
            min_interpretive_astro_basis_refs_per_chapter: self
                .document
                .quality
                .min_interpretive_astro_basis_refs_per_chapter,
            default_provider: parse_provider(&self.document.chapter_models.default_provider),
            default_model: Some(self.document.chapter_models.default_model.clone()),
            economic_model: self.document.chapter_models.summary_model.clone(),
        }
    }

    pub fn to_premium_evidence_policy(&self) -> Option<PremiumEvidencePolicy> {
        if !self.evidence_enabled() {
            return None;
        }
        let p = self.document.evidence.policy.as_ref()?;
        Some(PremiumEvidencePolicy {
            product_code: self.profile_code.clone(),
            min_evidence_per_chapter: p.min_evidence_per_chapter,
            min_distinct_kind_families: p.min_distinct_kind_families,
            min_non_placement_if_available: p.min_non_placement_if_available,
            max_core_overlap_ratio: p.max_core_overlap_ratio,
            domain_score_counts_in_minimum: p.domain_score_counts_in_minimum,
            max_core_evidence: p.max_core_evidence,
            max_supporting_evidence: p.max_supporting_evidence,
            max_nuance_evidence: p.max_nuance_evidence,
            max_avoid_repeating: p.max_avoid_repeating,
            max_supporting_semantic_chapters: p.max_supporting_semantic_chapters,
        })
    }

    pub fn has_final_synthesis_chapter(&self) -> bool {
        self.document
            .chapter_types
            .iter()
            .any(|c| c == SYNTHESIS_CHAPTER_CODE)
    }

    pub fn astrological_chapter_types(&self) -> Vec<String> {
        self.document
            .chapter_types
            .iter()
            .filter(|c| *c != SYNTHESIS_CHAPTER_CODE)
            .cloned()
            .collect()
    }

    pub fn uses_rich_editorial_structure(&self) -> bool {
        self.document.chapter_word_targets.target >= 420
    }

    /// Profils dont l'ordre `chapter_types` definit la lecture (ex. `natal_premium_plus`).
    pub fn uses_fixed_chapter_sequence(&self) -> bool {
        let astro = self.astrological_chapter_types();
        !astro.is_empty()
            && self.default_domain_count() as usize == astro.len()
    }

    pub fn planned_chapter_count(&self, engine_domain_count: Option<u8>) -> u8 {
        let astro_len = self.astrological_chapter_types().len();
        let domain_n = engine_domain_count
            .unwrap_or_else(|| self.default_domain_count())
            .max(1) as usize;
        let astro_count = if astro_len == 0 {
            domain_n
        } else {
            domain_n.min(astro_len)
        };
        let mut total = astro_count as u8;
        if self.has_final_synthesis_chapter() {
            total = total.saturating_add(1);
        }
        total
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.product_code != NATAL_PROMPTER_PRODUCT {
            return Err(format!(
                "product_code must be {NATAL_PROMPTER_PRODUCT}, got {}",
                self.product_code
            ));
        }
        if self.profile_code.trim().is_empty() {
            return Err("profile_code is required".into());
        }
        if self.document.chapter_word_targets.min > self.document.chapter_word_targets.max {
            return Err("chapter_word_targets.min must be <= max".into());
        }
        if self.document.evidence.enabled && self.document.evidence.policy.is_none() {
            return Err("evidence.policy required when evidence.enabled is true".into());
        }
        Ok(())
    }
}

pub fn parse_provider(raw: &str) -> Option<ProviderKind> {
    match raw.trim().to_lowercase().as_str() {
        "openai" => Some(ProviderKind::OpenAi),
        "anthropic" => Some(ProviderKind::Anthropic),
        "mistral" => Some(ProviderKind::Mistral),
        "fake" => Some(ProviderKind::Fake),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn premium_profile_parses_from_fixture_shape() {
        let json = include_str!("../../../../config/natal_interpretation_profiles/natal_premium.json");
        let doc: InterpretationProfileDocument = serde_json::from_str(json).expect("parse");
        let profile = InterpretationProfile::from_document(doc);
        assert!(profile.validate().is_ok());
        assert!(profile.evidence_enabled());
        assert!(profile.blocking_quality_gate());
    }

    #[test]
    fn premium_plus_uses_fixed_chapter_sequence() {
        let json =
            include_str!("../../../../config/natal_interpretation_profiles/natal_premium_plus.json");
        let doc: InterpretationProfileDocument = serde_json::from_str(json).expect("parse");
        let profile = InterpretationProfile::from_document(doc);
        assert!(profile.uses_fixed_chapter_sequence());
        assert_eq!(profile.planned_chapter_count(None), 9);
    }

    #[test]
    fn premium_plus_profile_parses_from_fixture_shape() {
        let json =
            include_str!("../../../../config/natal_interpretation_profiles/natal_premium_plus.json");
        let doc: InterpretationProfileDocument = serde_json::from_str(json).expect("parse");
        let profile = InterpretationProfile::from_document(doc);
        assert!(profile.validate().is_ok());
        assert!(profile.has_final_synthesis_chapter());
        assert!(profile.uses_rich_editorial_structure());
        assert_eq!(profile.document.quality.min_words_per_chapter, 420);
        assert_eq!(profile.document.quality.min_astro_basis_refs_per_chapter, 6);
    }
}
