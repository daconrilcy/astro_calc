//! Complete astro_basis avec les slots CORE/SUPPORTING du pack lorsque le LLM en omet.

use std::collections::HashSet;

use astral_llm_domain::{
    generation_response::{AstroBasisItem, ReadingChapter},
    interpretive_evidence::ChapterEvidencePack,
};

pub struct ChapterEvidenceBasisEnricher;

impl ChapterEvidenceBasisEnricher {
    pub fn enrich_missing_pack_slots(chapter: &mut ReadingChapter, pack: &ChapterEvidencePack) {
        let cited: HashSet<_> = chapter
            .astro_basis
            .iter()
            .filter_map(|b| b.fact_id.as_deref())
            .collect();

        let mut missing: Vec<_> = pack
            .core
            .iter()
            .map(|e| (e, "core"))
            .filter(|(e, _)| !already_cited(&cited, e))
            .map(|(e, role)| (e.fact_id.clone(), e.label.clone(), role.to_string()))
            .collect();

        missing.extend(
            pack.supporting
                .iter()
                .map(|e| (e, "supporting"))
                .filter(|(e, _)| !already_cited(&cited, e))
                .filter(|(e, _)| {
                    // Identity : n'injecter que les supporting non-Soleil (Soleil reserve a career).
                    pack.chapter_code != "identity"
                        || !e.fact_id.contains(":sun:") && !e.semantic_fact_key.contains("sun")
                })
                .map(|(e, role)| (e.fact_id.clone(), e.label.clone(), role.to_string())),
        );

        missing.extend(
            pack.nuance
                .iter()
                .map(|e| (e, "nuance"))
                .filter(|(e, _)| !already_cited(&cited, e))
                .map(|(e, role)| (e.fact_id.clone(), e.label.clone(), role.to_string())),
        );

        for (fact_id, factor, role) in missing {
            chapter.astro_basis.push(AstroBasisItem {
                fact_id: Some(fact_id),
                label: None,
                factor,
                interpretive_role: role,
            });
        }
    }
}

fn already_cited(
    cited: &HashSet<&str>,
    ev: &astral_llm_domain::interpretive_evidence::InterpretiveEvidence,
) -> bool {
    if cited.contains(ev.fact_id.as_str()) {
        return true;
    }
    cited.iter().any(|id| {
        crate::evidence_fact_parse::compute_semantic_fact_key(
            id,
            &serde_json::json!({}),
            &std::collections::HashMap::new(),
        ) == ev.semantic_fact_key
    })
}
