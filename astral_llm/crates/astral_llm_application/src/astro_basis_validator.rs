use std::collections::HashSet;

use astral_llm_domain::{
    astro_fact::{AstroFactUsage, NormalizedAstroFacts},
    generation_response::ReadingChapter,
    interpretive_evidence::ChapterEvidencePack,
    GenerationError, GenerationErrorCode, ProductGenerationPolicy,
};

use crate::reading_catalog::AstroBasisRoleCatalogView;

pub struct AstroBasisValidator;

impl AstroBasisValidator {
    pub fn validate_chapters(
        chapters: &[ReadingChapter],
        facts: &NormalizedAstroFacts,
        catalog: AstroBasisRoleCatalogView<'_>,
        policy: &ProductGenerationPolicy,
    ) -> Result<(), GenerationError> {
        for chapter in chapters {
            Self::validate_chapter(chapter, facts, catalog, policy)?;
        }
        Ok(())
    }

    pub fn validate_chapter(
        chapter: &ReadingChapter,
        facts: &NormalizedAstroFacts,
        catalog: AstroBasisRoleCatalogView<'_>,
        policy: &ProductGenerationPolicy,
    ) -> Result<(), GenerationError> {
        Self::validate_chapter_with_pack(chapter, facts, None, catalog, policy)
    }

    pub fn validate_chapter_with_pack(
        chapter: &ReadingChapter,
        facts: &NormalizedAstroFacts,
        pack: Option<&ChapterEvidencePack>,
        catalog: AstroBasisRoleCatalogView<'_>,
        policy: &ProductGenerationPolicy,
    ) -> Result<(), GenerationError> {
        Self::validate_chapter_with_min_refs(
            chapter,
            facts,
            pack,
            catalog,
            policy.min_astro_basis_refs_per_chapter,
            policy.min_interpretive_astro_basis_refs_per_chapter,
        )
    }

    pub fn validate_chapter_with_min_refs(
        chapter: &ReadingChapter,
        facts: &NormalizedAstroFacts,
        pack: Option<&ChapterEvidencePack>,
        catalog: AstroBasisRoleCatalogView<'_>,
        min_refs: u8,
        min_interpretive_refs: u8,
    ) -> Result<(), GenerationError> {
        Self::validate_fact_ids(chapter, facts, pack)?;
        let allowed_roles = Self::allowed_basis_roles(catalog);
        Self::validate_interpretive_roles(chapter, &allowed_roles)?;

        let valid_refs = chapter
            .astro_basis
            .iter()
            .filter(|b| {
                b.fact_id.as_ref().is_some_and(|id| {
                    crate::evidence_fact_parse::resolve_canonical_fact_id(id, facts).is_some()
                })
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
                b.fact_id.as_ref().is_some_and(|id| {
                    crate::evidence_fact_parse::resolve_canonical_fact_id(id, facts)
                        .is_some_and(|resolved| facts.is_interpretive_fact_id(&resolved))
                })
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

    fn allowed_basis_roles(catalog: AstroBasisRoleCatalogView<'_>) -> HashSet<String> {
        catalog.allowed_roles()
    }

    fn validate_fact_ids(
        chapter: &ReadingChapter,
        facts: &NormalizedAstroFacts,
        pack: Option<&ChapterEvidencePack>,
    ) -> Result<(), GenerationError> {
        for basis in &chapter.astro_basis {
            let Some(fact_id) = &basis.fact_id else {
                continue;
            };
            if fact_id.starts_with("domain_score:") {
                continue;
            }
            let resolved = crate::evidence_fact_parse::resolve_canonical_fact_id(fact_id, facts)
                .unwrap_or_else(|| fact_id.clone());
            if !facts.contains_fact(&resolved) {
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
                    &resolved,
                    &serde_json::json!({}),
                    &std::collections::HashMap::new(),
                );
                if !pack.contains_fact_id_or_semantic(&resolved, &semantic_key) {
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
