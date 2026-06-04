use std::collections::HashSet;

use astral_llm_domain::{
    chapter_orchestration::ReadingPlan,
    interpretive_evidence::{
        ChapterEvidencePack, ChapterEvidenceSlot, EvidenceRequirementSeverity, EvidenceSlotRole,
        InterpretiveEvidence, InterpretiveEvidencePool, PremiumEvidencePolicy, KIND_DOMAIN_SCORE,
    },
    GenerationError, GenerationErrorCode,
};
use astral_llm_infra::EvidenceCanonicalCatalog;

use crate::evidence_fact_parse::{
    aspect_involves_object, fact_involves_house, fact_involves_object,
};
use crate::prior_chapter_usage::PriorChapterUsage;

pub struct ChapterEvidencePlanner;

impl ChapterEvidencePlanner {
    pub fn plan_all(
        pool: &InterpretiveEvidencePool,
        plan: &ReadingPlan,
        catalog: &EvidenceCanonicalCatalog,
        policy: &PremiumEvidencePolicy,
    ) -> Result<Vec<ChapterEvidencePack>, GenerationError> {
        let mut packs = Vec::new();
        let mut prior_usage = PriorChapterUsage::default();

        for chapter in &plan.chapters {
            let pack = Self::plan_chapter(
                pool,
                &chapter.code,
                catalog,
                policy,
                &prior_usage,
            )?;
            prior_usage.record_pack(&pack);
            packs.push(pack);
        }

        Self::check_global_overlap(&packs, policy)?;
        Ok(packs)
    }

    fn plan_chapter(
        pool: &InterpretiveEvidencePool,
        chapter_code: &str,
        catalog: &EvidenceCanonicalCatalog,
        policy: &PremiumEvidencePolicy,
        prior_usage: &PriorChapterUsage,
    ) -> Result<ChapterEvidencePack, GenerationError> {
        let prior_avoid: HashSet<String> = prior_usage.planner_avoid_keys();
        let prior_avoid_ref: HashSet<&str> = prior_avoid.iter().map(String::as_str).collect();
        let slots = catalog.slots_for_chapter(chapter_code);
        let mut core = Vec::new();
        let mut supporting = Vec::new();
        let mut nuance = Vec::new();
        let mut assigned: HashSet<String> = HashSet::new();
        let mut assigned_semantic: HashSet<String> = HashSet::new();

        let candidates = pool.matching_for_chapter(chapter_code);

        for slot in &slots {
            let target = match slot.slot_role {
                EvidenceSlotRole::Core => &mut core,
                EvidenceSlotRole::Supporting => &mut supporting,
                EvidenceSlotRole::Nuance => &mut nuance,
            };
            let cap = match slot.slot_role {
                EvidenceSlotRole::Core => policy.max_core_evidence,
                EvidenceSlotRole::Supporting => policy.max_supporting_evidence,
                EvidenceSlotRole::Nuance => policy.max_nuance_evidence,
            };
            if target.len() as u8 >= cap {
                continue;
            }
            let max = slot.max_items.min(cap.saturating_sub(target.len() as u8));
            if max == 0 {
                continue;
            }
            let picked = pick_for_slot(
                chapter_code,
                &candidates,
                slot,
                max,
                &assigned,
                &assigned_semantic,
                &prior_avoid_ref,
                prior_usage,
                policy,
                catalog,
                pool,
            );
            for ev in picked {
                assigned.insert(ev.fact_id.clone());
                assigned_semantic.insert(ev.semantic_fact_key.clone());
                target.push(ev);
            }
        }

        fill_minimums(
            &candidates,
            chapter_code,
            policy,
            catalog,
            pool,
            prior_usage,
            &mut core,
            &mut supporting,
            &mut nuance,
            &mut assigned,
            &mut assigned_semantic,
            &prior_avoid_ref,
        )?;

        inject_blocking_requirements(
            pool,
            chapter_code,
            catalog,
            &candidates,
            policy,
            prior_usage,
            &mut core,
            &mut supporting,
            &mut nuance,
            &mut assigned,
            &mut assigned_semantic,
            &prior_avoid_ref,
        );

        let avoid_repeating = prior_usage.build_avoid_repeating(policy);

        let pack = ChapterEvidencePack {
            chapter_code: chapter_code.to_string(),
            core,
            supporting,
            nuance,
            avoid_repeating,
        };

        validate_pack_structure(
            &pack,
            pool,
            policy,
            chapter_code,
            &assigned,
            &assigned_semantic,
            &prior_avoid_ref,
            &candidates,
        )?;
        validate_no_avoid_in_active_slots(&pack)?;
        Ok(pack)
    }

