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
            product_code: "natal_prompter".into(),
            interpretation_profile_code: Some("natal_premium".into()),
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
            allow_oracle_benchmark: false,
            summary_model: None,
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
    let err = pool_richness_check(&pool, &policy, 5).unwrap_err();
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
    pool_richness_check(&pool, &policy, 5).expect("rich enough");

    let request = premium_request(payload);
    let domains = vec![
        "identity".into(),
        "emotional_life".into(),
        "relationships".into(),
        "career".into(),
        "growth_path".into(),
    ];
    let plan = ReadingPlanBuilder::build(&request, &domains, None);
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

fn pack_contains_semantic(pack: &astral_llm_domain::ChapterEvidencePack, key: &str) -> bool {
    pack.contains_semantic_key(key)
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
        None,
    );
    let packs = ChapterEvidencePlanner::plan_all(&pool, &plan, &catalog.evidence, &policy)
        .expect("plan");

    let emotional = packs
        .iter()
        .find(|p| p.chapter_code == "emotional_life")
        .expect("emotional");
    let moon_ev = emotional
        .core
        .iter()
        .chain(emotional.supporting.iter())
        .find(|e| e.fact_id == "signal:object_position:moon")
        .or_else(|| {
            emotional
                .core
                .iter()
                .chain(emotional.supporting.iter())
                .find(|e| e.object_code.as_deref() == Some("moon"))
        })
        .expect("moon in emotional_life");
    let moon_key = moon_ev.semantic_fact_key.as_str();

    for code in ["relationships", "career", "growth_path"] {
        let pack = packs.iter().find(|p| p.chapter_code == code).expect(code);
        assert!(
            !pack_contains_semantic(pack, moon_key),
            "{code} must not repeat moon semantic key from emotional_life"
        );
        for avoid in &pack.avoid_repeating {
            assert!(
                !pack_contains_semantic(pack, avoid),
                "{code} pack must not include avoid_repeating key {avoid}"
            );
        }
    }
}

#[test]
fn relationships_pack_prefers_descendant_ruler_not_mc() {
    let payload = rich_payload_from_golden();
    let facts = normalize(&payload);
    let catalog = catalog_with_evidence();
    let pool =
        InterpretiveEvidenceBuilder::build(&facts, &catalog.evidence).expect("build pool");
    let has_dsc_ruler = pool
        .evidence
        .iter()
        .any(|e| e.fact_id.contains("ruler:angle:descendant"));
    assert!(
        has_dsc_ruler,
        "pool must expose descendant ruler from rulership_context"
    );
    let request = premium_request(payload);
    let plan = ReadingPlanBuilder::build(&request, &["relationships".into()], None);
    let packs = ChapterEvidencePlanner::plan_all(
        &pool,
        &plan,
        &catalog.evidence,
        &catalog.evidence.premium_policy,
    )
    .expect("plan");
    let rel = packs
        .iter()
        .find(|p| p.chapter_code == "relationships")
        .expect("relationships pack");
    let active: Vec<_> = rel
        .core
        .iter()
        .chain(rel.supporting.iter())
        .chain(rel.nuance.iter())
        .collect();
    assert!(
        active
            .iter()
            .any(|e| e.fact_id.contains("ruler:angle:descendant")),
        "relationships pack should include descendant ruler; pack={:?}",
        rel.all_fact_ids()
    );
    assert!(
        !active.iter().any(|e| e.fact_id.contains("ruler:angle:mc:")),
        "relationships pack must not carry mc ruler; pack={:?}",
        rel.all_fact_ids()
    );
}

#[test]
fn career_pack_prefers_mc_ruler_when_in_pool() {
    let payload = rich_payload_from_golden();
    let facts = normalize(&payload);
    let catalog = catalog_with_evidence();
    let pool =
        InterpretiveEvidenceBuilder::build(&facts, &catalog.evidence).expect("build pool");
    let has_mc_ruler = pool.evidence.iter().any(|e| e.fact_id.contains("ruler:angle:mc"));
    if !has_mc_ruler {
        return;
    }
    let request = premium_request(payload);
    let plan = ReadingPlanBuilder::build(&request, &["career".into()], None);
    let packs = ChapterEvidencePlanner::plan_all(
        &pool,
        &plan,
        &catalog.evidence,
        &catalog.evidence.premium_policy,
    )
    .expect("plan");
    let career = packs
        .iter()
        .find(|p| p.chapter_code == "career")
        .expect("career pack");
    let cites_ruler = career
        .core
        .iter()
        .chain(career.supporting.iter())
        .chain(career.nuance.iter())
        .any(|e| e.kind_code == "house_ruler" && (e.fact_id.contains("mc") || e.object_code.as_deref() == Some("sun")));
    assert!(
        cites_ruler,
        "career pack should include mc ruler when available; pack={:?}",
        career.all_fact_ids()
    );
}

