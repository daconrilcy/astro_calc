//! Module astral_calculator\src\features\natal\application\natal_calculation_service.rs du moteur astral_calculator.

use std::collections::HashSet;
use std::sync::Arc;

use crate::application::ports::{
    CalculationAttempt, NatalCalculationStore, PayloadCatalogStore, ReferenceCatalog,
};
use crate::astrology::ephemeris::EphemerisEngine;
use crate::domain::{
    BasicPayload, CalculatedChartFacts, CalculationReferenceData, NatalChartInput, RuntimeOptions,
};
use crate::features::natal::catalog::BasicPayloadCatalog;
use crate::features::natal::payload::build::build_basic_payload_with_accidental_references;
use crate::features::natal::payload::validate::{
    has_current_rulership_references, is_current_basic_payload,
};
use crate::features::natal::signals::aggregate_basic_signals;
use crate::features::natal::validate::{
    validate_accidental_condition_triggers, validate_accidental_dignity_condition_references,
    validate_accidental_polarity_bands, validate_accidental_scoring_params,
    validate_aspect_definitions, validate_calculation_references,
    validate_chart_object_signal_profiles, validate_house_axis_references,
    validate_lunar_phase_references, validate_object_sect_affinity_references,
};
use crate::shared::error::RuntimeError;
use crate::shared::idempotency::{advisory_lock_key, idempotency_key, input_hash};
use chrono::Utc;

/// Structure NatalCalculationService.
pub struct NatalCalculationService<C, P, R, E> {
    calculations: C,
    catalogs: P,
    references: R,
    ephemeris: Arc<E>,
    options: RuntimeOptions,
}

