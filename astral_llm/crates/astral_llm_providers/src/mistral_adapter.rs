use async_trait::async_trait;
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use serde_json::json;

use astral_llm_domain::{
    provider::{
        ProviderCapabilities, ProviderKind, StructuredOutputMode,
    },
    SafetyMode,
    ProviderKind as DomainProviderKind,
};

use crate::provider_trait::LlmProvider;
use crate::types::{
    PromptMessage, PromptRole, ProviderGenerationRequest, ProviderGenerationResponse,
    TokenUsage,
};
use crate::LlmProviderError;

pub struct MistralProvider {
    client: Client,
    api_key: SecretString,
    base_url: String,
}

impl MistralProvider {
    pub fn new(api_key: SecretString) -> Self {
        Self::with_base_url(api_key, "https://api.mistral.ai".to_string())
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
impl LlmProvider for MistralProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Mistral
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            structured_output: StructuredOutputMode::JsonSchemaStrict,
            supports_reasoning_effort: false,
            supports_streaming: true,
            supports_native_safety_prompt: true,
            supports_prompt_cache: false,
            max_input_tokens: Some(128_000),
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

impl MistralProvider {
    async fn generate_inner(
        &self,
        request: ProviderGenerationRequest,
    ) -> Result<ProviderGenerationResponse, LlmProviderError> {
        let messages = build_messages(&request.messages);
        let mut body = json!({
            "model": request.model,
            "messages": messages,
        });

        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        if let Some(max_tokens) = request.max_output_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        if request.safety_mode == SafetyMode::PlatformAndNative {
            body["safe_prompt"] = json!(true);
        }
        if let Some(schema) = &request.structured_schema {
            body["response_format"] = json!({
                "type": "json_schema",
                "json_schema": {
                    "name": "structured_reading",
                    "schema": schema,
                    "strict": true
                }
            });
        }

        let url = format!("{}/v1/chat/completions", self.base_url);
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

        let raw_text = payload
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmProviderError::InvalidResponse("missing content".into()))?
            .to_string();

        let parsed_json = serde_json::from_str(&raw_text).ok();
        let usage = payload.get("usage").map(parse_usage);

        Ok(ProviderGenerationResponse {
            raw_text,
            parsed_json,
            usage,
            provider_metadata: payload,
            model_used: request.model,
            provider_kind: DomainProviderKind::Mistral,
        })
    }
}

fn build_messages(messages: &[PromptMessage]) -> Vec<serde_json::Value> {
    messages
        .iter()
        .map(|m| {
            let role = match m.role {
                PromptRole::System | PromptRole::Developer => "system",
                PromptRole::User => "user",
                PromptRole::Assistant => "assistant",
            };
            json!({ "role": role, "content": m.content })
        })
        .collect()
}

fn parse_usage(value: &serde_json::Value) -> TokenUsage {
    TokenUsage {
        input_tokens: value
            .get("prompt_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        output_tokens: value
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
    }
}