#[test]
fn identity_pack_excludes_sun() {
    let payload = rich_payload_from_golden();
    let facts = normalize(&payload);
    let catalog = catalog_with_evidence();
    let pool =
        InterpretiveEvidenceBuilder::build(&facts, &catalog.evidence).expect("build pool");
    let request = premium_request(payload);
    let plan = ReadingPlanBuilder::build(&request, &["identity".into()], None);
    let packs = ChapterEvidencePlanner::plan_all(
        &pool,
        &plan,
        &catalog.evidence,
        &catalog.evidence.premium_policy,
    )
    .unwrap();
    let identity = &packs[0];
    let has_sun = identity
        .core
        .iter()
        .chain(identity.supporting.iter())
        .chain(identity.nuance.iter())
        .any(|e| e.object_code.as_deref() == Some("sun") || e.semantic_fact_key.contains(":sun:"));
    assert!(!has_sun, "identity must not carry sun (reserved for career)");
}

#[test]
fn emotional_excludes_aspect_already_in_identity_pack() {
    let payload = rich_payload_from_golden();
    let facts = normalize(&payload);
    let catalog = catalog_with_evidence();
    let pool =
        InterpretiveEvidenceBuilder::build(&facts, &catalog.evidence).expect("build pool");
    let request = premium_request(payload);
    let plan = ReadingPlanBuilder::build(
        &request,
        &["identity".into(), "emotional_life".into()],
        None,
    );
    let packs = ChapterEvidencePlanner::plan_all(
        &pool,
        &plan,
        &catalog.evidence,
        &catalog.evidence.premium_policy,
    )
    .unwrap();
    let identity = packs.iter().find(|p| p.chapter_code == "identity").unwrap();
    let aspect_key = "aspect:jupiter:uranus:opposition";
    let identity_used_aspect = identity
        .core
        .iter()
        .chain(identity.supporting.iter())
        .chain(identity.nuance.iter())
        .any(|e| e.semantic_fact_key == aspect_key);
    if !identity_used_aspect {
        return;
    }
    let emotional = packs.iter().find(|p| p.chapter_code == "emotional_life").unwrap();
    assert!(
        !emotional
            .core
            .iter()
            .chain(emotional.supporting.iter())
            .chain(emotional.nuance.iter())
            .any(|e| e.semantic_fact_key == aspect_key),
        "emotional must not repeat jupiter/uranus aspect from identity"
    );
    assert!(emotional.avoid_repeating.iter().any(|k| k == aspect_key));
}

#[test]
fn signal_sun_and_placement_sun_not_both_in_same_chapter_pack() {
    let payload = rich_payload_from_golden();
    let facts = normalize(&payload);
    let catalog = catalog_with_evidence();
    let pool =
        InterpretiveEvidenceBuilder::build(&facts, &catalog.evidence).expect("build pool");
    let sun_placement = pool
        .evidence
        .iter()
        .find(|e| e.fact_id.starts_with("placement:sun"))
        .map(|e| e.semantic_fact_key.clone())
        .expect("placement sun");
    let request = premium_request(payload);
    let plan = ReadingPlanBuilder::build(&request, &["identity".into()], None);
    let packs = ChapterEvidencePlanner::plan_all(
        &pool,
        &plan,
        &catalog.evidence,
        &catalog.evidence.premium_policy,
    )
    .unwrap();
    let pack = &packs[0];
    let sun_semantic: Vec<_> = pack
        .core
        .iter()
        .chain(pack.supporting.iter())
        .chain(pack.nuance.iter())
        .filter(|e| e.semantic_fact_key == sun_placement)
        .collect();
    assert!(
        sun_semantic.len() <= 1,
        "at most one evidence row per semantic sun placement"
    );
}

