//! Module astral_calculator\src\infra\db\projection_repository.rs du moteur astral_calculator.

use sqlx::PgPool;

use async_trait::async_trait;

use super::runtime_queries::RuntimeQueries;
use crate::application::ports::ProjectionCatalog;
use crate::engine::projection::LlmProjectionProfile;
use crate::shared::error::RuntimeError;

#[derive(Clone)]
/// Structure ProjectionRepository.
pub struct ProjectionRepository {
    inner: RuntimeQueries,
}

#[async_trait]
impl ProjectionCatalog for ProjectionRepository {
    async fn llm_projection_profile(
        &self,
        contract_version: &str,
        level: &str,
    ) -> Result<LlmProjectionProfile, RuntimeError> {
        ProjectionRepository::llm_projection_profile(self, contract_version, level).await
    }
}

impl ProjectionRepository {
    /// Fonction new.
    pub fn new(pool: PgPool) -> Self {
        Self {
            inner: RuntimeQueries::new(pool),
        }
    }

    /// Fonction llm_projection_profile.
    pub async fn llm_projection_profile(
        &self,
        contract_version: &str,
        level: &str,
    ) -> Result<LlmProjectionProfile, RuntimeError> {
        self.inner
            .llm_projection_profile(contract_version, level)
            .await
    }
}
