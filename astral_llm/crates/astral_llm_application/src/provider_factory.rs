use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use astral_llm_domain::{FallbackPolicy, ProviderKind};
use astral_llm_infra::{AppConfig, ProviderSecrets};

use crate::model_capability_registry::ModelCapabilityRegistry;
use crate::provider_router::{build_http_client, build_provider_map};
use astral_llm_providers::{
    AnthropicProvider, FakeProvider, LlmProvider, MistralProvider, OpenAiProvider,
    SharedLlmProvider,
};

pub fn build_fallback_policy(config: &AppConfig) -> FallbackPolicy {
    config.fallback_policy.clone()
}

pub fn build_capability_registry() -> Arc<ModelCapabilityRegistry> {
    Arc::new(ModelCapabilityRegistry::bootstrap_dev_fallback())
}

pub fn build_capability_registry_with_db(
    active_provider_codes: Vec<String>,
    db_models: Vec<astral_llm_domain::ModelCapability>,
) -> Arc<ModelCapabilityRegistry> {
    Arc::new(ModelCapabilityRegistry::from_db_catalog(
        active_provider_codes,
        db_models,
    ))
}

pub fn build_providers(
    config: &AppConfig,
    secrets: &ProviderSecrets,
) -> Result<HashMap<ProviderKind, SharedLlmProvider>, String> {
    let http_timeout = Duration::from_millis(config.limits.default_request_timeout_ms);
    let client = build_http_client(http_timeout);

    let mut providers: Vec<Arc<dyn LlmProvider>> = Vec::new();
    let mut real_count = 0usize;

    if secrets.has_openai() {
        providers.push(Arc::new(OpenAiProvider::with_client(
            client.clone(),
            secrets.openai_api_key.clone().expect("openai key checked"),
            config.openai_base_url.clone(),
        )));
        real_count += 1;
    } else if config.default_provider == ProviderKind::OpenAi {
        tracing::warn!("OPENAI_API_KEY absent dans .env");
    }

    if secrets.has_anthropic() {
        providers.push(Arc::new(AnthropicProvider::with_client(
            client.clone(),
            secrets
                .anthropic_api_key
                .clone()
                .expect("anthropic key checked"),
            config.anthropic_base_url.clone(),
        )));
        real_count += 1;
    }

    if secrets.has_mistral() {
        providers.push(Arc::new(MistralProvider::with_client(
            client,
            secrets
                .mistral_api_key
                .clone()
                .expect("mistral key checked"),
            config.mistral_base_url.clone(),
        )));
        real_count += 1;
    }

    if config.enable_fake_provider {
        providers.push(Arc::new(FakeProvider));
    }

    if real_count == 0 && !config.enable_fake_provider {
        return Err(
            "no LLM provider configured: set OPENAI_API_KEY (or other provider keys) or enable fake (ASTRAL_LLM_ENV=local with ASTRAL_LLM_ENABLE_FAKE=true)".into(),
        );
    }

    if providers.is_empty() {
        return Err("no LLM providers available".into());
    }

    Ok(build_provider_map(providers))
}

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::ServiceLimits;

    fn test_config(enable_fake: bool) -> AppConfig {
        AppConfig {
            runtime_env: astral_llm_domain::AstralLlmEnv::Local,
            production_exposure: astral_llm_domain::ProductionExposureMode::Internal,
            bind_addr: "127.0.0.1:8081".parse().unwrap(),
            allow_public_bind: false,
            database_url: None,
            prompts_dir: "astral_llm/prompts".into(),
            default_provider: ProviderKind::Fake,
            default_model: "fake-model".into(),
            fallback_policy: FallbackPolicy::disabled(),
            enable_fake_provider: enable_fake,
            enable_persistence: false,
            db_auto_migrate: false,
            store_sanitized_payloads: false,
            openai_base_url: "https://api.openai.com".into(),
            anthropic_base_url: "https://api.anthropic.com".into(),
            mistral_base_url: "https://api.mistral.ai".into(),
            api_key: None,
            privacy_policy: astral_llm_domain::PrivacyPolicy::default(),
            limits: ServiceLimits::default(),
            max_concurrent_requests: 32,
            max_concurrent_requests_per_key: 8,
            max_requests_per_minute_per_key: 120,
            max_premium_runs_per_key: 4,
            idempotency_ttl_hours: 24,
            circuit_breaker_failure_threshold: 5,
            circuit_breaker_open_secs: 60,
        }
    }

    #[test]
    fn fails_without_keys_and_without_fake() {
        let config = test_config(false);
        let secrets = ProviderSecrets::default();
        assert!(build_providers(&config, &secrets).is_err());
    }

    #[test]
    fn fallback_policy_does_not_force_openai_first() {
        let mut config = test_config(true);
        config.fallback_policy = FallbackPolicy {
            enabled: true,
            chain: vec![ProviderKind::Mistral, ProviderKind::Anthropic],
            ..FallbackPolicy::default()
        };
        let policy = build_fallback_policy(&config);
        assert_eq!(policy.chain.first(), Some(&ProviderKind::Mistral));
    }
}
