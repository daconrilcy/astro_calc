use std::path::Path;
use std::sync::Arc;

use crate::astrology::ephemeris::EphemerisEngine;
use crate::features::simplified::{
    calculate_simplified_natal, AstroSimplifiedNatalRequest, AstroSimplifiedNatalResponse,
};
use crate::infra::db::reference_repository::ReferenceRepository;
use crate::shared::error::RuntimeError;

pub struct SimplifiedNatalService<E> {
    repository: ReferenceRepository,
    ephemeris: Arc<E>,
}

impl<E> SimplifiedNatalService<E>
where
    E: EphemerisEngine,
{
    pub fn new(repository: ReferenceRepository, ephemeris: Arc<E>) -> Self {
        Self {
            repository,
            ephemeris,
        }
    }

    pub async fn calculate(
        &self,
        request: AstroSimplifiedNatalRequest,
        ephemeris_path: &Path,
    ) -> Result<AstroSimplifiedNatalResponse, RuntimeError> {
        calculate_simplified_natal(
            &self.repository,
            self.ephemeris.as_ref(),
            ephemeris_path,
            request,
        )
        .await
    }
}
