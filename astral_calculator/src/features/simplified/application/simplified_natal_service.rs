//! Module astral_calculator\src\features\simplified\application\simplified_natal_service.rs du moteur astral_calculator.

use std::path::Path;
use std::sync::Arc;

use crate::astrology::ephemeris::EphemerisEngine;
use crate::features::simplified::{
    calculate_simplified_natal, AstroSimplifiedNatalRequest, AstroSimplifiedNatalResponse,
};
use crate::infra::db::reference_repository::ReferenceRepository;
use crate::shared::error::RuntimeError;

/// Structure SimplifiedNatalService.
pub struct SimplifiedNatalService<E> {
    repository: ReferenceRepository,
    ephemeris: Arc<E>,
}

impl<E> SimplifiedNatalService<E>
where
    E: EphemerisEngine,
{
    /// Fonction new.
    pub fn new(repository: ReferenceRepository, ephemeris: Arc<E>) -> Self {
        Self {
            repository,
            ephemeris,
        }
    }

    /// Fonction calculate.
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
