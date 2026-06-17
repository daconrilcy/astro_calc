use std::path::Path;
use std::sync::Arc;

use sqlx::PgPool;

use crate::infra::db::reference_repository::ReferenceRepository;
use crate::natal::ephemeris::EphemerisEngine;
use crate::shared::error::RuntimeError;
use crate::simplified::{
    calculate_simplified_natal, AstroSimplifiedNatalRequest, AstroSimplifiedNatalResponse,
};

pub struct SimplifiedNatalService<E> {
    repository: ReferenceRepository,
    ephemeris: Arc<E>,
}

impl<E> SimplifiedNatalService<E>
where
    E: EphemerisEngine,
{
    pub fn new(pool: PgPool, ephemeris: Arc<E>) -> Self {
        Self {
            repository: ReferenceRepository::new(pool),
            ephemeris,
        }
    }

    pub async fn calculate(
        &self,
        request: AstroSimplifiedNatalRequest,
        ephemeris_path: &Path,
    ) -> Result<AstroSimplifiedNatalResponse, RuntimeError> {
        calculate_simplified_natal(&self.repository, self.ephemeris.as_ref(), ephemeris_path, request).await
    }
}
