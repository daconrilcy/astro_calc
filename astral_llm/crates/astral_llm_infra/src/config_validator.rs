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
