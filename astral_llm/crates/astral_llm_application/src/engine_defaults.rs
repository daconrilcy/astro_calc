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
    if let Some(model) = policy.default_model.as_ref().and_then(|m| trimmed_model(m)) {
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
