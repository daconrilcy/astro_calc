//! Tests astro_basis Premium : domain_score seul interdit, placement requis.

use std::sync::Arc;

use astral_llm_application::{
    build_provider_map, AstroBasisValidator, AstroPayloadNormalizer, GenerateReadingUseCase,
    ModelCapabilityRegistry, PromptCompiler, ProviderCircuitBreaker, ProviderRouter,
    ResponseValidator, SchemaRegistry,
};
use astral_llm_domain::{
    astro_fact::AstroFactUsage,
    astrologer_profile::{JargonLevel, ToneProfile, WordingStyle},
    engine_params::EngineParams,
    generation_response::{ConfidenceLevel, GenerateReadingResponse, ReadingChapter},
    generation_request::{AudienceLevel, GenerateReadingRequest, ProductContext},
    output_contract::{GenerationMode, OutputFormat, ResponseContract},
    provider::ProviderKind,
    AstroCalculationPayload, AstrologerProfile, EngineDefaults, FallbackPolicy, PrivacyPolicy,
    ProductGenerationPolicy, ServiceLimits,
};
use astral_llm_infra::{
    bootstrap_astro_object_labels, bootstrap_domains, bootstrap_product_policies,
    bootstrap_zodiac_sign_labels, CanonicalCatalog, SafetyPattern,
};
use astral_llm_providers::FakeProvider;

fn test_catalog() -> Arc<CanonicalCatalog> {
    Arc::new(CanonicalCatalog {
        astrological_domains: bootstrap_domains(),
        product_generation_policies: bootstrap_product_policies(),
        safety_patterns: vec![SafetyPattern {
            pattern_type: "symbolic".into(),
            locale: "fr".into(),
            pattern: "symbolique".into(),
        }],
        astro_object_labels: bootstrap_astro_object_labels(),
        zodiac_sign_labels: bootstrap_zodiac_sign_labels(),
        ..Default::default()
    })
}

fn normalize_facts(payload: &AstroCalculationPayload) -> astral_llm_domain::NormalizedAstroFacts {
    AstroPayloadNormalizer::normalize(
        payload,
        &PrivacyPolicy::default(),
        &test_catalog(),
        "fr",
    )
    .expect("normalize")
}

fn premium_payload_with_scores_only() -> serde_json::Value {
    serde_json::json!({
        "domain_scores": {
            "identity": 0.85,
            "relationships": 0.72
        }
    })
}

fn premium_payload_with_placements() -> serde_json::Value {
    serde_json::json!({
        "domain_scores": {
            "identity": 0.85,
            "relationships": 0.72
        },
        "planets": {
            "sun": { "house": 2, "sign": "capricorn" },
            "moon": { "house": 4, "sign": "pisces" },
            "ascendant": { "house": 1, "sign": "scorpio" }
        }
    })
}

fn premium_request(data: serde_json::Value) -> GenerateReadingRequest {
    GenerateReadingRequest {
        request_id: Some("astro-basis-test".into()),
        idempotency_key: None,
        product_context: ProductContext {
            product_code: "natal_premium".into(),
            user_language: "fr".into(),
            audience_level: AudienceLevel::Intermediate,
        },
        astro_result: AstroCalculationPayload {
            contract_version: "natal_structured_v13".into(),
            chart_type: "natal".into(),
            data,
        },
        astrologer_profile: AstrologerProfile {
            profile_id: None,
            name: None,
            tone: ToneProfile::Warm,
            jargon_level: JargonLevel::Beginner,
            wording_style: WordingStyle::Clear,
            preferred_domains: vec!["identity".into(), "relationships".into()],
            forbidden_wording: vec![],
            custom_instructions: None,
        },
        engine: EngineParams {
            provider: Some(ProviderKind::Fake),
            model: Some("fake-model".into()),
            reasoning_effort: None,
            temperature: Some(0.4),
            max_output_tokens: Some(2000),
            domain_count: Some(2),
            allow_fallback: false,
            timeout_ms: Some(30_000),
            allow_oracle_benchmark: false,
        },
        response_contract: ResponseContract {
            output_schema_version: "natal_reading_v1".into(),
            generation_mode: GenerationMode::ChapterOrchestrated,
            format: OutputFormat::StructuredJson,
            chapters: vec![],
            global_max_tokens: None,
            include_astro_sources: true,
            include_legal_disclaimer: true,
        },
        safety_policy: None,
    }
}

