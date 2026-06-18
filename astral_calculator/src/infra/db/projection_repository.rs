//! Module astral_calculator\src\infra\db\projection_repository.rs du moteur astral_calculator.

use sqlx::PgPool;

use super::runtime_repository::RuntimeRepository;
use crate::engine::projection::LlmProjectionProfile;
use crate::shared::error::RuntimeError;

#[derive(Clone)]
/// Structure ProjectionRepository.
pub struct ProjectionRepository {
    inner: RuntimeRepository,
}

impl ProjectionRepository {
    /// Fonction new.
    pub fn new(pool: PgPool) -> Self {
        Self {
            inner: RuntimeRepository::new(pool),
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
