//! Cas d'usage et orchestration du gateway LLM.

pub mod astro_basis_role_normalizer;
pub mod astro_basis_validator;
pub mod astro_fact_extractor;
pub mod astro_label_humanizer;
pub mod astro_payload_normalizer;
pub mod chapter_evidence_basis_enricher;
pub mod chapter_evidence_coherence;
pub mod chapter_evidence_planner;
pub mod chapter_orchestrator;
pub mod chapter_quality_repair;
pub mod chapter_writing_guidance;
pub mod core;
pub mod domain;
pub mod domain_resolver;
pub mod domain_selector;
pub mod editorial_validation;
pub mod evidence_diversity_validator;
pub mod evidence_fact_parse;
pub mod execution_audit;
pub mod french_typography;
pub mod horoscope;
pub mod interpretation_profile_resolver;
pub mod interpretive_evidence_builder;
pub mod prior_chapter_usage;
pub mod reading_opening_diversity_validator;
pub mod reading_plan;
pub mod reading_script_guard;
pub mod reasoning_generation;
pub mod simplified_reading;
pub mod simplified_reading_guard;
pub mod simplified_reading_postprocess;
pub mod single_pass_hardening;
pub mod text_reprocessing;
pub mod text_reprocessing_service_adapter;
pub mod text_trigrams;
pub mod writing_language;
pub use reading_plan::ReadingPlanBuilder;
pub use simplified_reading::{
    build_reading_request, merge_simplified_forbidden_wording, prompt_constraints_block,
    resolve_simplified_chapter_code, sun_sign_blocked, validate_simplified_calculation_request,
    SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE, SIMPLIFIED_CHAPTER_IDENTITY, SIMPLIFIED_PAYLOAD_CONTRACT,
    SIMPLIFIED_PROFILE, SIMPLIFIED_REQUEST_CONTRACT, SUN_SIGN_BLOCKED_CODE,
};
pub mod engine_defaults;
pub mod engine_reading;
pub mod final_synthesis_synthesizer;
pub mod generate_reading_use_case;
pub mod generation_trace;
pub mod infra;
pub mod integration_job_executor;
pub mod integration_job_result;
pub mod integration_job_validator;
pub mod model_capability_registry;
pub mod payload_sanitizer;
pub mod product_policy_validator;
pub mod prompt_compiler;
pub mod prompt_golden;
pub mod prompt_trace;
pub mod provider_circuit_breaker;
pub mod provider_factory;
pub mod provider_router;
pub mod provider_schema_compiler;
pub mod raw_provider_trace;
pub mod reading_quality_validator;
pub mod request_validator;
pub mod response_validator;
pub mod safety_guard;
pub mod safety_resolver;
pub mod schema_registry;
pub mod service;
pub mod summary_forbidden_patterns;
pub mod summary_synthesizer;
pub mod summary_ux_rules;
pub mod token_budget;
pub use astro_basis_validator::AstroBasisValidator;
pub use astro_payload_normalizer::AstroPayloadNormalizer;
pub use chapter_evidence_coherence::ChapterEvidenceCoherence;
pub use chapter_evidence_planner::{pack_for_chapter, ChapterEvidencePlanner};
pub use chapter_orchestrator::{new_run_id, ChapterOrchestrator};
pub use domain_resolver::DomainResolver;
pub use editorial_validation::{EditorialFixtureSpec, EditorialValidator};
pub use engine_defaults::{
    resolve_engine_params, resolve_service_engine_defaults, resolve_subtask_engine,
    ResolvedEngineParams,
};
pub use engine_reading::{build_reading_request_from_engine, validate_engine_response};
pub use evidence_diversity_validator::{compute_evidence_metrics, EvidenceDiversityValidator};
pub use execution_audit::ExecutionAudit;
pub use generate_reading_use_case::{GenerateReadingUseCase, UseCaseOutput};
pub use generation_trace::GenerationTraceContext;
pub use horoscope::{
    build_calculation_request_for_service, build_interpretation_request,
    build_period_calculation_request_for_service, build_period_writer_request,
    daily_writer_response, period_editorial_audit, period_style_editor_max_output_tokens,
    period_writer_max_output_tokens, period_writer_response_with_quality_loop, score_calculation,
    validate_horoscope_response_schema, validate_period_public_request,
    validate_period_response_contract, validate_public_request, validate_response_evidence,
    HoroscopePeriodPublicRequest, HoroscopePublicRequest,
};
pub use integration_job_executor::{
    supports_integration_service, IntegrationJobExecutor, UnifiedReadingOutcome,
    UnifiedReadingResult,
};
pub use integration_job_result::{
    job_error_from_reading, job_status_from_reading, unified_result_envelope,
};
pub use integration_job_validator::{IntegrationJobValidator, ValidatedIntegrationJob};
pub use interpretation_profile_resolver::{
    InterpretationProfileResolver, ResolvedInterpretationContext, ValidatedProductContext,
};
pub use interpretive_evidence_builder::{
    evidence_enabled_for_request, pool_richness_check, InterpretiveEvidenceBuilder,
};
pub use model_capability_registry::ModelCapabilityRegistry;
pub use prompt_compiler::{PromptBundle, PromptCompiler};
pub use provider_circuit_breaker::{CircuitBreakerState, ProviderCircuitBreaker};
pub use provider_factory::{
    build_capability_registry, build_capability_registry_with_db, build_fallback_policy,
    build_providers,
};
pub use provider_router::{
    build_http_client, build_provider_map, ProviderRouteResult, ProviderRouter,
};
pub use provider_schema_compiler::ProviderSchemaCompiler;
pub use reading_opening_diversity_validator::ReadingOpeningDiversityValidator;
pub use reading_quality_validator::{
    requires_blocking_quality_gate, PremiumQualityThresholds, ReadingQualityReport,
    ReadingQualityValidator,
};
pub use response_validator::ResponseValidator;
pub use safety_guard::{ensure_symbolic_framing_text, SafetyGuard};
pub use safety_resolver::SafetyResolver;
pub use schema_registry::SchemaRegistry;
pub use text_reprocessing::{
    LanguageRegistry, LanguageRuleSet, ProcessorRegistry, ServiceRegistry, ServiceRuleSet,
    TextRetreatmentPipeline, TextRetreatmentProcessor,
};
pub use text_reprocessing_service_adapter::{
    normalize_json_for_text_reprocessing_parity, reprocess_calculator_projection,
    reprocess_horoscope_daily, reprocess_horoscope_period, reprocess_natal_simplified,
    reprocess_natal_theme, reprocess_natal_theme_with_context, reprocess_prompt_trace,
    reprocess_shared_text, TextReprocessingApplicationError, TextReprocessingFieldAudit,
};
