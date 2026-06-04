use async_trait::async_trait;
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use serde_json::json;

use astral_llm_domain::{
    provider::{
        ProviderCapabilities, ProviderKind, ReasoningEffort, StructuredOutputMode,
    },
    ProviderKind as DomainProviderKind,
};

use crate::provider_trait::LlmProvider;
use crate::types::{
    PromptMessage, PromptRole, ProviderGenerationRequest, ProviderGenerationResponse,
    TokenUsage,
};
use crate::LlmProviderError;

pub struct AnthropicProvider {
    client: Client,
    api_key: SecretString,
    base_url: String,
}

impl AnthropicProvider {
    pub fn new(api_key: SecretString) -> Self {
        Self::with_base_url(api_key, "https://api.anthropic.com".to_string())
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

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Anthropic
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            structured_output: StructuredOutputMode::JsonSchemaStrict,
            supports_reasoning_effort: true,
            supports_streaming: true,
            supports_native_safety_prompt: false,
            supports_prompt_cache: false,
            max_input_tokens: Some(200_000),
            max_output_tokens: Some(8_192),
        }
    }

    async fn generate(
        &self,
        request: ProviderGenerationRequest,
    ) -> Result<ProviderGenerationResponse, LlmProviderError> {
        crate::http::with_timeout(request.timeout, self.generate_inner(request)).await
    }
}

impl AnthropicProvider {
    async fn generate_inner(
        &self,
        request: ProviderGenerationRequest,
    ) -> Result<ProviderGenerationResponse, LlmProviderError> {
        let (system, messages) = split_messages(&request.messages);
        let mut body = json!({
            "model": request.model,
            "max_tokens": request.max_output_tokens.unwrap_or(4096),
            "messages": messages,
        });

        if let Some(system) = system {
            body["system"] = json!(system);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        if let Some(schema) = &request.structured_schema {
            body["output_config"] = json!({
                "format": {
                    "type": "json_schema",
                    "schema": schema
                }
            });
        }
        if let Some(effort) = request.reasoning_effort {
            if !matches!(effort, ReasoningEffort::None) {
                body["thinking"] = json!({
                    "type": "enabled",
                    "budget_tokens": match effort {
                        ReasoningEffort::Minimal | ReasoningEffort::Low => 1024,
                        ReasoningEffort::Medium => 4096,
                        ReasoningEffort::High => 8192,
                        ReasoningEffort::None => 0,
                    }
                });
            }
        }

        let url = format!("{}/v1/messages", self.base_url);
        let response = self
            .client
            .post(&url)
            .header("x-api-key", self.api_key.expose_secret())
            .header("anthropic-version", "2023-06-01")
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

        let raw_text = extract_text(&payload)?;
        let parsed_json = serde_json::from_str(&raw_text).ok();
        let usage = payload.get("usage").map(parse_usage);

        Ok(ProviderGenerationResponse {
            raw_text,
            parsed_json,
            usage,
            provider_metadata: payload,
            model_used: request.model,
            provider_kind: DomainProviderKind::Anthropic,
        })
    }
}

fn split_messages(messages: &[PromptMessage]) -> (Option<String>, Vec<serde_json::Value>) {
    let mut system_parts = Vec::new();
    let mut out = Vec::new();

    for message in messages {
        match message.role {
            PromptRole::System | PromptRole::Developer => {
                system_parts.push(message.content.clone());
            }
            PromptRole::User => {
                out.push(json!({ "role": "user", "content": message.content }));
            }
            PromptRole::Assistant => {
                out.push(json!({ "role": "assistant", "content": message.content }));
            }
        }
    }

    let system = if system_parts.is_empty() {
        None
    } else {
        Some(system_parts.join("\n\n"))
    };

    (system, out)
}

fn extract_text(payload: &serde_json::Value) -> Result<String, LlmProviderError> {
    let content = payload
        .get("content")
        .and_then(|v| v.as_array())
        .ok_or_else(|| LlmProviderError::InvalidResponse("missing content".into()))?;

    for block in content {
        if block.get("type").and_then(|v| v.as_str()) == Some("text") {
            if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                return Ok(text.to_string());
            }
        }
    }

    Err(LlmProviderError::InvalidResponse(
        "no text block in response".into(),
    ))
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
