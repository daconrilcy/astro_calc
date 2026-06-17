pub use crate::engine::application::EngineFacadeService as ChartCalculationRuntimeService;
pub use crate::engine::{AstroEngineRequest, AstroEngineResponse};
pub use crate::infra::db::runtime_repository::parse_existing_basic_payload_value;
pub use crate::natal::payload::validate::{
    has_current_rulership_references, is_current_basic_payload,
};
pub use crate::natal::validate::{
    validate_accidental_condition_triggers, validate_accidental_dignity_condition_references,
    validate_accidental_polarity_bands, validate_accidental_scoring_params,
    validate_aspect_definitions, validate_calculation_references,
    validate_chart_object_signal_profiles, validate_house_axis_references,
    validate_lunar_phase_references, validate_object_sect_affinity_references,
};
pub use crate::shared::error::RuntimeError;

use std::sync::Arc;

use sqlx::PgPool;

use crate::astrology::ephemeris::EphemerisEngine;
use crate::domain::RuntimeOptions;
use crate::horoscope::application::HoroscopeService;
use crate::infra::db::{
    calculation_repository::CalculationRepository, catalog_repository::CatalogRepository,
    horoscope_repository::HoroscopeRepository, projection_repository::ProjectionRepository,
    reference_repository::ReferenceRepository,
};
use crate::natal::application::NatalCalculationService;
use crate::simplified::application::SimplifiedNatalService;

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
    let simplified =
        SimplifiedNatalService::new(ReferenceRepository::new(pool.clone()), ephemeris.clone());
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
