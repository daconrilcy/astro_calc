//! Tests couche evidence Premium : pool, packs, rejet payload minimal.

use std::collections::HashSet;

use astral_llm_application::{
    pool_richness_check, AstroPayloadNormalizer, ChapterEvidencePlanner,
    InterpretiveEvidenceBuilder, ReadingPlanBuilder,
};
use astral_llm_domain::{
    astro_fact::NormalizedAstroFacts,
    astrologer_profile::{JargonLevel, ToneProfile, WordingStyle},
    engine_params::EngineParams,
    output_contract::{ChapterContract, GenerationMode, OutputFormat},
    provider::ProviderKind,
    AstroCalculationPayload, AstrologerProfile, AudienceLevel, GenerateReadingRequest,
    GenerationErrorCode, PrivacyPolicy, ProductContext, ResponseContract,
};
use astral_llm_infra::{
    bootstrap_astro_object_labels, bootstrap_evidence_catalog, bootstrap_zodiac_sign_labels,
    CanonicalCatalog,
};

fn catalog_with_evidence() -> CanonicalCatalog {
    let mut c = CanonicalCatalog::default();
    c.evidence = bootstrap_evidence_catalog();
    c.astro_object_labels = bootstrap_astro_object_labels();
    c.zodiac_sign_labels = bootstrap_zodiac_sign_labels();
    c
}

fn minimal_payload() -> AstroCalculationPayload {
    AstroCalculationPayload {
        contract_version: "natal_structured_v13".into(),
        chart_type: "natal".into(),
        data: serde_json::json!({
            "domain_scores": {
                "identity": 0.85,
                "emotional_life": 0.78,
                "relationships": 0.72,
                "career": 0.55,
                "growth_path": 0.6
            },
            "planets": {
                "sun": { "house": 2, "sign": "capricorn" },
                "moon": { "house": 4, "sign": "pisces" },
                "ascendant": { "house": 1, "sign": "scorpio" }
            }
        }),
    }
}

fn rich_payload_from_golden() -> AstroCalculationPayload {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/golden/natal_payload_v13_paris_1990.json");
    let raw = std::fs::read_to_string(&path).expect("golden payload");
    let data: serde_json::Value = serde_json::from_str(&raw).expect("parse golden");
    AstroCalculationPayload {
        contract_version: "natal_structured_v13".into(),
        chart_type: "natal".into(),
        data,
    }
}

fn normalize(payload: &AstroCalculationPayload) -> NormalizedAstroFacts {
    let catalog = catalog_with_evidence();
    AstroPayloadNormalizer::normalize(payload, &PrivacyPolicy::default(), &catalog, "fr")
        .expect("normalize")
}

fn premium_request(payload: AstroCalculationPayload) -> GenerateReadingRequest {
    GenerateReadingRequest {
        request_id: Some("test".into()),
        idempotency_key: None,
        product_context: ProductContext {
            product_code: "natal_premium".into(),
            user_language: "fr".into(),
            audience_level: AudienceLevel::Beginner,
        },
        astro_result: payload,
        astrologer_profile: AstrologerProfile {
            profile_id: None,
            name: None,
            tone: ToneProfile::Warm,
            jargon_level: JargonLevel::Beginner,
            wording_style: WordingStyle::Clear,
            preferred_domains: vec![],
            forbidden_wording: vec![],
            custom_instructions: None,
        },
        engine: EngineParams {
            provider: Some(ProviderKind::Fake),
            model: Some("fake-model".into()),
            reasoning_effort: None,
            temperature: Some(0.4),
            max_output_tokens: Some(2000),
            domain_count: Some(5),
            allow_fallback: false,
            timeout_ms: Some(30_000),
        },
        response_contract: ResponseContract {
            output_schema_version: "natal_reading_v1".into(),
            generation_mode: GenerationMode::ChapterOrchestrated,
            format: OutputFormat::StructuredJson,
            chapters: vec![
                ChapterContract {
                    code: "identity".into(),
                    title: "Identite".into(),
                    min_words: Some(40),
                    max_words: Some(500),
                    target_tokens: None,
                    required_fields: vec![],
                },
                ChapterContract {
                    code: "emotional_life".into(),
                    title: "Emotions".into(),
                    min_words: Some(40),
                    max_words: Some(500),
                    target_tokens: None,
                    required_fields: vec![],
                },
                ChapterContract {
                    code: "relationships".into(),
                    title: "Relations".into(),
                    min_words: Some(40),
                    max_words: Some(500),
                    target_tokens: None,
                    required_fields: vec![],
                },
                ChapterContract {
                    code: "career".into(),
                    title: "Carriere".into(),
                    min_words: Some(40),
                    max_words: Some(500),
                    target_tokens: None,
                    required_fields: vec![],
                },
                ChapterContract {
                    code: "growth_path".into(),
                    title: "Croissance".into(),
                    min_words: Some(40),
                    max_words: Some(500),
                    target_tokens: None,
                    required_fields: vec![],
                },
            ],
            global_max_tokens: None,
            include_astro_sources: false,
            include_legal_disclaimer: true,
        },
        safety_policy: None,
    }
}

#[test]
fn premium_minimal_pool_fails_richness() {
    let facts = normalize(&minimal_payload());
    let pool = InterpretiveEvidenceBuilder::build(&facts, &catalog_with_evidence().evidence)
        .expect("build pool");
    let policy = catalog_with_evidence().evidence.premium_policy;
    let err = pool_richness_check(&pool, &policy).unwrap_err();
    assert_eq!(
        err.detail().code,
        GenerationErrorCode::PremiumEvidenceDiversityFailed
    );
}

