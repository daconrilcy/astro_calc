use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::Serialize;
use serde_json::Value;

use crate::{
    error::GatewayError,
    ports::{CalculatorPort, LlmPort},
};

#[derive(Clone)]
pub struct HttpCalculatorClient {
    base_url: String,
    api_key: Option<String>,
    client: Client,
}

impl HttpCalculatorClient {
    const SIMPLIFIED_NATAL_PATH: &'static str = "/v1/internal/calculations/natal/simplified";
    const FULL_NATAL_PATH: &'static str = "/v1/internal/calculations/natal";
    const HOROSCOPE_DAILY_NATAL_PATH: &'static str =
        "/v1/internal/calculations/horoscope/daily-natal";
    const HOROSCOPE_PERIOD_NATAL_PATH: &'static str =
        "/v1/internal/calculations/horoscope/period/natal";

    pub fn new(base_url: String, api_key: Option<String>, timeout_ms: u64) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_millis(timeout_ms.max(1_000)))
            .build()
            .map_err(|err| format!("calculator HTTP client: {err}"))?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            client,
        })
    }

    async fn post_json(&self, path: &str, request: &Value) -> Result<Value, GatewayError> {
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
            .map_err(|err| GatewayError::upstream(format!("calculator request failed: {err}")))?;
        let status = response.status();
        let body = response.json::<Value>().await.map_err(|err| {
            GatewayError::upstream(format!("calculator response parse failed: {err}"))
        })?;
        if !status.is_success() {
            return Err(GatewayError::bad_request(format!(
                "calculator rejected request: status={} body={}",
                status,
                truncate_for_error(&body.to_string())
            )));
        }
        Ok(body)
    }
}

#[async_trait]
impl CalculatorPort for HttpCalculatorClient {
    async fn calculate_simplified_natal(&self, request: &Value) -> Result<Value, GatewayError> {
        self.post_json(Self::SIMPLIFIED_NATAL_PATH, request).await
    }

    async fn calculate_full_natal(&self, request: &Value) -> Result<Value, GatewayError> {
        self.post_json(Self::FULL_NATAL_PATH, request).await
    }

    async fn calculate_horoscope_daily_natal(
        &self,
        request: &Value,
    ) -> Result<Value, GatewayError> {
        self.post_json(Self::HOROSCOPE_DAILY_NATAL_PATH, request)
            .await
    }

    async fn calculate_horoscope_period_natal(
        &self,
        request: &Value,
    ) -> Result<Value, GatewayError> {
        self.post_json(Self::HOROSCOPE_PERIOD_NATAL_PATH, request)
            .await
    }
}

#[derive(Clone)]
pub struct HttpLlmClient {
    base_url: String,
    api_key: Option<String>,
    client: Client,
}

