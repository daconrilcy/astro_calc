//! Module astral_calculator\src\features\horoscope\application\horoscope_service.rs du moteur astral_calculator.

use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;

use crate::application::chart_context::load_chart_context;
use crate::application::chart_context::ChartContextData;
use crate::application::ports::{
    HoroscopeCatalog, NatalCalculationStore, NatalReferenceStore, ReferenceSystemResolver,
};
use crate::application::transient_chart::calculate_transient_chart_facts;
use crate::astrology::ephemeris::EphemerisEngine;
use crate::features::horoscope::application::HoroscopeCapability;
use crate::features::horoscope::{
    calculate_horoscope_daily_from_transits, normalize_horoscope_period_request_utc,
    try_calculate_horoscope_period_from_transits_with_aspects, HoroscopeCalculationRequest,
    HoroscopeCalculationResponse, HoroscopePeriodCalculationRequest,
    HoroscopePeriodCalculationResponse, HoroscopeSignalThemeMapping, HoroscopeSupportedObject,
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
    R: ReferenceSystemResolver + NatalReferenceStore,
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
        let runtime = self
            .load_horoscope_runtime_context(&request.chart_calculation_id)
            .await?;
        let transit_slots = self.build_daily_transit_slots(&request, &runtime).await?;
        Ok(calculate_horoscope_daily_from_transits(
            request,
            &runtime.natal_positions,
            &transit_slots,
            runtime.max_major_aspect_orb_deg,
            &runtime.aspect_definitions,
            &runtime.theme_mappings,
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
        let runtime = self
            .load_horoscope_runtime_context(&request.chart_calculation_id)
            .await?;
        let transit_snapshots = self
            .build_period_transit_snapshots(&request, &runtime)
            .await?;
        try_calculate_horoscope_period_from_transits_with_aspects(
            request,
            &runtime.natal_positions,
            &transit_snapshots,
            runtime.max_major_aspect_orb_deg,
            &runtime.aspect_definitions,
            &runtime.theme_mappings,
        )
    }
}

#[async_trait]
impl<C, H, R, E> HoroscopeCapability for HoroscopeService<C, H, R, E>
where
    C: NatalCalculationStore + Send + Sync,
    H: HoroscopeCatalog + Send + Sync,
    R: ReferenceSystemResolver + NatalReferenceStore + Send + Sync,
    E: EphemerisEngine + Send + Sync,
{
    async fn calculate_daily(
        &self,
        request: HoroscopeCalculationRequest,
    ) -> Result<HoroscopeCalculationResponse, RuntimeError> {
        HoroscopeService::calculate_daily(self, request).await
    }

    async fn calculate_period(
        &self,
        request: HoroscopePeriodCalculationRequest,
    ) -> Result<HoroscopePeriodCalculationResponse, RuntimeError> {
        HoroscopeService::calculate_period(self, request).await
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

struct HoroscopeRuntimeContext {
    natal_positions: Vec<crate::domain::ObjectPositionFact>,
    natal_input: crate::domain::NatalChartInput,
    chart_context: ChartContextData,
    supported_objects: Vec<HoroscopeSupportedObject>,
    theme_mappings: Vec<HoroscopeSignalThemeMapping>,
    aspect_definitions: Vec<crate::domain::AspectDefinition>,
    max_major_aspect_orb_deg: f64,
}

impl<C, H, R, E> HoroscopeService<C, H, R, E>
where
    C: NatalCalculationStore,
    H: HoroscopeCatalog,
    R: ReferenceSystemResolver + NatalReferenceStore,
    E: EphemerisEngine,
{
    async fn load_horoscope_runtime_context(
        &self,
        chart_calculation_id: &str,
    ) -> Result<HoroscopeRuntimeContext, RuntimeError> {
        let chart_calculation_id = chart_calculation_id.parse::<i32>().map_err(|_| {
            RuntimeError::InvalidEngineRequest(
                "horoscope chart_calculation_id must be an integer".to_string(),
            )
        })?;
        let natal_positions = self
            .calculations
            .positions_for_payload(chart_calculation_id)
            .await?;
        if natal_positions.is_empty() {
            return Err(RuntimeError::InvalidEngineRequest(
                "horoscope requires persisted natal positions".to_string(),
            ));
        }
        let natal_input = self
            .calculations
            .natal_input_for_calculation(chart_calculation_id)
            .await?;
        let chart_context = load_chart_context(
            &self.references,
            natal_input.reference_version_id,
            natal_input.house_system_id,
        )
        .await?;
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
        let aspect_definitions = chart_context.aspect_definitions.clone();
        let max_major_aspect_orb_deg = self
            .horoscope
            .horoscope_orb_weight_bands()
            .await?
            .into_iter()
            .map(|band| band.max_orb_deg)
            .fold(0.0, f64::max);
        Ok(HoroscopeRuntimeContext {
            natal_positions,
            natal_input,
            chart_context,
            supported_objects,
            theme_mappings,
            aspect_definitions,
            max_major_aspect_orb_deg,
        })
    }

    async fn build_daily_transit_slots(
        &self,
        request: &HoroscopeCalculationRequest,
        runtime: &HoroscopeRuntimeContext,
    ) -> Result<Vec<(String, Vec<crate::domain::ObjectPositionFact>)>, RuntimeError> {
        self.build_transit_runtime(
            request
                .slots
                .iter()
                .map(|slot| (slot.slot_code.as_str(), slot.reference_local_time.as_str())),
            &request.period.date,
            &request.period.timezone,
            "horoscope_daily_transit",
            &runtime.natal_input,
            &runtime.chart_context,
            &runtime.supported_objects,
        )
        .await
    }

    async fn build_period_transit_snapshots(
        &self,
        request: &HoroscopePeriodCalculationRequest,
        runtime: &HoroscopeRuntimeContext,
    ) -> Result<Vec<(String, Vec<crate::domain::ObjectPositionFact>)>, RuntimeError> {
        self.build_transit_runtime(
            request.scan_plan.snapshots.iter().map(|snapshot| {
                (
                    snapshot.snapshot_key.as_str(),
                    snapshot.reference_datetime_utc.as_str(),
                )
            }),
            "",
            "",
            "horoscope_period_transit",
            &runtime.natal_input,
            &runtime.chart_context,
            &runtime.supported_objects,
        )
        .await
    }

    async fn build_transit_runtime<'a, I>(
        &self,
        items: I,
        date: &str,
        timezone: &str,
        transit_source: &'static str,
        natal_input: &crate::domain::NatalChartInput,
        chart_context: &ChartContextData,
        supported_objects: &[HoroscopeSupportedObject],
    ) -> Result<Vec<(String, Vec<crate::domain::ObjectPositionFact>)>, RuntimeError>
    where
        I: IntoIterator<Item = (&'a str, &'a str)>,
    {
        let mut transit_items = Vec::new();
        for (key, reference_local_time) in items {
            let reference_datetime_utc = if transit_source == "horoscope_daily_transit" {
                let reference_datetime_utc = crate::shared::time::reference_datetime_utc(
                    date,
                    timezone,
                    reference_local_time,
                )
                .ok_or_else(|| {
                    RuntimeError::InvalidEngineRequest(format!(
                        "invalid horoscope daily slot reference time {reference_local_time}"
                    ))
                })?;
                DateTime::parse_from_rfc3339(&reference_datetime_utc)
                    .map_err(|err| {
                        RuntimeError::InvalidEngineRequest(format!(
                            "invalid horoscope daily slot UTC: {err}"
                        ))
                    })?
                    .with_timezone(&Utc)
            } else {
                DateTime::parse_from_rfc3339(reference_local_time)
                    .map_err(|err| {
                        RuntimeError::InvalidEngineRequest(format!(
                            "invalid horoscope period snapshot UTC: {err}"
                        ))
                    })?
                    .with_timezone(&Utc)
            };
            let facts = calculate_transient_chart_facts(
                &*self.ephemeris,
                natal_input,
                reference_datetime_utc,
                transit_source,
                chart_context,
            )?;
            transit_items.push((
                key.to_string(),
                filter_supported_transit_positions(facts.positions, supported_objects),
            ));
        }
        Ok(transit_items)
    }
}
