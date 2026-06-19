//! Module astral_calculator\src\runtime\mod.rs du moteur astral_calculator.

pub use crate::engine::application::EngineFacadeService;
pub use crate::engine::{AstroEngineRequest, AstroEngineResponse};
pub use crate::shared::error::RuntimeError;

use std::sync::Arc;

use sqlx::PgPool;

use crate::astrology::ephemeris::EphemerisEngine;
use crate::domain::RuntimeOptions;
use crate::features::horoscope::application::HoroscopeService;
use crate::features::natal::application::NatalCalculationService;
use crate::features::simplified::application::SimplifiedNatalService;
use crate::infra::db::{
    calculation_repository::CalculationRepository, catalog_repository::CatalogRepository,
    horoscope_repository::HoroscopeRepository, projection_repository::ProjectionRepository,
    reference_repository::ReferenceRepository,
    simplified_catalog_repository::SimplifiedCatalogRepository,
};

pub mod compat;

/// Service runtime concret câblé sur PostgreSQL.
pub type ChartCalculationRuntimeService<E> = EngineFacadeService<
    CalculationRepository,
    CatalogRepository,
    ReferenceRepository,
    HoroscopeRepository,
    SimplifiedCatalogRepository,
    ProjectionRepository,
    E,
>;

/// Fonction build_runtime_service.
pub fn build_runtime_service<E>(
    pool: PgPool,
    ephemeris: E,
    options: RuntimeOptions,
) -> ChartCalculationRuntimeService<E>
where
    E: EphemerisEngine,
{
    let ephemeris = Arc::new(ephemeris);
    let natal = NatalCalculationService::new(
        CalculationRepository::new(pool.clone()),
        CatalogRepository::new(pool.clone()),
        ReferenceRepository::new(pool.clone()),
        ephemeris.clone(),
        options,
    );
    let simplified = SimplifiedNatalService::new(
        ReferenceRepository::new(pool.clone()),
        SimplifiedCatalogRepository::new(pool.clone()),
        ephemeris.clone(),
    );
    let horoscope = HoroscopeService::new(
        CalculationRepository::new(pool.clone()),
        HoroscopeRepository::new(pool.clone()),
        ReferenceRepository::new(pool.clone()),
        ephemeris,
    );

    ChartCalculationRuntimeService::new(
        natal,
        simplified,
        horoscope,
        ProjectionRepository::new(pool.clone()),
        ReferenceRepository::new(pool),
    )
}
