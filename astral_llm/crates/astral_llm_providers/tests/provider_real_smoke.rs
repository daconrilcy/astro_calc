//! Smoke tests providers reels (manuel, reseau + cles).
//!
//! ```bash
//! # Depuis la racine du depot, avec cles renseignees dans .env :
//! cargo test -p astral_llm_providers --test provider_real_smoke -- --ignored
//! ```

use std::time::Duration;

use astral_llm_domain::{ProviderKind, SafetyMode};
use astral_llm_providers::{
    anthropic_adapter::AnthropicProvider, mistral_adapter::MistralProvider,
    openai_adapter::OpenAiProvider, GenerationMetadata, LlmProvider, LlmProviderError,
    PromptMessage, PromptRole, ProviderGenerationRequest,
};
use secrecy::SecretString;

fn minimal_ok_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": { "ok": { "type": "boolean" } },
        "required": ["ok"],
        "additionalProperties": false
    })
}

fn minimal_structured_request(model: String, product_code: &str) -> ProviderGenerationRequest {
    ProviderGenerationRequest {
        model,
        messages: vec![PromptMessage {
            role: PromptRole::User,
            content: "Respond with JSON only: {\"ok\": true}".into(),
        }],
        structured_schema: Some(minimal_ok_schema()),
        reasoning_effort: None,
        temperature: Some(0.0),
        max_output_tokens: Some(128),
        safety_mode: SafetyMode::PlatformRulesOnly,
        timeout: Duration::from_secs(90),
        metadata: GenerationMetadata {
            run_id: "smoke".into(),
            request_id: None,
            product_code: product_code.into(),
            chapter_code: None,
        },
    }
}

async fn assert_invalid_api_key_rejected<P: LlmProvider>(provider: P) {
    let request = minimal_structured_request("smoke-model".into(), "natal_prompter");
    let err = provider
        .generate(request)
        .await
        .expect_err("invalid key must fail");
    let msg = err.to_string().to_lowercase();
    assert!(
        matches!(
            err,
            LlmProviderError::Http(_) | LlmProviderError::Api(_) | LlmProviderError::Config(_)
        ),
        "unexpected error variant: {err}"
    );
    assert!(
        msg.contains("401")
            || msg.contains("403")
            || msg.contains("unauthorized")
            || msg.contains("invalid")
            || msg.contains("authentication"),
        "expected auth-related failure, got: {msg}"
    );
}

/// Charge le `.env` a la racine du depot (`astral_calculation/.env`), pas seulement le cwd du test.
fn load_dotenv_repo_root() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidates = [manifest.join("../../../.env"), manifest.join("../../.env")];
    for path in &candidates {
        if path.is_file() {
            dotenvy::from_path(path).ok();
            return;
        }
    }
    dotenvy::dotenv().ok();
}

fn load_api_key(env_key: &str) -> SecretString {
    load_dotenv_repo_root();
    let value = std::env::var(env_key).unwrap_or_else(|_| {
        panic!(
            "{env_key} absent : renseigner dans le .env a la racine du depot \
             (ex. C:\\dev\\astral_calculation\\.env)"
        );
    });
    let trimmed = value.trim();
    if trimmed.is_empty() {
        panic!(
            "{env_key} est vide dans .env — coller la cle provider reelle \
             (les lignes OPENAI_API_KEY= / MISTRAL_API_KEY= / ANTHROPIC_API_KEY= sans valeur \
             provoquent un 401 avec cle vide)"
        );
    }
    SecretString::from(trimmed.to_string())
}

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY and network"]
async fn openai_structured_minimal_smoke() {
    let provider = OpenAiProvider::with_client(
        reqwest::Client::builder()
            .timeout(Duration::from_secs(90))
            .build()
            .expect("client"),
        load_api_key("OPENAI_API_KEY"),
        std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com".into()),
    );
    let model = std::env::var("OPENAI_DEFAULT_MODEL").unwrap_or_else(|_| "gpt-4.1".into());
    let response = provider
        .generate(minimal_structured_request(model, "natal_prompter"))
        .await
        .expect("openai ok");
    assert_eq!(provider.kind(), ProviderKind::OpenAi);
    assert_eq!(response.parsed_json.as_ref().unwrap()["ok"], true);
}

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY and network"]
async fn openai_invalid_api_key_smoke() {
    let bad = OpenAiProvider::with_client(
        reqwest::Client::new(),
        SecretString::from("sk-invalid-smoke-key"),
        std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com".into()),
    );
    assert_invalid_api_key_rejected(bad).await;
}