    fn check_global_overlap(
        packs: &[ChapterEvidencePack],
        policy: &PremiumEvidencePolicy,
    ) -> Result<(), GenerationError> {
        for i in 0..packs.len() {
            for j in (i + 1)..packs.len() {
                let a: HashSet<_> = packs[i]
                    .core
                    .iter()
                    .map(|e| e.semantic_fact_key.as_str())
                    .collect();
                let b: HashSet<_> = packs[j]
                    .core
                    .iter()
                    .map(|e| e.semantic_fact_key.as_str())
                    .collect();
                if a.is_empty() || b.is_empty() {
                    continue;
                }
                let overlap = a.intersection(&b).count() as f32;
                let ratio = overlap / a.len().max(b.len()) as f32;
                if ratio > policy.max_core_overlap_ratio {
                    return Err(GenerationError::with_details(
                        GenerationErrorCode::PremiumEvidenceDiversityFailed,
                        "Chapter core evidence overlap exceeds policy limit",
                        serde_json::json!({
                            "chapter_a": packs[i].chapter_code,
                            "chapter_b": packs[j].chapter_code,
                            "overlap_ratio": ratio,
                            "max_allowed": policy.max_core_overlap_ratio,
                        }),
                    ));
                }
            }
        }
        Ok(())
    }
}

fn validate_no_avoid_in_active_slots(pack: &ChapterEvidencePack) -> Result<(), GenerationError> {
    let avoid: HashSet<_> = pack.avoid_repeating.iter().map(String::as_str).collect();
    for ev in pack
        .core
        .iter()
        .chain(pack.supporting.iter())
        .chain(pack.nuance.iter())
    {
        if avoid.contains(ev.semantic_fact_key.as_str()) {
            return Err(GenerationError::with_details(
                GenerationErrorCode::PremiumEvidenceDiversityFailed,
                format!(
                    "chapter '{}' pack lists semantic key in avoid_repeating and active slots",
                    pack.chapter_code
                ),
                serde_json::json!({
                    "chapter": pack.chapter_code,
                    "semantic_fact_key": ev.semantic_fact_key,
                    "fact_id": ev.fact_id,
                }),
            ));
        }
    }
    Ok(())
}

/// Exclusions par chapitre (matrice editorial) : le Soleil sort de identity (reserve a career).
fn chapter_excludes_candidate(chapter_code: &str, ev: &InterpretiveEvidence) -> bool {
    if chapter_code == "identity" {
        let sun_key = ev
            .object_code
            .as_deref()
            .is_some_and(|o| o == "sun")
            || ev.semantic_fact_key.contains(":sun:")
            || ev.fact_id.contains(":sun:");
        if sun_key {
            return true;
        }
    }
    if chapter_code == "relationships"
        && ev.kind_code == "house_ruler"
        && (ev.fact_id.contains(":mc:") || ev.fact_id.contains("ruler:angle:mc:"))
    {
        return true;
    }
    false
}

fn supporting_semantic_cap_blocks(
    prior_usage: &PriorChapterUsage,
    ev: &InterpretiveEvidence,
    chapter_code: &str,
    policy: &PremiumEvidencePolicy,
    catalog: &EvidenceCanonicalCatalog,
    pool: &InterpretiveEvidencePool,
) -> bool {
    if supporting_cap_exempt_for_chapter(ev, chapter_code, catalog, pool) {
        return false;
    }
    prior_usage.exceeds_supporting_semantic_cap(
        &ev.semantic_fact_key,
        policy.max_supporting_semantic_chapters,
    )
}

