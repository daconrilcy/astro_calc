use std::collections::HashSet;

use chrono::Utc;
use sqlx::PgPool;

use crate::domain::{
    BasicPayload, CalculatedChartFacts, CalculationReferenceData, NatalChartInput, RuntimeOptions,
};
use crate::ephemeris::EphemerisEngine;
use crate::idempotency::{advisory_lock_key, idempotency_key, input_hash};
use crate::models::ChartCalculationRow;
use crate::payload::build_basic_payload_with_all_references;
use crate::repositories::RuntimeRepository;
use crate::signals::aggregate_basic_signals;

use super::error::RuntimeError;
use super::payload_freshness::{has_current_rulership_references, is_current_basic_payload};
use super::references::{
    validate_calculation_references, validate_chart_object_signal_profiles,
    validate_house_axis_references, validate_lunar_phase_references,
};

pub struct ChartCalculationRuntimeService<E> {
    repository: RuntimeRepository,
    ephemeris: E,
    options: RuntimeOptions,
}

impl<E> ChartCalculationRuntimeService<E>
where
    E: EphemerisEngine,
{
    pub fn new(pool: PgPool, ephemeris: E, options: RuntimeOptions) -> Self {
        Self {
            repository: RuntimeRepository::new(pool),
            ephemeris,
            options,
        }
    }

    pub async fn calculate_natal_basic(
        &self,
        input: NatalChartInput,
    ) -> Result<BasicPayload, RuntimeError> {
        let product_code = input.product_code().to_string();
        let payload_language_id = self.repository.language_id_for_code("en").await?;
        let input_hash = input_hash(&input)?;
        let idempotency_key = idempotency_key(&input, &self.options)?;
        let lock_key = advisory_lock_key(&idempotency_key);

        let chart_objects = self
            .repository
            .active_chart_objects(input.reference_version_id)
            .await?;
        validate_chart_object_signal_profiles(&chart_objects)?;
        let aspect_definitions = self.repository.aspect_definitions().await?;
        let house_system = self.repository.house_system(input.house_system_id).await?;
        let references = CalculationReferenceData {
            signs: self.repository.sign_references().await?,
            houses: self.repository.house_references().await?,
            motion_states: self.repository.motion_state_references().await?,
            horizon_positions: self.repository.horizon_position_references().await?,
            angle_points: self.repository.angle_point_references().await?,
        };
        validate_calculation_references(&references)?;
        let domicile_rulers = self
            .repository
            .domicile_ruler_references(input.reference_version_id)
            .await?;
        let house_axes = self.repository.house_axis_references().await?;
        validate_house_axis_references(&house_axes)?;
        let lunar_phases = self.repository.lunar_phase_references().await?;
        validate_lunar_phase_references(&lunar_phases)?;

        let mut tx = self.repository.pool().begin().await?;
        RuntimeRepository::lock_idempotency(&mut tx, lock_key).await?;

        let existing = RuntimeRepository::calculations_for_key(&mut tx, &idempotency_key).await?;
        if let Some(completed) = existing.iter().find(|row| row.status == "completed") {
            let completed_id = completed.id;
            if let Some(payload) = self
                .repository
                .existing_basic_payload(completed_id, &product_code, Some(payload_language_id))
                .await?
            {
                if is_current_basic_payload(&payload)
                    && has_current_rulership_references(&payload, &domicile_rulers)
                {
                    tx.commit().await?;
                    return Ok(payload);
                }
            }
            let positions = self.repository.positions_for_payload(completed_id).await?;
            if has_reusable_persisted_positions(&positions, &references) {
                let aspects = self.repository.aspects_for_payload(completed_id).await?;
                let signal_drafts = aggregate_basic_signals(&CalculatedChartFacts {
                    positions: positions.clone(),
                    house_cusps: Vec::new(),
                    aspects,
                });
                tx.commit().await?;
                let mut payload_tx = self.repository.pool().begin().await?;
                let signals = RuntimeRepository::persist_signals(
                    &mut payload_tx,
                    completed_id,
                    input.reference_version_id,
                    &signal_drafts,
                )
                .await?;
                let payload = build_basic_payload_with_all_references(
                    completed_id,
                    &input,
                    &positions,
                    &signals,
                    &domicile_rulers,
                    &house_axes,
                    &lunar_phases,
                );
                RuntimeRepository::persist_basic_payload(
                    &mut payload_tx,
                    &input,
                    Some(payload_language_id),
                    &payload,
                )
                .await?;
                payload_tx.commit().await?;
                return Ok(payload);
            }
        } else if let Some(running) = existing.iter().find(|row| row.status == "running") {
            if is_stale(running, self.options.stale_after_seconds) {
                RuntimeRepository::mark_stale_failed(&mut tx, running.id).await?;
            } else {
                let chart_calculation_id = running.id;
                tx.commit().await?;
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
        let chart_calculation_id = RuntimeRepository::insert_running_calculation(
            &mut tx,
            &input,
            &self.options,
            &input_hash,
            &idempotency_key,
            next_attempt,
        )
        .await?;
        RuntimeRepository::heartbeat(&mut tx, chart_calculation_id, "calculating_facts").await?;

        let facts = match self.ephemeris.calculate_natal(
            &input,
            &chart_objects,
            &aspect_definitions,
            &house_system,
            &references,
        ) {
            Ok(value) => value,
            Err(error) => {
                RuntimeRepository::mark_failed(&mut tx, chart_calculation_id, &error).await?;
                tx.commit().await?;
                return Err(error);
            }
        };

        RuntimeRepository::persist_facts(&mut tx, chart_calculation_id, &facts).await?;
        RuntimeRepository::heartbeat(&mut tx, chart_calculation_id, "aggregating_signals").await?;
        let aspects =
            RuntimeRepository::aspects_for_payload_in_tx(&mut tx, chart_calculation_id).await?;
        let enriched_facts = CalculatedChartFacts {
            positions: facts.positions,
            house_cusps: Vec::new(),
            aspects,
        };
        let signal_drafts = aggregate_basic_signals(&enriched_facts);
        let signal_rows = RuntimeRepository::persist_signals(
            &mut tx,
            chart_calculation_id,
            input.reference_version_id,
            &signal_drafts,
        )
        .await?;

        RuntimeRepository::heartbeat(&mut tx, chart_calculation_id, "building_payload").await?;
        let payload = build_basic_payload_with_all_references(
            chart_calculation_id,
            &input,
            &enriched_facts.positions,
            &signal_rows,
            &domicile_rulers,
            &house_axes,
            &lunar_phases,
        );
        RuntimeRepository::persist_basic_payload(
            &mut tx,
            &input,
            Some(payload_language_id),
            &payload,
        )
        .await?;
        RuntimeRepository::mark_completed(&mut tx, chart_calculation_id).await?;
        tx.commit().await?;

        Ok(payload)
    }
}

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

fn has_reusable_horizon_context(position: &crate::domain::ObjectPositionFact) -> bool {
    position
        .horizon_position_id
        .is_some_and(|horizon_position_id| horizon_position_id > 0)
        && position.is_visible.is_some()
}

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

fn is_stale(row: &ChartCalculationRow, default_stale_after_seconds: i32) -> bool {
    let Some(heartbeat_at) = row.heartbeat_at else {
        return true;
    };
    let threshold = row
        .stale_after_seconds
        .unwrap_or(default_stale_after_seconds)
        .max(1);
    Utc::now().signed_duration_since(heartbeat_at).num_seconds() > i64::from(threshold)
}
