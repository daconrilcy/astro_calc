use astral_llm_domain::{EngineDefaults, ProviderKind, ServiceLimits};

use crate::canonical::service_limits_from_env;
use crate::url_validator::{
    validate_anthropic_base_url, validate_mistral_base_url, validate_openai_base_url,
};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind_addr: std::net::SocketAddr,
    pub database_url: Option<String>,
    pub prompts_dir: String,
    pub default_provider: ProviderKind,
    pub default_model: String,
    pub fallback_providers: Vec<ProviderKind>,
    pub enable_fake_provider: bool,
    pub enable_persistence: bool,
    pub openai_base_url: String,
    pub anthropic_base_url: String,
    pub mistral_base_url: String,
    pub api_key: Option<String>,
    pub limits: ServiceLimits,
}

impl AppConfig {
    pub fn from_env() -> Self {
        load_dotenv();

        let host = env_var("ASTRAL_LLM_HOST").unwrap_or_else(|| "127.0.0.1".into());
        let port: u16 = env_var("ASTRAL_LLM_PORT")
            .and_then(|v| v.parse().ok())
            .unwrap_or(8081);

        let prompts_dir =
            env_var("ASTRAL_LLM_PROMPTS_DIR").unwrap_or_else(|| "astral_llm/prompts".into());

        let default_provider = env_var("ASTRAL_LLM_DEFAULT_PROVIDER")
            .map(|v| parse_provider_kind(&v))
            .unwrap_or(ProviderKind::OpenAi);

        let default_model = env_var("ASTRAL_LLM_DEFAULT_MODEL")
            .or_else(|| env_var("OPENAI_DEFAULT_MODEL"))
            .unwrap_or_else(|| "gpt-4.1".into());

        let fallback_providers = env_var("ASTRAL_LLM_FALLBACK_PROVIDERS")
            .map(|v| parse_provider_list(&v))
            .unwrap_or_else(default_fallback_chain);

        let openai_base_url = env_var("OPENAI_BASE_URL")
            .unwrap_or_else(|| "https://api.openai.com".into());
        let anthropic_base_url = env_var("ANTHROPIC_BASE_URL")
            .unwrap_or_else(|| "https://api.anthropic.com".into());
        let mistral_base_url = env_var("MISTRAL_BASE_URL")
            .unwrap_or_else(|| "https://api.mistral.ai".into());

        validate_openai_base_url(&openai_base_url).expect("invalid OPENAI_BASE_URL");
        validate_anthropic_base_url(&anthropic_base_url).expect("invalid ANTHROPIC_BASE_URL");
        validate_mistral_base_url(&mistral_base_url).expect("invalid MISTRAL_BASE_URL");

        Self {
            bind_addr: format!("{host}:{port}")
                .parse()
                .expect("valid ASTRAL_LLM bind address"),
            database_url: env_var("DATABASE_URL"),
            prompts_dir,
            default_provider,
            default_model,
            fallback_providers,
            enable_fake_provider: env_bool("ASTRAL_LLM_ENABLE_FAKE", false),
            enable_persistence: env_bool("ASTRAL_LLM_ENABLE_PERSISTENCE", false),
            openai_base_url,
            anthropic_base_url,
            mistral_base_url,
            api_key: env_var("ASTRAL_LLM_API_KEY"),
            limits: service_limits_from_env(),
        }
    }

    pub fn engine_defaults(&self) -> EngineDefaults {
        EngineDefaults {
            provider: self.default_provider.clone(),
            model: self.default_model.clone(),
        }
    }

    pub fn requires_auth(&self) -> bool {
        self.api_key.as_ref().is_some_and(|k| !k.is_empty())
    }
}

pub fn load_dotenv() {
    dotenvy::dotenv().ok();
}

pub fn env_var(key: &str) -> Option<String> {
    let value = std::env::var(key).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn env_bool(key: &str, default: bool) -> bool {
    env_var(key)
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(default)
}

pub fn parse_provider_kind(raw: &str) -> ProviderKind {
    match raw.trim().to_lowercase().as_str() {
        "openai" | "open_ai" => ProviderKind::OpenAi,
        "anthropic" => ProviderKind::Anthropic,
        "mistral" => ProviderKind::Mistral,
        "fake" => ProviderKind::Fake,
        _ => ProviderKind::OpenAi,
    }
}

pub fn parse_provider_list(raw: &str) -> Vec<ProviderKind> {
    raw.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(parse_provider_kind)
        .collect()
}

fn default_fallback_chain() -> Vec<ProviderKind> {
    vec![
        ProviderKind::OpenAi,
        ProviderKind::Mistral,
        ProviderKind::Anthropic,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_provider_is_openai() {
        assert_eq!(parse_provider_kind("openai"), ProviderKind::OpenAi);
    }

    #[test]
    fn unknown_provider_maps_to_openai() {
        assert_eq!(parse_provider_kind("unknown_vendor"), ProviderKind::OpenAi);
    }
}
