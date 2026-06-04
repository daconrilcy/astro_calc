//! Gate coherence evidence pack ↔ astro_basis ↔ corps (Premium).

use astral_llm_application::ChapterEvidenceCoherence;
use astral_llm_domain::{
    generation_response::{AstroBasisItem, ConfidenceLevel, ReadingChapter},
    interpretive_evidence::{ChapterEvidencePack, EvidenceKindFamily, InterpretiveEvidence, SlotEligibility},
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

#[test]
fn premium_career_drift_jupiter_in_body_rejected() {
    let pack = ChapterEvidencePack {
        chapter_code: "career".into(),
        core: vec![InterpretiveEvidence {
            fact_id: "placement:mc:leo:house:10".into(),
            kind_code: "placement".into(),
            family: EvidenceKindFamily::Placement,
            label: String::new(),
            interpretive_hint: String::new(),
            chapter_affinity: vec![],
            weight: 1.0,
            slot_eligibility: SlotEligibility::default(),
            object_code: Some("mc".into()),
            house_number: Some(10),
        }],
        supporting: vec![],
        nuance: vec![],
        avoid_repeating: vec![],
    };
    let chapter = ReadingChapter {
        code: "career".into(),
        title: "Carriere".into(),
        body: "Jupiter en Cancer en maison 8 colore votre parcours professionnel.".into(),
        astro_basis: vec![AstroBasisItem {
            fact_id: Some("placement:mc:leo:house:10".into()),
            label: Some("Milieu du Ciel en Lion en maison 10".into()),
            factor: "Milieu du Ciel en Lion en maison 10".into(),
            interpretive_role: "core".into(),
        }],
        confidence: ConfidenceLevel::High,
        safety_flags: vec![],
    };
    assert!(ChapterEvidenceCoherence::validate_premium(&chapter, &pack, &catalog(), "fr").is_err());
}