#[test]
fn prompt_pack_labels_localized_for_fr() {
    let facts = normalize(&rich_payload_from_golden());
    let catalog = catalog_with_evidence();
    let pool =
        InterpretiveEvidenceBuilder::build(&facts, &catalog.evidence).expect("build pool");
    let request = premium_request(rich_payload_from_golden());
    let plan = ReadingPlanBuilder::build(&request, &["identity".into()], None);
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
fn prompt_pack_humanizes_ruler_labels_in_french() {
    let payload = rich_payload_from_golden();
    let facts = normalize(&payload);
    let catalog = catalog_with_evidence();
    let pool =
        InterpretiveEvidenceBuilder::build(&facts, &catalog.evidence).expect("build pool");
    let request = premium_request(payload);
    let plan = ReadingPlanBuilder::build(&request, &["career".into()], None);
    let packs = ChapterEvidencePlanner::plan_all(
        &pool,
        &plan,
        &catalog.evidence,
        &catalog.evidence.premium_policy,
    )
    .unwrap();
    let career = packs
        .iter()
        .find(|p| p.chapter_code == "career")
        .expect("career pack");
    let block = AstroPayloadNormalizer::to_chapter_evidence_pack_block(
        career,
        &catalog,
        "fr",
        &facts,
    );
    let labels: Vec<String> = ["core", "supporting", "nuance"]
        .iter()
        .flat_map(|tier| {
            block[*tier]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|v| v["label"].as_str().map(str::to_string))
        })
        .collect();
    assert!(
        labels.iter().any(|l| l.contains("Maître du Milieu du Ciel")),
        "expected humanized MC ruler label, got: {labels:?}"
    );
    assert!(
        !labels.iter().any(|l| l.contains("Maitre (mc)")),
        "raw ruler label should not leak to prompt pack"
    );
}

#[test]
fn sun_supporting_semantic_key_capped_at_three_chapters() {
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
        None,
    );
    let packs = ChapterEvidencePlanner::plan_all(&pool, &plan, &catalog.evidence, &policy)
        .expect("plan");

    let mut supporting_chapters_by_key: std::collections::HashMap<String, u8> =
        std::collections::HashMap::new();
    for pack in &packs {
        let mut seen = HashSet::new();
        for ev in &pack.supporting {
            let sun = ev.object_code.as_deref() == Some("sun")
                || ev.semantic_fact_key.contains(":sun:");
            if !sun {
                continue;
            }
            if seen.insert(ev.semantic_fact_key.clone()) {
                *supporting_chapters_by_key
                    .entry(ev.semantic_fact_key.clone())
                    .or_insert(0) += 1;
            }
        }
    }
    for (key, count) in &supporting_chapters_by_key {
        assert!(
            *count <= policy.max_supporting_semantic_chapters,
            "sun supporting key {key} in {count} chapters (max {})",
            policy.max_supporting_semantic_chapters
        );
    }
}

#[test]
fn prompt_pack_smaller_than_global_facts_block() {
    let facts = normalize(&rich_payload_from_golden());
    let catalog = catalog_with_evidence();
    let pool =
        InterpretiveEvidenceBuilder::build(&facts, &catalog.evidence).expect("build pool");
    let request = premium_request(rich_payload_from_golden());
    let plan = ReadingPlanBuilder::build(&request, &["identity".into()], None);
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

#[test]
fn premium_plus_rich_pool_plans_synthesis_with_global_dominants() {
    let payload = rich_payload_from_golden();
    let facts = normalize(&payload);
    let catalog = catalog_with_evidence();
    let pool =
        InterpretiveEvidenceBuilder::build(&facts, &catalog.evidence).expect("build pool");
    let profiles = astral_llm_infra::bootstrap_interpretation_profiles();
    let profile = profiles.get("natal_premium_plus").expect("profile");
    let policy = profile.to_premium_evidence_policy().expect("policy");
    pool_richness_check(&pool, &policy, 9).expect("rich enough");

    let mut request = premium_request(payload);
    request.product_context.interpretation_profile_code = Some("natal_premium_plus".into());
    let ctx = astral_llm_application::interpretation_profile_resolver::ResolvedInterpretationContext {
        profile: profile.clone(),
        effective_policy: profile.to_product_generation_policy(),
    };
    let domains = ctx.profile.astrological_chapter_types();
    let plan = ReadingPlanBuilder::build(&request, &domains, Some(&ctx));
    assert_eq!(plan.chapters.len(), 9);
    let packs = ChapterEvidencePlanner::plan_all(&pool, &plan, &catalog.evidence, &policy)
        .expect("plan all chapters");
    let synthesis = packs
        .iter()
        .find(|p| p.chapter_code == "synthesis")
        .expect("synthesis pack");
    assert!(
        synthesis.total_count() >= policy.min_evidence_per_chapter as usize,
        "synthesis pack too small: {}",
        synthesis.total_count()
    );
    assert!(
        synthesis
            .core
            .iter()
            .chain(synthesis.supporting.iter())
            .any(|e| e.kind_code == "dominant_planet" || e.kind_code == "house_emphasis"),
        "synthesis should cite global dominants"
    );
}
