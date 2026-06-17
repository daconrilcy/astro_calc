use sqlx::PgPool;
use std::sync::Arc;

use crate::domain::{BasicPayload, NatalChartInput, RuntimeOptions};
use crate::engine::{
    build_engine_response, validate_and_resolve_request, validate_request_early,
    AstroEngineRequest, AstroEngineResponse, LLM_PROJECTION_CONTRACT_VERSION,
};
use crate::horoscope::{
    HoroscopeCalculationRequest, HoroscopeCalculationResponse, HoroscopePeriodCalculationRequest,
    HoroscopePeriodCalculationResponse,
};
use crate::infra::db::{
    calculation_repository::CalculationRepository, projection_repository::ProjectionRepository,
    reference_repository::ReferenceRepository,
};
use crate::natal::application::NatalCalculationService;
use crate::natal::ephemeris::EphemerisEngine;
use crate::shared::error::RuntimeError;
use crate::simplified::application::SimplifiedNatalService;

use crate::horoscope::application::HoroscopeService;
use crate::simplified::{AstroSimplifiedNatalRequest, AstroSimplifiedNatalResponse};

pub struct EngineFacadeService<E> {
    natal: NatalCalculationService<E>,
    simplified: SimplifiedNatalService<E>,
    horoscope: HoroscopeService<E>,
    projections: ProjectionRepository,
    references: ReferenceRepository,
}

impl<E> EngineFacadeService<E>
where
    E: EphemerisEngine,
{
    pub fn new(pool: PgPool, ephemeris: E, options: RuntimeOptions) -> Self {
        let ephemeris = Arc::new(ephemeris);
        Self {
            natal: NatalCalculationService::new(
                CalculationRepository::new(pool.clone()),
                crate::infra::db::catalog_repository::CatalogRepository::new(pool.clone()),
                ReferenceRepository::new(pool.clone()),
                ephemeris.clone(),
                options,
            ),
            simplified: SimplifiedNatalService::new(pool.clone(), ephemeris.clone()),
            horoscope: HoroscopeService::new(pool.clone(), ephemeris),
            projections: ProjectionRepository::new(pool.clone()),
            references: ReferenceRepository::new(pool),
        }
    }

    pub async fn calculate_natal_engine(
        &self,
        request: AstroEngineRequest,
    ) -> Result<AstroEngineResponse, RuntimeError> {
        validate_request_early(&request)?;

        let reference_version_id = self.references.default_reference_version_id().await?;
        let zodiacal_id = self
            .references
            .zodiacal_reference_system_id_by_key(&request.calculation.zodiacal_reference_system)
            .await?;
        let coordinate_id = self
            .references
            .coordinate_reference_system_id_by_key(&request.calculation.coordinate_reference_system)
            .await?;
        let house_system_id = self
            .references
            .house_system_id_by_code(&request.calculation.house_system)
            .await?;

        let resolved = validate_and_resolve_request(
            &request,
            reference_version_id,
            zodiacal_id,
            coordinate_id,
            house_system_id,
        )?;

        let profile = self
            .projections
            .llm_projection_profile(LLM_PROJECTION_CONTRACT_VERSION, &resolved.projection_level)
            .await?;
        let zodiac_label = self
            .references
            .zodiacal_reference_system_display_name(zodiacal_id)
            .await?;
        let coordinate_label = self
            .references
            .coordinate_reference_system_display_name(coordinate_id)
            .await?;
        let house_system = self.references.house_system(house_system_id).await?;
        let house_system_label = house_system.name;
        let house_axes = self.references.house_axis_references().await?;
        let audit = self
            .natal
            .calculate_basic(resolved.natal_input.clone())
            .await?;

        build_engine_response(
            &resolved,
            audit,
            self.natal.options(),
            &zodiac_label,
            &coordinate_label,
            &house_system_label,
            &house_axes,
            &profile,
        )
    }

    pub async fn calculate_simplified_natal_engine(
        &self,
        request: AstroSimplifiedNatalRequest,
        ephemeris_path: &std::path::Path,
    ) -> Result<AstroSimplifiedNatalResponse, RuntimeError> {
        self.simplified.calculate(request, ephemeris_path).await
    }

    pub async fn calculate_horoscope_daily_natal(
        &self,
        request: HoroscopeCalculationRequest,
    ) -> Result<HoroscopeCalculationResponse, RuntimeError> {
        self.horoscope.calculate_daily(request).await
    }

    pub async fn calculate_horoscope_period_natal(
        &self,
        request: HoroscopePeriodCalculationRequest,
    ) -> Result<HoroscopePeriodCalculationResponse, RuntimeError> {
        self.horoscope.calculate_period(request).await
    }

    pub async fn calculate_natal_basic(
        &self,
        input: NatalChartInput,
    ) -> Result<BasicPayload, RuntimeError> {
        self.natal.calculate_basic(input).await
    }
}
