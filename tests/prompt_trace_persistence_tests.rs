use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use astral_llm_api::{routes, state::AppState};
use astral_llm_application::{
    build_period_writer_request, build_provider_map, daily_writer_response,
    period_writer_response_with_quality_loop, validate_period_public_request,
    GenerateReadingUseCase, IntegrationJobValidator, ModelCapabilityRegistry, PromptCompiler,
    ProviderCircuitBreaker, ProviderRouter, ResponseValidator, SchemaRegistry,
};
use astral_llm_domain::{
    output_contract::GenerationMode,
    provider::{ProviderCapabilities, ProviderKind, StructuredOutputMode},
    EngineDefaults, FallbackPolicy, PrivacyPolicy, ServiceLimits,
};
use astral_llm_infra::{
    bootstrap_domains, bootstrap_interpretation_profiles, bootstrap_product_policies, load_dotenv,
    CanonicalCatalog, RunPersistence,
};
use astral_llm_providers::{
    FakeProvider, LlmProvider, LlmProviderError, ProviderGenerationRequest,
    ProviderGenerationResponse, TokenUsage,
};
use async_trait::async_trait;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use tokio::sync::Semaphore;
use tower::ServiceExt;
use uuid::Uuid;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .expect("repo root")
}

fn prompts_root() -> PathBuf {
    repo_root().join("astral_llm").join("prompts")
}

fn load_json_fixture(relative: &str) -> Value {
    let path = repo_root().join(relative);
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("json")
}

fn test_catalog() -> Arc<CanonicalCatalog> {
    Arc::new(CanonicalCatalog {
        astrological_domains: bootstrap_domains(),
        product_generation_policies: bootstrap_product_policies(),
        interpretation_profiles: bootstrap_interpretation_profiles(),
        ..Default::default()
    })
}

async fn test_pool() -> sqlx::PgPool {
    load_dotenv();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .expect("db")
}

fn build_fake_use_case(persistence: Arc<RunPersistence>) -> GenerateReadingUseCase {
    let router = ProviderRouter::new(
        build_provider_map(vec![Arc::new(FakeProvider)]),
        FallbackPolicy::disabled(),
        Arc::new(ModelCapabilityRegistry::bootstrap()),
        PrivacyPolicy::default(),
        Arc::new(ProviderCircuitBreaker::new(5, 60)),
        Some(persistence.clone()),
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
        Some(persistence),
    )
}

#[derive(Clone)]
struct SequenceOpenAiProvider {
    responses: Arc<Mutex<VecDeque<ProviderGenerationResponse>>>,
}

#[async_trait]
impl LlmProvider for SequenceOpenAiProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::OpenAi
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
        request: ProviderGenerationRequest,
    ) -> Result<ProviderGenerationResponse, LlmProviderError> {
        let mut response = self
            .responses
            .lock()
            .expect("provider sequence mutex")
            .pop_front()
            .expect("queued provider response");
        response.model_used = request.model;
        Ok(response)
    }
}

fn build_openai_use_case(
    persistence: Arc<RunPersistence>,
    responses: Vec<ProviderGenerationResponse>,
) -> GenerateReadingUseCase {
    let router = ProviderRouter::new(
        build_provider_map(vec![Arc::new(SequenceOpenAiProvider {
            responses: Arc::new(Mutex::new(VecDeque::from(responses))),
        })]),
        FallbackPolicy::disabled(),
        Arc::new(ModelCapabilityRegistry::bootstrap_dev_fallback()),
        PrivacyPolicy::default(),
        Arc::new(ProviderCircuitBreaker::new(5, 60)),
        Some(persistence.clone()),
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
        Some(persistence),
    )
}

fn provider_response_from_json(value: Value) -> ProviderGenerationResponse {
    ProviderGenerationResponse {
        raw_text: value.to_string(),
        parsed_json: Some(value),
        usage: Some(TokenUsage::simple(100, 200)),
        provider_metadata: json!({ "fixture": true }),
        model_used: "gpt-5-mini".into(),
        provider_kind: ProviderKind::OpenAi,
    }
}

fn provider_response_from_raw_text(raw_text: &str) -> ProviderGenerationResponse {
    ProviderGenerationResponse {
        raw_text: raw_text.to_string(),
        parsed_json: None,
        usage: Some(TokenUsage::simple(80, 120)),
        provider_metadata: json!({ "fixture": true }),
        model_used: "gpt-5-mini".into(),
        provider_kind: ProviderKind::OpenAi,
    }
}