#[test]
fn premium_rich_pool_plans_distinct_chapters() {
    let payload = rich_payload_from_golden();
    let facts = normalize(&payload);
    let catalog = catalog_with_evidence();
    let pool =
        InterpretiveEvidenceBuilder::build(&facts, &catalog.evidence).expect("build pool");
    let policy = catalog.evidence.premium_policy.clone();
    pool_richness_check(&pool, &policy).expect("rich enough");

    let request = premium_request(payload);
    let domains = vec![
        "identity".into(),
        "emotional_life".into(),
        "relationships".into(),
        "career".into(),
        "growth_path".into(),
    ];
    let plan = ReadingPlanBuilder::build(&request, &domains);
    let packs = ChapterEvidencePlanner::plan_all(&pool, &plan, &catalog.evidence, &policy)
        .expect("plan");
    assert_eq!(packs.len(), 5);
    let core_sets: Vec<HashSet<_>> = packs
        .iter()
        .map(|p| p.core.iter().map(|e| e.fact_id.as_str()).collect())
        .collect();
    let all_same = core_sets.windows(2).all(|w| w[0] == w[1]);
    assert!(!all_same, "core fact sets should differ across chapters");
}

fn pack_contains_fact(pack: &astral_llm_domain::ChapterEvidencePack, fact_id: &str) -> bool {
    pack.core
        .iter()
        .chain(pack.supporting.iter())
        .chain(pack.nuance.iter())
        .any(|e| e.fact_id == fact_id)
}

#[test]
fn later_chapters_exclude_prior_chapter_fact_ids_from_pack() {
    let payload = rich_payload_from_golden();
    let facts = normalize(&payload);
    let catalog = catalog_with_evidence();
    let pool =
        InterpretiveEvidenceBuilder::build(&facts, &catalog.evidence).expect("build pool");
    let policy = catalog.evidence.premium_policy.clone();
    let request = premium_request(payload);
    let plan = ReadingPlanBuilder::build(
        &request,
        &[
            "identity".into(),
            "emotional_life".into(),
            "relationships".into(),
            "career".into(),
            "growth_path".into(),
        ],
    );
    let packs = ChapterEvidencePlanner::plan_all(&pool, &plan, &catalog.evidence, &policy)
        .expect("plan");

    let emotional = packs
        .iter()
        .find(|p| p.chapter_code == "emotional_life")
        .expect("emotional");
    let moon_id = "signal:object_position:moon";
    assert!(pack_contains_fact(emotional, moon_id));

    for code in ["relationships", "career", "growth_path"] {
        let pack = packs.iter().find(|p| p.chapter_code == code).expect(code);
        assert!(
            !pack_contains_fact(pack, moon_id),
            "{code} must not repeat moon from emotional_life"
        );
        for avoid in &pack.avoid_repeating {
            assert!(
                !pack_contains_fact(pack, avoid),
                "{code} pack must not include avoid_repeating fact {avoid}"
            );
        }
    }

    if pack_contains_fact(emotional, "signal:aspect:jupiter:uranus:opposition") {
        let career = packs.iter().find(|p| p.chapter_code == "career").unwrap();
        assert!(
            !pack_contains_fact(career, "signal:aspect:jupiter:uranus:opposition"),
            "career must not repeat jupiter/uranus aspect already core in emotional_life"
        );
    }
}

#[test]
fn prompt_pack_labels_localized_for_fr() {
    let facts = normalize(&rich_payload_from_golden());
    let catalog = catalog_with_evidence();
    let pool =
        InterpretiveEvidenceBuilder::build(&facts, &catalog.evidence).expect("build pool");
    let request = premium_request(rich_payload_from_golden());
    let plan = ReadingPlanBuilder::build(&request, &["identity".into()]);
    let packs = ChapterEvidencePlanner::plan_all(
        &pool,
        &plan,
        &catalog.evidence,
        &catalog.evidence.premium_policy,
    )
    .unwrap();
    let block = AstroPayloadNormalizer::to_chapter_evidence_pack_block(
        &packs[0],
        &catalog,
        "fr",
        &facts,
    );
    let asc = block["core"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v["label"].as_str())
        .unwrap_or("");
    assert!(
        asc.contains("Scorpion"),
        "expected capitalized sign in FR label, got: {asc}"
    );
}

#[test]
fn prompt_pack_smaller_than_global_facts_block() {
    let facts = normalize(&rich_payload_from_golden());
    let catalog = catalog_with_evidence();
    let pool =
        InterpretiveEvidenceBuilder::build(&facts, &catalog.evidence).expect("build pool");
    let request = premium_request(rich_payload_from_golden());
    let plan = ReadingPlanBuilder::build(&request, &["identity".into()]);
    let packs = ChapterEvidencePlanner::plan_all(
        &pool,
        &plan,
        &catalog.evidence,
        &catalog.evidence.premium_policy,
    )
    .unwrap();
    let block = AstroPayloadNormalizer::to_chapter_evidence_pack_block(
        &packs[0],
        &catalog,
        "fr",
        &facts,
    );
    assert_eq!(block["_type"], "chapter_evidence_pack");
    assert!(block.get("facts").is_none());
    let global = AstroPayloadNormalizer::to_chapter_prompt_data_block(&facts, "identity");
    assert!(global["facts"].as_array().unwrap().len() > packs[0].total_count());
}
