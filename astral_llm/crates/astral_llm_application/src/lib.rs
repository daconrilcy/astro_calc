//! Cas d'usage et orchestration du gateway LLM.

pub mod chapter_orchestrator;
pub mod domain_selector;
pub mod engine_defaults;
pub mod generate_reading_use_case;
pub mod payload_sanitizer;
pub mod prompt_compiler;
pub mod provider_factory;
pub mod provider_router;
pub mod request_validator;
pub mod response_validator;
pub mod safety_guard;
pub mod safety_resolver;
pub mod schema_registry;
pub mod token_budget;

pub use chapter_orchestrator::ChapterOrchestrator;
pub use engine_defaults::{resolve_engine_params, ResolvedEngineParams};
pub use generate_reading_use_case::GenerateReadingUseCase;
pub use prompt_compiler::{PromptBundle, PromptCompiler};
pub use provider_factory::{build_fallback_policy, build_providers};
pub use provider_router::{build_http_client, build_provider_map, FallbackPolicy, ProviderRouter, ProviderRouteResult};
pub use response_validator::ResponseValidator;
pub use safety_guard::SafetyGuard;
pub use safety_resolver::SafetyResolver;
pub use schema_registry::SchemaRegistry;