impl<C, P, R, E> NatalCalculationService<C, P, R, E>
where
    C: NatalCalculationStore,
    P: PayloadCatalogStore,
    R: ReferenceCatalog,
    E: EphemerisEngine,
{
    /// Fonction new.
    pub fn new(
        calculations: C,
        catalogs: P,
        references: R,
        ephemeris: Arc<E>,
        options: RuntimeOptions,
    ) -> Self {
        Self {
            calculations,
            catalogs,
            references,
            ephemeris,
            options,
        }
    }

    /// Fonction options.
    pub fn options(&self) -> &RuntimeOptions {
        &self.options
    }

    /// Fonction calculate_basic.
    pub async fn calculate_basic(
        &self,
        input: NatalChartInput,
    ) -> Result<BasicPayload, RuntimeError> {
        let (payload, _) = self.calculate_basic_with_catalog(input).await?;
        Ok(payload)
    }

    /// Fonction calculate_basic_with_catalog.
    pub async fn calculate_basic_with_catalog(
        &self,
        input: NatalChartInput,
    ) -> Result<(BasicPayload, BasicPayloadCatalog), RuntimeError> {
        let product_code = input.product_code().to_string();
        let payload_language_id = self.references.language_id_for_code("en").await?;
        let input_hash = input_hash(&input)?;
        let idempotency_key = idempotency_key(&input, &self.options)?;
        let lock_key = advisory_lock_key(&idempotency_key);

        let chart_objects = self
            .references
            .active_chart_objects(input.reference_version_id)
            .await?;
        validate_chart_object_signal_profiles(&chart_objects)?;
        let aspect_definitions = self.references.aspect_definitions().await?;
        let major_aspect_family = self.references.major_aspect_family_reference().await?;
        let catalog = self
            .catalogs
            .basic_payload_catalog(
                &product_code,
                "natal_structured_v14",
                input.reference_version_id,
            )
            .await?;
        validate_aspect_definitions(
            &aspect_definitions,
            catalog.product_scoring.default_major_orb_deg,
            major_aspect_family.expected_aspect_count as usize,
            major_aspect_family.max_default_orb_deg,
        )?;
        let house_system = self.references.house_system(input.house_system_id).await?;
        let references = CalculationReferenceData {
            signs: self.references.sign_references().await?,
            houses: self.references.house_references().await?,
            motion_states: self.references.motion_state_references().await?,
            horizon_positions: self.references.horizon_position_references().await?,
            angle_points: self.references.angle_point_references().await?,
        };
        validate_calculation_references(&references)?;
        let domicile_rulers = self
            .references
            .domicile_ruler_references(input.reference_version_id)
            .await?;
        let house_axes = self.references.house_axis_references().await?;
        validate_house_axis_references(&house_axes, &references.houses)?;
        let lunar_phases = self.references.lunar_phase_references().await?;
        validate_lunar_phase_references(&lunar_phases)?;
        let accidental_conditions = self
            .references
            .accidental_dignity_condition_references()
            .await?;
        validate_accidental_dignity_condition_references(
            &accidental_conditions,
            &catalog.accidental_triggers,
        )?;
        validate_accidental_condition_triggers(&catalog.accidental_triggers)?;
        validate_accidental_scoring_params(&catalog.accidental_scoring)?;
        validate_accidental_polarity_bands(&catalog.accidental_polarity_bands)?;
        let sect_affinities = self.references.object_sect_affinity_references().await?;
        validate_object_sect_affinity_references(&sect_affinities)?;

        let mut tx = self.calculations.begin().await?;
        self.calculations
            .lock_idempotency(&mut tx, lock_key)
            .await?;

        let existing = self
            .calculations
            .calculations_for_key(&mut tx, &idempotency_key)
            .await?;
        if let Some(completed) = existing.iter().find(|row| row.status == "completed") {
            let completed_id = completed.id;
            if let Some(payload) = self
                .calculations
                .existing_basic_payload(completed_id, &product_code, Some(payload_language_id))
                .await?
            {
                if is_current_basic_payload(&payload, &catalog.projection_reason_definitions)
                    && has_current_rulership_references(&payload, &domicile_rulers)
                {
                    self.calculations.commit(tx).await?;
                    return Ok((payload, catalog));
                }
            }
            let positions = self
                .calculations
                .positions_for_payload(completed_id)
                .await?;
            if has_reusable_persisted_positions(&positions, &references) {
                let aspects = self.calculations.aspects_for_payload(completed_id).await?;
                let signal_drafts = aggregate_basic_signals(
                    &CalculatedChartFacts {
                        positions: positions.clone(),
                        house_cusps: Vec::new(),
                        aspects,
                    },
                    &catalog,
                );
                self.calculations.commit(tx).await?;
                let mut payload_tx = self.calculations.begin().await?;
                let signals = self
                    .calculations
                    .persist_signals(
                        &mut payload_tx,
                        completed_id,
                        input.reference_version_id,
                        &signal_drafts,
                    )
                    .await?;
                let payload = build_basic_payload_with_accidental_references(
                    completed_id,
                    &input,
                    &positions,
                    &signals,
                    &domicile_rulers,
                    &house_axes,
                    &lunar_phases,
                    &accidental_conditions,
                    &sect_affinities,
                    &catalog,
                );
                self.calculations
                    .persist_basic_payload(
                        &mut payload_tx,
                        &input,
                        Some(payload_language_id),
                        &payload,
                    )
                    .await?;
                self.calculations.commit(payload_tx).await?;
                return Ok((payload, catalog));
            }
        } else if let Some(running) = existing.iter().find(|row| row.status == "running") {
            if is_stale(running, self.options.stale_after_seconds) {
                self.calculations
                    .mark_stale_failed(&mut tx, running.id)
                    .await?;
            } else {
                let chart_calculation_id = running.id;
                self.calculations.commit(tx).await?;
                return Err(RuntimeError::RunningCalculationInProgress {
                    idempotency_key,
                    chart_calculation_id,
                });
            }
        }

        let next_attempt = existing
            .first()
            .map(|row| row.execution_attempt + 1)
            .unwrap_or(1);
        let chart_calculation_id = self
            .calculations
            .insert_running_calculation(
                &mut tx,
                &input,
                &self.options,
                &input_hash,
                &idempotency_key,
                next_attempt,
            )
            .await?;
        self.calculations
            .heartbeat(&mut tx, chart_calculation_id, "calculating_facts")
            .await?;

        let facts = match self.ephemeris.calculate_chart(
            &input,
            &chart_objects,
            &aspect_definitions,
            &house_system,
            &references,
        ) {
            Ok(value) => value,
            Err(error) => {
                self.calculations
                    .mark_failed(&mut tx, chart_calculation_id, &error)
                    .await?;
                self.calculations.commit(tx).await?;
                return Err(error);
            }
        };

        self.calculations
            .persist_facts(&mut tx, chart_calculation_id, &facts)
            .await?;
        self.calculations
            .heartbeat(&mut tx, chart_calculation_id, "aggregating_signals")
            .await?;
        let aspects = self
            .calculations
            .aspects_for_payload_in_tx(&mut tx, chart_calculation_id)
            .await?;
        let enriched_facts = CalculatedChartFacts {
            positions: facts.positions,
            house_cusps: Vec::new(),
            aspects,
        };
        let signal_drafts = aggregate_basic_signals(&enriched_facts, &catalog);
        let signal_rows = self
            .calculations
            .persist_signals(
                &mut tx,
                chart_calculation_id,
                input.reference_version_id,
                &signal_drafts,
            )
            .await?;

        self.calculations
            .heartbeat(&mut tx, chart_calculation_id, "building_payload")
            .await?;
        let payload = build_basic_payload_with_accidental_references(
            chart_calculation_id,
            &input,
            &enriched_facts.positions,
            &signal_rows,
            &domicile_rulers,
            &house_axes,
            &lunar_phases,
            &accidental_conditions,
            &sect_affinities,
            &catalog,
        );
        self.calculations
            .persist_basic_payload(&mut tx, &input, Some(payload_language_id), &payload)
            .await?;
        self.calculations
            .mark_completed(&mut tx, chart_calculation_id)
            .await?;
        self.calculations.commit(tx).await?;

        Ok((payload, catalog))
    }
}

