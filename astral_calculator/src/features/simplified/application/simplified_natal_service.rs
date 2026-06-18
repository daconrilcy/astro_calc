//! Module astral_calculator\src\features\simplified\application\simplified_natal_service.rs du moteur astral_calculator.

use std::path::Path;
use std::sync::Arc;

use crate::application::ports::{ReferenceCatalog, SimplifiedCatalogStore};
use crate::astrology::ephemeris::EphemerisEngine;
use crate::features::simplified::{
    calculate_simplified_natal, AstroSimplifiedNatalRequest, AstroSimplifiedNatalResponse,
};
use crate::shared::error::RuntimeError;

/// Structure SimplifiedNatalService.
pub struct SimplifiedNatalService<R, S, E> {
    references: R,
    catalogs: S,
    ephemeris: Arc<E>,
}

impl<R, S, E> SimplifiedNatalService<R, S, E>
where
    R: ReferenceCatalog,
    S: SimplifiedCatalogStore,
    E: EphemerisEngine,
{
    /// Fonction new.
    pub fn new(references: R, catalogs: S, ephemeris: Arc<E>) -> Self {
        Self {
            references,
            catalogs,
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
            &self.references,
            &self.catalogs,
            self.ephemeris.as_ref(),
            ephemeris_path,
            request,
        )
        .await
    }
}
