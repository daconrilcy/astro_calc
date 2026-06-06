use astral_llm_domain::{
    astrologer_profile::{JargonLevel, ToneProfile, WordingStyle},
    engine_params::EngineParams,
    generation_request::{AudienceLevel, GenerateReadingRequest, ProductContext},
    interpretation_profile::NATAL_PROMPTER_PRODUCT,
    output_contract::{GenerationMode, OutputFormat, ResponseContract},
    AstroCalculationPayload, AstrologerProfile, GenerationError, GenerationErrorCode, ProviderKind,
};
use serde_json::Value;

const DEFAULT_PREFERRED_DOMAINS: &[&str] = &[
    "identity",
    "emotional_life",
    "relationships",
    "career",
    "growth_path",
];

pub fn validate_engine_response(engine: &Value) -> Result<(), GenerationError> {
    for key in [
        "response_contract_version",
        "calculation_result",
        "audit_payload",
    ] {
        if engine.get(key).is_none() {
            return Err(GenerationError::new(
                GenerationErrorCode::InvalidInput,
                format!("engine response missing {key}"),
            ));
        }
    }
    let version = engine
        .get("response_contract_version")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if version != "astro_engine_response_v1" {
        return Err(GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            format!("unsupported response_contract_version: {version}"),
            Value::Null,
        ));
    }
    let status = engine
        .pointer("/calculation_result/status")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if status != "completed" {
        return Err(GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            format!("engine calculation not completed: {status}"),
            engine
                .pointer("/calculation_result")
                .cloned()
                .unwrap_or(Value::Null),
        ));
    }
    if engine
        .pointer("/audit_payload/contract_version")
        .and_then(|v| v.as_str())
        .is_none()
    {
        return Err(GenerationError::new(
            GenerationErrorCode::InvalidInput,
            "audit_payload.contract_version is required",
        ));
    }
    if engine.pointer("/audit_payload/payload").is_none() {
        return Err(GenerationError::new(
            GenerationErrorCode::InvalidInput,
            "audit_payload.payload is required",
        ));
    }
    Ok(())
}

pub fn build_reading_request_from_engine(
    engine: &Value,
    profile_code: &str,
    user_language: &str,
    audience_level: AudienceLevel,
    provider: Option<&str>,
    model: Option<&str>,
) -> Result<GenerateReadingRequest, GenerationError> {
    validate_engine_response(engine)?;

    let contract_version = engine
        .pointer("/audit_payload/contract_version")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let payload = engine
        .pointer("/audit_payload/payload")
        .cloned()
        .unwrap_or(Value::Null);

    let mut engine_params = EngineParams {
        allow_fallback: true,
        ..EngineParams::default()
    };
    if let Some(p) = provider {
        engine_params.provider = parse_provider(p);
    }
    if let Some(m) = model {
        engine_params.model = Some(m.to_string());
    }

    Ok(GenerateReadingRequest {
        request_id: engine
            .get("request_id")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        idempotency_key: None,
        product_context: ProductContext {
            product_code: NATAL_PROMPTER_PRODUCT.to_string(),
            interpretation_profile_code: Some(profile_code.to_string()),
            user_language: user_language.to_string(),
            audience_level,
        },
        astro_result: AstroCalculationPayload {
            contract_version,
            chart_type: "natal".to_string(),
            data: payload,
        },
        astrologer_profile: AstrologerProfile {
            profile_id: None,
            name: None,
            tone: ToneProfile::Warm,
            jargon_level: JargonLevel::Beginner,
            wording_style: WordingStyle::Clear,
            preferred_domains: DEFAULT_PREFERRED_DOMAINS
                .iter()
                .map(|d| d.to_string())
                .collect(),
            forbidden_wording: vec![],
            custom_instructions: None,
        },
        engine: engine_params,
        response_contract: ResponseContract {
            output_schema_version: "natal_reading_v1".to_string(),
            generation_mode: GenerationMode::ChapterOrchestrated,
            format: OutputFormat::StructuredJson,
            chapters: vec![],
            global_max_tokens: None,
            include_astro_sources: true,
            include_legal_disclaimer: true,
        },
        safety_policy: None,
    })
}

fn parse_provider(raw: &str) -> Option<ProviderKind> {
    match raw.trim().to_lowercase().as_str() {
        "openai" | "open_ai" => Some(ProviderKind::OpenAi),
        "anthropic" => Some(ProviderKind::Anthropic),
        "mistral" => Some(ProviderKind::Mistral),
        "fake" => Some(ProviderKind::Fake),
        _ => None,
    }
}
