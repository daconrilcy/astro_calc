use std::collections::HashSet;

use astral_llm_domain::{
    astro_fact::{AstroFactUsage, NormalizedAstroFacts},
    generation_response::ReadingChapter,
    interpretive_evidence::ChapterEvidencePack,
    GenerationError, GenerationErrorCode, ProductGenerationPolicy,
};
use astral_llm_infra::bootstrap_astro_basis_roles;

pub struct AstroBasisValidator;

impl AstroBasisValidator {
    pub fn validate_chapters(
        chapters: &[ReadingChapter],
        facts: &NormalizedAstroFacts,
        policy: &ProductGenerationPolicy,
    ) -> Result<(), GenerationError> {
        for chapter in chapters {
            Self::validate_chapter(chapter, facts, policy)?;
        }
        Ok(())
    }

    pub fn validate_chapter(
        chapter: &ReadingChapter,
        facts: &NormalizedAstroFacts,
        policy: &ProductGenerationPolicy,
    ) -> Result<(), GenerationError> {
        Self::validate_chapter_with_pack(chapter, facts, None, policy)
    }

    pub fn validate_chapter_with_pack(
        chapter: &ReadingChapter,
        facts: &NormalizedAstroFacts,
        pack: Option<&ChapterEvidencePack>,
        policy: &ProductGenerationPolicy,
    ) -> Result<(), GenerationError> {
        Self::validate_chapter_with_min_refs(
            chapter,
            facts,
            pack,
            policy.min_astro_basis_refs_per_chapter,
            policy.min_interpretive_astro_basis_refs_per_chapter,
        )
    }

    pub fn validate_chapter_with_min_refs(
        chapter: &ReadingChapter,
        facts: &NormalizedAstroFacts,
        pack: Option<&ChapterEvidencePack>,
        min_refs: u8,
        min_interpretive_refs: u8,
    ) -> Result<(), GenerationError> {
        Self::validate_fact_ids(chapter, facts, pack)?;
        let allowed_roles = Self::allowed_basis_roles();
        Self::validate_interpretive_roles(chapter, &allowed_roles)?;

        let valid_refs = chapter
            .astro_basis
            .iter()
            .filter(|b| {
                b.fact_id
                    .as_ref()
                    .is_some_and(|id| facts.contains_fact(id))
            })
            .count();

        if valid_refs < min_refs as usize {
            return Err(Self::basis_error(
                chapter,
                format!(
                    "chapter '{}' requires at least {min_refs} valid astro_basis references",
                    chapter.code
                ),
                valid_refs,
                min_refs,
                0,
                min_interpretive_refs,
                facts,
            ));
        }

        let interpretive_refs = chapter
            .astro_basis
            .iter()
            .filter(|b| {
                b.fact_id
                    .as_ref()
                    .is_some_and(|id| facts.is_interpretive_fact_id(id))
            })
            .count();

        if min_interpretive_refs > 0 && interpretive_refs < min_interpretive_refs as usize {
            return Err(Self::basis_error(
                chapter,
                format!(
                    "chapter '{}' requires at least {min_interpretive_refs} interpretive astro_basis \
                     (placement, aspect, angle, dignity or ruler — domain_score alone is insufficient)",
                    chapter.code
                ),
                valid_refs,
                min_refs,
                interpretive_refs,
                min_interpretive_refs,
                facts,
            ));
        }

        Ok(())
    }

    fn validate_interpretive_roles(
        chapter: &ReadingChapter,
        allowed: &HashSet<String>,
    ) -> Result<(), GenerationError> {
        for basis in &chapter.astro_basis {
            let role = basis.interpretive_role.trim().to_lowercase();
            if !allowed.contains(&role) {
                return Err(GenerationError::with_details(
                    GenerationErrorCode::AstroBasisInvalid,
                    format!(
                        "chapter '{}' has invalid interpretive_role '{}'",
                        chapter.code, basis.interpretive_role
                    ),
                    serde_json::json!({
                        "chapter": chapter.code,
                        "interpretive_role": basis.interpretive_role,
                        "allowed": allowed.iter().collect::<Vec<_>>(),
                    }),
                ));
            }
        }
        Ok(())
    }

    fn allowed_basis_roles() -> HashSet<String> {
        bootstrap_astro_basis_roles()
    }