#[tokio::test]
#[ignore = "requires MISTRAL_API_KEY and network"]
async fn mistral_structured_minimal_smoke() {
    let provider = MistralProvider::with_client(
        reqwest::Client::builder()
            .timeout(Duration::from_secs(90))
            .build()
            .expect("client"),
        load_api_key("MISTRAL_API_KEY"),
        std::env::var("MISTRAL_BASE_URL").unwrap_or_else(|_| "https://api.mistral.ai".into()),
    );
    let model =
        std::env::var("MISTRAL_DEFAULT_MODEL").unwrap_or_else(|_| "mistral-small-latest".into());
    let response = provider
        .generate(minimal_structured_request(model, "natal_prompter"))
        .await
        .expect("mistral ok");
    assert_eq!(provider.kind(), ProviderKind::Mistral);
    assert!(response.parsed_json.is_some());
}

#[tokio::test]
#[ignore = "requires MISTRAL_API_KEY and network"]
async fn mistral_invalid_api_key_smoke() {
    let bad = MistralProvider::with_base_url(
        SecretString::from("invalid-mistral-key"),
        std::env::var("MISTRAL_BASE_URL").unwrap_or_else(|_| "https://api.mistral.ai".into()),
    );
    assert_invalid_api_key_rejected(bad).await;
}

#[tokio::test]
#[ignore = "requires ANTHROPIC_API_KEY and network"]
async fn anthropic_structured_minimal_smoke() {
    let provider = AnthropicProvider::with_client(
        reqwest::Client::builder()
            .timeout(Duration::from_secs(90))
            .build()
            .expect("client"),
        load_api_key("ANTHROPIC_API_KEY"),
        std::env::var("ANTHROPIC_BASE_URL").unwrap_or_else(|_| "https://api.anthropic.com".into()),
    );
    let model = std::env::var("ANTHROPIC_DEFAULT_MODEL")
        .unwrap_or_else(|_| "claude-sonnet-4-20250514".into());
    let response = provider
        .generate(minimal_structured_request(model, "natal_prompter"))
        .await
        .expect("anthropic ok");
    assert_eq!(provider.kind(), ProviderKind::Anthropic);
    assert!(response.parsed_json.is_some());
}

#[tokio::test]
#[ignore = "requires ANTHROPIC_API_KEY and network"]
async fn anthropic_invalid_api_key_smoke() {
    let bad = AnthropicProvider::with_base_url(
        SecretString::from("invalid-anthropic-key"),
        std::env::var("ANTHROPIC_BASE_URL").unwrap_or_else(|_| "https://api.anthropic.com".into()),
    );
    assert_invalid_api_key_rejected(bad).await;
}

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY and network"]
async fn openai_chapter_provider_schema_smoke() {
    use astral_llm_application::{
        provider_schema_compiler::pin_chapter_code, reasoning_generation::effective_temperature,
        ModelCapabilityRegistry, ProviderSchemaCompiler, SchemaRegistry,
    };
    use std::sync::Arc;

    let provider = OpenAiProvider::with_client(
        reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("client"),
        load_api_key("OPENAI_API_KEY"),
        std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com".into()),
    );
    let model = std::env::var("OPENAI_DEFAULT_MODEL").unwrap_or_else(|_| "gpt-4.1".into());
    let registry = SchemaRegistry::new();
    let mut schema = registry
        .provider_schema("chapter_provider_v1")
        .expect("chapter schema")
        .clone();
    pin_chapter_code(&mut schema, "identity");
    let cap = Arc::new(ModelCapabilityRegistry::bootstrap_dev_fallback())
        .require(&ProviderKind::OpenAi, &model)
        .expect("capability")
        .clone();
    let compiled = ProviderSchemaCompiler::compile(&schema, &cap).expect("compile schema");

    let request = ProviderGenerationRequest {
        model,
        messages: vec![
            PromptMessage {
                role: PromptRole::System,
                content: "Return JSON for a natal chapter. Cite astro_basis with fact_ids from data.".into(),
            },
            PromptMessage {
                role: PromptRole::User,
                content: "Chapter identity. Data: {\"facts\":[{\"id\":\"domain_score:identity\"},{\"id\":\"placement:sun:capricorn:house:2\"}]}".into(),
            },
        ],
        structured_schema: Some(compiled),
        reasoning_effort: None,
        temperature: effective_temperature(&cap, Some(0.4)),
        max_output_tokens: Some(600),
        safety_mode: SafetyMode::PlatformRulesOnly,
        timeout: Duration::from_secs(120),
        metadata: GenerationMetadata {
            run_id: "chapter-smoke".into(),
            request_id: None,
            product_code: "natal_prompter".into(),
            chapter_code: Some("identity".into()),
        },
    };

    let response = provider.generate(request).await.expect("openai chapter ok");
    assert!(
        response.parsed_json.is_some(),
        "expected parsed JSON, raw={:?}",
        response.raw_text
    );
    let json = response.parsed_json.unwrap();
    assert_eq!(json.get("code").and_then(|v| v.as_str()), Some("identity"));
}
