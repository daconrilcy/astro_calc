//! Tests i18n : directive langue dans le prompt, humanizer post-LLM.

use astral_llm_application::{
    astro_basis_role_normalizer::AstroBasisRoleNormalizer,
    astro_label_humanizer::AstroLabelHumanizer, writing_language::WritingLanguageDirective,
};
use astral_llm_domain::{
    astro_fact::{AstroFactKind, AstroFactUsage, NormalizedAstroFact, NormalizedAstroFacts},
    generation_response::{AstroBasisItem, ConfidenceLevel, ReadingChapter},
};
use astral_llm_infra::{
    bootstrap_aspect_type_labels, bootstrap_astro_object_labels,
    bootstrap_extra_object_sign_labels, bootstrap_writing_locales, bootstrap_zodiac_sign_labels,
    CanonicalCatalog,
};
use std::sync::Arc;

fn catalog() -> Arc<CanonicalCatalog> {
    let mut objects = bootstrap_astro_object_labels();
    let mut signs = bootstrap_zodiac_sign_labels();
    bootstrap_extra_object_sign_labels(&mut objects, &mut signs);
    Arc::new(CanonicalCatalog {
        writing_locales: bootstrap_writing_locales(),
        astro_object_labels: objects,
        zodiac_sign_labels: signs,
        aspect_type_labels: bootstrap_aspect_type_labels(),
        ..CanonicalCatalog::default()
    })
}

#[test]
fn writing_directive_german_in_prompt_block() {
    let block = WritingLanguageDirective::prompt_block(&catalog(), "de");
    assert!(block.contains("OUTPUT_LANGUAGE: de"));
}

#[test]
fn writing_directive_unknown_language_uses_generic_fallback() {
    let block = WritingLanguageDirective::prompt_block(&catalog(), "it");
    assert_eq!(
        block,
        "OUTPUT_LANGUAGE: it. Write title, body, summary fields, and human-readable astro_basis strings (factor, label) in language it. Never translate fact_ids."
    );
}

#[test]
fn public_abbreviation_rule_expands_french_angle_codes() {
    let rule = WritingLanguageDirective::public_abbreviation_rule("fr");
    assert!(rule.contains("PUBLIC_ASTRO_ABBREVIATIONS"));
    assert!(rule.contains("Milieu du Ciel"));
    assert!(rule.contains("au lieu de \"MC\""));
    assert!(rule.contains("Fond du Ciel"));
    assert!(rule.contains("fact_id"));
    assert!(rule.contains("signal_key"));
}

#[test]
fn humanizer_sets_french_factor_on_basis_item() {
    let cat = catalog();
    let h = AstroLabelHumanizer::new(cat.as_ref());
    let facts = NormalizedAstroFacts {
        contract_version: "natal_structured_v14".into(),
        facts: vec![NormalizedAstroFact {
            id: "signal:object_position:moon".into(),
            kind: AstroFactKind::PlanetPosition,
            kind_code: "placement".into(),
            usage: AstroFactUsage::InterpretiveBasis,
            label: "Moon in Pisces, house 4".into(),
            value: serde_json::json!({
                "evidence": { "object_code": "moon", "sign_code": "pisces", "house_number": 4 }
            }),
            interpretive_weight: None,
            domains: vec![],
        }],
    };
    let mut items = vec![AstroBasisItem {
        fact_id: Some("signal:object_position:moon".into()),
        label: Some("Moon in Pisces, house 4".into()),
        factor: "Moon in Pisces, house 4".into(),
        interpretive_role: "core".into(),
    }];
    h.enrich_chapter_astro_basis(&mut items, &facts, "fr");
    assert_eq!(
        items[0].label.as_deref(),
        Some("Lune en Poissons en maison 4")
    );
    assert_eq!(items[0].factor.as_str(), "Lune en Poissons en maison 4");
}

