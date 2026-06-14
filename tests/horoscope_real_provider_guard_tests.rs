use astral_llm_application::{
    build_provider_map, daily_writer_response, period_writer_response_with_quality_loop,
    GenerateReadingUseCase, ModelCapabilityRegistry, PromptCompiler, ProviderCircuitBreaker,
    ProviderRouter, ResponseValidator, SchemaRegistry, validate_response_evidence,
};
use astral_llm_domain::{
    provider::{ProviderCapabilities, ProviderKind, StructuredOutputMode},
    EngineDefaults, FallbackPolicy, PrivacyPolicy, ServiceLimits,
};
use astral_llm_infra::{
    bootstrap_domains, bootstrap_interpretation_profiles, bootstrap_product_policies,
    CanonicalCatalog,
};
use astral_llm_providers::{
    LlmProvider, ProviderGenerationRequest, ProviderGenerationResponse, TokenUsage,
};
use async_trait::async_trait;
use astral_llm_domain::provider::ReasoningEffort;
use serde_json::Value;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Clone)]
struct FixtureOpenAiProvider {
    fixture: Value,
}

struct SequenceOpenAiProvider {
    responses: Mutex<VecDeque<ProviderGenerationResponse>>,
}

#[async_trait]
impl LlmProvider for FixtureOpenAiProvider {
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
    ) -> Result<ProviderGenerationResponse, astral_llm_providers::LlmProviderError> {
        assert_daily_request_keeps_real_provider_budget(&request);
        let raw_text = self.fixture.to_string();
        Ok(ProviderGenerationResponse {
            raw_text,
            parsed_json: Some(self.fixture.clone()),
            usage: Some(TokenUsage {
                input_tokens: 100,
                output_tokens: 200,
            }),
            provider_metadata: serde_json::json!({ "fixture": true }),
            model_used: request.model,
            provider_kind: ProviderKind::OpenAi,
        })
    }
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
    ) -> Result<ProviderGenerationResponse, astral_llm_providers::LlmProviderError> {
        assert_daily_request_keeps_real_provider_budget(&request);
        let mut response = self
            .responses
            .lock()
            .expect("sequence provider mutex")
            .pop_front()
            .expect("queued provider response");
        response.model_used = request.model;
        Ok(response)
    }
}

fn assert_daily_request_keeps_real_provider_budget(request: &ProviderGenerationRequest) {
    if !request.metadata.product_code.contains("daily") {
        return;
    }
    assert_eq!(request.reasoning_effort, Some(ReasoningEffort::Minimal));
    assert!(
        request.max_output_tokens.unwrap_or_default() >= 4_000,
        "daily horoscope real-provider requests must reserve enough output tokens"
    );
}

fn test_catalog() -> Arc<CanonicalCatalog> {
    Arc::new(CanonicalCatalog {
        astrological_domains: bootstrap_domains(),
        product_generation_policies: bootstrap_product_policies(),
        interpretation_profiles: bootstrap_interpretation_profiles(),
        ..Default::default()
    })
}

fn test_use_case(provider: Arc<dyn LlmProvider>, model: &str) -> GenerateReadingUseCase {
    let router = ProviderRouter::new(
        build_provider_map(vec![provider]),
        FallbackPolicy::disabled(),
        Arc::new(ModelCapabilityRegistry::bootstrap_dev_fallback()),
        PrivacyPolicy::default(),
        Arc::new(ProviderCircuitBreaker::new(5, 60)),
    );
    let prompts_root = PathBuf::from("astral_llm/prompts");
    GenerateReadingUseCase::new(
        router,
        PromptCompiler::new(prompts_root),
        ResponseValidator::new(Arc::new(SchemaRegistry::new())),
        EngineDefaults {
            provider: ProviderKind::OpenAi,
            model: model.to_string(),
        },
        ServiceLimits::default(),
        test_catalog(),
        PrivacyPolicy::default(),
        true,
    )
}

fn load_json_fixture(relative: &str) -> Value {
    let path = repo_root().join(relative);
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture"))
        .expect("valid json fixture")
}

