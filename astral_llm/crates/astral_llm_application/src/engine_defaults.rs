use astral_llm_domain::{engine_params::EngineParams, EngineDefaults};

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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::provider::ProviderKind;

    #[test]
    fn applies_openai_defaults_when_missing() {
        let params = EngineParams {
            provider: None,
            model: None,
            reasoning_effort: None,
            temperature: None,
            max_output_tokens: None,
            domain_count: None,
            allow_fallback: true,
            timeout_ms: None,
        };
        let defaults = EngineDefaults {
            provider: ProviderKind::OpenAi,
            model: "gpt-4.1".into(),
        };

        let resolved = resolve_engine_params(&params, &defaults, 60_000);
        assert_eq!(resolved.provider, ProviderKind::OpenAi);
        assert_eq!(resolved.model, "gpt-4.1");
        assert_eq!(resolved.timeout_ms, Some(60_000));
    }
}
