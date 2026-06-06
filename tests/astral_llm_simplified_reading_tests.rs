//! Tests natal simplifie : validation requete, anti-hallucination prompt, golden fixture.

use std::sync::Arc;

use astral_llm_application::{
    astro_payload_normalizer::AstroPayloadNormalizer,
    build_reading_request, merge_simplified_forbidden_wording, prompt_constraints_block,
    resolve_simplified_chapter_code, sun_sign_blocked, validate_simplified_calculation_request,
    PromptCompiler, SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE, SIMPLIFIED_CHAPTER_IDENTITY,
    SIMPLIFIED_PAYLOAD_CONTRACT, SIMPLIFIED_PROFILE, SUN_SIGN_BLOCKED_CODE,
};
use astral_llm_application::french_typography::{french_elision_violations, restore_french_elisions};
use astral_llm_application::simplified_reading_postprocess::{
    build_compact_summary_from_body, normalize_simplified_interpretive_roles,
};
use astral_llm_application::prompt_compiler::PromptCompilationInput;
use astral_llm_domain::{
    generation_request::AudienceLevel, PrivacyPolicy, SafetyPolicy,
};
use astral_llm_infra::{bootstrap_interpretation_profiles, CanonicalCatalog};
use serde_json::json;

fn load_golden(name: &str) -> serde_json::Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/golden")
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(path).expect("golden fixture")).expect("json")
}

fn stable_calculation() -> serde_json::Value {
    load_golden("simplified_natal_calculation_stable_1990-06-15.json")
}

fn equinox_calculation() -> serde_json::Value {
    load_golden("simplified_natal_calculation_equinox_1990-03-21.json")
}

#[test]
fn validate_simplified_request_rejects_bad_contract_and_missing_date() {
    let err = validate_simplified_calculation_request(&json!({})).expect_err("empty");
    assert!(err.to_string().contains("request_contract_version"));

    let err = validate_simplified_calculation_request(&json!({
        "request_contract_version": "wrong",
        "birth": { "date": "1990-01-01" }
    }))
    .expect_err("wrong contract");
    assert!(err.to_string().contains("unsupported"));

    let err = validate_simplified_calculation_request(&json!({
        "request_contract_version": "astro_simplified_natal_request_v1",
        "birth": { "date": "1990-01-01", "time": "12:00:00" }
    }))
    .expect_err("time without tz");
    assert!(err.to_string().contains("timezone"));
}

#[test]
fn build_reading_request_scrubs_blocked_objects_from_payload() {
    let calculation = equinox_calculation();
    let request =
        build_reading_request(&calculation, "fr", AudienceLevel::Beginner).expect("build");

    assert!(request.astro_result.data["planets"].get("sun").is_none());
    assert!(request.astro_result.data["planets"].get("moon").is_none());
    assert!(request.astro_result.data.get("position_count").is_none());
    let facts = request.astro_result.data["facts"].as_array().expect("facts");
    assert!(!facts.iter().any(|f| f["object_code"] == "sun"));
    assert!(!facts.iter().any(|f| f["object_code"] == "moon"));
    assert!(
        request
            .astrologer_profile
            .forbidden_wording
            .contains(&"moon.sign".to_string())
    );
}

#[test]
fn prompt_constraints_block_mentions_blocked_moon_and_excluded_ascendant() {
    let controls = equinox_calculation()["llm_payload"].clone();
    let block = prompt_constraints_block(&controls);
    assert!(block.contains("SIMPLIFIED NATAL CONSTRAINTS"));
    assert!(block.contains("moon.sign"));
    assert!(block.contains("placement:mercury"));
    assert!(block.contains("Allowed astro_basis.fact_id"));
    assert!(block.contains("Profile excluded"));
    assert!(block.contains("Never affirm Ascendant"));
    assert!(block.contains("never \"degraded\""));
}

#[test]
fn build_reading_request_routes_ambiguous_core_when_sun_blocked() {
    let calculation = equinox_calculation();
    let controls = calculation["llm_payload"].clone();
    assert!(sun_sign_blocked(&controls));
    assert_eq!(
        resolve_simplified_chapter_code(&controls),
        SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE
    );
    let request =
        build_reading_request(&calculation, "fr", AudienceLevel::Beginner).expect("build");
    assert_eq!(
        request.response_contract.chapters[0].code,
        SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE
    );
    assert!(request.astrologer_profile.custom_instructions.is_some());
}

