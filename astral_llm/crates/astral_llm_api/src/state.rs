use std::sync::Arc;

use astral_llm_application::{GenerateReadingUseCase, IntegrationJobValidator, SchemaRegistry};
use astral_llm_infra::{AppConfig, CalculatorClient, JobPersistence, RunPersistence};
use tokio::sync::Semaphore;

use crate::rate_limit::ApiKeyRateLimiter;

#[derive(Clone)]
pub struct AppState {
    pub use_case: Arc<GenerateReadingUseCase>,
    pub schema_registry: Arc<SchemaRegistry>,
    pub config: AppConfig,
    pub persistence: Option<Arc<RunPersistence>>,
    pub job_persistence: Option<Arc<JobPersistence>>,
    pub integration_job_validator: Option<Arc<IntegrationJobValidator>>,
    pub concurrency_limit: Option<Arc<Semaphore>>,
    pub api_key_limiter: Option<Arc<ApiKeyRateLimiter>>,
    pub interpretation_profile_count: usize,
    pub calculator_client: Option<CalculatorClient>,
}