fn build_use_case(catalog: Arc<CanonicalCatalog>) -> GenerateReadingUseCase {
    let router = ProviderRouter::new(
        build_provider_map(vec![Arc::new(FakeProvider)]),
        FallbackPolicy::disabled(),
        Arc::new(ModelCapabilityRegistry::bootstrap()),
        PrivacyPolicy::default(),
        Arc::new(ProviderCircuitBreaker::new(5, 60)),
    );
    let prompts = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../prompts");
    GenerateReadingUseCase::new(
        router,
        PromptCompiler::new(prompts),
        ResponseValidator::new(Arc::new(SchemaRegistry::new())),
        EngineDefaults {
            provider: ProviderKind::Fake,
            model: "fake-model".into(),
        },
        ServiceLimits::default(),
        catalog,
        PrivacyPolicy::default(),
    )
}

#[test]
fn premium_rejects_domain_score_only_chapter_basis() {
    let facts = normalize_facts(&AstroCalculationPayload {
        contract_version: "natal_structured_v13".into(),
        chart_type: "natal".into(),
        data: premium_payload_with_scores_only(),
    });

    assert!(
        !facts.facts.iter().any(|f| f.usage == AstroFactUsage::InterpretiveBasis),
        "scores-only payload must not produce interpretive facts"
    );

    let chapter = ReadingChapter {
        code: "identity".into(),
        title: "Identite".into(),
        body: "texte".into(),
        astro_basis: vec![astral_llm_domain::AstroBasisItem {
            fact_id: Some("domain_score:identity".into()),
            label: None,
            factor: "identity".into(),
            interpretive_role: "domain_score".into(),
        }],
        confidence: ConfidenceLevel::Medium,
        safety_flags: vec![],
    };

    let policy = ProductGenerationPolicy::bootstrap_premium();
    assert!(AstroBasisValidator::validate_chapter(&chapter, &facts, &policy).is_err());
}

#[test]
fn premium_accepts_domain_score_plus_placement() {
    let facts = normalize_facts(&AstroCalculationPayload {
        contract_version: "natal_structured_v13".into(),
        chart_type: "natal".into(),
        data: premium_payload_with_placements(),
    });

    let chapter = ReadingChapter {
        code: "identity".into(),
        title: "Identite".into(),
        body: "texte".into(),
        astro_basis: vec![
            astral_llm_domain::AstroBasisItem {
                fact_id: Some("domain_score:identity".into()),
                label: None,
                factor: "identity".into(),
                interpretive_role: "domain_score".into(),
            },
            astral_llm_domain::AstroBasisItem {
                fact_id: Some("placement:sun:capricorn:house:2".into()),
                label: None,
                factor: "sun".into(),
                interpretive_role: "core".into(),
            },
        ],
        confidence: ConfidenceLevel::Medium,
        safety_flags: vec![],
    };

    let policy = ProductGenerationPolicy::bootstrap_premium();
    assert!(AstroBasisValidator::validate_chapter(&chapter, &facts, &policy).is_ok());
}

#[tokio::test]
async fn premium_e2e_fails_without_interpretive_payload() {
    let use_case = build_use_case(test_catalog());
    let request = premium_request(premium_payload_with_scores_only());
    let response = use_case.execute(request).await;
    assert!(
        matches!(response, GenerateReadingResponse::Failed(_)),
        "expected failure when payload has no interpretive facts"
    );
}

fn v13_golden_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/golden/natal_payload_v13_paris_1990.json")
}

fn rich_premium_payload() -> serde_json::Value {
    let raw = std::fs::read_to_string(v13_golden_path()).expect("golden payload");
    serde_json::from_str(&raw).expect("parse golden")
}

#[tokio::test]
async fn premium_e2e_minimal_placements_fails_diversity() {
    let use_case = build_use_case(test_catalog());
    let request = premium_request(premium_payload_with_placements());
    let response = use_case.execute(request).await;
    match response {
        GenerateReadingResponse::Failed(failed) => {
            assert_eq!(
                failed.error.code.as_str(),
                "PREMIUM_EVIDENCE_DIVERSITY_FAILED"
            );
        }
        other => panic!("expected PREMIUM_EVIDENCE_DIVERSITY_FAILED, got {other:?}"),
    }
}

#[tokio::test]
async fn premium_e2e_succeeds_with_rich_payload_and_summary() {
    let use_case = build_use_case(test_catalog());
    let request = premium_request(rich_premium_payload());
    let response = use_case.execute(request).await;

    match response {
        GenerateReadingResponse::Success(success) => {
            assert_eq!(success.reading.chapters.len(), 2);
            for chapter in &success.reading.chapters {
                let has_interpretive = chapter.astro_basis.iter().any(|b| {
                    b.fact_id
                        .as_ref()
                        .is_some_and(|id| !id.starts_with("domain_score:"))
                });
                assert!(has_interpretive, "chapter {} lacks interpretive basis", chapter.code);
            }
            let summary = &success.reading.summary;
            assert!(!summary.short_text.to_lowercase().contains("generation chapitre"));
            assert!(!summary.title.to_lowercase().contains("natal_premium"));
        }
        other => panic!("expected success with placements, got {other:?}"),
    }
}
