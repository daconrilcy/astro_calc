use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use astral_llm_application::reading_persistence::{
    hash_json_value, PersistedGenerationRunRecord, PersistedPromptTraceRecord,
    PersistedTokenUsageRecord, ReadingPersistence, ReadingPersistenceError,
};
use astral_llm_application::{
    build_provider_map, daily_writer_response, GenerateReadingUseCase, ModelCapabilityRegistry,
    PromptCompiler, ProviderCircuitBreaker, ProviderRouter, ResponseValidator, SchemaRegistry,
};
use astral_llm_domain::{
    astrologer_profile::{JargonLevel, ToneProfile, WordingStyle},
    chapter_orchestration::GenerationStepRecord,
    engine_params::EngineParams,
    generation_request::{AudienceLevel, GenerateReadingRequest, ProductContext},
    model_usage_tier::ModelRouteContext,
    output_contract::{GenerationMode, OutputFormat, ResponseContract},
    provider::{ProviderCapabilities, ProviderKind, StructuredOutputMode},
    AstroCalculationPayload, AstrologerProfile, EngineDefaults, FallbackPolicy, PrivacyPolicy,
    SafetyMode, ServiceLimits,
};
use astral_llm_infra::{
    bootstrap_astro_basis_roles, bootstrap_domains, bootstrap_interpretation_profiles,
    bootstrap_product_policies, CanonicalCatalog, SafetyPattern,
};
use astral_llm_providers::{
    FakeProvider, LlmProvider, LlmProviderError, PromptMessage, PromptRole,
    ProviderGenerationRequest, ProviderGenerationResponse,
};
use async_trait::async_trait;
use serde_json::json;
use uuid::Uuid;

#[derive(Clone, Default)]
struct RecordingPersistence {
    prompt_traces: Arc<Mutex<Vec<PersistedPromptTraceRecord>>>,
    run_records: Arc<Mutex<Vec<PersistedGenerationRunRecord>>>,
}

#[async_trait]
impl ReadingPersistence for RecordingPersistence {
    async fn upsert_run(
        &self,
        record: &PersistedGenerationRunRecord,
    ) -> Result<(), ReadingPersistenceError> {
        self.run_records
            .lock()
            .expect("run record mutex")
            .push(record.clone());
        Ok(())
    }

    async fn insert_prompt_trace(
        &self,
        record: &PersistedPromptTraceRecord,
    ) -> Result<(), ReadingPersistenceError> {
        self.prompt_traces
            .lock()
            .expect("prompt trace mutex")
            .push(record.clone());
        Ok(())
    }

    async fn insert_steps(
        &self,
        _run_id: Uuid,
        _steps: &[GenerationStepRecord],
    ) -> Result<Vec<Uuid>, ReadingPersistenceError> {
        Ok(Vec::new())
    }

    async fn replace_run_token_usages(
        &self,
        _run_id: Uuid,
        _usage_records: &[PersistedTokenUsageRecord],
    ) -> Result<(), ReadingPersistenceError> {
        Ok(())
    }

    async fn replace_step_token_usages(
        &self,
        _step_id: Uuid,
        _usage_records: &[PersistedTokenUsageRecord],
    ) -> Result<(), ReadingPersistenceError> {
        Ok(())
    }
}

#[derive(Clone)]
struct SequenceProvider {
    kind: ProviderKind,
    responses: Arc<Mutex<VecDeque<Result<ProviderGenerationResponse, LlmProviderError>>>>,
}

#[async_trait]
impl LlmProvider for SequenceProvider {
    fn kind(&self) -> ProviderKind {
        self.kind.clone()
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            structured_output: StructuredOutputMode::JsonSchemaStrict,
            supports_reasoning_effort: true,
            supports_streaming: false,
            supports_native_safety_prompt: false,
            supports_prompt_cache: false,
            max_input_tokens: Some(1_000_000),
            max_output_tokens: Some(128_000),
        }
    }

    async fn generate(
        &self,
        _request: ProviderGenerationRequest,
    ) -> Result<ProviderGenerationResponse, LlmProviderError> {
        self.responses
            .lock()
            .expect("provider sequence mutex")
            .pop_front()
            .expect("queued provider response")
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repo root")
}

fn prompts_root() -> PathBuf {
    repo_root().join("astral_llm").join("prompts")
}

fn load_json_fixture(relative: &str) -> serde_json::Value {
    let path = repo_root().join(relative);
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("json")
}

fn test_catalog() -> Arc<CanonicalCatalog> {
    Arc::new(CanonicalCatalog {
        astrological_domains: bootstrap_domains(),
        astro_basis_roles: bootstrap_astro_basis_roles(),
        product_generation_policies: bootstrap_product_policies(),
        interpretation_profiles: bootstrap_interpretation_profiles(),
        safety_patterns: vec![
            SafetyPattern {
                pattern_type: "symbolic".into(),
                locale: "fr".into(),
                pattern: "symbolique".into(),
            },
            SafetyPattern {
                pattern_type: "injection".into(),
                locale: "en".into(),
                pattern: "ignore previous".into(),
            },
        ],
        ..Default::default()
    })
}

