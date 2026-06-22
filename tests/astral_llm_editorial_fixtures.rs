//! Fixtures redactionnelles de reference (non-regression qualite).

use std::path::PathBuf;

use astral_llm_application::editorial_validation::{EditorialFixtureSpec, EditorialValidator};
use astral_llm_domain::{
    generation_request::{AudienceLevel, GenerateReadingRequest, ProductContext},
    generation_response::NatalReadingResponse,
    output_contract::{GenerationMode, OutputFormat, ResponseContract},
    AstroCalculationPayload, AstrologerProfile, EngineParams,
};

#[derive(serde::Deserialize)]
struct EditorialFixtureFile {
    fixture_id: String,
    product_code: String,
    #[serde(default)]
    interpretation_profile_code: Option<String>,
    audience_level: String,
    user_language: String,
    generation_mode: String,
    reading: NatalReadingResponse,
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../tests/fixtures/astral_llm/editorial")
}

fn parse_audience(raw: &str) -> AudienceLevel {
    match raw {
        "expert" => AudienceLevel::Expert,
        "intermediate" => AudienceLevel::Intermediate,
        _ => AudienceLevel::Beginner,
    }
}

fn parse_mode(raw: &str) -> GenerationMode {
    match raw {
        "chapter_orchestrated" => GenerationMode::ChapterOrchestrated,
        _ => GenerationMode::SinglePass,
    }
}

fn request_from_fixture(file: &EditorialFixtureFile) -> GenerateReadingRequest {
    GenerateReadingRequest {
        request_id: Some(format!("fixture-{}", file.fixture_id)),
        idempotency_key: None,
        product_context: ProductContext {
            product_code: file.product_code.clone(),
            interpretation_profile_code: Some(
                file.interpretation_profile_code
                    .clone()
                    .expect("fixture must set interpretation_profile_code"),
            ),
            user_language: file.user_language.clone(),
            audience_level: parse_audience(&file.audience_level),
        },
        astro_result: AstroCalculationPayload {
            contract_version: "natal_structured_v14".into(),
            chart_type: "natal".into(),
            data: serde_json::json!({
                "domain_scores": {
                    "identity": 0.7,
                    "relationships": 0.6,
                    "vocation": 0.5
                }
            }),
        },
        astrologer_profile: AstrologerProfile {
            profile_id: None,
            name: None,
            tone: astral_llm_domain::ToneProfile::Warm,
            jargon_level: astral_llm_domain::JargonLevel::Beginner,
            wording_style: astral_llm_domain::WordingStyle::Clear,
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
            generation_mode: parse_mode(&file.generation_mode),
            format: OutputFormat::StructuredJson,
            chapters: vec![],
            global_max_tokens: None,
            include_astro_sources: true,
            include_legal_disclaimer: true,
        },
        safety_policy: None,
    }
}

fn load_fixture(name: &str) -> EditorialFixtureFile {
    let path = fixtures_dir().join(format!("{name}.json"));
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {path:?}: {e}"))
}

fn validate_named_fixture(name: &str) {
    let file = load_fixture(name);
    let spec = EditorialFixtureSpec {
        fixture_id: file.fixture_id.clone(),
        product_code: file.product_code.clone(),
        audience_level: parse_audience(&file.audience_level),
        user_language: file.user_language.clone(),
        generation_mode: parse_mode(&file.generation_mode),
    };
    let request = request_from_fixture(&file);
    EditorialValidator::validate_fixture(&spec, &request, &file.reading)
        .unwrap_or_else(|e| panic!("fixture {name} failed editorial validation: {e}"));
}

#[test]
fn natal_basic_beginner_fr_fixture() {
    validate_named_fixture("natal_basic_beginner_fr");
}

#[test]
fn natal_premium_psychological_fr_fixture() {
    validate_named_fixture("natal_premium_psychological_fr");
}

#[test]
fn natal_premium_traditional_en_fixture() {
    validate_named_fixture("natal_premium_traditional_en");
}

#[test]
fn rejects_cold_fact_list_fixture() {
    let file = load_fixture("natal_basic_beginner_fr");
    let spec = EditorialFixtureSpec {
        fixture_id: "negative_cold_list".into(),
        product_code: "natal_prompter".into(),
        audience_level: AudienceLevel::Beginner,
        user_language: "fr".into(),
        generation_mode: GenerationMode::SinglePass,
    };
    let mut file = file;
    file.product_code = "natal_prompter".into();
    file.interpretation_profile_code = Some("natal_light".into());
    let request = request_from_fixture(&file);
    let mut reading = file.reading;
    reading.chapters[0].body = "Soleil en Belier. Lune en Cancer. Mars carre Saturne.".into();
    assert!(EditorialValidator::validate_fixture(&spec, &request, &reading).is_err());
}
