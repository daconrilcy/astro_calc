use async_trait::async_trait;
use serde_json::Value;

use crate::error::GatewayError;
use astral_llm_domain::{GenerateReadingRequest, GenerateReadingResponse};

#[async_trait]
pub trait CalculatorPort: Send + Sync {
    async fn calculate_simplified_natal(&self, request: &Value) -> Result<Value, GatewayError>;
    async fn calculate_full_natal(&self, request: &Value) -> Result<Value, GatewayError>;

    async fn calculate_horoscope_daily_natal(
        &self,
        _request: &Value,
    ) -> Result<Value, GatewayError> {
        Err(GatewayError::Internal(
            "horoscope daily calculation is not implemented for this calculator port".to_string(),
        ))
    }

    async fn calculate_horoscope_period_natal(
        &self,
        _request: &Value,
    ) -> Result<Value, GatewayError> {
        Err(GatewayError::Internal(
            "horoscope period calculation is not implemented for this calculator port".to_string(),
        ))
    }
}

#[async_trait]
pub trait LlmPort: Send + Sync {
    async fn generate_reading(
        &self,
        request: &GenerateReadingRequest,
    ) -> Result<GenerateReadingResponse, GatewayError>;

    async fn render_horoscope_daily(&self, _request: &Value) -> Result<Value, GatewayError> {
        Err(GatewayError::Internal(
            "horoscope daily rendering is not implemented for this LLM port".to_string(),
        ))
    }

    async fn render_horoscope_period(&self, _request: &Value) -> Result<Value, GatewayError> {
        Err(GatewayError::Internal(
            "horoscope period rendering is not implemented for this LLM port".to_string(),
        ))
    }

    async fn get_run_audit(&self, _run_id: &str) -> Result<Value, GatewayError> {
        Err(GatewayError::Internal(
            "run audit lookup is not implemented for this LLM port".to_string(),
        ))
    }
}

pub trait IntegrationCatalogPort: Send + Sync {}
