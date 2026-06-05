use reqwest::Client;
use serde_json::Value;

use astral_llm_domain::{GenerationError, GenerationErrorCode};

#[derive(Clone)]
pub struct CalculatorClient {
    base_url: String,
    api_key: Option<String>,
    client: Client,
}

impl CalculatorClient {
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

    pub async fn calculate_simplified_natal(
        &self,
        request: &Value,
    ) -> Result<Value, GenerationError> {
        let url = format!("{}/v1/calculations/natal/simplified", self.base_url);
        let mut builder = self.client.post(url).json(request);
        if let Some(key) = &self.api_key {
            builder = builder.header("X-API-Key", key).header("Authorization", format!("Bearer {key}"));
        }

        let response = builder.send().await.map_err(|err| {
            GenerationError::with_details(
                GenerationErrorCode::ProviderUnavailable,
                format!("calculator request failed: {err}"),
                Value::Null,
            )
        })?;

        let status = response.status();
        let body: Value = response.json().await.map_err(|err| {
            GenerationError::with_details(
                GenerationErrorCode::ProviderUnavailable,
                format!("calculator response parse failed: {err}"),
                Value::Null,
            )
        })?;

        if !status.is_success() {
            return Err(GenerationError::with_details(
                GenerationErrorCode::InvalidInput,
                "calculator rejected simplified natal request".to_string(),
                body,
            ));
        }

        Ok(body)
    }
}

pub fn calculator_base_url_from_env() -> String {
    let host = std::env::var("ASTRAL_CALCULATOR_HOST")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "127.0.0.1".into());
    let port = std::env::var("ASTRAL_CALCULATOR_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);
    format!("http://{host}:{port}")
}

pub fn calculator_api_key_from_env() -> Option<String> {
    std::env::var("ASTRAL_CALCULATOR_API_KEY")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}
