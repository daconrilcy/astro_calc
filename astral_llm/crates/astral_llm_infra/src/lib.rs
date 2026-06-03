//! Infrastructure : configuration, secrets, persistence, telemetry.

pub mod canonical;
pub mod config;
pub mod persistence;
pub mod secrets;
pub mod telemetry;
pub mod url_validator;

pub use canonical::{
    bootstrap_domains, load_canonical_catalog, service_limits_from_env, CanonicalCatalog,
    ProductPromptFamily, SafetyPattern, SharedCanonicalCatalog,
};
pub use config::{AppConfig, env_bool, env_var, load_dotenv, parse_provider_kind};
pub use persistence::{error_code, hash_json, GenerationRunRecord, RunPersistence, RunStatus, SafetyStatus};
pub use secrets::ProviderSecrets;
pub use telemetry::init_tracing;
pub use url_validator::{
    validate_anthropic_base_url, validate_mistral_base_url, validate_openai_base_url,
};
