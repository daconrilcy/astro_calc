//! Module astral_calculator\src\features\horoscope\application\horoscope_service.rs du moteur astral_calculator.

use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::sync::Arc;

use crate::application::ports::{HoroscopeCatalog, NatalCalculationStore, ReferenceCatalog};
use crate::astrology::ephemeris::EphemerisEngine;
use crate::domain::CalculationReferenceData;
use crate::features::horoscope::{
    calculate_horoscope_daily_from_transits, calculate_horoscope_period_from_transits_with_aspects,
    normalize_horoscope_period_request_utc, HoroscopeCalculationRequest,
    HoroscopeCalculationResponse, HoroscopePeriodCalculationRequest,
    HoroscopePeriodCalculationResponse, HoroscopeSupportedObject,
};
use crate::shared::error::RuntimeError;

/// Structure HoroscopeService.
pub struct HoroscopeService<C, H, R, E> {
    calculations: C,
    horoscope: H,
    references: R,
    ephemeris: Arc<E>,
}

impl<C, H, R, E> HoroscopeService<C, H, R, E>
where
    C: NatalCalculationStore,
    H: HoroscopeCatalog,
    R: ReferenceCatalog,
    E: EphemerisEngine,
{
    /// Fonction new.
    pub fn new(calculations: C, horoscope: H, references: R, ephemeris: Arc<E>) -> Self {
        Self {
            calculations,
            horoscope,
            references,
            ephemeris,
        }
    }

    /// Fonction calculate_daily.
    pub async fn calculate_daily(
        &self,
        request: HoroscopeCalculationRequest,
    ) -> Result<HoroscopeCalculationResponse, RuntimeError> {
        let chart_calculation_id = request.chart_calculation_id.parse::<i32>().map_err(|_| {
            RuntimeError::InvalidEngineRequest(
                "horoscope daily chart_calculation_id must be an integer".to_string(),
            )
        })?;
        let natal_positions = self
            .calculations
            .positions_for_payload(chart_calculation_id)
            .await?;
        if natal_positions.is_empty() {
            return Err(RuntimeError::InvalidEngineRequest(
                "horoscope daily requires persisted natal positions".to_string(),
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
        let supported_objects = self.horoscope.horoscope_supported_objects().await?;
        if supported_objects.is_empty() {
            return Err(RuntimeError::InvalidRuntimeTable(
                "missing active horoscope_supported_objects".to_string(),
            ));
        }
        let theme_mappings = self.horoscope.horoscope_signal_theme_mappings().await?;
        if theme_mappings.is_empty() {
            return Err(RuntimeError::InvalidRuntimeTable(
                "missing horoscope_signal_theme_mappings".to_string(),
            ));
        }
        let mut transit_slots = Vec::new();
        for slot in &request.slots {
            let reference_datetime_utc = crate::shared::time::reference_datetime_utc(
                &request.period.date,
                &request.period.timezone,
                &slot.reference_local_time,
            )
            .ok_or_else(|| {
                RuntimeError::InvalidEngineRequest(format!(
                    "invalid horoscope daily slot reference time {}",
                    slot.reference_local_time
                ))
            })?;
            let reference_datetime_utc = DateTime::parse_from_rfc3339(&reference_datetime_utc)
                .map_err(|err| {
                    RuntimeError::InvalidEngineRequest(format!(
                        "invalid horoscope daily slot UTC: {err}"
                    ))
                })?
                .with_timezone(&Utc);
            let mut transit_input = natal_input.clone();
            transit_input.birth_datetime_utc = reference_datetime_utc;
            transit_input.product_code = Some("horoscope_daily_transit".to_string());
            let facts = self.ephemeris.calculate_chart(
                &transit_input,
                &chart_objects,
                &aspect_definitions,
                &house_system,
                &references,
            )?;
            transit_slots.push((
                slot.slot_code.clone(),
                filter_supported_transit_positions(facts.positions, &supported_objects),
            ));
        }
        let max_major_aspect_orb_deg = self
            .horoscope
            .horoscope_orb_weight_bands()
            .await?
            .into_iter()
            .map(|band| band.max_orb_deg)
            .fold(0.0, f64::max);
        Ok(calculate_horoscope_daily_from_transits(
            request,
            &natal_positions,
            &transit_slots,
            max_major_aspect_orb_deg,
            &aspect_definitions,
            &theme_mappings,
        ))
    }

    /// Fonction calculate_period.
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
        let supported_objects = self.horoscope.horoscope_supported_objects().await?;
        if supported_objects.is_empty() {
            return Err(RuntimeError::InvalidRuntimeTable(
                "missing active horoscope_supported_objects".to_string(),
            ));
        }
        let theme_mappings = self.horoscope.horoscope_signal_theme_mappings().await?;
        if theme_mappings.is_empty() {
            return Err(RuntimeError::InvalidRuntimeTable(
                "missing horoscope_signal_theme_mappings".to_string(),
            ));
        }
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
            let facts = self.ephemeris.calculate_chart(
                &transit_input,
                &chart_objects,
                &aspect_definitions,
                &house_system,
                &references,
            )?;
            transit_snapshots.push((
                snapshot.snapshot_key.clone(),
                filter_supported_transit_positions(facts.positions, &supported_objects),
            ));
        }
        let period_max_major_aspect_orb_deg = self
            .horoscope
            .horoscope_orb_weight_bands()
            .await?
            .into_iter()
            .map(|band| band.max_orb_deg)
            .fold(0.0, f64::max);
        Ok(calculate_horoscope_period_from_transits_with_aspects(
            request,
            &natal_positions,
            &transit_snapshots,
            period_max_major_aspect_orb_deg,
            &aspect_definitions,
            &theme_mappings,
        ))
    }
}

fn filter_supported_transit_positions(
    positions: Vec<crate::domain::ObjectPositionFact>,
    supported_objects: &[HoroscopeSupportedObject],
) -> Vec<crate::domain::ObjectPositionFact> {
    let supported_codes = supported_objects
        .iter()
        .map(|object| object.object_code.as_str())
        .collect::<HashSet<_>>();
    let mut filtered = positions
        .into_iter()
        .filter(|position| supported_codes.contains(position.object_code.as_str()))
        .collect::<Vec<_>>();
    filtered.sort_by(|left, right| {
        supported_weight(right.object_code.as_str(), supported_objects).total_cmp(
            &supported_weight(left.object_code.as_str(), supported_objects),
        )
    });
    filtered
}

fn supported_weight(code: &str, supported_objects: &[HoroscopeSupportedObject]) -> f64 {
    supported_objects
        .iter()
        .find(|object| object.object_code == code)
        .map(|object| object.weight)
        .unwrap_or(0.0)
}
