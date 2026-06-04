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

        let missing: Vec<_> = pack
            .core
            .iter()
            .map(|e| (e, "core"))
            .chain(pack.supporting.iter().map(|e| (e, "supporting")))
            .filter(|(e, _)| !cited.contains(e.fact_id.as_str()))
            .map(|(e, role)| {
                (
                    e.fact_id.clone(),
                    e.label.clone(),
                    role.to_string(),
                )
            })
            .collect();

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

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::{
        generation_response::{ConfidenceLevel, ReadingChapter},
        interpretive_evidence::{EvidenceKindFamily, InterpretiveEvidence, SlotEligibility},
    };

    fn evidence(id: &str, label: &str) -> InterpretiveEvidence {
        InterpretiveEvidence {
            fact_id: id.into(),
            kind_code: "signal".into(),
            family: EvidenceKindFamily::Other,
            label: label.into(),
            interpretive_hint: String::new(),
            chapter_affinity: vec![],
            weight: 1.0,
            slot_eligibility: SlotEligibility::default(),
            object_code: None,
            house_number: None,
        }
    }

    #[test]
    fn appends_missing_supporting_from_pack() {
        let pack = ChapterEvidencePack {
            chapter_code: "identity".into(),
            core: vec![evidence("placement:ascendant:scorpio:house:1", "Asc")],
            supporting: vec![evidence("signal:object_position:sun", "Sun")],
            nuance: vec![],
            avoid_repeating: vec![],
        };
        let mut chapter = ReadingChapter {
            code: "identity".into(),
            title: "t".into(),
            body: "body".into(),
            astro_basis: vec![AstroBasisItem {
                fact_id: Some("placement:ascendant:scorpio:house:1".into()),
                label: None,
                factor: "Asc".into(),
                interpretive_role: "core".into(),
            }],
            confidence: ConfidenceLevel::Medium,
            safety_flags: vec![],
        };
        ChapterEvidenceBasisEnricher::enrich_missing_pack_slots(&mut chapter, &pack);
        assert_eq!(chapter.astro_basis.len(), 2);
        assert!(
            chapter
                .astro_basis
                .iter()
                .any(|b| b.fact_id.as_deref() == Some("signal:object_position:sun"))
        );
    }

    #[test]
    fn enricher_fixes_identity_pack_gap_for_coherence() {
        use crate::chapter_evidence_coherence::ChapterEvidenceCoherence;
        use astral_llm_infra::{
            bootstrap_astro_object_labels, bootstrap_zodiac_sign_labels, CanonicalCatalog,
        };

        let catalog = CanonicalCatalog {
            astro_object_labels: bootstrap_astro_object_labels(),
            zodiac_sign_labels: bootstrap_zodiac_sign_labels(),
            ..CanonicalCatalog::default()
        };
        let pack = ChapterEvidencePack {
            chapter_code: "identity".into(),
            core: vec![evidence("placement:ascendant:scorpio:house:1", "Asc")],
            supporting: vec![evidence("signal:object_position:sun", "Sun")],
            nuance: vec![],
            avoid_repeating: vec![],
        };
        let mut chapter = ReadingChapter {
            code: "identity".into(),
            title: "t".into(),
            body: "Ascendant et opposition.".into(),
            astro_basis: vec![AstroBasisItem {
                fact_id: Some("placement:ascendant:scorpio:house:1".into()),
                label: None,
                factor: "Asc".into(),
                interpretive_role: "core".into(),
            }],
            confidence: ConfidenceLevel::Medium,
            safety_flags: vec![],
        };
        ChapterEvidenceBasisEnricher::enrich_missing_pack_slots(&mut chapter, &pack);
        let v = ChapterEvidenceCoherence::detect(&chapter, &pack, &catalog, "fr");
        assert!(v.missing_pack_fact_ids.is_empty());
        assert!(v.orphan_object_codes.is_empty());
    }
}