fn supporting_cap_exempt_for_chapter(
    ev: &InterpretiveEvidence,
    chapter_code: &str,
    catalog: &EvidenceCanonicalCatalog,
    pool: &InterpretiveEvidencePool,
) -> bool {
    if ev.kind_code != "house_ruler" {
        return false;
    }
    catalog.requirements_for_chapter(chapter_code).iter().any(|req| {
        req.severity == EvidenceRequirementSeverity::Blocking
            && requirement_pool_match(pool, req)
                .iter()
                .any(|m| m.semantic_fact_key == ev.semantic_fact_key)
    })
}

fn pick_for_slot<'a>(
    chapter_code: &str,
    candidates: &[&'a InterpretiveEvidence],
    slot: &ChapterEvidenceSlot,
    max: u8,
    assigned: &HashSet<String>,
    assigned_semantic: &HashSet<String>,
    prior_avoid: &HashSet<&str>,
    prior_usage: &PriorChapterUsage,
    policy: &PremiumEvidencePolicy,
    catalog: &EvidenceCanonicalCatalog,
    pool: &InterpretiveEvidencePool,
) -> Vec<InterpretiveEvidence> {
    let cap_supporting = slot.slot_role == EvidenceSlotRole::Supporting;
    let mut scored: Vec<_> = candidates
        .iter()
        .filter(|e| !assigned.contains(&e.fact_id))
        .filter(|e| !assigned_semantic.contains(&e.semantic_fact_key))
        .filter(|e| !prior_avoid.contains(e.semantic_fact_key.as_str()))
        .filter(|e| !chapter_excludes_candidate(chapter_code, e))
        .filter(|e| e.kind_code != KIND_DOMAIN_SCORE)
        .filter(|e| {
            !cap_supporting
                || !supporting_semantic_cap_blocks(
                    prior_usage, e, chapter_code, policy, catalog, pool,
                )
        })
        .filter(|e| matches_slot(e, slot))
        .map(|e| (*e, e.weight))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored
        .into_iter()
        .take(max as usize)
        .map(|(e, _)| (*e).clone())
        .collect()
}

fn matches_slot(ev: &InterpretiveEvidence, slot: &ChapterEvidenceSlot) -> bool {
    if let Some(ref k) = slot.kind_code {
        if ev.kind_code != *k && !(k == "placement" && ev.kind_code == "angle") {
            return false;
        }
    }
    if let Some(ref obj) = slot.object_code {
        if !fact_involves_object(&ev.fact_id, obj)
            && ev.object_code.as_deref() != Some(obj.as_str())
        {
            return false;
        }
    }
    if let Some(h) = slot.house_number {
        if !fact_involves_house(&ev.fact_id, &serde_json::json!({}), h)
            && ev.house_number != Some(h)
        {
            return false;
        }
    }
    true
}

struct PackPushContext<'a> {
    prior_usage: &'a PriorChapterUsage,
    chapter_code: &'a str,
    policy: &'a PremiumEvidencePolicy,
    catalog: &'a EvidenceCanonicalCatalog,
    pool: &'a InterpretiveEvidencePool,
}

