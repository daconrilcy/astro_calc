use astral_llm_domain::{
    engine_params::EngineParams, interpretation_profile::NATAL_PROMPTER_PRODUCT, EngineDefaults,
    GenerateReadingRequest,
};
use astral_llm_infra::CanonicalCatalog;

#[derive(Debug, Clone)]
pub struct ResolvedEngineParams {
    pub provider: astral_llm_domain::ProviderKind,
    pub model: String,
    pub reasoning_effort: Option<astral_llm_domain::ReasoningEffort>,
    pub temperature: Option<f32>,
    pub max_output_tokens: Option<u32>,
    pub domain_count: Option<u8>,
    pub allow_fallback: bool,
    pub timeout_ms: Option<u64>,
    pub allow_oracle_benchmark: bool,
}

/// Fusionne les defauts service (.env), la politique produit et le profil d'interpretation.
pub fn resolve_service_engine_defaults(
    global: &EngineDefaults,
    catalog: &CanonicalCatalog,
    request: &GenerateReadingRequest,
) -> EngineDefaults {
    let product_code = request.product_context.product_code.as_str();
    let mut out = global.clone();

    let policy = if product_code == NATAL_PROMPTER_PRODUCT {
        request
            .product_context
            .interpretation_profile_code
            .as_deref()
            .and_then(|code| catalog.interpretation_profile(code))
            .map(|p| p.to_product_generation_policy())
            .or_else(|| catalog.product_policy(product_code).cloned())
    } else {
        catalog.product_policy(product_code).cloned()
    };

    let Some(policy) = policy else {
        return out;
    };

    if let Some(provider) = policy.default_provider.clone() {
        out.provider = provider;
    }
    if let Some(model) = policy
        .default_model
        .as_ref()
        .and_then(|m| trimmed_model(m))
    {
        out.model = model;
    }
    out
}

pub fn resolve_engine_params(
    params: &EngineParams,
    defaults: &EngineDefaults,
    default_timeout_ms: u64,
) -> ResolvedEngineParams {
    ResolvedEngineParams {
        provider: params
            .provider
            .clone()
            .unwrap_or_else(|| defaults.provider.clone()),
        model: params
            .model
            .as_ref()
            .map(|m| m.trim())
            .filter(|m| !m.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| defaults.model.clone()),
        reasoning_effort: params.reasoning_effort,
        temperature: params.temperature,
        max_output_tokens: params.max_output_tokens,
        domain_count: params.domain_count,
        allow_fallback: params.allow_fallback,
        timeout_ms: Some(params.timeout_ms.unwrap_or(default_timeout_ms)),
        allow_oracle_benchmark: params.allow_oracle_benchmark,
    }
}

pub fn drop_unsupported_reasoning(
    engine: &mut ResolvedEngineParams,
    registry: &crate::model_capability_registry::ModelCapabilityRegistry,
) {
    let Some(effort) = engine.reasoning_effort else {
        return;
    };
    let Ok(cap) = registry.require(&engine.provider, &engine.model) else {
        return;
    };
    if !cap.allows_reasoning(effort) {
        tracing::info!(
            provider = engine.provider.as_str(),
            model = %engine.model,
            ?effort,
            "dropping unsupported reasoning_effort for model"
        );
        engine.reasoning_effort = None;
    }
}

fn request_specified_primary_model(params: &EngineParams) -> bool {
    params
        .model
        .as_ref()
        .map(|m| !m.trim().is_empty())
        .unwrap_or(false)
}

fn trimmed_model(model: &str) -> Option<String> {
    let m = model.trim();
    if m.is_empty() {
        None
    } else {
        Some(m.to_string())
    }
}

/// Moteur pour SummarySynthesizer : `engine.summary_model`, sinon `economic_model` politique,
/// sinon le moteur chapitres (si la requete n'a pas fixe `engine.model`).
pub fn resolve_subtask_engine(
    chapter_engine: &ResolvedEngineParams,
    request_engine: &EngineParams,
    product_policy: Option<&astral_llm_domain::ProductGenerationPolicy>,
) -> ResolvedEngineParams {
    let mut out = chapter_engine.clone();

    if let Some(model) = request_engine
        .summary_model
        .as_ref()
        .and_then(|m| trimmed_model(m))
    {
        out.model = model;
        return out;
    }

    if request_specified_primary_model(request_engine) {
        return out;
    }

    if let Some(policy) = product_policy {
        if let Some(model) = policy
            .economic_model
            .as_ref()
            .and_then(|m| trimmed_model(m))
        {
            out.model = model;
        }
    }

    out
}

