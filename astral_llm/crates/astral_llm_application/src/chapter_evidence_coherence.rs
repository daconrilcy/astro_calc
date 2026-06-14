//! Coherence pack evidence ↔ astro_basis ↔ corps du chapitre (Premium).

use std::collections::HashSet;

use astral_llm_domain::{
    generation_response::ReadingChapter, interpretive_evidence::ChapterEvidencePack,
    GenerationError, GenerationErrorCode,
};
use astral_llm_infra::CanonicalCatalog;

use crate::astro_label_humanizer::AstroLabelHumanizer;
use crate::evidence_fact_parse::object_codes_from_fact_id;

pub struct ChapterEvidenceCoherence;

#[derive(Debug, Clone)]
pub struct CoherenceViolation {
    pub missing_pack_fact_ids: Vec<String>,
    pub orphan_object_codes: Vec<String>,
}

impl ChapterEvidenceCoherence {
    pub fn validate_premium(
        chapter: &ReadingChapter,
        pack: &ChapterEvidencePack,
        catalog: &CanonicalCatalog,
        language: &str,
    ) -> Result<(), GenerationError> {
        let violation = Self::detect(chapter, pack, catalog, language);
        if violation.missing_pack_fact_ids.is_empty() && violation.orphan_object_codes.is_empty() {
            return Ok(());
        }
        Err(Self::to_error(chapter, &violation))
    }

    pub fn detect(
        chapter: &ReadingChapter,
        pack: &ChapterEvidencePack,
        catalog: &CanonicalCatalog,
        language: &str,
    ) -> CoherenceViolation {
        let missing_pack_fact_ids = Self::missing_pack_slots_in_basis(chapter, pack);
        let orphan_object_codes = Self::orphan_body_mentions(chapter, catalog, language);
        CoherenceViolation {
            missing_pack_fact_ids,
            orphan_object_codes,
        }
    }

    fn missing_pack_slots_in_basis(
        chapter: &ReadingChapter,
        pack: &ChapterEvidencePack,
    ) -> Vec<String> {
        let cited: HashSet<_> = chapter
            .astro_basis
            .iter()
            .filter_map(|b| b.fact_id.as_deref())
            .collect();
        pack.core
            .iter()
            .chain(pack.supporting.iter())
            .chain(pack.nuance.iter())
            .filter(|e| {
                !cited.contains(e.fact_id.as_str())
                    && !cited.iter().any(|id| {
                        crate::evidence_fact_parse::compute_semantic_fact_key(
                            id,
                            &serde_json::json!({}),
                            &std::collections::HashMap::new(),
                        ) == e.semantic_fact_key
                    })
            })
            .map(|e| e.fact_id.clone())
            .collect()
    }

    fn allowed_object_codes(chapter: &ReadingChapter) -> HashSet<String> {
        chapter
            .astro_basis
            .iter()
            .filter_map(|b| b.fact_id.as_deref())
            .flat_map(object_codes_from_fact_id)
            .map(|c| c.to_lowercase())
            .collect()
    }

    fn orphan_body_mentions(
        chapter: &ReadingChapter,
        catalog: &CanonicalCatalog,
        language: &str,
    ) -> Vec<String> {
        let locale = AstroLabelHumanizer::locale_key(language);
        let allowed = Self::allowed_object_codes(chapter);
        let mut orphans = Vec::new();
        let mut seen = HashSet::new();

        let mut labels: Vec<(String, String)> = catalog
            .astro_object_labels
            .iter()
            .filter(|((loc, _), _)| loc == locale)
            .map(|((_, code), label)| (code.clone(), label.clone()))
            .collect();
        labels.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

        for (code, label) in labels {
            if label.len() < 3 {
                continue;
            }
            let code_lc = code.to_lowercase();
            if allowed.contains(&code_lc) {
                continue;
            }
            if contains_word(&chapter.body, &label) && seen.insert(code_lc.clone()) {
                orphans.push(code);
            }
        }
        orphans
    }

    fn to_error(chapter: &ReadingChapter, v: &CoherenceViolation) -> GenerationError {
        let mut parts = Vec::new();
        if !v.missing_pack_fact_ids.is_empty() {
            parts.push(format!(
                "{} pack slot(s) missing from astro_basis",
                v.missing_pack_fact_ids.len()
            ));
        }
        if !v.orphan_object_codes.is_empty() {
            parts.push(format!(
                "body mentions {} object(s) not backed by astro_basis",
                v.orphan_object_codes.len()
            ));
        }
        GenerationError::with_details(
            GenerationErrorCode::AstroBasisInvalid,
            format!(
                "chapter '{}' evidence coherence: {}",
                chapter.code,
                parts.join("; ")
            ),
            serde_json::json!({
                "chapter": chapter.code,
                "missing_pack_fact_ids": v.missing_pack_fact_ids,
                "orphan_object_codes": v.orphan_object_codes,
                "basis_fact_ids": chapter.astro_basis.iter().filter_map(|b| b.fact_id.clone()).collect::<Vec<_>>(),
            }),
        )
    }
}

fn contains_word(haystack: &str, needle: &str) -> bool {
    let hay = haystack.to_lowercase();
    let ned = needle.to_lowercase();
    if ned.is_empty() {
        return false;
    }
    let mut start = 0;
    while let Some(rel) = hay[start..].find(&ned) {
        let idx = start + rel;
        let before_ok = idx == 0 || !hay.as_bytes()[idx - 1].is_ascii_alphabetic();
        let end = idx + ned.len();
        let after_ok = end >= hay.len() || !hay.as_bytes()[end].is_ascii_alphabetic();
        if before_ok && after_ok {
            return true;
        }
        start = idx + 1;
    }
    false
}
