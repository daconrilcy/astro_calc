use std::sync::Arc;
use std::time::Duration;

use astral_llm_domain::ProviderKind;
use astral_llm_infra::{AppConfig, ProviderSecrets};

use crate::provider_router::{build_http_client, build_provider_map, FallbackPolicy};
use astral_llm_providers::{
    AnthropicProvider, FakeProvider, LlmProvider, MistralProvider, OpenAiProvider,
    SharedLlmProvider,
};

use std::collections::HashMap;

pub fn build_fallback_policy(config: &AppConfig) -> FallbackPolicy {
    let mut fallback_order = config.fallback_providers.clone();

    if config.enable_fake_provider && !fallback_order.contains(&ProviderKind::Fake) {
        fallback_order.push(ProviderKind::Fake);
    }

    ensure_openai_first(&mut fallback_order);

    FallbackPolicy {
        fallback_order,
        max_retries: 1,
    }
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
            secrets.anthropic_api_key.clone().expect("anthropic key checked"),
            config.anthropic_base_url.clone(),
        )));
        real_count += 1;
    }

    if secrets.has_mistral() {
        providers.push(Arc::new(MistralProvider::with_client(
            client,
            secrets.mistral_api_key.clone().expect("mistral key checked"),
            config.mistral_base_url.clone(),
        )));
        real_count += 1;
    }

    if config.enable_fake_provider {
        providers.push(Arc::new(FakeProvider));
    }

    if real_count == 0 && !config.enable_fake_provider {
        return Err(
            "no LLM provider configured: set OPENAI_API_KEY (or other provider keys) or ASTRAL_LLM_ENABLE_FAKE=true".into(),
        );
    }

    if providers.is_empty() {
        return Err("no LLM providers available".into());
    }

    Ok(build_provider_map(providers))
}

fn ensure_openai_first(order: &mut Vec<ProviderKind>) {
    if let Some(index) = order.iter().position(|kind| *kind == ProviderKind::OpenAi) {
        if index != 0 {
            order.remove(index);
            order.insert(0, ProviderKind::OpenAi);
        }
    } else {
        order.insert(0, ProviderKind::OpenAi);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::ServiceLimits;

    fn test_config(enable_fake: bool) -> AppConfig {
        AppConfig {
            bind_addr: "127.0.0.1:8081".parse().unwrap(),
            database_url: None,
            prompts_dir: "astral_llm/prompts".into(),
            default_provider: ProviderKind::OpenAi,
            default_model: "gpt-4.1".into(),
            fallback_providers: vec![ProviderKind::OpenAi],
            enable_fake_provider: enable_fake,
            enable_persistence: false,
            openai_base_url: "https://api.openai.com".into(),
            anthropic_base_url: "https://api.anthropic.com".into(),
            mistral_base_url: "https://api.mistral.ai".into(),
            api_key: None,
            limits: ServiceLimits::default(),
        }
    }

    #[test]
    fn fails_without_keys_and_without_fake() {
        let config = test_config(false);
        let secrets = ProviderSecrets::default();
        assert!(build_providers(&config, &secrets).is_err());
    }
}
