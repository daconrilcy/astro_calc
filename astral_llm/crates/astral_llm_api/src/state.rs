use std::sync::Arc;

use astral_llm_application::{GenerateReadingUseCase, SchemaRegistry};
use astral_llm_infra::{AppConfig, RunPersistence};
use tokio::sync::Semaphore;

use crate::rate_limit::ApiKeyRateLimiter;

#[derive(Clone)]
pub struct AppState {
    pub use_case: Arc<GenerateReadingUseCase>,
    pub schema_registry: Arc<SchemaRegistry>,
    pub config: AppConfig,
    pub persistence: Option<Arc<RunPersistence>>,
    pub concurrency_limit: Option<Arc<Semaphore>>,
    pub api_key_limiter: Option<Arc<ApiKeyRateLimiter>>,
}
