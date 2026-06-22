use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use astral_llm_domain::{FallbackPolicy, ProviderKind};
use secrecy::{ExposeSecret, SecretString};

use crate::model_capability_registry::ModelCapabilityRegistry;
use crate::provider_router::{build_http_client, build_provider_map};
use astral_llm_providers::{
    AnthropicProvider, FakeProvider, LlmProvider, MistralProvider, OpenAiProvider,
    SharedLlmProvider,
};

#[derive(Debug, Clone)]
pub struct ProviderBootstrapConfig {
    pub default_provider: ProviderKind,
    pub fallback_policy: FallbackPolicy,
    pub enable_fake_provider: bool,
    pub default_request_timeout_ms: u64,
    pub openai_base_url: String,
    pub anthropic_base_url: String,
    pub mistral_base_url: String,
}

#[derive(Debug, Clone, Default)]
pub struct ProviderBootstrapSecrets {
    pub openai_api_key: Option<SecretString>,
    pub anthropic_api_key: Option<SecretString>,
    pub mistral_api_key: Option<SecretString>,
}

impl ProviderBootstrapSecrets {
    pub fn has_openai(&self) -> bool {
        secret_is_set(self.openai_api_key.as_ref())
    }

    pub fn has_anthropic(&self) -> bool {
        secret_is_set(self.anthropic_api_key.as_ref())
    }

    pub fn has_mistral(&self) -> bool {
        secret_is_set(self.mistral_api_key.as_ref())
    }
}

pub fn build_fallback_policy(config: &ProviderBootstrapConfig) -> FallbackPolicy {
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
    config: &ProviderBootstrapConfig,
    secrets: &ProviderBootstrapSecrets,
) -> Result<HashMap<ProviderKind, SharedLlmProvider>, String> {
    let http_timeout = Duration::from_millis(config.default_request_timeout_ms);
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

fn secret_is_set(secret: Option<&SecretString>) -> bool {
    secret
        .map(|value| !value.expose_secret().trim().is_empty())
        .unwrap_or(false)
}
