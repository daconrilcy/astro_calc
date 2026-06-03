//! Infrastructure : configuration, secrets, persistence, telemetry.

pub mod canonical;
pub mod config;
pub mod config_validator;
pub mod model_catalog;
pub mod payload_redaction;
pub mod persistence;
mod sql_script;
pub mod run_audit_view;
pub mod secrets;
pub mod telemetry;
pub mod url_validator;

pub use canonical::{
    bootstrap_domains, bootstrap_product_policies, bootstrap_safety_patterns,
    enrich_catalog_from_bootstrap, load_canonical_catalog, service_limits_from_env,
    CanonicalCatalog, ProductPromptFamily, SafetyPattern, SharedCanonicalCatalog,
};
pub use config::{AppConfig, env_bool, env_var, load_dotenv, parse_provider_kind};
pub use config_validator::{ConfigValidationError, ConfigValidator};
pub use model_catalog::load_model_capabilities;
pub use payload_redaction::{redact_request_for_storage, redact_value};
pub use persistence::{
    error_code, hash_json, GenerationRunRecord, IdempotencyClaim, IdempotencyHit, RunPersistence,
    RunStatus, SafetyStatus,
};
pub use run_audit_view::{RunAuditStepView, RunAuditView};
pub use secrets::ProviderSecrets;
pub use telemetry::init_tracing;
pub use url_validator::{
    validate_anthropic_base_url, validate_mistral_base_url, validate_openai_base_url,
};