fn inject_blocking_requirements(
    pool: &InterpretiveEvidencePool,
    chapter_code: &str,
    catalog: &EvidenceCanonicalCatalog,
    _candidates: &[&InterpretiveEvidence],
    policy: &PremiumEvidencePolicy,
    prior_usage: &PriorChapterUsage,
    core: &mut Vec<InterpretiveEvidence>,
    supporting: &mut Vec<InterpretiveEvidence>,
    nuance: &mut Vec<InterpretiveEvidence>,
    assigned: &mut HashSet<String>,
    assigned_semantic: &mut HashSet<String>,
    prior_avoid: &HashSet<&str>,
) {
    let ctx = PackPushContext {
        prior_usage,
        chapter_code,
        policy,
        catalog,
        pool,
    };
    for req in catalog.requirements_for_chapter(chapter_code) {
        if !req.required_if_available || req.severity != EvidenceRequirementSeverity::Blocking {
            continue;
        }
        let available = requirement_pool_match(pool, req);
        if available.len() < req.min_count as usize {
            continue;
        }
        let selected_count = core
            .iter()
            .chain(supporting.iter())
            .chain(nuance.iter())
            .filter(|e| available.iter().any(|a| a.semantic_fact_key == e.semantic_fact_key))
            .count();
        if selected_count >= req.min_count as usize {
            continue;
        }
        let pool: Vec<_> = available
            .into_iter()
            .filter(|e| !assigned.contains(&e.fact_id))
            .filter(|e| !assigned_semantic.contains(&e.semantic_fact_key))
            .filter(|e| !prior_avoid.contains(e.semantic_fact_key.as_str()))
            .collect();
        if let Some(ev) = best_by_weight(pool) {
            let _ = push_into_pack(
                ev,
                policy,
                core,
                supporting,
                nuance,
                assigned,
                assigned_semantic,
                &ctx,
            );
        }
    }
}

fn collect_families(
    core: &[InterpretiveEvidence],
    supporting: &[InterpretiveEvidence],
    nuance: &[InterpretiveEvidence],
) -> HashSet<String> {
    core.iter()
        .chain(supporting.iter())
        .chain(nuance.iter())
        .map(|e| e.family.as_str().to_string())
        .collect()
}

fn eligible_candidates<'a>(
    candidates: &[&'a InterpretiveEvidence],
    chapter: &str,
    assigned: &HashSet<String>,
    assigned_semantic: &HashSet<String>,
    prior_avoid: &HashSet<&str>,
    families: &HashSet<String>,
    require_new_family: bool,
) -> Vec<&'a InterpretiveEvidence> {
    candidates
        .iter()
        .copied()
        .filter(|e| !assigned.contains(&e.fact_id))
        .filter(|e| !assigned_semantic.contains(&e.semantic_fact_key))
        .filter(|e| !prior_avoid.contains(e.semantic_fact_key.as_str()))
        .filter(|e| !chapter_excludes_candidate(chapter, e))
        .filter(|e| e.kind_code != KIND_DOMAIN_SCORE)
        .filter(|e| {
            !require_new_family || !families.contains(&e.family.as_str().to_string())
        })
        .collect()
}

fn best_by_weight(candidates: Vec<&InterpretiveEvidence>) -> Option<InterpretiveEvidence> {
    candidates
        .into_iter()
        .max_by(|a, b| a.weight.partial_cmp(&b.weight).unwrap_or(std::cmp::Ordering::Equal))
        .map(|e| e.clone())
}

fn push_into_pack(
    ev: InterpretiveEvidence,
    policy: &PremiumEvidencePolicy,
    core: &mut Vec<InterpretiveEvidence>,
    supporting: &mut Vec<InterpretiveEvidence>,
    nuance: &mut Vec<InterpretiveEvidence>,
    assigned: &mut HashSet<String>,
    assigned_semantic: &mut HashSet<String>,
    ctx: &PackPushContext<'_>,
) -> bool {
    if supporting.len() < policy.max_supporting_evidence as usize
        && !supporting_semantic_cap_blocks(
            ctx.prior_usage,
            &ev,
            ctx.chapter_code,
            ctx.policy,
            ctx.catalog,
            ctx.pool,
        )
    {
        assigned.insert(ev.fact_id.clone());
        assigned_semantic.insert(ev.semantic_fact_key.clone());
        supporting.push(ev);
        return true;
    }
    if nuance.len() < policy.max_nuance_evidence as usize {
        assigned.insert(ev.fact_id.clone());
        assigned_semantic.insert(ev.semantic_fact_key.clone());
        nuance.push(ev);
        return true;
    }
    if core.len() < policy.max_core_evidence as usize {
        assigned.insert(ev.fact_id.clone());
        assigned_semantic.insert(ev.semantic_fact_key.clone());
        core.push(ev);
        return true;
    }
    false
}

