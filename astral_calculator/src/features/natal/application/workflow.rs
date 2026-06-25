use std::sync::Arc;

use super::snapshot_loader::NatalReferenceSnapshot;
use crate::application::ports::{
    CalculationAttempt, CalculationAttemptStore, CalculationFactStore, CalculationProgressState,
    CalculationTransactionManager, PayloadStore, SignalStore,
};
use crate::astrology::ephemeris::EphemerisEngine;
use crate::domain::{BasicPayload, CalculatedChartFacts, NatalChartInput, RuntimeOptions};
use crate::features::natal::payload::build::build_basic_payload_with_accidental_references;
use crate::features::natal::signals::aggregate_basic_signals;
use crate::shared::error::RuntimeError;

pub(super) struct NatalCalculationWorkflow<'a, C, E> {
    calculations: &'a C,
    ephemeris: &'a Arc<E>,
    options: &'a RuntimeOptions,
    snapshot: &'a NatalReferenceSnapshot,
}

impl<'a, C, E> NatalCalculationWorkflow<'a, C, E>
where
    C: CalculationTransactionManager
        + CalculationAttemptStore
        + CalculationFactStore
        + PayloadStore
        + SignalStore,
    E: EphemerisEngine,
{
    pub(super) fn new(
        calculations: &'a C,
        ephemeris: &'a Arc<E>,
        options: &'a RuntimeOptions,
        snapshot: &'a NatalReferenceSnapshot,
    ) -> Self {
        Self {
            calculations,
            ephemeris,
            options,
            snapshot,
        }
    }

    pub(super) async fn execute(
        &self,
        mut tx: C::Tx,
        input: NatalChartInput,
        payload_language_id: i32,
        input_hash: String,
        idempotency_key: String,
        existing: Vec<CalculationAttempt>,
    ) -> Result<BasicPayload, RuntimeError> {
        let next_attempt = existing
            .first()
            .map(|row| row.execution_attempt + 1)
            .unwrap_or(1);
        let chart_calculation_id = self
            .calculations
            .insert_running_calculation(
                &mut tx,
                &input,
                self.options,
                &input_hash,
                &idempotency_key,
                next_attempt,
            )
            .await?;
        self.calculations
            .heartbeat(
                &mut tx,
                chart_calculation_id,
                CalculationProgressState::CalculatingFacts,
            )
            .await?;

        let facts = match self.ephemeris.calculate_chart(
            &input,
            &self.snapshot.chart_objects,
            &self.snapshot.aspect_definitions,
            &self.snapshot.house_system,
            &self.snapshot.references,
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
            .heartbeat(
                &mut tx,
                chart_calculation_id,
                CalculationProgressState::AggregatingSignals,
            )
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
        let signal_drafts = aggregate_basic_signals(
            &enriched_facts,
            &self.snapshot.catalog,
            input.language_code.as_deref().unwrap_or("en"),
        );
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
            .heartbeat(
                &mut tx,
                chart_calculation_id,
                CalculationProgressState::BuildingPayload,
            )
            .await?;
        let payload = build_basic_payload_with_accidental_references(
            chart_calculation_id,
            &input,
            &enriched_facts.positions,
            &signal_rows,
            &self.snapshot.domicile_rulers,
            &self.snapshot.house_axes,
            &self.snapshot.lunar_phases,
            &self.snapshot.accidental_conditions,
            &self.snapshot.sect_affinities,
            &self.snapshot.catalog,
            input.language_code.as_deref().unwrap_or("en"),
        );
        self.calculations
            .persist_basic_payload(&mut tx, &input, Some(payload_language_id), &payload)
            .await?;
        self.calculations
            .mark_completed(&mut tx, chart_calculation_id)
            .await?;
        self.calculations.commit(tx).await?;

        Ok(payload)
    }
}
