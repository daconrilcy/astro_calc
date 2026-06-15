use std::time::Duration;

use astral_llm_domain::{provider::ReasoningEffort, SafetyMode};
use astral_llm_providers::{
    GenerationMetadata, LlmProvider, LlmProviderError, OpenAiProvider, PromptMessage, PromptRole,
    ProviderGenerationRequest,
};
use secrecy::SecretString;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
};

fn request_with_reasoning(reasoning_effort: Option<ReasoningEffort>) -> ProviderGenerationRequest {
    ProviderGenerationRequest {
        model: "gpt-5-mini".into(),
        messages: vec![PromptMessage {
            role: PromptRole::User,
            content: "Return JSON".into(),
        }],
        structured_schema: None,
        reasoning_effort,
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

async fn spawn_openai_stub(
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
async fn openai_provider_uses_top_level_output_text() {
    let (base_url, _captured, handle) = spawn_openai_stub(serde_json::json!({
        "output_text": "{\"ok\":true}",
        "usage": { "input_tokens": 12, "output_tokens": 7 }
    }))
    .await;
    let provider = OpenAiProvider::with_base_url(SecretString::from("test-key"), base_url);

    let response = provider
        .generate(request_with_reasoning(None))
        .await
        .expect("provider response");

    assert_eq!(response.raw_text, "{\"ok\":true}");
    assert_eq!(
        response
            .parsed_json
            .as_ref()
            .and_then(|json| json.get("ok")),
        Some(&serde_json::json!(true))
    );
    handle.await.expect("stub join");
}

#[tokio::test]
async fn openai_provider_falls_back_to_output_messages() {
    let (base_url, _captured, handle) = spawn_openai_stub(serde_json::json!({
        "output": [
            { "type": "reasoning", "id": "r1" },
            {
                "type": "message",
                "role": "assistant",
                "content": [
                    { "type": "output_text", "text": "{\"chapter\":\"identity\"}" }
                ]
            }
        ]
    }))
    .await;
    let provider = OpenAiProvider::with_base_url(SecretString::from("test-key"), base_url);

    let response = provider
        .generate(request_with_reasoning(None))
        .await
        .expect("provider response");

    assert_eq!(response.raw_text, "{\"chapter\":\"identity\"}");
    assert_eq!(
        response
            .parsed_json
            .as_ref()
            .and_then(|json| json.get("chapter"))
            .and_then(|value| value.as_str()),
        Some("identity")
    );
    handle.await.expect("stub join");
}

#[tokio::test]
async fn openai_provider_reports_reasoning_only_responses() {
    let (base_url, _captured, handle) = spawn_openai_stub(serde_json::json!({
        "status": "completed",
        "output": [{ "type": "reasoning", "id": "r1" }]
    }))
    .await;
    let provider = OpenAiProvider::with_base_url(SecretString::from("test-key"), base_url);

    let error = provider
        .generate(request_with_reasoning(None))
        .await
        .expect_err("reasoning only should fail");

    assert!(matches!(error, LlmProviderError::InvalidResponse(_)));
    assert!(error.to_string().contains("reasoning only"));
    handle.await.expect("stub join");
}

#[tokio::test]
async fn openai_provider_downgrades_none_reasoning_effort_to_minimal() {
    let (base_url, captured, handle) = spawn_openai_stub(serde_json::json!({
        "output_text": "{\"ok\":true}"
    }))
    .await;
    let provider = OpenAiProvider::with_base_url(SecretString::from("test-key"), base_url);

    provider
        .generate(request_with_reasoning(Some(ReasoningEffort::None)))
        .await
        .expect("provider response");

    let request_json = captured.await.expect("captured request");
    assert_eq!(
        request_json
            .get("reasoning")
            .and_then(|value| value.get("effort"))
            .and_then(|value| value.as_str()),
        Some("minimal")
    );
    handle.await.expect("stub join");
}
