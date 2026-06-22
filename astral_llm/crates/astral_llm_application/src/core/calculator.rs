use astral_llm_domain::GenerationError;
use serde_json::Value;

#[allow(async_fn_in_trait)]
pub trait CalculatorPort {
    async fn calculate_simplified_natal(&self, request: &Value) -> Result<Value, GenerationError>;

    async fn calculate_natal(&self, request: &Value) -> Result<Value, GenerationError>;

    async fn calculate_horoscope_daily_natal(
        &self,
        request: &Value,
    ) -> Result<Value, GenerationError>;

    async fn calculate_horoscope_period_natal(
        &self,
        request: &Value,
    ) -> Result<Value, GenerationError>;
}

impl CalculatorPort for astral_llm_infra::CalculatorClient {
    async fn calculate_simplified_natal(&self, request: &Value) -> Result<Value, GenerationError> {
        astral_llm_infra::CalculatorClient::calculate_simplified_natal(self, request).await
    }

    async fn calculate_natal(&self, request: &Value) -> Result<Value, GenerationError> {
        astral_llm_infra::CalculatorClient::calculate_natal(self, request).await
    }

    async fn calculate_horoscope_daily_natal(
        &self,
        request: &Value,
    ) -> Result<Value, GenerationError> {
        astral_llm_infra::CalculatorClient::calculate_horoscope_daily_natal(self, request).await
    }

    async fn calculate_horoscope_period_natal(
        &self,
        request: &Value,
    ) -> Result<Value, GenerationError> {
        astral_llm_infra::CalculatorClient::calculate_horoscope_period_natal(self, request).await
    }
}
