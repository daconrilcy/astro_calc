//! Tests natal simplifie : validation requete, anti-hallucination prompt, golden fixture.

use std::sync::Arc;

use astral_llm_application::{
    astro_payload_normalizer::AstroPayloadNormalizer,
    build_reading_request, merge_simplified_forbidden_wording, prompt_constraints_block,
    validate_simplified_calculation_request, PromptCompiler, SIMPLIFIED_PAYLOAD_CONTRACT,
    SIMPLIFIED_PROFILE,
};
use astral_llm_application::prompt_compiler::PromptCompilationInput;
use astral_llm_domain::{
    generation_request::AudienceLevel, PrivacyPolicy, SafetyPolicy,
};
use astral_llm_infra::{bootstrap_interpretation_profiles, CanonicalCatalog};
use serde_json::json;

fn golden_calculation() -> serde_json::Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/golden/simplified_natal_calculation_date_only_1990-03-21.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("golden fixture")).expect("json")
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
fn build_reading_request_injects_controls_and_forbidden_wording() {
    let calculation = golden_calculation();
    let request =
        build_reading_request(&calculation, "fr", AudienceLevel::Beginner).expect("build");

    assert_eq!(
        request.product_context.interpretation_profile_code.as_deref(),
        Some(SIMPLIFIED_PROFILE)
    );
    assert_eq!(request.engine.domain_count, Some(1));
    assert_eq!(
        request.astro_result.contract_version,
        SIMPLIFIED_PAYLOAD_CONTRACT
    );
    assert!(request.astro_result.data.get("llm_controls").is_some());
    assert!(
        request
            .astrologer_profile
            .forbidden_wording
            .contains(&"moon.sign".to_string())
    );
    assert!(
        !request
            .astrologer_profile
            .forbidden_wording
            .contains(&"sect".to_string())
    );
    assert!(request.astro_result.data["planets"]["moon"].is_null()
        || request.astro_result.data["planets"].get("moon").is_none());
}

#[test]
fn prompt_constraints_block_mentions_blocked_moon_and_excluded_ascendant() {
    let controls = golden_calculation()["llm_payload"].clone();
    let block = prompt_constraints_block(&controls);
    assert!(block.contains("SIMPLIFIED NATAL CONSTRAINTS"));
    assert!(block.contains("moon.sign"));
    assert!(block.contains("ascendant"));
    assert!(block.contains("Never affirm Ascendant"));
    assert!(block.contains("never \"degraded\""));
}

#[test]
fn merge_forbidden_wording_deduplicates_controls() {
    let controls = golden_calculation()["llm_payload"].clone();
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
    let calculation = golden_calculation();
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
            chapter_code: Some("identity"),
            chapter_evidence_pack: None,
            catalog: &catalog,
            interpretation: Some(&ctx),
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
    let calculation = golden_calculation();
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
