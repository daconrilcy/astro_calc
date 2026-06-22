use astral_llm_application::core::calculator::CalculatorPort;
use astral_llm_domain::GenerationError;
use astral_llm_infra::CalculatorClient;
use serde_json::Value;

pub struct WorkerCalculatorPort {
    client: CalculatorClient,
}

impl WorkerCalculatorPort {
    pub fn new(client: CalculatorClient) -> Self {
        Self { client }
    }
}

impl CalculatorPort for WorkerCalculatorPort {
    async fn calculate_simplified_natal(&self, request: &Value) -> Result<Value, GenerationError> {
        self.client.calculate_simplified_natal(request).await
    }

    async fn calculate_natal(&self, request: &Value) -> Result<Value, GenerationError> {
        self.client.calculate_natal(request).await
    }

    async fn calculate_horoscope_daily_natal(
        &self,
        request: &Value,
    ) -> Result<Value, GenerationError> {
        self.client.calculate_horoscope_daily_natal(request).await
    }

    async fn calculate_horoscope_period_natal(
        &self,
        request: &Value,
    ) -> Result<Value, GenerationError> {
        self.client.calculate_horoscope_period_natal(request).await
    }
}