fn load_text_fixture(relative: &str) -> String {
    let path = repo_root().join(relative);
    std::fs::read_to_string(path).expect("text fixture")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root")
}

#[tokio::test]
async fn horoscope_daily_stays_on_real_provider_when_configured_real() {
    let request = load_json_fixture(
        "tests/golden/horoscope_interpretation_request_basic_daily_paris_1990.json",
    );
    let response_fixture =
        load_json_fixture("tests/golden/horoscope_response_basic_daily_fake.json");
    let use_case = test_use_case(
        Arc::new(FixtureOpenAiProvider {
            fixture: response_fixture,
        }),
        "gpt-5-mini",
    );

    let response = daily_writer_response(&use_case, &request, None)
        .await
        .expect("daily horoscope response");

    assert_eq!(response["quality"]["provider"], "openai");
    assert_eq!(response["quality"]["model"], "gpt-5-mini");
    assert_eq!(response["quality"]["fallback_used"], false);
}

#[tokio::test]
async fn horoscope_daily_repairs_non_json_real_provider_response_without_fake_fallback() {
    let request = load_json_fixture(
        "tests/golden/horoscope_interpretation_request_basic_daily_paris_1990.json",
    );
    let response_fixture =
        load_json_fixture("tests/golden/horoscope_response_basic_daily_fake.json");
    let mut responses = VecDeque::new();
    responses.push_back(ProviderGenerationResponse {
        raw_text: "Je ne peux pas produire ce rendu au format demandé.".to_string(),
        parsed_json: None,
        usage: None,
        provider_metadata: serde_json::json!({
            "fixture": true,
            "incomplete_details": { "reason": "content_filter" }
        }),
        model_used: "initial-placeholder".to_string(),
        provider_kind: ProviderKind::OpenAi,
    });
    responses.push_back(ProviderGenerationResponse {
        raw_text: response_fixture.to_string(),
        parsed_json: Some(response_fixture),
        usage: Some(TokenUsage {
            input_tokens: 120,
            output_tokens: 240,
        }),
        provider_metadata: serde_json::json!({ "fixture": true, "repair": true }),
        model_used: "repair-placeholder".to_string(),
        provider_kind: ProviderKind::OpenAi,
    });
    let provider = Arc::new(SequenceOpenAiProvider {
        responses: Mutex::new(responses),
    });
    let use_case = test_use_case(provider.clone(), "gpt-5-mini");

    let response = daily_writer_response(&use_case, &request, None)
        .await
        .expect("daily horoscope response repaired by real provider");

    assert_eq!(response["quality"]["provider"], "openai");
    assert_eq!(response["quality"]["model"], "gpt-5-mini");
    assert_eq!(response["quality"]["fallback_used"], false);
    assert_eq!(response["quality"]["repair_attempted"], true);
    assert_eq!(
        provider
            .responses
            .lock()
            .expect("sequence provider mutex")
            .len(),
        0
    );
}