#[tokio::test]
#[ignore = "requires DATABASE_URL and PostgreSQL schema access"]
async fn run_audit_stores_prompt_traces_for_single_pass_and_chapter_modes() {
    let pool = test_pool().await;
    let persistence = Arc::new(RunPersistence::new(pool));
    persistence.ensure_schema().await.expect("schema");
    let use_case = build_fake_use_case(persistence.clone());

    let mut single_pass_request: astral_llm_domain::GenerateReadingRequest =
        serde_json::from_value(load_json_fixture(
            "contracts/integration/examples/generate_reading_request_v1.premium_plus.compact.json",
        ))
        .expect("request");
    single_pass_request
        .product_context
        .interpretation_profile_code = Some("natal_light".into());
    single_pass_request.response_contract.generation_mode = GenerationMode::SinglePass;
    single_pass_request.request_id = Some(format!("prompt-trace-single-{}", Uuid::new_v4()));

    let single_run_id = Uuid::new_v4().to_string();
    let single = use_case
        .execute_with_audit(single_pass_request, single_run_id.clone())
        .await;
    assert!(matches!(
        single.response,
        astral_llm_domain::GenerateReadingResponse::Success { .. }
    ));

    let single_audit = persistence
        .get_run_audit(Uuid::parse_str(&single_run_id).expect("uuid"))
        .await
        .expect("audit query")
        .expect("run audit");
    assert_eq!(single_audit.prompt_traces.len(), 1);
    assert_eq!(
        single_audit.prompt_traces[0].step_type.as_deref(),
        Some("single_pass_generate")
    );

    let mut chapter_request: astral_llm_domain::GenerateReadingRequest =
        serde_json::from_value(load_json_fixture(
            "contracts/integration/examples/generate_reading_request_v1.premium_plus.compact.json",
        ))
        .expect("request");
    chapter_request.request_id = Some(format!("prompt-trace-chapter-{}", Uuid::new_v4()));
    let chapter_run_id = Uuid::new_v4().to_string();
    let chapter = use_case
        .execute_with_audit(chapter_request, chapter_run_id.clone())
        .await;
    assert!(matches!(
        chapter.response,
        astral_llm_domain::GenerateReadingResponse::Success { .. }
    ));

    let chapter_audit = persistence
        .get_run_audit(Uuid::parse_str(&chapter_run_id).expect("uuid"))
        .await
        .expect("audit query")
        .expect("run audit");
    assert!(chapter_audit.prompt_traces.len() >= 3);
    assert!(chapter_audit
        .prompt_traces
        .iter()
        .any(|trace| trace.step_type.as_deref() == Some("chapter_generate")));
    assert!(chapter_audit
        .prompt_traces
        .iter()
        .any(|trace| trace.step_type.as_deref() == Some("summary_generate")));
}