impl HttpLlmClient {
    const MAX_TIMEOUT_RETRIES: u8 = 1;

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
        let response = self.send_llm_json_with_timeout_retry(&url, request).await?;
        response
            .json::<Value>()
            .await
            .map_err(|err| GatewayError::upstream(format!("llm response parse failed: {err}")))
    }

    async fn post_internal_json_without_retry(
        &self,
        path: &str,
        request: &Value,
    ) -> Result<Value, GatewayError> {
        let url = format!("{}{}", self.base_url, path);
        let response = self.send_llm_json_without_retry(&url, request).await?;
        response
            .json::<Value>()
            .await
            .map_err(|err| GatewayError::upstream(format!("llm response parse failed: {err}")))
    }

    async fn get_internal_json(&self, path: &str) -> Result<Value, GatewayError> {
        let url = format!("{}{}", self.base_url, path);
        let mut builder = self.client.get(url);
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

    async fn send_llm_json_with_timeout_retry<T: Serialize + ?Sized>(
        &self,
        url: &str,
        request: &T,
    ) -> Result<reqwest::Response, GatewayError> {
        let mut attempt = 0;
        loop {
            let response = self.send_llm_json_once(url, request).await;
            match response {
                Ok(response) => return Ok(response),
                Err(LlmHttpError::Timeout(message)) if attempt < Self::MAX_TIMEOUT_RETRIES => {
                    attempt += 1;
                    tracing::warn!(
                        attempt,
                        max_retries = Self::MAX_TIMEOUT_RETRIES,
                        error = %message,
                        "llm request timed out, retrying"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                }
                Err(LlmHttpError::Timeout(message)) => {
                    return Err(GatewayError::upstream(format!(
                        "llm request timed out after {} attempt(s): {message}",
                        attempt + 1
                    )));
                }
                Err(LlmHttpError::Rejected { status, body }) => {
                    return Err(GatewayError::upstream(format!(
                        "llm rejected request: status={} body={}",
                        status,
                        truncate_for_error(&body)
                    )));
                }
                Err(LlmHttpError::Transport(message)) => {
                    return Err(GatewayError::upstream(format!(
                        "llm request failed: {message}"
                    )));
                }
            }
        }
    }

    async fn send_llm_json_without_retry<T: Serialize + ?Sized>(
        &self,
        url: &str,
        request: &T,
    ) -> Result<reqwest::Response, GatewayError> {
        match self.send_llm_json_once(url, request).await {
            Ok(response) => Ok(response),
            Err(LlmHttpError::Timeout(message)) => Err(GatewayError::upstream(format!(
                "llm request timed out after 1 attempt(s): {message}"
            ))),
            Err(LlmHttpError::Rejected { status, body }) => Err(GatewayError::upstream(format!(
                "llm rejected request: status={} body={}",
                status,
                truncate_for_error(&body)
            ))),
            Err(LlmHttpError::Transport(message)) => Err(GatewayError::upstream(format!(
                "llm request failed: {message}"
            ))),
        }
    }

    async fn send_llm_json_once<T: Serialize + ?Sized>(
        &self,
        url: &str,
        request: &T,
    ) -> Result<reqwest::Response, LlmHttpError> {
        let mut builder = self.client.post(url).json(request);
        if let Some(key) = &self.api_key {
            builder = builder
                .header("X-API-Key", key)
                .header("Authorization", format!("Bearer {key}"));
        }
        let response = builder.send().await.map_err(|err| {
            if err.is_timeout() {
                LlmHttpError::Timeout(err.to_string())
            } else {
                LlmHttpError::Transport(err.to_string())
            }
        })?;
        let status = response.status();
        if status == StatusCode::REQUEST_TIMEOUT {
            let body = response.text().await.unwrap_or_default();
            return Err(LlmHttpError::Timeout(format!(
                "status=408 body={}",
                truncate_for_error(&body)
            )));
        }
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<unreadable body>".to_string());
            return Err(LlmHttpError::Rejected { status, body });
        }
        Ok(response)
    }
}

enum LlmHttpError {
    Timeout(String),
    Rejected { status: StatusCode, body: String },
    Transport(String),
}

#[async_trait]
impl LlmPort for HttpLlmClient {
    async fn generate_reading(&self, request: &Value) -> Result<Value, GatewayError> {
        let url = format!("{}/v1/internal/readings/render", self.base_url);
        let response = self.send_llm_json_with_timeout_retry(&url, request).await?;
        let body = response
            .json::<Value>()
            .await
            .map_err(|err| GatewayError::upstream(format!("llm response parse failed: {err}")))?;
        Ok(body)
    }

    async fn build_horoscope_daily_calculation_request(
        &self,
        request: &Value,
    ) -> Result<Value, GatewayError> {
        self.post_internal_json("/v1/internal/horoscope/daily/calculation-request", request)
            .await
    }

    async fn build_horoscope_period_calculation_request(
        &self,
        request: &Value,
    ) -> Result<Value, GatewayError> {
        self.post_internal_json_without_retry(
            "/v1/internal/horoscope/period/calculation-request",
            request,
        )
        .await
    }

    async fn render_horoscope_daily(&self, request: &Value) -> Result<Value, GatewayError> {
        self.post_internal_json("/v1/internal/horoscope/daily/render", request)
            .await
    }

    async fn render_horoscope_period(&self, request: &Value) -> Result<Value, GatewayError> {
        self.post_internal_json_without_retry("/v1/internal/horoscope/period/render", request)
            .await
    }

    async fn render_horoscope_daily_gateway(&self, request: &Value) -> Result<Value, GatewayError> {
        self.post_internal_json("/v1/internal/horoscope/daily/render-gateway", request)
            .await
    }

    async fn render_horoscope_period_gateway(
        &self,
        request: &Value,
    ) -> Result<Value, GatewayError> {
        self.post_internal_json_without_retry(
            "/v1/internal/horoscope/period/render-gateway",
            request,
        )
        .await
    }

    async fn get_run_audit(&self, run_id: &str) -> Result<Value, GatewayError> {
        self.get_internal_json(&format!("/v1/runs/{run_id}")).await
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
