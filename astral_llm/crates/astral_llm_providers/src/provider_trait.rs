use std::sync::Arc;

use async_trait::async_trait;

use astral_llm_domain::{ProviderCapabilities, ProviderKind};

use crate::types::{ProviderGenerationRequest, ProviderGenerationResponse};
use crate::LlmProviderError;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn kind(&self) -> ProviderKind;

    fn capabilities(&self) -> ProviderCapabilities;

    async fn generate(
        &self,
        request: ProviderGenerationRequest,
    ) -> Result<ProviderGenerationResponse, LlmProviderError>;
}

pub type SharedLlmProvider = Arc<dyn LlmProvider>;