    fn validate_fact_ids(
        chapter: &ReadingChapter,
        facts: &NormalizedAstroFacts,
        pack: Option<&ChapterEvidencePack>,
    ) -> Result<(), GenerationError> {
        for basis in &chapter.astro_basis {
            let Some(fact_id) = &basis.fact_id else { continue };
            if fact_id.starts_with("domain_score:") {
                continue;
            }
            if !facts.contains_fact(fact_id) {
                return Err(GenerationError::with_details(
                    GenerationErrorCode::AstroBasisInvalid,
                    format!(
                        "chapter '{}' cites unknown astro fact_id: {fact_id}",
                        chapter.code
                    ),
                    serde_json::json!({
                        "chapter": chapter.code,
                        "fact_id": fact_id,
                        "available_facts": facts.fact_ids(),
                    }),
                ));
            }
            if let Some(pack) = pack {
                let semantic_key = crate::evidence_fact_parse::compute_semantic_fact_key(
                    fact_id,
                    &serde_json::json!({}),
                    &std::collections::HashMap::new(),
                );
                if !pack.contains_fact_id_or_semantic(fact_id, &semantic_key) {
                    return Err(GenerationError::with_details(
                        GenerationErrorCode::AstroBasisInvalid,
                        format!(
                            "chapter '{}' cites fact_id not in chapter evidence pack: {fact_id}",
                            chapter.code
                        ),
                        serde_json::json!({
                            "chapter": chapter.code,
                            "fact_id": fact_id,
                            "pack_fact_ids": pack.all_fact_ids(),
                        }),
                    ));
                }
            }
        }
        Ok(())
    }

    fn basis_error(
        chapter: &ReadingChapter,
        message: String,
        valid_refs: usize,
        min_refs: u8,
        interpretive_refs: usize,
        min_interpretive_refs: u8,
        facts: &NormalizedAstroFacts,
    ) -> GenerationError {
        GenerationError::with_details(
            GenerationErrorCode::SchemaValidationFailed,
            message,
            serde_json::json!({
                "chapter": chapter.code,
                "valid_refs": valid_refs,
                "min_refs": min_refs,
                "interpretive_refs": interpretive_refs,
                "min_interpretive_refs": min_interpretive_refs,
                "available_facts": facts.fact_ids(),
                "interpretive_facts": facts.interpretive_fact_ids(),
                "domain_selection_only": chapter.astro_basis.iter().all(|b| {
                    b.fact_id.as_ref().is_none_or(|id| {
                        facts
                            .fact_by_id(id)
                            .is_some_and(|f| f.usage == AstroFactUsage::DomainSelection)
                    })
                }),
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::{
        astro_fact::{AstroFactKind, NormalizedAstroFact},
        generation_response::{ConfidenceLevel, ReadingChapter},
    };

    fn sample_facts() -> NormalizedAstroFacts {
        NormalizedAstroFacts {
            contract_version: "natal_structured_v13".into(),
            facts: vec![
                NormalizedAstroFact {
                    id: "domain_score:identity".into(),
                    kind: AstroFactKind::DomainScore,
                    kind_code: "domain_score".into(),
                    usage: AstroFactUsage::DomainSelection,
                    label: "Score identity".into(),
                    value: serde_json::json!(0.8),
                    interpretive_weight: Some(0.8),
                    domains: vec!["identity".into()],
                },
                NormalizedAstroFact {
                    id: "placement:sun:capricorn:house:2".into(),
                    kind: AstroFactKind::PlanetPosition,
                    kind_code: "placement".into(),
                    usage: AstroFactUsage::InterpretiveBasis,
                    label: "Sun Capricorn H2".into(),
                    value: serde_json::json!({}),
                    interpretive_weight: None,
                    domains: vec![],
                },
            ],
        }
    }

    fn chapter_with_basis(fact_ids: Vec<(&str, &str)>) -> ReadingChapter {
        ReadingChapter {
            code: "identity".into(),
            title: "Identite".into(),
            body: "body".into(),
            astro_basis: fact_ids
                .into_iter()
                .map(|(id, role)| astral_llm_domain::AstroBasisItem {
                    fact_id: Some(id.to_string()),
                    label: None,
                    factor: id.to_string(),
                    interpretive_role: role.to_string(),
                })
                .collect(),
            confidence: ConfidenceLevel::Medium,
            safety_flags: vec![],
        }
    }

    #[test]
    fn premium_rejects_domain_score_only() {
        let facts = sample_facts();
        let chapter = chapter_with_basis(vec![("domain_score:identity", "domain_score")]);
        let policy = ProductGenerationPolicy::bootstrap_premium();
        assert!(AstroBasisValidator::validate_chapter(&chapter, &facts, &policy).is_err());
    }

    #[test]
    fn premium_accepts_domain_score_plus_placement() {
        let facts = sample_facts();
        let chapter = chapter_with_basis(vec![
            ("domain_score:identity", "domain_score"),
            ("placement:sun:capricorn:house:2", "core"),
        ]);
        let policy = ProductGenerationPolicy::bootstrap_premium();
        assert!(AstroBasisValidator::validate_chapter(&chapter, &facts, &policy).is_ok());
    }

    #[test]
    fn basic_allows_domain_score_only() {
        let facts = sample_facts();
        let chapter = chapter_with_basis(vec![("domain_score:identity", "domain_score")]);
        let policy = ProductGenerationPolicy::bootstrap_basic();
        assert!(AstroBasisValidator::validate_chapter(&chapter, &facts, &policy).is_ok());
    }
}
