use sqlx::PgPool;

use super::runtime_repository::RuntimeRepository;
use crate::engine::projection::LlmProjectionProfile;
use crate::shared::error::RuntimeError;

#[derive(Clone)]
pub struct ProjectionRepository {
    inner: RuntimeRepository,
}

impl ProjectionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            inner: RuntimeRepository::new(pool),
        }
    }

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
