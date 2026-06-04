use astral_llm_domain::{
    generation_response::ReadingChapter,
    interpretive_evidence::{
        ChapterEvidencePack, EvidenceRequirementSeverity, InterpretiveEvidencePool,
        PremiumEvidencePolicy, KIND_ASPECT,
    },
    GenerationError, GenerationErrorCode,
};
use astral_llm_infra::EvidenceCanonicalCatalog;

use crate::chapter_evidence_planner::requirement_pool_match;
use crate::interpretive_evidence_builder::is_premium_product;

pub struct EvidenceDiversityValidator;

impl EvidenceDiversityValidator {
    pub fn validate_packs(
        product_code: &str,
        pool: &InterpretiveEvidencePool,
        packs: &[ChapterEvidencePack],
        catalog: &EvidenceCanonicalCatalog,
        policy: &PremiumEvidencePolicy,
    ) -> Result<(), GenerationError> {
        if !is_premium_product(product_code) {
            return Ok(());
        }

        crate::interpretive_evidence_builder::pool_richness_check(pool, policy)?;

        for pack in packs {
            Self::validate_chapter_requirements(pool, pack, catalog)?;
        }

        if pool.pool_has_aspects() {
            let any_aspect = packs.iter().any(|p| {
                p.core.iter().chain(p.supporting.iter()).chain(p.nuance.iter()).any(|e| {
                    e.kind_code == KIND_ASPECT
                })
            });
            if !any_aspect {
                return Err(Self::diversity_error(
                    "Pool contains aspects but no chapter pack selected an aspect",
                    serde_json::json!({ "rule": "global_aspect_usage" }),
                ));
            }
        }

        Ok(())
    }

    pub fn validate_packs_planned(
        product_code: &str,
        pool: &InterpretiveEvidencePool,
        packs: &[ChapterEvidencePack],
        catalog: &EvidenceCanonicalCatalog,
        policy: &PremiumEvidencePolicy,
    ) -> Result<(), GenerationError> {
        if !is_premium_product(product_code) {
            return Ok(());
        }
        crate::interpretive_evidence_builder::pool_richness_check(pool, policy)?;
        for pack in packs {
            Self::validate_chapter_requirements(pool, pack, catalog)?;
        }
        Ok(())
    }

    fn validate_chapter_requirements(
        pool: &InterpretiveEvidencePool,
        pack: &ChapterEvidencePack,
        catalog: &EvidenceCanonicalCatalog,
    ) -> Result<(), GenerationError> {
        for req in catalog.requirements_for_chapter(&pack.chapter_code) {
            if !req.required_if_available {
                continue;
            }
            let available = requirement_pool_match(pool, req);
            if available.len() < req.min_count as usize {
                continue;
            }
            let selected: Vec<_> = pack
                .core
                .iter()
                .chain(pack.supporting.iter())
                .chain(pack.nuance.iter())
                .filter(|e| available.iter().any(|a| a.fact_id == e.fact_id))
                .collect();
            if selected.len() < req.min_count as usize {
                match req.severity {
                    EvidenceRequirementSeverity::Blocking => {
                        return Err(Self::diversity_error(
                            format!(
                                "Requirement '{}' not satisfied for chapter '{}'",
                                req.requirement_code, pack.chapter_code
                            ),
                            serde_json::json!({
                                "requirement": req.requirement_code,
                                "chapter": pack.chapter_code,
                                "available_in_pool": available.len(),
                                "selected": selected.len(),
                            }),
                        ));
                    }
                    EvidenceRequirementSeverity::Warning => {}
                }
            }
        }
        Ok(())
    }

    pub fn validate_reading(
        product_code: &str,
        pool: &InterpretiveEvidencePool,
        chapters: &[ReadingChapter],
        packs: &[ChapterEvidencePack],
    ) -> Result<(), GenerationError> {
        if !is_premium_product(product_code) {
            return Ok(());
        }

        let mut all_basis: Vec<String> = Vec::new();
        for ch in chapters {
            for b in &ch.astro_basis {
                if let Some(id) = &b.fact_id {
                    all_basis.push(id.clone());
                }
            }
        }

        let only_trio = !all_basis.is_empty()
            && all_basis.iter().all(|id| {
                id.starts_with("placement:ascendant")
                    || id.starts_with("placement:sun")
                    || id.starts_with("placement:moon")
                    || id.starts_with("domain_score:")
            });

        if only_trio && pool.interpretive_evidence().count() > 3 {
            return Err(Self::diversity_error(
                "All chapters rely only on ascendant/sun/moon placements",
                serde_json::json!({ "rule": "big_three_recycle" }),
            ));
        }

        for ch in chapters {
            let pack = packs.iter().find(|p| p.chapter_code == ch.code);
            let Some(pack) = pack else { continue };
            for b in &ch.astro_basis {
                let Some(id) = &b.fact_id else { continue };
                if id.starts_with("domain_score:") {
                    continue;
                }
                if !pack.contains_fact_id(id) && !pool.fact_id_is_domain_score(id) {
                    // post-LLM basis invalid handled by AstroBasisValidator
                }
            }
        }

        if pool.pool_has_rulers() {
            let cites_ruler = chapters.iter().any(|ch| {
                ch.astro_basis.iter().any(|b| {
                    b.fact_id
                        .as_deref()
                        .is_some_and(|id| id.contains("ruler") || id.contains("house_ruler"))
                })
            });
            if !cites_ruler {
                tracing::warn!("premium reading generated without citing rulers though pool has them");
            }
        }

        Ok(())
    }

    fn diversity_error(message: impl Into<String>, details: serde_json::Value) -> GenerationError {
        GenerationError::with_details(
            GenerationErrorCode::PremiumEvidenceDiversityFailed,
            message,
            details,
        )
    }
}

pub fn compute_evidence_metrics(
    packs: &[ChapterEvidencePack],
    chapters: &[ReadingChapter],
) -> astral_llm_domain::EvidenceMetrics {
    let mut ids = std::collections::HashSet::new();
    let mut families = std::collections::HashSet::new();
    let mut chapters_with_non_placement = 0u32;
    let mut max_overlap = 0f32;

    for pack in packs {
        for e in pack.core.iter().chain(pack.supporting.iter()).chain(pack.nuance.iter()) {
            ids.insert(e.fact_id.clone());
            families.insert(e.family.as_str());
        }
        if pack.has_non_placement() {
            chapters_with_non_placement += 1;
        }
    }

    for i in 0..packs.len() {
        for j in (i + 1)..packs.len() {
            let a: std::collections::HashSet<_> =
                packs[i].core.iter().map(|e| e.fact_id.as_str()).collect();
            let b: std::collections::HashSet<_> =
                packs[j].core.iter().map(|e| e.fact_id.as_str()).collect();
            if !a.is_empty() && !b.is_empty() {
                let ratio = a.intersection(&b).count() as f32 / a.len().max(b.len()) as f32;
                max_overlap = max_overlap.max(ratio);
            }
        }
    }

    let domain_score_used = chapters.iter().any(|ch| {
        ch.astro_basis.iter().any(|b| {
            b.fact_id
                .as_deref()
                .is_some_and(|id| id.starts_with("domain_score:"))
        })
    });

    astral_llm_domain::EvidenceMetrics {
        total_unique_facts: ids.len() as u32,
        distinct_kind_families: families.len() as u32,
        max_core_overlap_ratio: max_overlap,
        domain_score_used_as_basis: domain_score_used,
        chapters_with_non_placement,
    }
}