#[test]
fn build_reading_request_uses_identity_when_sun_stable() {
    let calculation = stable_calculation();
    let request =
        build_reading_request(&calculation, "fr", AudienceLevel::Beginner).expect("build");
    assert_eq!(
        request.response_contract.chapters[0].code,
        SIMPLIFIED_CHAPTER_IDENTITY
    );
    assert!(request.astrologer_profile.custom_instructions.is_none());
    let controls = calculation["llm_payload"].clone();
    assert!(!sun_sign_blocked(&controls));
    assert_eq!(
        controls["allowed_astro_basis_fact_ids"][0].as_str(),
        Some("placement:sun")
    );
    assert!(
        !controls["blocked_interpretation_fact_codes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_str() == Some(SUN_SIGN_BLOCKED_CODE)),
        "sun.sign must not be blocked when stable"
    );
}

#[test]
fn merge_forbidden_wording_deduplicates_controls() {
    let controls = equinox_calculation()["llm_payload"].clone();
    let merged = merge_simplified_forbidden_wording(&controls, vec!["custom".into(), "moon.sign".into()]);
    assert!(merged.contains(&"custom".to_string()));
    assert!(merged.contains(&"moon.sign".to_string()));
    assert_eq!(merged.iter().filter(|v| *v == "moon.sign").count(), 1, "deduped");
    assert!(
        !merged.contains(&"sect".to_string()),
        "excluded features must not become substring forbidden wording"
    );
}

#[test]
fn compiled_simplified_prompt_injects_constraints_and_data_controls() {
    let calculation = equinox_calculation();
    let reading =
        build_reading_request(&calculation, "fr", AudienceLevel::Beginner).expect("build");
    let profiles = bootstrap_interpretation_profiles();
    let profile = profiles.get(SIMPLIFIED_PROFILE).expect("profile");
    let ctx = astral_llm_application::interpretation_profile_resolver::ResolvedInterpretationContext {
        profile: profile.clone(),
        effective_policy: profile.to_product_generation_policy(),
    };
    let catalog = Arc::new({
        let mut c = CanonicalCatalog::default();
        astral_llm_infra::enrich_catalog_from_bootstrap(&mut c);
        c
    });
    let facts = AstroPayloadNormalizer::normalize(
        &reading.astro_result,
        &PrivacyPolicy::default(),
        &catalog,
        "fr",
    )
    .expect("normalize");
    let prompts = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../prompts");
    let safety = SafetyPolicy::mandatory();
    let bundle = PromptCompiler::new(prompts)
        .compile(PromptCompilationInput {
            request: &reading,
            safety_policy: &safety,
            astro_facts: &facts,
            selected_domains: &["identity".to_string()],
            chapter_code: Some(SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE),
            chapter_evidence_pack: None,
            catalog: &catalog,
            interpretation: Some(&ctx),
            repair_instruction: None,
        })
        .expect("compile");

    assert!(
        bundle.task_instructions.contains("SIMPLIFIED NATAL CONSTRAINTS"),
        "constraints must be in task instructions"
    );
    assert!(bundle.task_instructions.contains("moon.sign"));
    assert!(bundle.data_payload.get("llm_controls").is_some());
    assert!(bundle.data_payload.get("excluded_features").is_some());
}

#[test]
fn golden_fixture_matches_simplified_payload_schema_keys() {
    let calculation = stable_calculation();
    let payload = &calculation["simplified_payload"]["payload"];
    for key in [
        "payload_contract",
        "computed_scope",
        "input_precision_level",
        "facts",
        "ambiguous_facts",
        "excluded_features",
        "planets",
    ] {
        assert!(payload.get(key).is_some(), "missing key {key}");
    }
    assert_eq!(
        payload["payload_contract"].as_str(),
        Some(SIMPLIFIED_PAYLOAD_CONTRACT)
    );
}

#[test]
fn french_elision_restoration_fixes_llm_spacing_patterns() {
    let (fixed, changed) = restore_french_elisions(
        "l impression générale est celle d une personne qui n hésite pas. Ce n est pas figé.",
    );
    assert!(changed);
    assert!(fixed.contains("l'impression"));
    assert!(fixed.contains("d'une"));
    assert!(fixed.contains("n'hésite"));
    assert!(fixed.contains("n'est"));
    assert!(french_elision_violations(&fixed).is_empty());
}

#[test]
fn compact_summary_is_autonomous_without_ellipsis() {
    let body = "Votre signature identitaire semble portée par un mélange vif de curiosité. \
                Avec le Soleil en Gémeaux, l'impression générale est celle d'une personnalité mobile. \
                Troisième phrase qui ne doit pas être incluse.";
    let summary = build_compact_summary_from_body(body, "fr");
    assert!(!summary.contains('…'));
    assert!(summary.ends_with('.') || summary.ends_with('!') || summary.ends_with('?'));
    assert!(!summary.contains("Troisième phrase"));
}

#[test]
fn simplified_interpretive_roles_exclude_domain_score() {
    use astral_llm_domain::generation_response::{
        AstroBasisItem, ConfidenceLevel, NatalReadingResponse, QualityMetadata, ReadingChapter,
        ReadingSummary,
    };
    use astral_llm_domain::output_contract::GenerationMode;

    let mut reading = NatalReadingResponse {
        schema_version: "natal_reading_v1".into(),
        language: "fr".into(),
        reading_type: "natal_prompter".into(),
        summary: ReadingSummary {
            title: "T".into(),
            short_text: "S".into(),
        },
        chapters: vec![ReadingChapter {
            code: SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE.into(),
            title: "T".into(),
            body: "B".into(),
            astro_basis: vec![AstroBasisItem {
                fact_id: Some("placement:saturn".into()),
                label: None,
                factor: "Saturne".into(),
                interpretive_role: "domain_score".into(),
            }],
            confidence: ConfidenceLevel::Medium,
            safety_flags: vec![],
        }],
        legal: astral_llm_domain::generation_response::LegalBlock {
            disclaimer: String::new(),
        },
        quality: QualityMetadata {
            used_provider: "fake".into(),
            used_model: "fake".into(),
            generation_mode: GenerationMode::SinglePass,
            prompt_family: "natal_prompter".into(),
            prompt_version: "v1".into(),
            astro_contract_version: "natal_simplified_structured_v1".into(),
            fallback_used: false,
        },
    };
    assert_eq!(normalize_simplified_interpretive_roles(&mut reading), 1);
    assert_eq!(
        reading.chapters[0].astro_basis[0].interpretive_role,
        "supporting"
    );
}