/// Fonction has_reusable_persisted_positions.
fn has_reusable_persisted_positions(
    positions: &[crate::domain::ObjectPositionFact],
    references: &CalculationReferenceData,
) -> bool {
    let position_object_ids: HashSet<i32> = positions
        .iter()
        .map(|position| position.chart_object_id)
        .collect();

    references
        .angle_points
        .iter()
        .all(|angle| position_object_ids.contains(&angle.chart_object_id))
        && positions.iter().all(|position| {
            has_reusable_horizon_context(position)
                && (is_angle_position(position)
                    || position
                        .altitude_deg
                        .is_some_and(|altitude| altitude.is_finite()))
        })
}

/// Fonction has_reusable_horizon_context.
fn has_reusable_horizon_context(position: &crate::domain::ObjectPositionFact) -> bool {
    position
        .horizon_position_id
        .is_some_and(|horizon_position_id| horizon_position_id > 0)
        && position.is_visible.is_some()
}

/// Fonction is_angle_position.
fn is_angle_position(position: &crate::domain::ObjectPositionFact) -> bool {
    position
        .facts_json
        .as_ref()
        .and_then(|facts| facts.get("object_context"))
        .and_then(|context| context.get("role"))
        .and_then(|value| value.as_str())
        == Some("angle")
        || position
            .facts_json
            .as_ref()
            .and_then(|facts| facts.get("angle_context"))
            .is_some()
}

/// Fonction is_stale.
fn is_stale(row: &CalculationAttempt, default_stale_after_seconds: i32) -> bool {
    let Some(heartbeat_at) = row.heartbeat_at else {
        return true;
    };
    let threshold = row
        .stale_after_seconds
        .unwrap_or(default_stale_after_seconds)
        .max(1);
    Utc::now().signed_duration_since(heartbeat_at).num_seconds() > i64::from(threshold)
}
