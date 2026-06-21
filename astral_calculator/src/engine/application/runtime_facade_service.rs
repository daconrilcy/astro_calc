//! Module astral_calculator\src\engine\application\runtime_facade_service.rs du moteur astral_calculator.

use crate::application::ports::{NatalReferenceStore, ProjectionCatalog, ReferenceSystemResolver};
use crate::domain::{
    AccidentalDignityConditionReference, AnglePointReference, BasicPayload, HouseAxisReference,
    HouseReference, MotionStateReference, NatalChartInput,
};
use crate::engine::{
    build_engine_response, validate_and_resolve_request, validate_request_early,
    AstroEngineRequest, AstroEngineResponse, LLM_PROJECTION_CONTRACT_VERSION,
};
use crate::features::horoscope::application::HoroscopeCapability;
use crate::features::horoscope::{
    HoroscopeCalculationRequest, HoroscopeCalculationResponse, HoroscopePeriodCalculationRequest,
    HoroscopePeriodCalculationResponse,
};
use crate::features::natal::application::NatalCalculationCapability;
use crate::features::simplified::application::SimplifiedNatalCapability;
use crate::shared::error::RuntimeError;

use crate::features::simplified::{AstroSimplifiedNatalRequest, AstroSimplifiedNatalResponse};

/// Structure EngineFacadeService.
pub struct EngineFacadeService<N, S, H, L, R> {
    natal: N,
    simplified: S,
    horoscope: H,
    projections: L,
    references: R,
}

impl<N, S, H, L, R> EngineFacadeService<N, S, H, L, R>
where
    N: NatalCalculationCapability,
    S: SimplifiedNatalCapability,
    H: HoroscopeCapability,
    L: ProjectionCatalog,
    R: ReferenceSystemResolver + NatalReferenceStore + Clone,
{
    /// Fonction new.
    pub fn new(natal: N, simplified: S, horoscope: H, projections: L, references: R) -> Self {
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
        let labels = self.load_engine_reference_labels(
            zodiacal_id,
            coordinate_id,
            house_system_id,
        )
        .await?;
        let (audit, payload_catalog) = self
            .natal
            .calculate_basic_with_catalog(resolved.natal_input.clone())
            .await?;

        build_engine_response(
            &resolved,
            audit,
            self.natal.options(),
            &labels.zodiac_label,
            &labels.coordinate_label,
            &labels.house_system_label,
            &labels.house_references,
            &labels.house_axes,
            &labels.angle_points,
            &labels.motion_states,
            &labels.accidental_condition_definitions,
            &payload_catalog.essential_dignity_rules,
            &payload_catalog.projection_reason_definitions,
            &payload_catalog.projection_label_definitions,
            &profile,
        )
    }

    /// Fonction calculate_simplified_natal_engine.
    pub async fn calculate_simplified_natal_engine(
        &self,
        request: AstroSimplifiedNatalRequest,
        ephemeris_path: &std::path::Path,
    ) -> Result<AstroSimplifiedNatalResponse, RuntimeError> {
        self.simplified
            .calculate_simplified(request, ephemeris_path)
            .await
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

    async fn load_engine_reference_labels(
        &self,
        zodiacal_id: i32,
        coordinate_id: i32,
        house_system_id: i32,
    ) -> Result<EngineReferenceLabels, RuntimeError> {
        Ok(EngineReferenceLabels {
            zodiac_label: self
                .references
                .zodiacal_reference_system_display_name(zodiacal_id)
                .await?,
            coordinate_label: self
                .references
                .coordinate_reference_system_display_name(coordinate_id)
                .await?,
            house_system_label: self.references.house_system(house_system_id).await?.name,
            house_references: self.references.house_references().await?,
            house_axes: self.references.house_axis_references().await?,
            angle_points: self.references.angle_point_references().await?,
            motion_states: self.references.motion_state_references().await?,
            accidental_condition_definitions: self
                .references
                .accidental_dignity_condition_references()
                .await?,
        })
    }
}

struct EngineReferenceLabels {
    zodiac_label: String,
    coordinate_label: String,
    house_system_label: String,
    house_references: Vec<HouseReference>,
    house_axes: Vec<HouseAxisReference>,
    angle_points: Vec<AnglePointReference>,
    motion_states: Vec<MotionStateReference>,
    accidental_condition_definitions: Vec<AccidentalDignityConditionReference>,
}
