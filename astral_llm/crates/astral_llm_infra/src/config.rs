use astral_llm_domain::{
    AstralLlmEnv, EngineDefaults, FallbackPolicy, PrivacyPolicy, ProductionExposureMode,
    ProviderKind, ServiceLimits,
};

use crate::canonical::service_limits_from_env;
use crate::url_validator::{
    validate_anthropic_base_url, validate_mistral_base_url, validate_openai_base_url,
};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub runtime_env: AstralLlmEnv,
    pub production_exposure: ProductionExposureMode,
    pub bind_addr: std::net::SocketAddr,
    pub allow_public_bind: bool,
    pub database_url: Option<String>,
    pub prompts_dir: String,
    pub default_provider: ProviderKind,
    pub default_model: String,
    pub fallback_policy: FallbackPolicy,
    pub enable_fake_provider: bool,
    pub enable_persistence: bool,
    pub db_auto_migrate: bool,
    pub store_sanitized_payloads: bool,
    pub openai_base_url: String,
    pub anthropic_base_url: String,
    pub mistral_base_url: String,
    pub api_key: Option<String>,
    pub privacy_policy: PrivacyPolicy,
    pub limits: ServiceLimits,
    pub max_concurrent_requests: usize,
    pub max_concurrent_requests_per_key: usize,
    pub max_requests_per_minute_per_key: u32,
    pub max_premium_runs_per_key: u32,
    pub idempotency_ttl_hours: i64,
    pub circuit_breaker_failure_threshold: u32,
    pub circuit_breaker_open_secs: u64,
}

