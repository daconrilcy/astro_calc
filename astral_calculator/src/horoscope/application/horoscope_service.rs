use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::sync::Arc;

use crate::domain::CalculationReferenceData;
use crate::horoscope::{
    calculate_horoscope_daily_natal, calculate_horoscope_period_natal_from_transits,
    normalize_horoscope_period_request_utc, HoroscopeCalculationRequest,
    HoroscopeCalculationResponse, HoroscopePeriodCalculationRequest,
    HoroscopePeriodCalculationResponse,
};
use crate::infra::db::{
    calculation_repository::CalculationRepository, horoscope_repository::HoroscopeRepository,
    reference_repository::ReferenceRepository,
};
use crate::natal::ephemeris::EphemerisEngine;
use crate::shared::error::RuntimeError;

pub struct HoroscopeService<E> {
    calculations: CalculationRepository,
    horoscope: HoroscopeRepository,
    references: ReferenceRepository,
    ephemeris: Arc<E>,
}

impl<E> HoroscopeService<E>
where
    E: EphemerisEngine,
{
    pub fn new(pool: PgPool, ephemeris: Arc<E>) -> Self {
        Self {
            calculations: CalculationRepository::new(pool.clone()),
            horoscope: HoroscopeRepository::new(pool.clone()),
            references: ReferenceRepository::new(pool),
            ephemeris,
        }
    }

    pub async fn calculate_daily(
        &self,
        request: HoroscopeCalculationRequest,
    ) -> Result<HoroscopeCalculationResponse, RuntimeError> {
        Ok(calculate_horoscope_daily_natal(request))
    }

    pub async fn calculate_period(
        &self,
        request: HoroscopePeriodCalculationRequest,
    ) -> Result<HoroscopePeriodCalculationResponse, RuntimeError> {
        let request = normalize_horoscope_period_request_utc(request).map_err(|err| {
            RuntimeError::InvalidEngineRequest(format!(
                "invalid horoscope period UTC normalization: {err}"
            ))
        })?;
        let chart_calculation_id = request.chart_calculation_id.parse::<i32>().map_err(|_| {
            RuntimeError::InvalidEngineRequest(
                "horoscope period chart_calculation_id must be an integer".to_string(),
            )
        })?;
        let natal_positions = self
            .calculations
            .positions_for_payload(chart_calculation_id)
            .await?;
        if natal_positions.is_empty() {
            return Err(RuntimeError::InvalidEngineRequest(
                "horoscope period requires persisted natal positions".to_string(),
            ));
        }
        let natal_input = self
            .calculations
            .natal_input_for_calculation(chart_calculation_id)
            .await?;
        let chart_objects = self
            .references
            .active_chart_objects(natal_input.reference_version_id)
            .await?;
        let aspect_definitions = self.references.aspect_definitions().await?;
        let house_system = self
            .references
            .house_system(natal_input.house_system_id)
            .await?;
        let references = CalculationReferenceData {
            signs: self.references.sign_references().await?,
            houses: self.references.house_references().await?,
            motion_states: self.references.motion_state_references().await?,
            horizon_positions: self.references.horizon_position_references().await?,
            angle_points: self.references.angle_point_references().await?,
        };
        let mut transit_snapshots = Vec::new();
        for snapshot in &request.scan_plan.snapshots {
            let reference_datetime_utc =
                DateTime::parse_from_rfc3339(&snapshot.reference_datetime_utc)
                    .map_err(|err| {
                        RuntimeError::InvalidEngineRequest(format!(
                            "invalid horoscope period snapshot UTC: {err}"
                        ))
                    })?
                    .with_timezone(&Utc);
            let mut transit_input = natal_input.clone();
            transit_input.birth_datetime_utc = reference_datetime_utc;
            transit_input.product_code = Some("horoscope_period_transit".to_string());
            let facts = self.ephemeris.calculate_natal(
                &transit_input,
                &chart_objects,
                &aspect_definitions,
                &house_system,
                &references,
            )?;
            transit_snapshots.push((snapshot.snapshot_key.clone(), facts.positions));
        }
        let period_max_major_aspect_orb_deg = self
            .horoscope
            .horoscope_orb_weight_bands()
            .await?
            .into_iter()
            .map(|band| band.max_orb_deg)
            .fold(0.0, f64::max);
        Ok(calculate_horoscope_period_natal_from_transits(
            request,
            &natal_positions,
            &transit_snapshots,
            period_max_major_aspect_orb_deg,
        ))
    }
}
