//! Infrastructure : configuration, secrets, persistence, telemetry.

pub mod benchmark_catalog;
pub mod calculator_client;
pub mod canonical;
pub mod canonical_json_hash;
pub mod config;
pub mod config_validator;
pub mod evidence_canonical;
pub mod i18n_canonical;
pub mod integration_service;
pub mod job_persistence;
pub mod mercure_publisher;
pub mod model_catalog;
pub mod payload_redaction;
pub mod persistence;
pub mod provider_catalog;
pub mod run_audit_view;
pub mod secrets;
mod sql_script;
pub mod telemetry;
pub mod url_validator;

pub use benchmark_catalog::{
    load_benchmark_catalog, BenchmarkCatalog, BenchmarkUsageModelRow, BenchmarkUsageRow,
};
pub use calculator_client::{
    calculator_api_key_from_env, calculator_base_url_from_env, CalculatorClient,
};
pub use canonical::{
    bootstrap_astro_object_labels, bootstrap_domains, bootstrap_interpretation_profiles,
    bootstrap_product_policies, bootstrap_safety_patterns, bootstrap_zodiac_sign_labels,
    enrich_catalog_from_bootstrap, load_canonical_catalog, service_limits_from_env,
    CanonicalCatalog, ProductPromptFamily, SafetyPattern, SharedCanonicalCatalog,
};
pub use canonical_json_hash::{canonical_json_hash, job_logical_payload};
pub use config::{env_bool, env_var, load_dotenv, parse_provider_kind, AppConfig};
pub use config_validator::{ConfigValidationError, ConfigValidator};
pub use evidence_canonical::{bootstrap_evidence_catalog, EvidenceCanonicalCatalog};
pub use i18n_canonical::{
    bootstrap_aspect_type_labels, bootstrap_astro_basis_roles, bootstrap_extra_object_sign_labels,
    bootstrap_writing_locales, WritingLocale,
};
pub use integration_service::{integration_services_map, load_integration_services};
pub use job_persistence::{IdempotentJobClaim, JobPersistence, JobRecord, NewJobRecord};
pub use mercure_publisher::MercurePublisher;
pub use model_catalog::{load_active_provider_codes, load_model_capabilities};
pub use payload_redaction::{redact_request_for_storage, redact_value};
pub use persistence::{
    error_code, hash_json, GenerationPromptTraceRecord, GenerationRunRecord, IdempotencyClaim,
    IdempotencyHit, RunPersistence, RunStatus, SafetyStatus,
};
pub use provider_catalog::{
    LlmProviderModelRow, LlmProviderRow, ProviderCatalogRepository, UpsertProviderModelInput,
};
pub use run_audit_view::{RunAuditPromptTraceView, RunAuditStepView, RunAuditView};
pub use secrets::ProviderSecrets;
pub use telemetry::init_tracing;
pub use url_validator::{
    validate_anthropic_base_url, validate_mistral_base_url, validate_openai_base_url,
};