fn sample_request() -> GenerateReadingRequest {
    GenerateReadingRequest {
        request_id: Some("test-req-1".into()),
        idempotency_key: None,
        product_context: ProductContext {
            product_code: "natal_prompter".into(),
            interpretation_profile_code: Some("natal_light".into()),
            user_language: "fr".into(),
            audience_level: AudienceLevel::Beginner,
        },
        astro_result: AstroCalculationPayload {
            contract_version: "natal_structured_v14".into(),
            chart_type: "natal".into(),
            data: serde_json::json!({
                "domain_scores": {
                    "identity": 0.8,
                    "relationships": 0.6
                },
                "planets": {
                    "sun": { "house": 2, "sign": "capricorn" },
                    "moon": { "house": 4, "sign": "pisces" }
                }
            }),
        },
        astrologer_profile: AstrologerProfile {
            profile_id: None,
            name: None,
            tone: ToneProfile::Warm,
            jargon_level: JargonLevel::Beginner,
            wording_style: WordingStyle::Clear,
            preferred_domains: vec!["identity".into()],
            forbidden_wording: vec![],
            custom_instructions: None,
        },
        engine: EngineParams {
            provider: Some(ProviderKind::Fake),
            model: Some("fake-model".into()),
            reasoning_effort: None,
            temperature: Some(0.4),
            max_output_tokens: Some(2000),
            domain_count: Some(1),
            allow_fallback: false,
            timeout_ms: Some(30_000),
            allow_oracle_benchmark: false,
            summary_model: None,
        },
        response_contract: ResponseContract {
            output_schema_version: "natal_reading_v1".into(),
            generation_mode: GenerationMode::SinglePass,
            format: OutputFormat::StructuredJson,
            chapters: vec![],
            global_max_tokens: None,
            include_astro_sources: true,
            include_legal_disclaimer: true,
        },
        safety_policy: None,
    }
}

fn build_fake_use_case(persistence: RecordingPersistence) -> GenerateReadingUseCase {
    let router = ProviderRouter::new(
        build_provider_map(vec![Arc::new(FakeProvider)]),
        FallbackPolicy::disabled(),
        Arc::new(ModelCapabilityRegistry::bootstrap()),
        PrivacyPolicy::default(),
        Arc::new(ProviderCircuitBreaker::new(5, 60)),
        Some(Arc::new(persistence.clone())),
    );

    GenerateReadingUseCase::new(
        router,
        PromptCompiler::new(prompts_root()),
        ResponseValidator::new(Arc::new(SchemaRegistry::new())),
        EngineDefaults {
            provider: ProviderKind::Fake,
            model: "fake-model".into(),
        },
        ServiceLimits::default(),
        test_catalog(),
        PrivacyPolicy::default(),
        true,
        Some(Arc::new(persistence)),
    )
}

fn build_openai_use_case(persistence: RecordingPersistence) -> GenerateReadingUseCase {
    let response = load_json_fixture("tests/golden/horoscope_response_basic_daily_fake.json");
    let router = ProviderRouter::new(
        build_provider_map(vec![Arc::new(SequenceProvider {
            kind: ProviderKind::OpenAi,
            responses: Arc::new(Mutex::new(VecDeque::from(vec![Ok(
                ProviderGenerationResponse {
                    raw_text: response.to_string(),
                    parsed_json: Some(response),
                    usage: None,
                    provider_metadata: json!({ "fixture": true }),
                    model_used: "gpt-5-mini".into(),
                    provider_kind: ProviderKind::OpenAi,
                },
            )]))),
        })]),
        FallbackPolicy::disabled(),
        Arc::new(ModelCapabilityRegistry::bootstrap_dev_fallback()),
        PrivacyPolicy::default(),
        Arc::new(ProviderCircuitBreaker::new(5, 60)),
        Some(Arc::new(persistence.clone())),
    );

    GenerateReadingUseCase::new(
        router,
        PromptCompiler::new(prompts_root()),
        ResponseValidator::new(Arc::new(SchemaRegistry::new())),
        EngineDefaults {
            provider: ProviderKind::OpenAi,
            model: "gpt-5-mini".into(),
        },
        ServiceLimits::default(),
        test_catalog(),
        PrivacyPolicy::default(),
        true,
        Some(Arc::new(persistence)),
    )
}

fn provider_request(run_id: Uuid) -> ProviderGenerationRequest {
    ProviderGenerationRequest {
        model: "fake-model".into(),
        messages: vec![
            PromptMessage {
                role: PromptRole::System,
                content: "system".into(),
            },
            PromptMessage {
                role: PromptRole::User,
                content: "user".into(),
            },
        ],
        structured_schema: Some(json!({
            "type": "object",
            "additionalProperties": false,
        })),
        reasoning_effort: None,
        temperature: Some(0.2),
        max_output_tokens: Some(512),
        safety_mode: SafetyMode::PlatformRulesOnly,
        timeout: Duration::from_secs(30),
        metadata: astral_llm_providers::GenerationMetadata {
            run_id: run_id.to_string(),
            request_id: Some("req-1".into()),
            product_code: "natal_prompter".into(),
            chapter_code: Some("identity".into()),
            prompt_trace_step: Some("single_pass_generate".into()),
            prompt_trace_attempt: Some("primary".into()),
            prompt_family: Some("natal_prompter".into()),
            prompt_version: Some("v1".into()),
        },
    }
}

