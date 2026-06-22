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
