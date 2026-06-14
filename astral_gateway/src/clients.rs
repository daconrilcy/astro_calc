use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

use astral_llm_domain::{GenerateReadingRequest, GenerateReadingResponse};
use astral_llm_infra::CalculatorClient;

use crate::{
    error::GatewayError,
    ports::{CalculatorPort, LlmPort},
};

#[async_trait]
impl CalculatorPort for CalculatorClient {
    async fn calculate_simplified_natal(&self, request: &Value) -> Result<Value, GatewayError> {
        CalculatorClient::calculate_simplified_natal(self, request)
            .await
            .map_err(|err| GatewayError::upstream(err.detail().message.clone()))
    }

    async fn calculate_full_natal(&self, request: &Value) -> Result<Value, GatewayError> {
        CalculatorClient::calculate_natal(self, request)
            .await
            .map_err(|err| GatewayError::upstream(err.detail().message.clone()))
    }

    async fn calculate_horoscope_daily_natal(
        &self,
        request: &Value,
    ) -> Result<Value, GatewayError> {
        CalculatorClient::calculate_horoscope_daily_natal(self, request)
            .await
            .map_err(|err| GatewayError::upstream(err.detail().message.clone()))
    }

    async fn calculate_horoscope_period_natal(
        &self,
        request: &Value,
    ) -> Result<Value, GatewayError> {
        CalculatorClient::calculate_horoscope_period_natal(self, request)
            .await
            .map_err(|err| GatewayError::upstream(err.detail().message.clone()))
    }
}

#[derive(Clone)]
pub struct HttpLlmClient {
    base_url: String,
    api_key: Option<String>,
    client: Client,
}

impl HttpLlmClient {
    pub fn new(base_url: String, api_key: Option<String>, timeout_ms: u64) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_millis(timeout_ms.max(1_000)))
            .build()
            .map_err(|err| format!("llm HTTP client: {err}"))?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            client,
        })
    }

    async fn post_internal_json(&self, path: &str, request: &Value) -> Result<Value, GatewayError> {
        let url = format!("{}{}", self.base_url, path);
        let mut builder = self.client.post(url).json(request);
        if let Some(key) = &self.api_key {
            builder = builder
                .header("X-API-Key", key)
                .header("Authorization", format!("Bearer {key}"));
        }
        let response = builder
            .send()
            .await
            .map_err(|err| GatewayError::upstream(format!("llm request failed: {err}")))?;
        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<unreadable body>".to_string());
            return Err(GatewayError::upstream(format!(
                "llm rejected request: status={} body={}",
                status,
                truncate_for_error(&body)
            )));
        }
        response
            .json::<Value>()
            .await
            .map_err(|err| GatewayError::upstream(format!("llm response parse failed: {err}")))
    }
}

#[async_trait]
impl LlmPort for HttpLlmClient {
    async fn generate_reading(
        &self,
        request: &GenerateReadingRequest,
    ) -> Result<GenerateReadingResponse, GatewayError> {
        let url = format!("{}/v1/internal/readings/render", self.base_url);
        let mut builder = self.client.post(url).json(request);
        if let Some(key) = &self.api_key {
            builder = builder
                .header("X-API-Key", key)
                .header("Authorization", format!("Bearer {key}"));
        }
        let response = builder
            .send()
            .await
            .map_err(|err| GatewayError::upstream(format!("llm request failed: {err}")))?;
        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<unreadable body>".to_string());
            return Err(GatewayError::upstream(format!(
                "llm rejected request: status={} body={}",
                status,
                truncate_for_error(&body)
            )));
        }
        let body = response
            .json::<GenerateReadingResponse>()
            .await
            .map_err(|err| GatewayError::upstream(format!("llm response parse failed: {err}")))?;
        Ok(body)
    }

    async fn render_horoscope_daily(&self, request: &Value) -> Result<Value, GatewayError> {
        self.post_internal_json("/v1/internal/horoscope/daily/render", request)
            .await
    }

    async fn render_horoscope_period(&self, request: &Value) -> Result<Value, GatewayError> {
        self.post_internal_json("/v1/internal/horoscope/period/render", request)
            .await
    }
}

fn truncate_for_error(body: &str) -> String {
    const MAX: usize = 400;
    if body.len() <= MAX {
        body.to_string()
    } else {
        format!("{}...", &body[..MAX])
    }
}
