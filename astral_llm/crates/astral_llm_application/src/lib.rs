//! Cas d'usage et orchestration du gateway LLM.

pub mod astro_basis_validator;
pub mod astro_fact_extractor;
pub mod astro_payload_normalizer;
pub mod chapter_orchestrator;
pub mod chapter_quality_repair;
pub mod domain_resolver;
pub mod editorial_validation;
pub mod domain_selector;
pub mod execution_audit;
pub mod reading_plan;
pub mod engine_defaults;
pub mod generation_trace;
pub mod generate_reading_use_case;
pub mod model_capability_registry;
pub mod payload_sanitizer;
pub mod product_policy_validator;
pub mod prompt_compiler;
pub mod provider_circuit_breaker;
pub mod provider_factory;
pub mod provider_router;
pub mod provider_schema_compiler;
pub mod prompt_golden;
pub mod reading_quality_validator;
pub mod request_validator;
pub mod response_validator;
pub mod safety_guard;
pub mod safety_resolver;
pub mod schema_registry;
pub mod summary_synthesizer;
pub mod token_budget;

pub use chapter_orchestrator::{new_run_id, ChapterOrchestrator};
pub use engine_defaults::{resolve_engine_params, ResolvedEngineParams};
pub use domain_resolver::DomainResolver;
pub use execution_audit::ExecutionAudit;
pub use generation_trace::GenerationTraceContext;
pub use generate_reading_use_case::{GenerateReadingUseCase, UseCaseOutput};
pub use prompt_compiler::{PromptBundle, PromptCompiler};
pub use astro_payload_normalizer::AstroPayloadNormalizer;
pub use astro_basis_validator::AstroBasisValidator;
pub use model_capability_registry::ModelCapabilityRegistry;
pub use provider_factory::{
    build_capability_registry, build_capability_registry_with_db, build_fallback_policy,
    build_providers,
};
pub use provider_circuit_breaker::{CircuitBreakerState, ProviderCircuitBreaker};
pub use provider_router::{build_http_client, build_provider_map, ProviderRouteResult, ProviderRouter};
pub use editorial_validation::{EditorialFixtureSpec, EditorialValidator};
pub use reading_quality_validator::{
    requires_blocking_quality_gate, PremiumQualityThresholds, ReadingQualityReport,
    ReadingQualityValidator,
};
pub use provider_schema_compiler::ProviderSchemaCompiler;
pub use response_validator::ResponseValidator;
pub use safety_guard::SafetyGuard;
pub use safety_resolver::SafetyResolver;
pub use schema_registry::SchemaRegistry;
