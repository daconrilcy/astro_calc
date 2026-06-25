use chrono::Utc;

use super::persisted_position_reuse::has_reusable_persisted_positions;
use super::snapshot_loader::NatalReferenceSnapshot;
use crate::application::ports::{
    CalculationAttempt, CalculationAttemptStore, CalculationFactStore, CalculationStatus,
    CalculationTransactionManager, PayloadStore, SignalStore,
};
use crate::domain::{BasicPayload, CalculatedChartFacts, NatalChartInput};
use crate::features::natal::payload::build::build_basic_payload_with_accidental_references;
use crate::features::natal::payload::validate::{
    has_current_rulership_references, is_current_basic_payload,
};
use crate::features::natal::signals::aggregate_basic_signals;
use crate::shared::error::RuntimeError;

pub(super) struct NatalReusePolicy<'a, C> {
    calculations: &'a C,
    snapshot: &'a NatalReferenceSnapshot,
    product_code: &'a str,
    stale_after_seconds: i32,
}

#[derive(Debug)]
pub(super) enum NatalReuseResolution<Tx> {
    Return(BasicPayload),
    Proceed(Tx),
}

impl<'a, C> NatalReusePolicy<'a, C>
where
    C: CalculationTransactionManager
        + CalculationAttemptStore
        + CalculationFactStore
        + PayloadStore
        + SignalStore,
{
    pub(super) fn new(
        calculations: &'a C,
        snapshot: &'a NatalReferenceSnapshot,
        product_code: &'a str,
        stale_after_seconds: i32,
    ) -> Self {
        Self {
            calculations,
            snapshot,
            product_code,
            stale_after_seconds,
        }
    }

    pub(super) async fn resolve(
        &self,
        input: &NatalChartInput,
        payload_language_id: i32,
        idempotency_key: &str,
        existing: &[CalculationAttempt],
        tx: C::Tx,
    ) -> Result<NatalReuseResolution<C::Tx>, RuntimeError> {
        if let Some(completed) = existing
            .iter()
            .find(|row| row.status == CalculationStatus::Completed)
        {
            let completed_id = completed.id;
            if let Some(payload) = self
                .calculations
                .existing_basic_payload(completed_id, self.product_code, Some(payload_language_id))
                .await?
            {
                if is_current_basic_payload(
                    &payload,
                    &self.snapshot.catalog.projection_reason_definitions,
                    &self.snapshot.house_axes,
                ) && has_current_rulership_references(&payload, &self.snapshot.domicile_rulers)
                {
                    self.calculations.commit(tx).await?;
                    return Ok(NatalReuseResolution::Return(payload));
                }
            }

            let positions = self
                .calculations
                .positions_for_payload(completed_id)
                .await?;
            if has_reusable_persisted_positions(&positions, &self.snapshot.references) {
                let aspects = self.calculations.aspects_for_payload(completed_id).await?;
                let signal_drafts = aggregate_basic_signals(
                    &CalculatedChartFacts {
                        positions: positions.clone(),
                        house_cusps: Vec::new(),
                        aspects,
                    },
                    &self.snapshot.catalog,
                    input.language_code.as_deref().unwrap_or("en"),
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
                    input,
                    &positions,
                    &signals,
                    &self.snapshot.domicile_rulers,
                    &self.snapshot.house_axes,
                    &self.snapshot.lunar_phases,
                    &self.snapshot.accidental_conditions,
                    &self.snapshot.sect_affinities,
                    &self.snapshot.catalog,
                    input.language_code.as_deref().unwrap_or("en"),
                );
                self.calculations
                    .persist_basic_payload(
                        &mut payload_tx,
                        input,
                        Some(payload_language_id),
                        &payload,
                    )
                    .await?;
                self.calculations.commit(payload_tx).await?;
                return Ok(NatalReuseResolution::Return(payload));
            }

            return Ok(NatalReuseResolution::Proceed(tx));
        }

        if let Some(running) = existing
            .iter()
            .find(|row| row.status == CalculationStatus::Running)
        {
            if is_stale(running, self.stale_after_seconds) {
                let mut tx = tx;
                self.calculations
                    .mark_stale_failed(&mut tx, running.id)
                    .await?;
                return Ok(NatalReuseResolution::Proceed(tx));
            }

            self.calculations.commit(tx).await?;
            return Err(RuntimeError::RunningCalculationInProgress {
                idempotency_key: idempotency_key.to_string(),
                chart_calculation_id: running.id,
            });
        }

        Ok(NatalReuseResolution::Proceed(tx))
    }
}

pub(super) fn is_stale(row: &CalculationAttempt, default_stale_after_seconds: i32) -> bool {
    let Some(heartbeat_at) = row.heartbeat_at else {
        return true;
    };
    let threshold = row
        .stale_after_seconds
        .unwrap_or(default_stale_after_seconds)
        .max(1);
    Utc::now().signed_duration_since(heartbeat_at).num_seconds() > i64::from(threshold)
}
