//! Tests des profils d'interpretation natal_prompter.

use astral_llm_application::InterpretationProfileResolver;
use astral_llm_domain::{
    astrologer_profile::{JargonLevel, ToneProfile, WordingStyle},
    generation_request::{AudienceLevel, ProductContext},
    interpretation_profile::{InterpretationProfile, InterpretationProfileDocument, NATAL_PROMPTER_PRODUCT},
    output_contract::{GenerationMode, OutputFormat, ResponseContract},
    AstroCalculationPayload, AstrologerProfile, GenerateReadingRequest,
};
use astral_llm_infra::{bootstrap_interpretation_profiles, CanonicalCatalog};
use std::sync::Arc;

#[test]
fn bootstrap_profiles_load_five_tiers() {
    let profiles = bootstrap_interpretation_profiles();
    assert!(profiles.contains_key("natal_light"));
    assert!(profiles.contains_key("natal_basic"));
    assert!(profiles.contains_key("natal_premium"));
    assert!(profiles.contains_key("natal_premium_plus"));
    assert!(profiles.contains_key("natal_simplified"));
}

#[test]
fn premium_plus_profile_targets_rich_reading() {
    let profiles = bootstrap_interpretation_profiles();
    let profile = profiles.get("natal_premium_plus").expect("natal_premium_plus");
    assert!(profile.evidence_enabled());
    assert!(profile.blocking_quality_gate());
    assert!(profile.has_final_synthesis_chapter());
    assert_eq!(profile.astrological_chapter_types().len(), 8);
    assert_eq!(profile.document.chapter_word_targets.target, 720);
    assert_eq!(profile.document.chapter_word_targets.min, 520);
    assert!(profile.chapter_needs_length_expansion("growth_path"));
    assert!(!profile.chapter_needs_length_expansion("identity"));
    assert!(profile.uses_fixed_chapter_sequence());
    assert_eq!(profile.planned_chapter_count(None), 9);
    let policy = profile.to_premium_evidence_policy().expect("policy");
    assert_eq!(policy.min_evidence_per_chapter, 6);
}

#[test]
fn premium_profile_enables_evidence_and_blocking_gate() {
    let profiles = bootstrap_interpretation_profiles();
    let profile = profiles.get("natal_premium").expect("natal_premium");
    assert!(profile.evidence_enabled());
    assert!(profile.blocking_quality_gate());
    assert!(profile.allows_chapter_orchestration());
}

#[test]
fn light_profile_single_pass_no_evidence() {
    let profiles = bootstrap_interpretation_profiles();
    let profile = profiles.get("natal_light").expect("natal_light");
    assert!(!profile.evidence_enabled());
    assert!(!profile.blocking_quality_gate());
    assert_eq!(profile.document.generation_mode, GenerationMode::SinglePass);
}

#[test]
fn profile_json_fixture_roundtrip() {
    let json = include_str!("../config/natal_interpretation_profiles/natal_basic.json");
    let doc: InterpretationProfileDocument = serde_json::from_str(json).expect("parse");
    let profile = InterpretationProfile::from_document(doc);
    assert_eq!(profile.product_code, NATAL_PROMPTER_PRODUCT);
    assert!(profile.validate().is_ok());
    assert!(!profile.evidence_enabled());
}

#[test]
fn simplified_profile_single_pass_with_disclaimer() {
    let profiles = bootstrap_interpretation_profiles();
    let profile = profiles.get("natal_simplified").expect("natal_simplified");
    assert!(!profile.evidence_enabled());
    assert!(profile.document.quality.require_disclaimer);
    assert_eq!(profile.document.generation_mode, GenerationMode::SinglePass);
}

#[test]
fn legacy_product_code_rejects_profile_mismatch() {
    let catalog = Arc::new(CanonicalCatalog {
        interpretation_profiles: bootstrap_interpretation_profiles(),
        ..Default::default()
    });
    let mut request = GenerateReadingRequest {
        request_id: None,
        idempotency_key: None,
        product_context: ProductContext {
            product_code: "natal_basic".into(),
            interpretation_profile_code: Some("natal_premium".into()),
            user_language: "fr".into(),
            audience_level: AudienceLevel::Beginner,
        },
        astro_result: AstroCalculationPayload {
            contract_version: "natal_structured_v13".into(),
            chart_type: "natal".into(),
            data: serde_json::json!({}),
        },
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
        engine: Default::default(),
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
    };
    let err = InterpretationProfileResolver::normalize_request(&mut request, &catalog)
        .expect_err("legacy basic + premium profile");
    assert_eq!(
        err.detail().code,
        astral_llm_domain::GenerationErrorCode::ProductPolicyViolation
    );
}

#[test]
fn legacy_natal_premium_product_code_migrates_at_normalize() {
    let catalog = Arc::new(CanonicalCatalog {
        interpretation_profiles: bootstrap_interpretation_profiles(),
        ..Default::default()
    });
    let mut request = GenerateReadingRequest {
        request_id: None,
        idempotency_key: None,
        product_context: ProductContext {
            product_code: "natal_premium".into(),
            interpretation_profile_code: None,
            user_language: "fr".into(),
            audience_level: AudienceLevel::Beginner,
        },
        astro_result: AstroCalculationPayload {
            contract_version: "natal_structured_v13".into(),
            chart_type: "natal".into(),
            data: serde_json::json!({}),
        },
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
        engine: Default::default(),
        response_contract: ResponseContract {
            output_schema_version: "natal_reading_v1".into(),
            generation_mode: GenerationMode::SinglePass,
            format: OutputFormat::StructuredJson,
            chapters: vec![],
            global_max_tokens: None,
            include_astro_sources: true,
            include_legal_disclaimer: true,
        },
        safety_policy: None,
    };
    InterpretationProfileResolver::normalize_request(&mut request, &catalog).unwrap();
    assert_eq!(request.product_context.product_code, NATAL_PROMPTER_PRODUCT);
    assert_eq!(
        request.product_context.interpretation_profile_code.as_deref(),
        Some("natal_premium")
    );
    assert_eq!(
        request.response_contract.generation_mode,
        GenerationMode::ChapterOrchestrated
    );
}
