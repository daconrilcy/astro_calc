use async_trait::async_trait;
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use serde_json::json;

use astral_llm_domain::{
    provider::{ProviderCapabilities, ProviderKind, ReasoningEffort, StructuredOutputMode},
    ProviderKind as DomainProviderKind,
};

use crate::provider_trait::LlmProvider;
use crate::types::{
    PromptMessage, PromptRole, ProviderGenerationRequest, ProviderGenerationResponse, TokenUsage,
};
use crate::LlmProviderError;

pub struct OpenAiProvider {
    client: Client,
    api_key: SecretString,
    base_url: String,
}

impl OpenAiProvider {
    pub fn new(api_key: SecretString) -> Self {
        Self::with_base_url(api_key, "https://api.openai.com".to_string())
    }

    pub fn with_client(client: Client, api_key: SecretString, base_url: String) -> Self {
        Self {
            client,
            api_key,
            base_url,
        }
    }

    pub fn with_base_url(api_key: SecretString, base_url: String) -> Self {
        Self::with_client(Client::new(), api_key, base_url)
    }
}

#[cfg(test)]
impl OpenAiProvider {
    pub fn with_base_url_for_test(api_key: SecretString, base_url: String) -> Self {
        Self::with_base_url(api_key, base_url)
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::OpenAi
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            structured_output: StructuredOutputMode::JsonSchemaStrict,
            supports_reasoning_effort: true,
            supports_streaming: true,
            supports_native_safety_prompt: false,
            supports_prompt_cache: true,
            max_input_tokens: Some(128_000),
            max_output_tokens: Some(16_384),
        }
    }

    async fn generate(
        &self,
        request: ProviderGenerationRequest,
    ) -> Result<ProviderGenerationResponse, LlmProviderError> {
        crate::http::with_timeout(request.timeout, self.generate_inner(request)).await
    }
}

impl OpenAiProvider {
    async fn generate_inner(
        &self,
        request: ProviderGenerationRequest,
    ) -> Result<ProviderGenerationResponse, LlmProviderError> {
        let input = build_input(&request.messages);
        let mut body = json!({
            "model": request.model,
            "input": input,
            "store": false,
        });

        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        if let Some(max_tokens) = request.max_output_tokens {
            body["max_output_tokens"] = json!(max_tokens);
        }
        if let Some(effort) = request.reasoning_effort {
            body["reasoning"] = json!({ "effort": reasoning_effort_str(effort) });
        }
        if let Some(schema) = &request.structured_schema {
            body["text"] = json!({
                "format": {
                    "type": "json_schema",
                    "name": "structured_reading",
                    "schema": schema,
                    "strict": true
                }
            });
        }

        let url = format!("{}/v1/responses", self.base_url);
        let response = self
            .client
            .post(&url)
            .bearer_auth(self.api_key.expose_secret())
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmProviderError::Http(e.to_string()))?;

        if response.status().as_u16() == 429 {
            return Err(LlmProviderError::RateLimited);
        }
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(LlmProviderError::Api(format!("{status}: {text}")));
        }

        let payload: serde_json::Value = response
            .json()
            .await
            .map_err(|e| LlmProviderError::InvalidResponse(e.to_string()))?;

        let raw_text = extract_output_text(&payload)?;
        let parsed_json = serde_json::from_str(&raw_text).ok();
        let usage = payload.get("usage").map(parse_usage);

        Ok(ProviderGenerationResponse {
            raw_text,
            parsed_json,
            usage,
            provider_metadata: payload,
            model_used: request.model,
            provider_kind: DomainProviderKind::OpenAi,
        })
    }
}

fn build_input(messages: &[PromptMessage]) -> Vec<serde_json::Value> {
    messages
        .iter()
        .map(|m| {
            let role = match m.role {
                PromptRole::System | PromptRole::Developer => "developer",
                PromptRole::User => "user",
                PromptRole::Assistant => "assistant",
            };
            json!({
                "role": role,
                "content": [{ "type": "input_text", "text": m.content }]
            })
        })
        .collect()
}

fn reasoning_effort_str(effort: ReasoningEffort) -> &'static str {
    match effort {
        ReasoningEffort::None => "none",
        ReasoningEffort::Minimal => "minimal",
        ReasoningEffort::Low => "low",
        ReasoningEffort::Medium => "medium",
        ReasoningEffort::High => "high",
    }
}

fn extract_output_text(payload: &serde_json::Value) -> Result<String, LlmProviderError> {
    if let Some(text) = payload.get("output_text").and_then(|v| v.as_str()) {
        if !text.is_empty() {
            return Ok(text.to_string());
        }
    }

    let assembled = collect_assistant_output_text(payload);
    if !assembled.is_empty() {
        return Ok(assembled);
    }

    Err(missing_output_text_error(payload))
}

/// Concatene tous les blocs texte des messages assistant (Responses API GPT-5).
fn collect_assistant_output_text(payload: &serde_json::Value) -> String {
    let Some(outputs) = payload.get("output").and_then(|v| v.as_array()) else {
        return String::new();
    };

    let mut parts = Vec::new();
    for item in outputs {
        if item.get("type").and_then(|v| v.as_str()) != Some("message") {
            continue;
        }
        let role = item
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("assistant");
        if role != "assistant" {
            continue;
        }
        let Some(content) = item.get("content").and_then(|v| v.as_array()) else {
            continue;
        };
        for part in content {
            let part_type = part.get("type").and_then(|v| v.as_str());
            if !matches!(part_type, Some("output_text") | Some("text")) {
                continue;
            }
            if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                if !text.is_empty() {
                    parts.push(text);
                }
            }
        }
    }
    parts.join("")
}

fn missing_output_text_error(payload: &serde_json::Value) -> LlmProviderError {
    let status = payload
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    if status == "incomplete" {
        let details = payload
            .get("incomplete_details")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        return LlmProviderError::InvalidResponse(format!(
            "incomplete response ({details}): no assistant message text"
        ));
    }

    if output_has_only_reasoning(payload) {
        return LlmProviderError::InvalidResponse(
            "missing output text: response contains reasoning only; increase max_output_tokens"
                .to_string(),
        );
    }

    LlmProviderError::InvalidResponse("missing output text".to_string())
}

fn output_has_only_reasoning(payload: &serde_json::Value) -> bool {
    let Some(outputs) = payload.get("output").and_then(|v| v.as_array()) else {
        return false;
    };
    !outputs.is_empty()
        && outputs
            .iter()
            .all(|item| item.get("type").and_then(|v| v.as_str()) == Some("reasoning"))
}

#[cfg(test)]
mod extract_tests {
    use super::*;

    #[test]
    fn uses_top_level_output_text() {
        let payload = json!({ "output_text": "{\"ok\":true}" });
        assert_eq!(extract_output_text(&payload).unwrap(), "{\"ok\":true}");
    }

    #[test]
    fn falls_back_to_output_array_messages() {
        let payload = json!({
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
        });
        assert_eq!(
            extract_output_text(&payload).unwrap(),
            "{\"chapter\":\"identity\"}"
        );
    }

    #[test]
    fn reasoning_only_yields_actionable_error() {
        let payload = json!({
            "status": "completed",
            "output": [{ "type": "reasoning", "id": "r1" }]
        });
        let err = extract_output_text(&payload).unwrap_err().to_string();
        assert!(err.contains("reasoning only"));
    }
}

fn parse_usage(value: &serde_json::Value) -> TokenUsage {
    TokenUsage {
        input_tokens: value
            .get("input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        output_tokens: value
            .get("output_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
    }
}
