//! Module astral_calculator\src\engine\application\runtime_facade_service.rs du moteur astral_calculator.

use crate::application::ports::{
    HoroscopeCatalog, NatalCalculationStore, ProjectionCatalog, ReferenceCatalog,
    SimplifiedCatalogStore,
};
use crate::astrology::ephemeris::EphemerisEngine;
use crate::domain::{BasicPayload, NatalChartInput};
use crate::engine::{
    build_engine_response, validate_and_resolve_request, validate_request_early,
    AstroEngineRequest, AstroEngineResponse, LLM_PROJECTION_CONTRACT_VERSION,
};
use crate::features::horoscope::{
    HoroscopeCalculationRequest, HoroscopeCalculationResponse, HoroscopePeriodCalculationRequest,
    HoroscopePeriodCalculationResponse,
};
use crate::features::natal::application::NatalCalculationService;
use crate::features::simplified::application::SimplifiedNatalService;
use crate::shared::error::RuntimeError;

use crate::features::horoscope::application::HoroscopeService;
use crate::features::simplified::{AstroSimplifiedNatalRequest, AstroSimplifiedNatalResponse};

/// Structure EngineFacadeService.
pub struct EngineFacadeService<C, P, R, H, S, L, E> {
    natal: NatalCalculationService<C, P, R, E>,
    simplified: SimplifiedNatalService<R, S, E>,
    horoscope: HoroscopeService<C, H, R, E>,
    projections: L,
    references: R,
}

impl<C, P, R, H, S, L, E> EngineFacadeService<C, P, R, H, S, L, E>
where
    C: NatalCalculationStore + Clone,
    P: crate::application::ports::PayloadCatalogStore,
    R: ReferenceCatalog + Clone,
    H: HoroscopeCatalog,
    S: SimplifiedCatalogStore,
    L: ProjectionCatalog,
    E: EphemerisEngine,
{
    /// Fonction new.
    pub fn new(
        natal: NatalCalculationService<C, P, R, E>,
        simplified: SimplifiedNatalService<R, S, E>,
        horoscope: HoroscopeService<C, H, R, E>,
        projections: L,
        references: R,
    ) -> Self {
        Self {
            natal,
            simplified,
            horoscope,
            projections,
            references,
        }
    }

    /// Fonction calculate_natal_engine.
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
        let (audit, payload_catalog) = self
            .natal
            .calculate_basic_with_catalog(resolved.natal_input.clone())
            .await?;

        build_engine_response(
            &resolved,
            audit,
            self.natal.options(),
            &zodiac_label,
            &coordinate_label,
            &house_system_label,
            &house_axes,
            &payload_catalog.projection_reason_definitions,
            &profile,
        )
    }

    /// Fonction calculate_simplified_natal_engine.
    pub async fn calculate_simplified_natal_engine(
        &self,
        request: AstroSimplifiedNatalRequest,
        ephemeris_path: &std::path::Path,
    ) -> Result<AstroSimplifiedNatalResponse, RuntimeError> {
        self.simplified.calculate(request, ephemeris_path).await
    }

    /// Fonction calculate_horoscope_daily_natal.
    pub async fn calculate_horoscope_daily_natal(
        &self,
        request: HoroscopeCalculationRequest,
    ) -> Result<HoroscopeCalculationResponse, RuntimeError> {
        self.calculate_horoscope_daily(request).await
    }

    /// Fonction calculate_horoscope_period_natal.
    pub async fn calculate_horoscope_period_natal(
        &self,
        request: HoroscopePeriodCalculationRequest,
    ) -> Result<HoroscopePeriodCalculationResponse, RuntimeError> {
        self.calculate_horoscope_period(request).await
    }

    /// Fonction calculate_horoscope_daily.
    pub async fn calculate_horoscope_daily(
        &self,
        request: HoroscopeCalculationRequest,
    ) -> Result<HoroscopeCalculationResponse, RuntimeError> {
        self.horoscope.calculate_daily(request).await
    }

    /// Fonction calculate_horoscope_period.
    pub async fn calculate_horoscope_period(
        &self,
        request: HoroscopePeriodCalculationRequest,
    ) -> Result<HoroscopePeriodCalculationResponse, RuntimeError> {
        self.horoscope.calculate_period(request).await
    }

    /// Fonction calculate_natal_basic.
    pub async fn calculate_natal_basic(
        &self,
        input: NatalChartInput,
    ) -> Result<BasicPayload, RuntimeError> {
        self.natal.calculate_basic(input).await
    }
}