impl AppConfig {
    pub fn from_env() -> Self {
        load_dotenv();

        let runtime_env = env_var("ASTRAL_LLM_ENV")
            .map(|v| AstralLlmEnv::parse(&v))
            .unwrap_or(AstralLlmEnv::Local);

        let host = env_var("ASTRAL_LLM_HOST").unwrap_or_else(|| default_host(&runtime_env));
        let port: u16 = env_var("ASTRAL_LLM_PORT")
            .and_then(|v| v.parse().ok())
            .unwrap_or(8081);

        let prompts_dir =
            env_var("ASTRAL_LLM_PROMPTS_DIR").unwrap_or_else(|| "astral_llm/prompts".into());

        let enable_fake_provider = env_bool(
            "ASTRAL_LLM_ENABLE_FAKE",
            runtime_env.allows_fake_by_default(),
        );

        let default_provider = env_var("ASTRAL_LLM_DEFAULT_PROVIDER")
            .map(|v| parse_provider_kind(&v))
            .unwrap_or_else(|| default_provider_for(&runtime_env, enable_fake_provider));

        let default_model = env_var("ASTRAL_LLM_DEFAULT_MODEL")
            .or_else(|| env_var("OPENAI_DEFAULT_MODEL"))
            .unwrap_or_else(|| default_model_for(&default_provider));

        let fallback_policy = build_fallback_policy_from_env(&default_provider);

        let openai_base_url =
            env_var("OPENAI_BASE_URL").unwrap_or_else(|| "https://api.openai.com".into());
        let anthropic_base_url =
            env_var("ANTHROPIC_BASE_URL").unwrap_or_else(|| "https://api.anthropic.com".into());
        let mistral_base_url =
            env_var("MISTRAL_BASE_URL").unwrap_or_else(|| "https://api.mistral.ai".into());

        validate_openai_base_url(&openai_base_url).expect("invalid OPENAI_BASE_URL");
        validate_anthropic_base_url(&anthropic_base_url).expect("invalid ANTHROPIC_BASE_URL");
        validate_mistral_base_url(&mistral_base_url).expect("invalid MISTRAL_BASE_URL");

        let allow_public_bind = env_bool("ASTRAL_LLM_ALLOW_PUBLIC_BIND", false);
        let production_exposure = env_var("ASTRAL_LLM_PRODUCTION_MODE")
            .map(|v| ProductionExposureMode::parse(&v))
            .unwrap_or_else(|| {
                if runtime_env.is_production() && allow_public_bind {
                    ProductionExposureMode::Public
                } else {
                    ProductionExposureMode::Internal
                }
            });
        let enable_persistence = env_bool("ASTRAL_LLM_ENABLE_PERSISTENCE", false);
        let db_auto_migrate = env_bool(
            "ASTRAL_LLM_DB_AUTO_MIGRATE",
            runtime_env != AstralLlmEnv::Production,
        );
        let store_sanitized_payloads = env_bool("ASTRAL_LLM_STORE_SANITIZED_PAYLOADS", false);

        Self {
            runtime_env,
            production_exposure,
            bind_addr: format!("{host}:{port}")
                .parse()
                .expect("valid ASTRAL_LLM bind address"),
            allow_public_bind,
            database_url: env_var("DATABASE_URL"),
            prompts_dir,
            default_provider,
            default_model,
            fallback_policy,
            enable_fake_provider,
            enable_persistence,
            db_auto_migrate,
            store_sanitized_payloads,
            openai_base_url,
            anthropic_base_url,
            mistral_base_url,
            api_key: env_var("ASTRAL_LLM_API_KEY"),
            privacy_policy: PrivacyPolicy::for_env(runtime_env.is_production()),
            limits: service_limits_from_env(),
            max_concurrent_requests: env_var("ASTRAL_LLM_MAX_CONCURRENT_REQUESTS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(32),
            max_concurrent_requests_per_key: env_var("ASTRAL_LLM_MAX_CONCURRENT_REQUESTS_PER_KEY")
                .and_then(|v| v.parse().ok())
                .unwrap_or(8),
            max_requests_per_minute_per_key: env_var("ASTRAL_LLM_MAX_REQUESTS_PER_MINUTE_PER_KEY")
                .and_then(|v| v.parse().ok())
                .unwrap_or(120),
            max_premium_runs_per_key: env_var("ASTRAL_LLM_MAX_PREMIUM_RUNS_PER_KEY")
                .and_then(|v| v.parse().ok())
                .unwrap_or(4),
            idempotency_ttl_hours: env_var("ASTRAL_LLM_IDEMPOTENCY_TTL_HOURS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(24),
            circuit_breaker_failure_threshold: env_var("ASTRAL_LLM_CIRCUIT_BREAKER_FAILURES")
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            circuit_breaker_open_secs: env_var("ASTRAL_LLM_CIRCUIT_BREAKER_OPEN_SECS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
        }
    }

    /// Production exposee (public ou bind 0.0.0.0 autorise) : audit PostgreSQL obligatoire.
    pub fn requires_strict_persistence(&self) -> bool {
        self.runtime_env.is_production()
            && (self.production_exposure.is_public() || self.allow_public_bind)
    }

    pub fn is_public_exposure(&self) -> bool {
        self.requires_strict_persistence()
    }

    pub fn engine_defaults(&self) -> EngineDefaults {
        EngineDefaults {
            provider: self.default_provider.clone(),
            model: self.default_model.clone(),
        }
    }

    pub fn requires_auth(&self) -> bool {
        self.runtime_env.requires_api_key() || self.api_key.as_ref().is_some_and(|k| !k.is_empty())
    }
}

fn default_host(env: &AstralLlmEnv) -> String {
    if env.is_production() {
        "127.0.0.1".into()
    } else {
        "127.0.0.1".into()
    }
}

fn default_provider_for(env: &AstralLlmEnv, fake_enabled: bool) -> ProviderKind {
    if fake_enabled && !env.is_production() {
        ProviderKind::Fake
    } else {
        ProviderKind::OpenAi
    }
}

fn default_model_for(provider: &ProviderKind) -> String {
    match provider {
        ProviderKind::Fake => "fake-model".into(),
        ProviderKind::Anthropic => "claude-sonnet-4-20250514".into(),
        ProviderKind::Mistral => "mistral-large-latest".into(),
        _ => "gpt-5.4-mini".into(),
    }
}

fn build_fallback_policy_from_env(default_provider: &ProviderKind) -> FallbackPolicy {
    let enabled = env_bool("ASTRAL_LLM_FALLBACK_ENABLED", true);
    let chain = env_var("ASTRAL_LLM_FALLBACK_PROVIDERS")
        .map(|v| parse_provider_list(&v))
        .unwrap_or_else(|| Vec::new());

    let chain = if chain.is_empty() { Vec::new() } else { chain };

    let allow_cross = env_bool("ASTRAL_LLM_ALLOW_CROSS_PROVIDER_FALLBACK", false);

    let policy = FallbackPolicy {
        enabled,
        chain,
        fallback_on: vec![
            astral_llm_domain::FallbackReason::Timeout,
            astral_llm_domain::FallbackReason::RateLimited,
            astral_llm_domain::FallbackReason::ProviderUnavailable,
        ],
        require_same_structured_output_level: true,
        allow_cross_vendor_data_transfer: allow_cross,
        max_retries_per_provider: env_var("ASTRAL_LLM_FALLBACK_MAX_RETRIES")
            .and_then(|v| v.parse().ok())
            .unwrap_or(1),
    };

    if policy.chain.is_empty() && policy.enabled {
        tracing::info!(
            default_provider = default_provider.as_str(),
            "no ASTRAL_LLM_FALLBACK_PROVIDERS configured; fallback uses only requested provider"
        );
    }

    policy
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
        other if !other.is_empty() => ProviderKind::Custom(other.to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_fake_provider() {
        assert_eq!(parse_provider_kind("fake"), ProviderKind::Fake);
    }

    #[test]
    fn empty_fallback_chain_when_unset() {
        let policy = FallbackPolicy {
            enabled: true,
            chain: vec![],
            ..FallbackPolicy::default()
        };
        let chain = policy.candidate_chain(&ProviderKind::OpenAi, true);
        assert_eq!(chain, vec![ProviderKind::OpenAi]);
    }
}