#[tokio::test]
async fn horoscope_daily_repairs_public_slot_code_leaks_from_real_provider() {
    let request = load_json_fixture("tests/golden/horoscope_interpretation_request_basic_daily_paris_1990.json");
    let mut response_fixture = load_json_fixture("tests/golden/horoscope_response_basic_daily_fake.json");
    response_fixture["slots"][0]["title"] = serde_json::json!("[morning]");
    response_fixture["slots"][0]["text"] =
        serde_json::json!("slot:morning La Lune donne un repère astrologique concret.");
    let use_case = test_use_case(
        Arc::new(FixtureOpenAiProvider {
            fixture: response_fixture,
        }),
        "gpt-5-mini",
    );

    let response = daily_writer_response(&use_case, &request, None)
        .await
        .expect("daily horoscope response");

    assert_ne!(response["slots"][0]["title"], "[morning]");
    assert!(
        !response["slots"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .contains("slot:morning")
    );
    validate_response_evidence(&request, &response).expect("repaired public slot text validates");
}

#[tokio::test]
async fn horoscope_period_stays_on_real_provider_when_configured_real() {
    let request = load_json_fixture(
        "tests/golden/horoscope_period_interpretation_request_free_next_7_days_paris_1990.json",
    );
    let response_fixture =
        load_json_fixture("tests/golden/horoscope_period_response_free_next_7_days_fake.json");
    let use_case = test_use_case(
        Arc::new(FixtureOpenAiProvider {
            fixture: response_fixture,
        }),
        "gpt-5-mini",
    );

    let response = period_writer_response_with_quality_loop(&use_case, &request, None)
        .await
        .expect("period horoscope response");

    assert_eq!(response["quality"]["provider"], "openai");
    assert_eq!(response["quality"]["model"], "gpt-5-mini");
    assert_eq!(response["quality"]["fallback_used"], false);
    assert_ne!(response["quality"]["provider"], "fake");
}

#[tokio::test]
async fn horoscope_period_free_neutralizes_key_day_best_day_language() {
    let request = load_json_fixture(
        "tests/golden/horoscope_period_interpretation_request_free_next_7_days_paris_1990.json",
    );
    let mut response_fixture =
        load_json_fixture("tests/golden/horoscope_period_response_free_next_7_days_fake.json");
    let second_key_day = response_fixture["key_days"][0].clone();
    response_fixture["key_days"]
        .as_array_mut()
        .expect("key_days array")
        .push(second_key_day);
    response_fixture["key_days"][1]["title"] = serde_json::json!("Meilleur créneau");
    response_fixture["key_days"][1]["reason"] =
        serde_json::json!("Cette journée est la meilleure fenêtre favorable pour profiter du climat.");
    let use_case = test_use_case(
        Arc::new(FixtureOpenAiProvider {
            fixture: response_fixture,
        }),
        "gpt-5-mini",
    );

    let response = period_writer_response_with_quality_loop(&use_case, &request, None)
        .await
        .expect("period horoscope response");

    assert_eq!(response["key_days"][1]["title"], "Jour à retenir");
    assert!(
        !response["key_days"][1]["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("favorable")
    );
}

#[tokio::test]
async fn horoscope_period_free_expands_too_short_real_provider_response() {
    let request = load_json_fixture(
        "tests/golden/horoscope_period_interpretation_request_free_next_7_days_paris_1990.json",
    );
    let mut response_fixture =
        load_json_fixture("tests/golden/horoscope_period_response_free_next_7_days_fake.json");
    response_fixture["summary"]["text"] = serde_json::json!("Période courte à observer.");
    response_fixture["dominant_theme"]["text"] =
        serde_json::json!("Un thème simple sert de repère.");
    response_fixture["key_days"][0]["reason"] =
        serde_json::json!("Ce jour donne un repère neutre pour observer le fil dominant.");
    response_fixture["advice"] = serde_json::json!("Gardez une priorité simple.");
    response_fixture["watch_summary"]["text"] =
        serde_json::json!("Ralentissez si une réaction paraît plus forte que la situation.");
    let use_case = test_use_case(
        Arc::new(FixtureOpenAiProvider {
            fixture: response_fixture,
        }),
        "gpt-5-mini",
    );

    let response = period_writer_response_with_quality_loop(&use_case, &request, None)
        .await
        .expect("free period response should be expanded before word-count validation");

    assert!(
        response["summary"]["text"]
            .as_str()
            .unwrap_or_default()
            .split_whitespace()
            .count()
            > 40
    );
}

#[test]
fn horoscope_product_model_config_does_not_force_fake() {
    let config = load_text_fixture("config/llm_product_models.conf");
    let horoscope_row = config
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .find(|line| line.split_whitespace().next() == Some("horoscope"))
        .expect("horoscope product model row");
    let columns = horoscope_row.split_whitespace().collect::<Vec<_>>();

    assert!(
        columns.len() >= 4,
        "horoscope row must define product, chapter model, summary model, provider"
    );
    assert_ne!(columns[1], "fake-model");
    assert_ne!(columns[2], "fake-model");
    assert_ne!(columns[3], "fake");
}