#[tokio::test]
#[ignore = "requires DATABASE_URL and PostgreSQL schema access"]
async fn run_audit_route_returns_prompt_traces() {
    let pool = test_pool().await;
    let persistence = Arc::new(RunPersistence::new(pool));
    persistence.ensure_schema().await.expect("schema");
    let use_case = build_fake_use_case(persistence.clone());

    let run_id = Uuid::new_v4();
    let run_record = astral_llm_infra::GenerationRunRecord {
        id: run_id,
        request_id: Some("route-test".into()),
        idempotency_key: None,
        product_code: "natal_prompter".into(),
        user_language: "fr".into(),
        astro_contract_version: "natal_structured_v14".into(),
        output_schema_version: "natal_reading_v1".into(),
        prompt_family: "test".into(),
        prompt_version: "v1".into(),
        safety_policy_version: "runtime".into(),
        provider_requested: "openai".into(),
        provider_used: Some("openai".into()),
        model_requested: "gpt-5-mini".into(),
        model_used: Some("gpt-5-mini".into()),
        generation_mode: "single_pass".into(),
        fallback_used: false,
        selected_domains: None,
        status: astral_llm_infra::RunStatus::Success,
        safety_status: astral_llm_infra::SafetyStatus::Passed,
        input_hash: "input".into(),
        output_hash: Some("output".into()),
        token_input: Some(10),
        token_output: Some(20),
        latency_ms: Some(30),
        error_code: None,
        created_at: chrono::Utc::now(),
    };
    persistence
        .upsert_run(&run_record)
        .await
        .expect("run insert");
    persistence
        .insert_prompt_trace(&astral_llm_infra::GenerationPromptTraceRecord {
            run_id,
            chapter_code: Some("identity".into()),
            step_type: Some("chapter_generate".into()),
            attempt: Some("primary".into()),
            prompt_family: Some("natal_prompter".into()),
            prompt_version: Some("v1".into()),
            message_count: 2,
            compiled_prompt: "compiled".into(),
            messages_json: json!([{ "role": "system", "content": "test" }]),
        })
        .await
        .expect("trace insert");

    let state = AppState {
        use_case: Arc::new(use_case),
        schema_registry: Arc::new(SchemaRegistry::new()),
        config: astral_llm_infra::AppConfig::try_from_env().expect("config from env"),
        persistence: Some(persistence),
        job_persistence: None,
        integration_job_validator: Some(Arc::new(IntegrationJobValidator::new())),
        concurrency_limit: Some(Arc::new(Semaphore::new(1))),
        api_key_limiter: None,
        interpretation_profile_count: 0,
        calculator_client: None,
    };

    let response = routes::router(state)
        .oneshot(
            Request::get(format!("/v1/runs/{run_id}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body"),
    )
    .expect("json");
    assert_eq!(body["prompt_traces"][0]["attempt"], "primary");
    assert_eq!(body["prompt_traces"][0]["compiled_prompt"], "compiled");
}

#[tokio::test]
#[ignore = "requires DATABASE_URL and PostgreSQL schema access"]
async fn horoscope_retries_store_multiple_prompt_attempts() {
    let pool = test_pool().await;
    let persistence = Arc::new(RunPersistence::new(pool));
    persistence.ensure_schema().await.expect("schema");

    let daily_request = load_json_fixture(
        "tests/golden/horoscope_interpretation_request_basic_daily_paris_1990.json",
    );
    let daily_response = load_json_fixture("tests/golden/horoscope_response_basic_daily_fake.json");
    let daily_use_case = build_openai_use_case(
        persistence.clone(),
        vec![
            provider_response_from_raw_text("not json"),
            provider_response_from_json(daily_response),
        ],
    );
    let daily_run_id = Uuid::new_v4().to_string();
    let daily = daily_writer_response(&daily_use_case, &daily_request, Some(&daily_run_id))
        .await
        .expect("daily render");
    assert_eq!(daily["quality"]["repair_attempted"], true);
    let daily_audit = persistence
        .get_run_audit(Uuid::parse_str(&daily_run_id).expect("uuid"))
        .await
        .expect("audit query")
        .expect("run audit");
    assert_eq!(daily_audit.prompt_traces.len(), 2);

    let public = validate_period_public_request(&json!({
        "anchor_date": "2026-06-07",
        "timezone": "Europe/Paris",
        "target_language": "fr",
        "target_language_code": "fr",
        "chart_calculation_id": "chart-1",
        "audience_level": "general",
        "astrologer_persona": null
    }))
    .expect("period public");
    let calculation = load_json_fixture(
        "tests/golden/horoscope_period_calculation_response_premium_next_7_days_paris_1990.json",
    );
    let writer_request =
        build_period_writer_request(&public, &calculation).expect("writer request");
    let mut invalid_period =
        load_json_fixture("tests/golden/horoscope_period_response_premium_next_7_days_fake.json");
    invalid_period["best_days"] = invalid_period["key_days"].clone();
    let valid_period =
        load_json_fixture("tests/golden/horoscope_period_response_premium_next_7_days_fake.json");
    let period_use_case = build_openai_use_case(
        persistence.clone(),
        vec![
            provider_response_from_json(invalid_period),
            provider_response_from_json(valid_period),
        ],
    );
    let period_run_id = Uuid::new_v4().to_string();
    let period = period_writer_response_with_quality_loop(
        &period_use_case,
        &writer_request,
        Some(&period_run_id),
    )
    .await
    .expect("period render");
    assert_eq!(period["quality"]["provider"], "openai");
    let period_audit = persistence
        .get_run_audit(Uuid::parse_str(&period_run_id).expect("uuid"))
        .await
        .expect("audit query")
        .expect("run audit");
    assert!(period_audit.prompt_traces.len() >= 2);
    assert!(period_audit
        .prompt_traces
        .iter()
        .any(|trace| { trace.step_type.as_deref() == Some("horoscope_period_quality_retry") }));
}