#[test]
fn enrich_replaces_unknown_label_from_evidence() {
    let cat = catalog();
    let h = AstroLabelHumanizer::new(cat.as_ref());
    let facts = NormalizedAstroFacts {
        contract_version: "natal_structured_v14".into(),
        facts: vec![NormalizedAstroFact {
            id: "signal:object_position:venus".into(),
            kind: AstroFactKind::PlanetPosition,
            kind_code: "placement".into(),
            usage: AstroFactUsage::InterpretiveBasis,
            label: "Venus in Aquarius, house 3".into(),
            value: serde_json::json!({
                "title": "Venus in Aquarius, house 3",
                "evidence": {
                    "object_code": "venus",
                    "sign_code": "aquarius",
                    "house_number": 3
                }
            }),
            interpretive_weight: None,
            domains: vec![],
        }],
    };
    let mut items = vec![AstroBasisItem {
        fact_id: Some("signal:object_position:venus".into()),
        label: Some("Vénus en Unknown".into()),
        factor: "Venus in Aquarius, house 3".into(),
        interpretive_role: "core".into(),
    }];
    h.enrich_chapter_astro_basis(&mut items, &facts, "fr");
    assert_eq!(
        items[0].label.as_deref(),
        Some("Vénus en Verseau en maison 3")
    );
    assert_eq!(items[0].factor.as_str(), "Vénus en Verseau en maison 3");
}

#[test]
fn enrich_overwrites_llm_paraphrase_factor_with_canonical_label() {
    let cat = catalog();
    let h = AstroLabelHumanizer::new(cat.as_ref());
    let facts = NormalizedAstroFacts {
        contract_version: "natal_structured_v14".into(),
        facts: vec![
            NormalizedAstroFact {
                id: "placement:mc:leo:house:10".into(),
                kind: AstroFactKind::Angle,
                kind_code: "angle".into(),
                usage: AstroFactUsage::InterpretiveBasis,
                label: "Midheaven in Leo".into(),
                value: serde_json::json!({}),
                interpretive_weight: None,
                domains: vec![],
            },
            NormalizedAstroFact {
                id: "placement:descendant:taurus:house:7".into(),
                kind: AstroFactKind::Angle,
                kind_code: "angle".into(),
                usage: AstroFactUsage::InterpretiveBasis,
                label: "Descendant in Taurus".into(),
                value: serde_json::json!({}),
                interpretive_weight: None,
                domains: vec![],
            },
        ],
    };
    let mut items = vec![
        AstroBasisItem {
            fact_id: Some("placement:mc:leo:house:10".into()),
            label: Some("Milieu du Ciel en Lion en maison 10".into()),
            factor: "Le Milieu du Ciel en Lion en maison 10".into(),
            interpretive_role: "core".into(),
        },
        AstroBasisItem {
            fact_id: Some("placement:descendant:taurus:house:7".into()),
            label: Some("Descendant en Taureau en maison 7".into()),
            factor: "Descendant en Taureau maison 7".into(),
            interpretive_role: "core".into(),
        },
    ];
    h.enrich_chapter_astro_basis(&mut items, &facts, "fr");
    assert_eq!(
        items[0].factor.as_str(),
        "Milieu du Ciel en Lion en maison 10"
    );
    assert_eq!(
        items[1].factor.as_str(),
        "Descendant en Taureau en maison 7"
    );
    assert_eq!(items[0].label.as_deref(), Some(items[0].factor.as_str()));
}

#[test]
fn role_normalizer_maps_free_text_to_core() {
    let mut chapter = ReadingChapter {
        code: "identity".into(),
        title: "T".into(),
        body: "B".into(),
        astro_basis: vec![AstroBasisItem {
            fact_id: Some("placement:sun:capricorn:house:2".into()),
            label: None,
            factor: "x".into(),
            interpretive_role: "Fondement principal".into(),
        }],
        confidence: ConfidenceLevel::High,
        safety_flags: vec![],
    };
    AstroBasisRoleNormalizer::normalize_chapter(&mut chapter, None);
    assert_eq!(chapter.astro_basis[0].interpretive_role, "core");
}
