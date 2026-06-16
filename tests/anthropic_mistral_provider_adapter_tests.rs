use std::time::Duration;

use astral_llm_domain::{
    provider::{ProviderKind, StructuredOutputMode},
    SafetyMode, TokenUsageType,
};
use astral_llm_providers::{
    AnthropicProvider, GenerationMetadata, LlmProvider, MistralProvider, PromptMessage, PromptRole,
    ProviderGenerationRequest,
};
use secrecy::SecretString;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
};

fn request(model: &str) -> ProviderGenerationRequest {
    ProviderGenerationRequest {
        model: model.into(),
        messages: vec![PromptMessage {
            role: PromptRole::User,
            content: "Return JSON".into(),
        }],
        structured_schema: None,
        reasoning_effort: None,
        temperature: Some(0.0),
        max_output_tokens: Some(128),
        safety_mode: SafetyMode::PlatformRulesOnly,
        timeout: Duration::from_secs(5),
        metadata: GenerationMetadata {
            run_id: "test-run".into(),
            request_id: None,
            product_code: "natal_prompter".into(),
            chapter_code: None,
            prompt_trace_step: None,
            prompt_trace_attempt: None,
            prompt_family: None,
            prompt_version: None,
        },
    }
}

async fn spawn_stub(
    response_json: serde_json::Value,
) -> (
    String,
    oneshot::Receiver<serde_json::Value>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind stub");
    let address = listener.local_addr().expect("stub addr");
    let (tx, rx) = oneshot::channel();
    let body = response_json.to_string();
    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let mut buffer = Vec::new();
        let mut header_end = None;

        loop {
            let mut chunk = [0_u8; 1024];
            let read = stream.read(&mut chunk).await.expect("read request");
            if read == 0 {
                break;
            }
            buffer.extend_from_slice(&chunk[..read]);
            if let Some(pos) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
                header_end = Some(pos + 4);
                break;
            }
        }

        let header_end = header_end.expect("request headers");
        let headers = String::from_utf8_lossy(&buffer[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if name.eq_ignore_ascii_case("content-length") {
                    value.trim().parse::<usize>().ok()
                } else {
                    None
                }
            })
            .expect("content-length");

        while buffer.len() < header_end + content_length {
            let mut chunk = vec![0_u8; content_length];
            let read = stream.read(&mut chunk).await.expect("read body");
            if read == 0 {
                break;
            }
            buffer.extend_from_slice(&chunk[..read]);
        }

        let request_body = &buffer[header_end..header_end + content_length];
        let request_json: serde_json::Value =
            serde_json::from_slice(request_body).expect("request json");
        tx.send(request_json).expect("capture request");

        let http_response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(http_response.as_bytes())
            .await
            .expect("write response");
    });

    (format!("http://{}", address), rx, handle)
}

#[tokio::test]
async fn anthropic_provider_maps_input_output_and_cache_subtypes() {
    let (base_url, _captured, handle) = spawn_stub(serde_json::json!({
        "content": [{ "type": "text", "text": "{\"ok\":true}" }],
        "usage": {
            "input_tokens": 80,
            "output_tokens": 25,
            "cache_read_input_tokens": 10,
            "cache_creation_input_tokens": 6
        }
    }))
    .await;
    let provider = AnthropicProvider::with_base_url(SecretString::from("test-key"), base_url);

    let response = provider
        .generate(request("claude-sonnet-4-20250514"))
        .await
        .expect("provider response");

    assert_eq!(response.provider_kind, ProviderKind::Anthropic);
    let usage = response.usage.expect("usage");
    assert_eq!(usage.tokens_for(TokenUsageType::Input), Some(80));
    assert_eq!(usage.tokens_for(TokenUsageType::Output), Some(25));
    assert_eq!(usage.tokens_for(TokenUsageType::Cache), Some(16));
    assert_eq!(usage.tokens_for(TokenUsageType::Reasoning), None);
    assert!(usage.items.iter().any(|item| {
        item.usage_type == TokenUsageType::Cache && item.usage_subtype.as_deref() == Some("read")
    }));
    assert!(usage.items.iter().any(|item| {
        item.usage_type == TokenUsageType::Cache && item.usage_subtype.as_deref() == Some("write")
    }));
    handle.await.expect("stub join");
}

#[tokio::test]
async fn mistral_provider_maps_input_output_and_cache_usage() {
    let (base_url, _captured, handle) = spawn_stub(serde_json::json!({
        "choices": [{ "message": { "content": "{\"ok\":true}" } }],
        "usage": {
            "prompt_tokens": 90,
            "completion_tokens": 30,
            "cached_tokens": 12
        }
    }))
    .await;
    let provider = MistralProvider::with_base_url(SecretString::from("test-key"), base_url);

    let response = provider
        .generate(request("mistral-large-latest"))
        .await
        .expect("provider response");

    assert_eq!(response.provider_kind, ProviderKind::Mistral);
    let usage = response.usage.expect("usage");
    assert_eq!(usage.tokens_for(TokenUsageType::Input), Some(90));
    assert_eq!(usage.tokens_for(TokenUsageType::Output), Some(30));
    assert_eq!(usage.tokens_for(TokenUsageType::Cache), Some(12));
    assert_eq!(usage.tokens_for(TokenUsageType::Reasoning), None);
    assert!(usage.items.iter().any(|item| {
        item.usage_type == TokenUsageType::Cache
            && item.usage_subtype.as_deref() == Some("read")
            && item.provider_metric_name.as_deref() == Some("cached_tokens")
    }));
    assert_eq!(
        provider.capabilities().structured_output,
        StructuredOutputMode::JsonSchemaStrict
    );
    handle.await.expect("stub join");
}