#[tokio::test]
async fn provider_router_persists_retry_prompt_traces_without_database() {
    let persistence = RecordingPersistence::default();
    let provider = SequenceProvider {
        kind: ProviderKind::Fake,
        responses: Arc::new(Mutex::new(VecDeque::from(vec![
            Err(LlmProviderError::Timeout),
            Ok(ProviderGenerationResponse {
                raw_text: "{\"ok\":true}".into(),
                parsed_json: Some(json!({ "ok": true })),
                usage: None,
                provider_metadata: json!({ "fixture": true }),
                model_used: "fake-model".into(),
                provider_kind: ProviderKind::Fake,
            }),
        ]))),
    };
    let router = ProviderRouter::new(
        build_provider_map(vec![Arc::new(provider)]),
        FallbackPolicy::disabled(),
        Arc::new(ModelCapabilityRegistry::bootstrap()),
        PrivacyPolicy::default(),
        Arc::new(ProviderCircuitBreaker::new(5, 60)),
        Some(Arc::new(persistence.clone())),
    );

    let result = router
        .generate(
            provider_request(Uuid::new_v4()),
            ProviderKind::Fake,
            "fake-model",
            false,
            true,
            ModelRouteContext::PrimaryReading,
        )
        .await
        .expect("provider router success after retry");

    assert_eq!(result.used_provider, ProviderKind::Fake);
    assert!(!result.fallback_used);

    let traces = persistence
        .prompt_traces
        .lock()
        .expect("prompt trace mutex");
    assert_eq!(traces.len(), 2);
    assert_eq!(traces[0].attempt.as_deref(), Some("primary"));
    assert_eq!(
        traces[1].attempt.as_deref(),
        Some("primary_provider_retry_1")
    );
    assert_eq!(traces[0].step_type.as_deref(), Some("single_pass_generate"));
    assert_eq!(traces[0].prompt_family.as_deref(), Some("natal_prompter"));
    assert_eq!(traces[0].prompt_version.as_deref(), Some("v1"));
    assert_eq!(traces[0].message_count, 2);
    assert_eq!(
        traces[0].messages_json,
        json!([
            { "role": "system", "content": "system" },
            { "role": "user", "content": "user" }
        ])
    );
}

#[test]
fn hash_json_value_matches_persisted_hash_algorithm() {
    let value = json!({
        "product_code": "natal_prompter",
        "input": { "user_language": "fr", "domains": ["identity", "career"] },
        "flags": [true, false, null]
    });

    assert_eq!(
        hash_json_value(&value),
        "8beed392f8aa383b2898614e0b5903a80e9eceaae4c13cc5a1f657062433bd95"
    );
    assert_eq!(hash_json_value(&value), hash_json_value(&value));
}

#[tokio::test]
async fn execute_with_audit_preserves_created_at_between_pending_and_final_run_records() {
    let persistence = RecordingPersistence::default();
    let use_case = build_fake_use_case(persistence.clone());

    let response = use_case
        .execute_with_audit(sample_request(), Uuid::new_v4().to_string())
        .await;

    assert!(matches!(
        response.response,
        astral_llm_domain::generation_response::GenerateReadingResponse::Success { .. }
    ));

    let run_records = persistence.run_records.lock().expect("run record mutex");
    assert_eq!(run_records.len(), 2);
    assert_eq!(
        run_records[0].status,
        astral_llm_application::reading_persistence::PersistedRunStatus::Pending
    );
    assert_eq!(
        run_records[1].status,
        astral_llm_application::reading_persistence::PersistedRunStatus::Success
    );
    assert_eq!(run_records[0].created_at, run_records[1].created_at);
}

#[tokio::test]
async fn horoscope_daily_persistence_preserves_created_at_between_pending_and_final_run_records() {
    let persistence = RecordingPersistence::default();
    let use_case = build_openai_use_case(persistence.clone());
    let request = load_json_fixture(
        "tests/golden/horoscope_interpretation_request_basic_daily_paris_1990.json",
    );
    let run_id = Uuid::new_v4().to_string();

    daily_writer_response(&use_case, &request, Some(&run_id))
        .await
        .expect("daily horoscope response");

    let run_records = persistence.run_records.lock().expect("run record mutex");
    assert_eq!(run_records.len(), 2);
    assert_eq!(
        run_records[0].status,
        astral_llm_application::reading_persistence::PersistedRunStatus::Pending
    );
    assert_eq!(
        run_records[1].status,
        astral_llm_application::reading_persistence::PersistedRunStatus::Success
    );
    assert_eq!(run_records[0].created_at, run_records[1].created_at);
}
