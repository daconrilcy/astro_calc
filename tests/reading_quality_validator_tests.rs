use astral_llm_application::{
    chapter_orchestrator::normalize_chapter_code, requires_blocking_quality_gate,
    ReadingQualityValidator, ResolvedInterpretationContext,
};
use astral_llm_domain::{
    generation_request::AudienceLevel,
    generation_response::{
        ConfidenceLevel, LegalBlock, NatalReadingResponse, QualityMetadata, ReadingChapter,
        ReadingSummary,
    },
    interpretation_profile::SYNTHESIS_CHAPTER_CODE,
    output_contract::GenerationMode,
    AstroBasisItem, AstroCalculationPayload, AstrologerProfile, EngineParams, GenerateReadingRequest,
    JargonLevel, OutputFormat, ProductContext, ResponseContract, ToneProfile, WordingStyle,
};

fn premium_ctx(profile_code: &str) -> ResolvedInterpretationContext {
    let profile = astral_llm_infra::bootstrap_interpretation_profiles()
        .get(profile_code)
        .unwrap_or_else(|| panic!("{profile_code} profile"))
        .clone();
    let effective_policy = profile.to_product_generation_policy();
    ResolvedInterpretationContext {
        profile,
        effective_policy,
    }
}

fn premium_request(profile_code: &str) -> GenerateReadingRequest {
    GenerateReadingRequest {
        request_id: None,
        idempotency_key: None,
        product_context: ProductContext {
            product_code: "natal_prompter".into(),
            interpretation_profile_code: Some(profile_code.into()),
            user_language: "fr".into(),
            audience_level: AudienceLevel::Intermediate,
        },
        astro_result: AstroCalculationPayload {
            contract_version: "natal_structured_v13".into(),
            chart_type: "natal".into(),
            data: serde_json::json!({
                "domain_scores": { "identity": 0.5 },
                "planets": {
                    "sun": { "house": 2, "sign": "capricorn" }
                }
            }),
        },
        astrologer_profile: AstrologerProfile {
            profile_id: None,
            name: None,
            tone: ToneProfile::Warm,
            jargon_level: JargonLevel::Balanced,
            wording_style: WordingStyle::Clear,
            preferred_domains: vec!["identity".into()],
            forbidden_wording: vec![],
            custom_instructions: None,
        },
        engine: EngineParams {
            provider: None,
            model: None,
            reasoning_effort: None,
            temperature: None,
            max_output_tokens: None,
            domain_count: None,
            allow_fallback: false,
            timeout_ms: None,
            allow_oracle_benchmark: false,
            summary_model: None,
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

fn basis(fact_id: &str, factor: &str, role: &str) -> AstroBasisItem {
    AstroBasisItem {
        fact_id: Some(fact_id.into()),
        label: None,
        factor: factor.into(),
        interpretive_role: role.into(),
    }
}

fn good_reading() -> NatalReadingResponse {
    NatalReadingResponse {
        schema_version: "natal_reading_v1".into(),
        language: "fr".into(),
        reading_type: "natal_prompter".into(),
        summary: ReadingSummary {
            title: "Titre".into(),
            short_text: "Resume".into(),
        },
        chapters: vec![ReadingChapter {
            code: "identity".into(),
            title: "Identite".into(),
            body: "Votre theme suggere une personnalite reflechie, orientee vers la comprehension symbolique des experiences et des transitions interieures. Vous avancez avec prudence lorsque le sens n'est pas clair, tout en montrant une grande capacite d'adaptation lorsque vous sentez une direction authentique. Cette configuration invite a accueillir les phases de questionnement comme des espaces creatifs, plutot que comme des blocages rigides.".into(),
            astro_basis: vec![
                basis("domain_score:identity", "identity", "domain_score"),
                basis("placement:sun:capricorn:house:2", "sun", "core"),
                basis("placement:moon:cancer:house:4", "moon", "core"),
                basis("aspect:sun:moon:trine", "sun_moon", "supporting"),
            ],
            confidence: ConfidenceLevel::Medium,
            safety_flags: vec![],
        }],
        legal: LegalBlock {
            disclaimer: "Lecture symbolique.".into(),
        },
        quality: QualityMetadata {
            used_provider: "fake".into(),
            used_model: "fake".into(),
            generation_mode: GenerationMode::ChapterOrchestrated,
            prompt_family: "natal_prompter".into(),
            prompt_version: "v1".into(),
            astro_contract_version: "natal_structured_v13".into(),
            fallback_used: false,
        },
    }
}

#[test]
fn premium_rejects_poor_quality() {
    let request = premium_request("natal_premium");
    let mut reading = good_reading();
    reading.chapters[0].body = "sun in aries. moon in cancer.".into();
    let ctx = premium_ctx("natal_premium");

    assert!(ReadingQualityValidator::validate_for_product(&request, &reading, Some(&ctx)).is_err());
}

#[test]
fn premium_accepts_rich_reading() {
    let request = premium_request("natal_premium");
    let reading = good_reading();
    let ctx = premium_ctx("natal_premium");

    assert!(ReadingQualityValidator::validate_for_product(&request, &reading, Some(&ctx)).is_ok());
}

#[test]
fn chapter_orchestrated_without_profile_does_not_block() {
    let request = premium_request("natal_premium");

    assert!(!requires_blocking_quality_gate(&request, None));
}

#[test]
fn premium_plus_rejects_short_synthesis_chapter() {
    let ctx = premium_ctx("natal_premium_plus");
    let mut request = premium_request("natal_premium_plus");
    let mut reading = good_reading();
    let (synthesis_min_words, _, _) = ctx.profile.synthesis_word_targets();

    reading.chapters.push(ReadingChapter {
        code: SYNTHESIS_CHAPTER_CODE.into(),
        title: "Synthese".into(),
        body: "Court.".into(),
        astro_basis: vec![
            basis("dominant_planet:jupiter", "jupiter", "core"),
            basis("dominant_planet:sun", "sun", "core"),
            basis("dominant_planet:moon", "moon", "core"),
            basis("dominant_planet:venus", "venus", "supporting"),
        ],
        confidence: ConfidenceLevel::Medium,
        safety_flags: vec![],
    });

    request.product_context.interpretation_profile_code = Some("natal_premium_plus".into());
    let report = ReadingQualityValidator::assess(&request, &reading, Some(&ctx));

    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.contains("synthesis") && warning.contains("too short")),
        "expected synthesis warning, got {:?}",
        report.warnings
    );
    assert!(synthesis_min_words > 2);
}

#[test]
fn premium_profile_blocks_even_in_single_pass_mode() {
    let mut request = premium_request("natal_premium");
    request.response_contract.generation_mode = GenerationMode::SinglePass;
    let mut reading = good_reading();
    reading.chapters[0].body = "sun aries. moon cancer.".into();
    let ctx = premium_ctx("natal_premium");

    assert!(ReadingQualityValidator::validate_for_product(&request, &reading, Some(&ctx)).is_err());
}

#[test]
fn normalize_chapter_code_accepts_model_suffix_drift() {
    assert_eq!(
        normalize_chapter_code("emotional_life_natal_premium_v1", "emotional_life").as_deref(),
        Some("emotional_life")
    );
}

#[test]
fn normalize_chapter_code_rejects_unrelated_values() {
    assert!(normalize_chapter_code("career", "emotional_life").is_none());
}
