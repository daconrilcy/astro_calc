use astral_llm_domain::{
    generation_response::ReadingChapter,
    interpretive_evidence::{
        ChapterEvidencePack, EvidenceRequirementSeverity, InterpretiveEvidencePool,
        RequirementAuditEntry, RequirementAuditStatus, KIND_ASPECT,
    },
    GenerationError, GenerationErrorCode,
};
use astral_llm_infra::EvidenceCanonicalCatalog;

use crate::chapter_evidence_planner::requirement_pool_match;

pub struct EvidenceDiversityValidator;

impl EvidenceDiversityValidator {
    pub fn validate_packs(
        evidence_enabled: bool,
        pool: &InterpretiveEvidencePool,
        packs: &[ChapterEvidencePack],
        catalog: &EvidenceCanonicalCatalog,
        policy: &astral_llm_domain::PremiumEvidencePolicy,
    ) -> Result<Vec<RequirementAuditEntry>, GenerationError> {
        if !evidence_enabled {
            return Ok(Vec::new());
        }

        crate::interpretive_evidence_builder::pool_richness_check(pool, policy)?;

        let mut audit = Vec::new();
        for pack in packs {
            audit.extend(Self::validate_chapter_requirements(pool, pack, catalog)?);
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

        Ok(audit)
    }

    pub fn validate_packs_planned(
        evidence_enabled: bool,
        pool: &InterpretiveEvidencePool,
        packs: &[ChapterEvidencePack],
        catalog: &EvidenceCanonicalCatalog,
        policy: &astral_llm_domain::PremiumEvidencePolicy,
    ) -> Result<Vec<RequirementAuditEntry>, GenerationError> {
        if !evidence_enabled {
            return Ok(Vec::new());
        }
        crate::interpretive_evidence_builder::pool_richness_check(pool, policy)?;
        let mut audit = Vec::new();
        for pack in packs {
            audit.extend(Self::validate_chapter_requirements(pool, pack, catalog)?);
        }
        Self::log_requirement_audit(&audit);
        Ok(audit)
    }

    fn validate_chapter_requirements(
        pool: &InterpretiveEvidencePool,
        pack: &ChapterEvidencePack,
        catalog: &EvidenceCanonicalCatalog,
    ) -> Result<Vec<RequirementAuditEntry>, GenerationError> {
        let mut entries = Vec::new();
        for req in catalog.requirements_for_chapter(&pack.chapter_code) {
            if !req.required_if_available {
                continue;
            }
            let available = requirement_pool_match(pool, req);
            if available.len() < req.min_count as usize {
                let detail = format!(
                    "{} not found in pool (available={})",
                    req.requirement_code,
                    available.len()
                );
                tracing::info!(
                    requirement = %req.requirement_code,
                    chapter = %pack.chapter_code,
                    status = "requirement_not_applicable",
                    detail = %detail
                );
                entries.push(RequirementAuditEntry {
                    requirement_code: req.requirement_code.clone(),
                    chapter_code: pack.chapter_code.clone(),
                    status: RequirementAuditStatus::NotApplicable,
                    detail: Some(detail),
                });
                continue;
            }
            let selected: Vec<_> = pack
                .core
                .iter()
                .chain(pack.supporting.iter())
                .chain(pack.nuance.iter())
                .filter(|e| {
                    available.iter().any(|a| {
                        a.semantic_fact_key == e.semantic_fact_key || a.fact_id == e.fact_id
                    })
                })
                .collect();
            if selected.len() < req.min_count as usize {
                let detail = format!(
                    "{} exists in pool but not selected (pool={}, selected={})",
                    req.requirement_code,
                    available.len(),
                    selected.len()
                );
                match req.severity {
                    EvidenceRequirementSeverity::Blocking => {
                        tracing::warn!(
                            requirement = %req.requirement_code,
                            chapter = %pack.chapter_code,
                            status = "requirement_failed",
                            detail = %detail
                        );
                        entries.push(RequirementAuditEntry {
                            requirement_code: req.requirement_code.clone(),
                            chapter_code: pack.chapter_code.clone(),
                            status: RequirementAuditStatus::Failed,
                            detail: Some(detail.clone()),
                        });
                        return Err(Self::diversity_error(
                            format!(
                                "Requirement '{}' not satisfied for chapter '{}'",
                                req.requirement_code, pack.chapter_code
                            ),
                            serde_json::json!({
                                "requirement": req.requirement_code,
                                "chapter": pack.chapter_code,
                                "status": "requirement_failed",
                                "available_in_pool": available.len(),
                                "selected": selected.len(),
                                "detail": detail,
                            }),
                        ));
                    }
                    EvidenceRequirementSeverity::Warning => {
                        tracing::warn!(
                            requirement = %req.requirement_code,
                            chapter = %pack.chapter_code,
                            status = "requirement_failed",
                            detail = %detail
                        );
                        entries.push(RequirementAuditEntry {
                            requirement_code: req.requirement_code.clone(),
                            chapter_code: pack.chapter_code.clone(),
                            status: RequirementAuditStatus::Failed,
                            detail: Some(detail),
                        });
                    }
                }
            } else {
                tracing::info!(
                    requirement = %req.requirement_code,
                    chapter = %pack.chapter_code,
                    status = "requirement_applied",
                    selected = selected.len()
                );
                entries.push(RequirementAuditEntry {
                    requirement_code: req.requirement_code.clone(),
                    chapter_code: pack.chapter_code.clone(),
                    status: RequirementAuditStatus::Applied,
                    detail: None,
                });
            }
        }
        Ok(entries)
    }

    fn log_requirement_audit(audit: &[RequirementAuditEntry]) {
        for entry in audit {
            let status = match entry.status {
                RequirementAuditStatus::Applied => "requirement_applied",
                RequirementAuditStatus::Failed => "requirement_failed",
                RequirementAuditStatus::NotApplicable => "requirement_not_applicable",
            };
            tracing::info!(
                requirement = %entry.requirement_code,
                chapter = %entry.chapter_code,
                status,
                detail = ?entry.detail
            );
        }
    }

    pub fn validate_reading(
        evidence_enabled: bool,
        pool: &InterpretiveEvidencePool,
        chapters: &[ReadingChapter],
        packs: &[ChapterEvidencePack],
    ) -> Result<(), GenerationError> {
        if !evidence_enabled {
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
    requirement_audit: Vec<RequirementAuditEntry>,
) -> astral_llm_domain::EvidenceMetrics {
    let mut ids = std::collections::HashSet::new();
    let mut semantic_keys = std::collections::HashSet::new();
    let mut families = std::collections::HashSet::new();
    let mut chapters_with_non_placement = 0u32;
    let mut max_overlap = 0f32;

    for pack in packs {
        for e in pack.core.iter().chain(pack.supporting.iter()).chain(pack.nuance.iter()) {
            ids.insert(e.fact_id.clone());
            semantic_keys.insert(e.semantic_fact_key.clone());
            families.insert(e.family.as_str());
        }
        if pack.has_non_placement() {
            chapters_with_non_placement += 1;
        }
    }

    for i in 0..packs.len() {
        for j in (i + 1)..packs.len() {
            let a: std::collections::HashSet<_> = packs[i]
                .core
                .iter()
                .map(|e| e.semantic_fact_key.as_str())
                .collect();
            let b: std::collections::HashSet<_> = packs[j]
                .core
                .iter()
                .map(|e| e.semantic_fact_key.as_str())
                .collect();
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
        total_unique_semantic_keys: semantic_keys.len() as u32,
        distinct_kind_families: families.len() as u32,
        max_core_overlap_ratio: max_overlap,
        domain_score_used_as_basis: domain_score_used,
        chapters_with_non_placement,
        requirement_audit,
    }
}
