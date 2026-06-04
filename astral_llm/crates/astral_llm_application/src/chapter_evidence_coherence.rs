//! Coherence pack evidence ↔ astro_basis ↔ corps du chapitre (Premium).

use std::collections::HashSet;

use astral_llm_domain::{
    generation_response::ReadingChapter,
    interpretive_evidence::ChapterEvidencePack,
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
        let orphan_object_codes =
            Self::orphan_body_mentions(chapter, catalog, language);
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
            .filter(|e| !cited.contains(e.fact_id.as_str()))
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

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::{
        generation_response::{AstroBasisItem, ConfidenceLevel, ReadingChapter},
        interpretive_evidence::{EvidenceKindFamily, InterpretiveEvidence, SlotEligibility},
    };
    use astral_llm_infra::{
        bootstrap_astro_object_labels, bootstrap_zodiac_sign_labels, CanonicalCatalog,
    };

    fn catalog() -> CanonicalCatalog {
        CanonicalCatalog {
            astro_object_labels: bootstrap_astro_object_labels(),
            zodiac_sign_labels: bootstrap_zodiac_sign_labels(),
            ..CanonicalCatalog::default()
        }
    }

    fn evidence(id: &str) -> InterpretiveEvidence {
        InterpretiveEvidence {
            fact_id: id.into(),
            kind_code: "placement".into(),
            family: EvidenceKindFamily::Placement,
            label: id.into(),
            interpretive_hint: String::new(),
            chapter_affinity: vec![],
            weight: 1.0,
            slot_eligibility: SlotEligibility::default(),
            object_code: None,
            house_number: None,
        }
    }

    fn pack_with_core(id: &str) -> ChapterEvidencePack {
        ChapterEvidencePack {
            chapter_code: "career".into(),
            core: vec![evidence(id)],
            supporting: vec![],
            nuance: vec![],
            avoid_repeating: vec![],
        }
    }

    #[test]
    fn detects_missing_pack_core_in_basis() {
        let pack = pack_with_core("placement:jupiter:cancer:house:8");
        let chapter = ReadingChapter {
            code: "career".into(),
            title: "t".into(),
            body: "Texte sur le MC.".into(),
            astro_basis: vec![AstroBasisItem {
                fact_id: Some("placement:mc:leo:house:10".into()),
                label: None,
                factor: "mc".into(),
                interpretive_role: "core".into(),
            }],
            confidence: ConfidenceLevel::Medium,
            safety_flags: vec![],
        };
        let v = ChapterEvidenceCoherence::detect(&chapter, &pack, &catalog(), "fr");
        assert!(v
            .missing_pack_fact_ids
            .contains(&"placement:jupiter:cancer:house:8".to_string()));
    }

    #[test]
    fn detects_jupiter_in_body_without_basis() {
        let pack = pack_with_core("placement:mc:leo:house:10");
        let chapter = ReadingChapter {
            code: "career".into(),
            title: "t".into(),
            body: "Jupiter en Cancer en maison 8 colore votre vocation.".into(),
            astro_basis: vec![AstroBasisItem {
                fact_id: Some("placement:mc:leo:house:10".into()),
                label: None,
                factor: "mc".into(),
                interpretive_role: "core".into(),
            }],
            confidence: ConfidenceLevel::Medium,
            safety_flags: vec![],
        };
        let v = ChapterEvidenceCoherence::detect(&chapter, &pack, &catalog(), "fr");
        assert!(v.orphan_object_codes.contains(&"jupiter".to_string()));
    }

    #[test]
    fn allows_ascendant_in_body_when_signal_angle_in_basis() {
        let pack = ChapterEvidencePack {
            chapter_code: "emotional_life".into(),
            core: vec![evidence("signal:object_position:moon")],
            supporting: vec![],
            nuance: vec![],
            avoid_repeating: vec![],
        };
        let chapter = ReadingChapter {
            code: "emotional_life".into(),
            title: "t".into(),
            body: "L'Ascendant en Scorpion colore votre sensibilite.".into(),
            astro_basis: vec![
                AstroBasisItem {
                    fact_id: Some("signal:object_position:moon".into()),
                    label: None,
                    factor: "Lune".into(),
                    interpretive_role: "core".into(),
                },
                AstroBasisItem {
                    fact_id: Some("signal:angle:ascendant:sign:scorpio".into()),
                    label: None,
                    factor: "Ascendant en Scorpion".into(),
                    interpretive_role: "supporting".into(),
                },
            ],
            confidence: ConfidenceLevel::Medium,
            safety_flags: vec![],
        };
        let v = ChapterEvidenceCoherence::detect(&chapter, &pack, &catalog(), "fr");
        assert!(!v.orphan_object_codes.contains(&"ascendant".to_string()));
    }

    #[test]
    fn accepts_body_when_jupiter_in_basis() {
        let pack = pack_with_core("placement:mc:leo:house:10");
        let chapter = ReadingChapter {
            code: "career".into(),
            title: "t".into(),
            body: "Jupiter en Cancer en maison 8 ouvre des perspectives.".into(),
            astro_basis: vec![AstroBasisItem {
                fact_id: Some("placement:jupiter:cancer:house:8".into()),
                label: None,
                factor: "j".into(),
                interpretive_role: "supporting".into(),
            }],
            confidence: ConfidenceLevel::Medium,
            safety_flags: vec![],
        };
        let v = ChapterEvidenceCoherence::detect(&chapter, &pack, &catalog(), "fr");
        assert!(v.orphan_object_codes.is_empty());
    }
}