pub fn drop_unsupported_temperature(
    engine: &mut ResolvedEngineParams,
    registry: &crate::model_capability_registry::ModelCapabilityRegistry,
) {
    if engine.temperature.is_none() {
        return;
    }
    let Ok(cap) = registry.require(&engine.provider, &engine.model) else {
        return;
    };
    if !cap.supports_temperature {
        tracing::info!(
            provider = engine.provider.as_str(),
            model = %engine.model,
            "dropping unsupported temperature for model"
        );
        engine.temperature = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::{
        astrologer_profile::{JargonLevel, ToneProfile, WordingStyle},
        generation_request::{AudienceLevel, ProductContext},
        output_contract::{GenerationMode, OutputFormat, ResponseContract},
        provider::ProviderKind,
        AstroCalculationPayload, AstrologerProfile, GenerateReadingRequest, ProductGenerationPolicy,
    };

    fn minimal_request(product_code: &str, profile: Option<&str>) -> GenerateReadingRequest {
        GenerateReadingRequest {
            request_id: None,
            idempotency_key: None,
            product_context: ProductContext {
                product_code: product_code.into(),
                interpretation_profile_code: profile.map(str::to_string),
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
            engine: EngineParams::default(),
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
        }
    }

    #[test]
    fn applies_openai_defaults_when_missing() {
        let params = EngineParams {
            allow_fallback: true,
            ..Default::default()
        };
        let defaults = EngineDefaults {
            provider: ProviderKind::OpenAi,
            model: "gpt-5.4-mini".into(),
        };

        let resolved = resolve_engine_params(&params, &defaults, 60_000);
        assert_eq!(resolved.provider, ProviderKind::OpenAi);
        assert_eq!(resolved.model, "gpt-5.4-mini");
        assert_eq!(resolved.timeout_ms, Some(60_000));
    }

    #[test]
    fn product_default_overrides_global_model_when_request_omits_model() {
        let mut catalog = CanonicalCatalog::default();
        catalog.interpretation_profiles = astral_llm_infra::bootstrap_interpretation_profiles();
        catalog
            .product_generation_policies
            .push(ProductGenerationPolicy {
                product_code: "natal_prompter".into(),
                default_provider: Some(ProviderKind::OpenAi),
                default_model: Some("gpt-5.4-mini".into()),
                ..ProductGenerationPolicy::bootstrap_natal_prompter()
            });
        let global = EngineDefaults {
            provider: ProviderKind::OpenAi,
            model: "gpt-4.1".into(),
        };
        let merged = resolve_service_engine_defaults(
            &global,
            &catalog,
            &minimal_request("natal_prompter", Some("natal_premium")),
        );
        assert_eq!(merged.model, "gpt-5.4-mini");

        let params = EngineParams {
            allow_fallback: true,
            ..Default::default()
        };
        let resolved = resolve_engine_params(&params, &merged, 60_000);
        assert_eq!(resolved.model, "gpt-5.4-mini");
    }

    #[test]
    fn explicit_request_model_overrides_product_default() {
        let mut catalog = CanonicalCatalog::default();
        catalog.interpretation_profiles = astral_llm_infra::bootstrap_interpretation_profiles();
        let merged = resolve_service_engine_defaults(
            &EngineDefaults {
                provider: ProviderKind::OpenAi,
                model: "gpt-4.1".into(),
            },
            &catalog,
            &minimal_request("natal_prompter", Some("natal_premium")),
        );
        let params = EngineParams {
            model: Some("gpt-5.5".into()),
            allow_fallback: true,
            ..Default::default()
        };
        let resolved = resolve_engine_params(&params, &merged, 60_000);
        assert_eq!(resolved.model, "gpt-5.5");
    }

    #[test]
    fn subtask_uses_economic_model_when_primary_not_specified() {
        let policy = ProductGenerationPolicy {
            product_code: "natal_prompter".into(),
            default_model: Some("gpt-5.4-mini".into()),
            economic_model: Some("gpt-5-mini".into()),
            ..ProductGenerationPolicy::bootstrap_natal_prompter()
        };
        let chapter = resolve_engine_params(
            &EngineParams {
                allow_fallback: true,
                ..Default::default()
            },
            &EngineDefaults {
                provider: ProviderKind::OpenAi,
                model: "gpt-5.4-mini".into(),
            },
            60_000,
        );
        let summary = resolve_subtask_engine(
            &chapter,
            &EngineParams {
                allow_fallback: true,
                ..Default::default()
            },
            Some(&policy),
        );
        assert_eq!(summary.model, "gpt-5-mini");
    }

    #[test]
    fn subtask_keeps_explicit_primary_model_for_benchmark() {
        let policy = ProductGenerationPolicy {
            product_code: "natal_prompter".into(),
            economic_model: Some("gpt-5-mini".into()),
            ..ProductGenerationPolicy::bootstrap_natal_prompter()
        };
        let chapter = resolve_engine_params(
            &EngineParams {
                model: Some("gpt-5.4".into()),
                allow_fallback: true,
                ..Default::default()
            },
            &EngineDefaults {
                provider: ProviderKind::OpenAi,
                model: "gpt-5.4-mini".into(),
            },
            60_000,
        );
        let summary = resolve_subtask_engine(
            &chapter,
            &EngineParams {
                model: Some("gpt-5.4".into()),
                allow_fallback: true,
                ..Default::default()
            },
            Some(&policy),
        );
        assert_eq!(summary.model, "gpt-5.4");
    }

    #[test]
    fn summary_model_overrides_economic_routing() {
        let policy = ProductGenerationPolicy {
            product_code: "natal_prompter".into(),
            economic_model: Some("gpt-5-mini".into()),
            ..ProductGenerationPolicy::bootstrap_natal_prompter()
        };
        let chapter = resolve_engine_params(
            &EngineParams {
                allow_fallback: true,
                ..Default::default()
            },
            &EngineDefaults {
                provider: ProviderKind::OpenAi,
                model: "gpt-5.4-mini".into(),
            },
            60_000,
        );
        let summary = resolve_subtask_engine(
            &chapter,
            &EngineParams {
                summary_model: Some("gpt-5-nano".into()),
                allow_fallback: true,
                ..Default::default()
            },
            Some(&policy),
        );
        assert_eq!(summary.model, "gpt-5-nano");
    }

    #[test]
    fn profile_chapter_models_override_product_default() {
        let catalog = CanonicalCatalog {
            interpretation_profiles: astral_llm_infra::bootstrap_interpretation_profiles(),
            ..Default::default()
        };
        let merged = resolve_service_engine_defaults(
            &EngineDefaults {
                provider: ProviderKind::OpenAi,
                model: "gpt-4.1".into(),
            },
            &catalog,
            &minimal_request("natal_prompter", Some("natal_premium")),
        );
        assert_eq!(merged.model, "gpt-5.4-mini");
    }
}
