use secrecy::{ExposeSecret, SecretString};

use crate::config::{load_dotenv, env_var};

#[derive(Debug, Clone, Default)]
pub struct ProviderSecrets {
    pub openai_api_key: Option<SecretString>,
    pub anthropic_api_key: Option<SecretString>,
    pub mistral_api_key: Option<SecretString>,
}

impl ProviderSecrets {
    pub fn from_env() -> Self {
        load_dotenv();

        Self {
            openai_api_key: secret_from_env("OPENAI_API_KEY"),
            anthropic_api_key: secret_from_env("ANTHROPIC_API_KEY"),
            mistral_api_key: secret_from_env("MISTRAL_API_KEY"),
        }
    }

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

fn secret_from_env(key: &str) -> Option<SecretString> {
    env_var(key).map(SecretString::from)
}

fn secret_is_set(secret: Option<&SecretString>) -> bool {
    secret
        .map(|value| !value.expose_secret().trim().is_empty())
        .unwrap_or(false)
}
