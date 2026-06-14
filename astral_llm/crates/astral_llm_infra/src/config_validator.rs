use astral_llm_domain::ProviderKind;

use crate::config::AppConfig;
use crate::secrets::ProviderSecrets;

#[derive(Debug, thiserror::Error)]
pub enum ConfigValidationError {
    #[error("{0}")]
    Message(String),
}

pub struct ConfigValidator;

impl ConfigValidator {
    pub fn validate(
        config: &AppConfig,
        secrets: &ProviderSecrets,
    ) -> Result<(), ConfigValidationError> {
        if config.runtime_env.is_production() {
            if config.enable_fake_provider {
                return Err(ConfigValidationError::Message(
                    "ASTRAL_LLM_ENABLE_FAKE=true is forbidden in production".into(),
                ));
            }

            if config.api_key.as_ref().is_none_or(|k| k.is_empty()) {
                return Err(ConfigValidationError::Message(
                    "ASTRAL_LLM_API_KEY is required in production".into(),
                ));
            }

            if !secrets.has_any_real_provider() {
                return Err(ConfigValidationError::Message(
                    "production requires at least one provider API key (OPENAI_API_KEY, ANTHROPIC_API_KEY, or MISTRAL_API_KEY)".into(),
                ));
            }

            if config.bind_addr.ip().is_unspecified() && !config.allow_public_bind {
                return Err(ConfigValidationError::Message(
                    "binding to 0.0.0.0 in production requires ASTRAL_LLM_ALLOW_PUBLIC_BIND=true"
                        .into(),
                ));
            }

            if config.db_auto_migrate {
                return Err(ConfigValidationError::Message(
                    "ASTRAL_LLM_DB_AUTO_MIGRATE=true is forbidden in production; apply SQL migrations explicitly".into(),
                ));
            }

            if config.requires_strict_persistence() {
                if !config.enable_persistence {
                    return Err(ConfigValidationError::Message(
                        "production public exposure requires ASTRAL_LLM_ENABLE_PERSISTENCE=true (idempotency and audit depend on PostgreSQL)".into(),
                    ));
                }
                if config.database_url.is_none() {
                    return Err(ConfigValidationError::Message(
                        "production public exposure requires DATABASE_URL".into(),
                    ));
                }
            }
        }

        if config.enable_persistence && config.database_url.is_none() {
            return Err(ConfigValidationError::Message(
                "ASTRAL_LLM_ENABLE_PERSISTENCE=true requires DATABASE_URL".into(),
            ));
        }

        if config.max_concurrent_requests == 0 {
            return Err(ConfigValidationError::Message(
                "ASTRAL_LLM_MAX_CONCURRENT_REQUESTS must be greater than 0".into(),
            ));
        }

        if config.max_concurrent_requests_per_key == 0 {
            return Err(ConfigValidationError::Message(
                "ASTRAL_LLM_MAX_CONCURRENT_REQUESTS_PER_KEY must be greater than 0".into(),
            ));
        }

        if config.max_requests_per_minute_per_key == 0 {
            return Err(ConfigValidationError::Message(
                "ASTRAL_LLM_MAX_REQUESTS_PER_MINUTE_PER_KEY must be greater than 0".into(),
            ));
        }

        if config.runtime_env.is_production() && config.default_provider == ProviderKind::Fake {
            return Err(ConfigValidationError::Message(
                "ASTRAL_LLM_DEFAULT_PROVIDER=fake is forbidden in production".into(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::{AstralLlmEnv, ServiceLimits};

    fn base_config(env: AstralLlmEnv) -> AppConfig {
        AppConfig {
            runtime_env: env,
            production_exposure: astral_llm_domain::ProductionExposureMode::Internal,
            bind_addr: "127.0.0.1:8081".parse().unwrap(),
            allow_public_bind: false,
            database_url: None,
            prompts_dir: "astral_llm/prompts".into(),
            default_provider: ProviderKind::Fake,
            default_model: "fake-model".into(),
            fallback_policy: astral_llm_domain::FallbackPolicy::disabled(),
            enable_fake_provider: true,
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
            enable_legacy_product_code_shim: true,
            legacy_product_code_shim_cutoff_date: None,
        }
    }

    #[test]
    fn local_allows_fake_without_keys() {
        let config = base_config(AstralLlmEnv::Local);
        let secrets = ProviderSecrets::default();
        assert!(ConfigValidator::validate(&config, &secrets).is_ok());
    }

    #[test]
    fn production_rejects_fake_enabled() {
        let mut config = base_config(AstralLlmEnv::Production);
        config.enable_fake_provider = true;
        config.api_key = Some("secret".into());
        let secrets = ProviderSecrets::default();
        assert!(ConfigValidator::validate(&config, &secrets).is_err());
    }

    #[test]
    fn production_requires_api_key() {
        let config = base_config(AstralLlmEnv::Production);
        let secrets = ProviderSecrets::default();
        assert!(ConfigValidator::validate(&config, &secrets).is_err());
    }

    #[test]
    fn production_public_bind_requires_persistence() {
        let mut config = base_config(AstralLlmEnv::Production);
        config.allow_public_bind = true;
        config.api_key = Some("secret".into());
        config.enable_fake_provider = false;
        config.default_provider = ProviderKind::OpenAi;
        config.enable_persistence = false;
        let mut secrets = ProviderSecrets::default();
        secrets.openai_api_key = Some(secrecy::SecretString::from("key".to_string()));
        assert!(ConfigValidator::validate(&config, &secrets).is_err());
    }
}
