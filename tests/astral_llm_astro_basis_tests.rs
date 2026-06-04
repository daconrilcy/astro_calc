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
use astral_llm_infra::{bootstrap_domains, bootstrap_product_policies, CanonicalCatalog, SafetyPattern};
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
        ..Default::default()
    })
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
    let facts = AstroPayloadNormalizer::normalize(
        &AstroCalculationPayload {
            contract_version: "natal_structured_v13".into(),
            chart_type: "natal".into(),
            data: premium_payload_with_scores_only(),
        },
        &PrivacyPolicy::default(),
    )
    .expect("normalize");

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
            interpretive_role: "signal".into(),
        }],
        confidence: ConfidenceLevel::Medium,
        safety_flags: vec![],
    };

    let policy = ProductGenerationPolicy::bootstrap_premium();
    assert!(AstroBasisValidator::validate_chapter(&chapter, &facts, &policy).is_err());
}

#[test]
fn premium_accepts_domain_score_plus_placement() {
    let facts = AstroPayloadNormalizer::normalize(
        &AstroCalculationPayload {
            contract_version: "natal_structured_v13".into(),
            chart_type: "natal".into(),
            data: premium_payload_with_placements(),
        },
        &PrivacyPolicy::default(),
    )
    .expect("normalize");

    let chapter = ReadingChapter {
        code: "identity".into(),
        title: "Identite".into(),
        body: "texte".into(),
        astro_basis: vec![
            astral_llm_domain::AstroBasisItem {
                fact_id: Some("domain_score:identity".into()),
                label: None,
                factor: "identity".into(),
                interpretive_role: "signal".into(),
            },
            astral_llm_domain::AstroBasisItem {
                fact_id: Some("placement:sun:capricorn:house:2".into()),
                label: None,
                factor: "sun".into(),
                interpretive_role: "placement".into(),
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

#[tokio::test]
async fn premium_e2e_succeeds_with_placements_and_summary() {
    let use_case = build_use_case(test_catalog());
    let request = premium_request(premium_payload_with_placements());
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
