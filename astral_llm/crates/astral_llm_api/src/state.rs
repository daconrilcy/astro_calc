use std::sync::Arc;

use astral_llm_application::{GenerateReadingUseCase, SchemaRegistry};
use astral_llm_infra::{AppConfig, RunPersistence};

#[derive(Clone)]
pub struct AppState {
    pub use_case: Arc<GenerateReadingUseCase>,
    pub schema_registry: Arc<SchemaRegistry>,
    pub config: AppConfig,
    pub persistence: Option<Arc<RunPersistence>>,
}
