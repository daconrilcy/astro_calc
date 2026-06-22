use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use astral_llm_application::reading_persistence::{
    hash_json_value, PersistedPromptTraceRecord, PersistedTokenUsageRecord, ReadingPersistence,
    ReadingPersistenceError,
};
use astral_llm_application::{
    build_provider_map, ModelCapabilityRegistry, ProviderCircuitBreaker, ProviderRouter,
};
use astral_llm_domain::{
    chapter_orchestration::GenerationStepRecord,
    model_usage_tier::ModelRouteContext,
    provider::{ProviderCapabilities, ProviderKind, StructuredOutputMode},
    FallbackPolicy, PrivacyPolicy, SafetyMode,
};
use astral_llm_providers::{
    LlmProvider, LlmProviderError, PromptMessage, PromptRole, ProviderGenerationRequest,
    ProviderGenerationResponse,
};
use async_trait::async_trait;
use serde_json::json;
use uuid::Uuid;

#[derive(Clone, Default)]
struct RecordingPersistence {
    prompt_traces: Arc<Mutex<Vec<PersistedPromptTraceRecord>>>,
}

#[async_trait]
impl ReadingPersistence for RecordingPersistence {
    async fn upsert_run(
        &self,
        _record: &astral_llm_application::reading_persistence::PersistedGenerationRunRecord,
    ) -> Result<(), ReadingPersistenceError> {
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
    responses: Arc<Mutex<VecDeque<Result<ProviderGenerationResponse, LlmProviderError>>>>,
}

#[async_trait]
impl LlmProvider for SequenceProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Fake
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
