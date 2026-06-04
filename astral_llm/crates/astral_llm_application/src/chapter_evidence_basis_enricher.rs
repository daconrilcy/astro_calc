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
                        || !e.fact_id.contains(":sun:")
                            && !e.semantic_fact_key.contains("sun")
                })
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

fn already_cited(cited: &HashSet<&str>, ev: &astral_llm_domain::interpretive_evidence::InterpretiveEvidence) -> bool {
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
            semantic_fact_key: id.into(),
            kind_code: "signal".into(),
            family: EvidenceKindFamily::Other,
            label: label.into(),
            interpretive_hint: String::new(),
            chapter_affinity: vec![],
            weight: 1.0,
            slot_eligibility: SlotEligibility::default(),
            object_code: None,
            sign_code: None,
            house_number: None,
        }
    }

    #[test]
    fn does_not_append_supporting_from_pack() {
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
        assert_eq!(chapter.astro_basis.len(), 1);
        assert!(
            !chapter
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
            core: vec![
                evidence("placement:ascendant:scorpio:house:1", "Asc"),
                evidence("ruler:angle:ascendant:mars", "Mars"),
            ],
            supporting: vec![],
            nuance: vec![],
            avoid_repeating: vec![],
        };
        let mut chapter = ReadingChapter {
            code: "identity".into(),
            title: "t".into(),
            body: "Ascendant et maître.".into(),
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
        assert!(
            chapter
                .astro_basis
                .iter()
                .any(|b| b.fact_id.as_deref() == Some("ruler:angle:ascendant:mars"))
        );
        assert!(v.orphan_object_codes.is_empty());
    }

    #[test]
    fn appends_supporting_for_career_coherence() {
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
            chapter_code: "career".into(),
            core: vec![
                evidence("placement:mc:leo:house:10", "MC"),
                evidence("ruler:angle:mc:sun", "Ruler"),
            ],
            supporting: vec![
                evidence("placement:saturn:capricorn:house:2", "Saturne"),
                evidence("signal:object_position:jupiter", "Jupiter"),
            ],
            nuance: vec![],
            avoid_repeating: vec![],
        };
        let mut chapter = ReadingChapter {
            code: "career".into(),
            title: "t".into(),
            body: "MC et Soleil.".into(),
            astro_basis: vec![
                AstroBasisItem {
                    fact_id: Some("placement:mc:leo:house:10".into()),
                    label: None,
                    factor: "MC".into(),
                    interpretive_role: "core".into(),
                },
                AstroBasisItem {
                    fact_id: Some("ruler:angle:mc:sun".into()),
                    label: None,
                    factor: "Ruler".into(),
                    interpretive_role: "core".into(),
                },
            ],
            confidence: ConfidenceLevel::Medium,
            safety_flags: vec![],
        };
        ChapterEvidenceBasisEnricher::enrich_missing_pack_slots(&mut chapter, &pack);
        let v = ChapterEvidenceCoherence::detect(&chapter, &pack, &catalog, "fr");
        assert!(v.missing_pack_fact_ids.is_empty());
        assert_eq!(chapter.astro_basis.len(), 4);
    }
}