/// Remplace le supporting le plus leger pour liberer une place (meme famille acceptee).
fn swap_supporting_for_any(
    new_ev: InterpretiveEvidence,
    supporting: &mut Vec<InterpretiveEvidence>,
    assigned: &mut HashSet<String>,
) -> bool {
    let Some((idx, _)) = supporting
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            a.weight
                .partial_cmp(&b.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    else {
        return false;
    };
    assigned.remove(&supporting[idx].fact_id);
    assigned.insert(new_ev.fact_id.clone());
    supporting[idx] = new_ev;
    true
}

/// Remplace le supporting le plus leger d'une famille deja representee pour faire place a une nouvelle famille.
fn swap_supporting_for_family_diversity(
    new_ev: InterpretiveEvidence,
    supporting: &mut Vec<InterpretiveEvidence>,
    assigned: &mut HashSet<String>,
    families: &HashSet<String>,
) -> bool {
    let new_family = new_ev.family.as_str().to_string();
    if families.contains(&new_family) {
        return false;
    }
    let Some((idx, _)) = supporting
        .iter()
        .enumerate()
        .filter(|(_, e)| families.contains(&e.family.as_str().to_string()))
        .min_by(|(_, a), (_, b)| {
            a.weight
                .partial_cmp(&b.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    else {
        return false;
    };
    assigned.remove(&supporting[idx].fact_id);
    assigned.insert(new_ev.fact_id.clone());
    supporting[idx] = new_ev;
    true
}

fn fill_minimums(
    candidates: &[&InterpretiveEvidence],
    chapter: &str,
    policy: &PremiumEvidencePolicy,
    catalog: &EvidenceCanonicalCatalog,
    pool: &InterpretiveEvidencePool,
    prior_usage: &PriorChapterUsage,
    core: &mut Vec<InterpretiveEvidence>,
    supporting: &mut Vec<InterpretiveEvidence>,
    nuance: &mut Vec<InterpretiveEvidence>,
    assigned: &mut HashSet<String>,
    assigned_semantic: &mut HashSet<String>,
    prior_avoid: &HashSet<&str>,
) -> Result<(), GenerationError> {
    let ctx = PackPushContext {
        prior_usage,
        chapter_code: chapter,
        policy,
        catalog,
        pool,
    };
    let cap_ok = |e: &&InterpretiveEvidence| {
        !supporting_semantic_cap_blocks(prior_usage, e, chapter, policy, catalog, pool)
    };
    let min_families = policy.min_distinct_kind_families as usize;

    const FAMILY_BOOST_KINDS: &[&str] = &[
        "aspect",
        "house_ruler",
        "essential_dignity",
        "accidental_dignity",
        "planetary_condition",
    ];

    loop {
        let families = collect_families(core, supporting, nuance);
        if families.len() >= min_families {
            break;
        }
        let pool: Vec<_> = eligible_candidates(
            candidates,
            chapter,
            assigned,
            assigned_semantic,
            prior_avoid,
            &families,
            true,
        )
        .into_iter()
        .filter(cap_ok)
        .collect();
        let mut injected = false;
        if let Some(ev) = best_by_weight(pool) {
            if push_into_pack(
                ev.clone(),
                policy,
                core,
                supporting,
                nuance,
                assigned,
                assigned_semantic,
                &ctx,
            ) || swap_supporting_for_family_diversity(ev, supporting, assigned, &families)
            {
                injected = true;
            }
        }
        if !injected {
            for kind in FAMILY_BOOST_KINDS {
                let pool: Vec<_> = candidates
                    .iter()
                    .copied()
                    .filter(|e| !assigned.contains(&e.fact_id))
                    .filter(|e| !assigned_semantic.contains(&e.semantic_fact_key))
                    .filter(|e| !prior_avoid.contains(e.semantic_fact_key.as_str()))
                    .filter(|e| !chapter_excludes_candidate(chapter, e))
                    .filter(|e| e.kind_code == *kind)
                    .filter(|e| !families.contains(&e.family.as_str().to_string()))
                    .filter(cap_ok)
                    .collect();
                if let Some(ev) = best_by_weight(pool) {
                    if push_into_pack(
                        ev.clone(),
                        policy,
                        core,
                        supporting,
                        nuance,
                        assigned,
                        assigned_semantic,
                        &ctx,
                    ) || swap_supporting_for_family_diversity(
                        ev,
                        supporting,
                        assigned,
                        &families,
                    )
                    {
                        injected = true;
                        break;
                    }
                }
            }
        }
        if !injected {
            break;
        }
    }

    while (core.len() + supporting.len() + nuance.len()) < policy.min_evidence_per_chapter as usize {
        let families = collect_families(core, supporting, nuance);
        let need_family = families.len() < min_families;
        let pool: Vec<_> = eligible_candidates(
            candidates,
            chapter,
            assigned,
            assigned_semantic,
            prior_avoid,
            &families,
            need_family,
        )
        .into_iter()
        .filter(cap_ok)
        .collect();
        let Some(ev) = best_by_weight(pool) else {
            break;
        };
        if push_into_pack(
            ev.clone(),
            policy,
            core,
            supporting,
            nuance,
            assigned,
            assigned_semantic,
            &ctx,
        ) {
            continue;
        }
        if swap_supporting_for_any(ev, supporting, assigned) {
            continue;
        }
        break;
    }

    if policy.min_non_placement_if_available > 0 {
        let families = collect_families(core, supporting, nuance);
        let has_non_placement = core
            .iter()
            .chain(supporting.iter())
            .chain(nuance.iter())
            .any(|e| e.family.counts_as_non_placement());
        if !has_non_placement {
            let pool: Vec<_> = candidates
                .iter()
                .copied()
                .filter(|e| !assigned.contains(&e.fact_id))
                .filter(|e| !assigned_semantic.contains(&e.semantic_fact_key))
                .filter(|e| !prior_avoid.contains(e.semantic_fact_key.as_str()))
                .filter(|e| e.kind_code != KIND_DOMAIN_SCORE)
                .filter(|e| e.family.counts_as_non_placement())
                .filter(cap_ok)
                .collect();
            if let Some(ev) = best_by_weight(pool) {
                if !push_into_pack(
                    ev.clone(),
                    policy,
                    core,
                    supporting,
                    nuance,
                    assigned,
                    assigned_semantic,
                    &ctx,
                ) {
                    let _ = swap_supporting_for_family_diversity(ev, supporting, assigned, &families);
                }
            }
        }
    }

    Ok(())
}

fn count_eligible_for_chapter(
    pool: &InterpretiveEvidencePool,
    chapter_code: &str,
    assigned: &HashSet<String>,
    assigned_semantic: &HashSet<String>,
    prior_avoid: &HashSet<&str>,
) -> usize {
    pool.matching_for_chapter(chapter_code)
        .into_iter()
        .filter(|e| !assigned.contains(&e.fact_id))
        .filter(|e| !assigned_semantic.contains(&e.semantic_fact_key))
        .filter(|e| !prior_avoid.contains(e.semantic_fact_key.as_str()))
        .filter(|e| e.kind_code != KIND_DOMAIN_SCORE)
        .count()
}

fn distinct_families_available(
    candidates: &[&InterpretiveEvidence],
    assigned: &HashSet<String>,
    assigned_semantic: &HashSet<String>,
    prior_avoid: &HashSet<&str>,
) -> usize {
    let mut families = HashSet::new();
    for ev in candidates {
        if assigned.contains(&ev.fact_id)
            || assigned_semantic.contains(&ev.semantic_fact_key)
            || prior_avoid.contains(ev.semantic_fact_key.as_str())
        {
            continue;
        }
        if ev.kind_code == KIND_DOMAIN_SCORE {
            continue;
        }
        families.insert(ev.family.as_str().to_string());
    }
    families.len()
}

fn validate_pack_structure(
    pack: &ChapterEvidencePack,
    pool: &InterpretiveEvidencePool,
    policy: &PremiumEvidencePolicy,
    chapter_code: &str,
    assigned: &HashSet<String>,
    assigned_semantic: &HashSet<String>,
    prior_avoid: &HashSet<&str>,
    candidates: &[&InterpretiveEvidence],
) -> Result<(), GenerationError> {
    if !pool.is_rich_enough_for_premium(policy.min_evidence_per_chapter) {
        return Ok(());
    }

    let min_count = policy.min_evidence_per_chapter as usize;
    let eligible_left = count_eligible_for_chapter(
        pool,
        chapter_code,
        assigned,
        assigned_semantic,
        prior_avoid,
    );
    let families_available =
        distinct_families_available(candidates, assigned, assigned_semantic, prior_avoid).max(1);
    let min_families = policy
        .min_distinct_kind_families
        .min(families_available as u8)
        .max(1) as usize;

    if pack.total_count() < min_count {
        let families_ok = pack.distinct_families() >= min_families;
        let near_min = pack.total_count() + 1 >= min_count;
        if families_ok && near_min {
            tracing::warn!(
                chapter = %pack.chapter_code,
                count = pack.total_count(),
                min_required = min_count,
                eligible_remaining = eligible_left,
                families = pack.distinct_families(),
                min_families,
                "chapter evidence one below minimum after prior-chapter exclusions; continuing"
            );
        } else {
            return Err(GenerationError::with_details(
                GenerationErrorCode::PremiumEvidenceDiversityFailed,
                format!(
                    "chapter '{}' could not reach minimum evidence count without weak filler",
                    pack.chapter_code
                ),
                serde_json::json!({
                    "chapter": pack.chapter_code,
                    "count": pack.total_count(),
                    "min_required": policy.min_evidence_per_chapter,
                    "eligible_remaining": eligible_left,
                    "families": pack.distinct_families(),
                    "min_families": min_families,
                }),
            ));
        }
    }

    if pack.distinct_families() < min_families {
        return Err(GenerationError::with_details(
            GenerationErrorCode::PremiumEvidenceDiversityFailed,
            format!("chapter '{}' lacks distinct evidence kind families", pack.chapter_code),
            serde_json::json!({
                "chapter": pack.chapter_code,
                "families": pack.distinct_families(),
                "min_required": min_families,
                "families_available": families_available,
            }),
        ));
    }

    if pool.pool_has_non_placement()
        && policy.min_non_placement_if_available > 0
        && !pack.has_non_placement()
    {
        let non_placement_available = candidates.iter().any(|e| {
            !assigned.contains(&e.fact_id)
                && !assigned_semantic.contains(&e.semantic_fact_key)
                && !prior_avoid.contains(e.semantic_fact_key.as_str())
                && e.kind_code != KIND_DOMAIN_SCORE
                && e.family.counts_as_non_placement()
        });
        if non_placement_available {
            return Err(GenerationError::with_details(
                GenerationErrorCode::PremiumEvidenceDiversityFailed,
                format!(
                    "chapter '{}' missing non-placement evidence while pool has some",
                    pack.chapter_code
                ),
                serde_json::json!({ "chapter": pack.chapter_code }),
            ));
        }
        tracing::warn!(
            chapter = %pack.chapter_code,
            "non-placement evidence unavailable for chapter after prior exclusions; continuing"
        );
    }

    Ok(())
}

pub fn pack_for_chapter<'a>(
    packs: &'a [ChapterEvidencePack],
    chapter_code: &str,
) -> Option<&'a ChapterEvidencePack> {
    packs.iter().find(|p| p.chapter_code == chapter_code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::chapter_orchestration::ReadingPlanChapter;

    fn pack_has_semantic(pack: &ChapterEvidencePack, key: &str) -> bool {
        pack.contains_semantic_key(key)
    }

    #[test]
    fn relationships_pack_has_two_families_after_prior_chapters() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tests/golden/natal_payload_v13_paris_1990.json");
        let data: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let payload = astral_llm_domain::AstroCalculationPayload {
            contract_version: "natal_structured_v13".into(),
            chart_type: "natal".into(),
            data,
        };
        let mut catalog = astral_llm_infra::CanonicalCatalog::default();
        catalog.evidence = astral_llm_infra::bootstrap_evidence_catalog();
        catalog.astro_object_labels = astral_llm_infra::bootstrap_astro_object_labels();
        catalog.zodiac_sign_labels = astral_llm_infra::bootstrap_zodiac_sign_labels();
        let facts = crate::AstroPayloadNormalizer::normalize(
            &payload,
            &astral_llm_domain::PrivacyPolicy::default(),
            &catalog,
            "fr",
        )
        .unwrap();
        let pool =
            crate::InterpretiveEvidenceBuilder::build(&facts, &catalog.evidence).unwrap();
        let policy = catalog.evidence.premium_policy.clone();
        let plan = astral_llm_domain::chapter_orchestration::ReadingPlan {
            product_code: "natal_prompter".into(),
            domain_count: 5,
            selected_domains: vec![
                "identity".into(),
                "emotional_life".into(),
                "relationships".into(),
                "career".into(),
                "growth_path".into(),
            ],
            chapters: ["identity", "emotional_life", "relationships", "career", "growth_path"]
                .into_iter()
                .map(|code| ReadingPlanChapter {
                    code: code.into(),
                    title: code.into(),
                    min_words: 40,
                    target_words: 200,
                    max_words: 500,
                })
                .collect(),
        };
        let packs = ChapterEvidencePlanner::plan_all(&pool, &plan, &catalog.evidence, &policy)
            .expect("plan all five chapters");
        let rel = packs
            .iter()
            .find(|p| p.chapter_code == "relationships")
            .expect("relationships pack");
        assert!(
            rel.distinct_families() >= policy.min_distinct_kind_families as usize,
            "families={}",
            rel.distinct_families()
        );
        assert!(
            rel.total_count() >= policy.min_evidence_per_chapter as usize
                || rel.total_count() + 1 >= policy.min_evidence_per_chapter as usize,
            "count={} ids={:?}",
            rel.total_count(),
            rel.all_fact_ids()
        );
        let growth = packs.iter().find(|p| p.chapter_code == "growth_path").unwrap();
        assert!(
            growth.total_count() >= 2,
            "growth_path count={} ids={:?}",
            growth.total_count(),
            growth.all_fact_ids()
        );
        for pack in &packs {
            for avoid in &pack.avoid_repeating {
                assert!(
                    !pack_has_semantic(pack, avoid),
                    "{} must not include avoid semantic key {}",
                    pack.chapter_code,
                    avoid
                );
            }
        }
    }
}

pub fn requirement_pool_match<'a>(
    pool: &'a InterpretiveEvidencePool,
    req: &'a astral_llm_domain::EvidenceRequirement,
) -> Vec<&'a InterpretiveEvidence> {
    pool
        .matching_for_chapter(&req.chapter_code)
        .into_iter()
        .filter(|e| evidence_matches_requirement(e, req))
        .collect()
}

fn evidence_matches_requirement(
    e: &InterpretiveEvidence,
    req: &astral_llm_domain::EvidenceRequirement,
) -> bool {
    if !req.accepted_kind_codes.is_empty()
        && !req.accepted_kind_codes.iter().any(|k| k == &e.kind_code)
    {
        return false;
    }
    if !req.accepted_object_codes.is_empty() {
        let obj_match = req.accepted_object_codes.iter().any(|o| {
            crate::evidence_fact_parse::matches_requirement_object(
                &e.fact_id,
                e.object_code.as_deref(),
                o,
            ) || aspect_involves_object(&e.fact_id, &e.label, o)
        });
        if !obj_match {
            return false;
        }
    }
    if !req.accepted_house_numbers.is_empty() {
        let h_match = req.accepted_house_numbers.iter().any(|h| {
            e.house_number == Some(*h)
                || fact_involves_house(&e.fact_id, &serde_json::json!({}), *h)
                || e.fact_id.contains(&format!("house_{h}"))
                || e.fact_id.contains(&format!(":house:{h}"))
        });
        if !h_match {
            return false;
        }
    }
    true
}
