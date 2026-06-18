//! Module astral_calculator\src\infra\db\calculation_repository.rs du moteur astral_calculator.

use sqlx::{postgres::PgPool, Postgres, Transaction};

use super::models::ChartCalculationRow;
use super::runtime_repository::RuntimeRepository;
use crate::domain::{
    AspectFact, BasicPayload, CalculatedChartFacts, InterpretationSignalRow, NatalChartInput,
    ObjectPositionFact, RuntimeOptions,
};
use crate::shared::error::RuntimeError;

#[derive(Clone)]
/// Structure CalculationRepository.
pub struct CalculationRepository {
    inner: RuntimeRepository,
}

impl CalculationRepository {
    /// Fonction new.
    pub fn new(pool: PgPool) -> Self {
        Self {
            inner: RuntimeRepository::new(pool),
        }
    }

    /// Fonction pool.
    pub fn pool(&self) -> &PgPool {
        self.inner.pool()
    }

    /// Fonction existing_basic_payload.
    pub async fn existing_basic_payload(
        &self,
        chart_calculation_id: i32,
        product_code: &str,
        language_id: Option<i32>,
    ) -> Result<Option<BasicPayload>, RuntimeError> {
        self.inner
            .existing_basic_payload(chart_calculation_id, product_code, language_id)
            .await
    }

    /// Fonction positions_for_payload.
    pub async fn positions_for_payload(
        &self,
        chart_calculation_id: i32,
    ) -> Result<Vec<ObjectPositionFact>, RuntimeError> {
        self.inner.positions_for_payload(chart_calculation_id).await
    }

    /// Fonction aspects_for_payload.
    pub async fn aspects_for_payload(
        &self,
        chart_calculation_id: i32,
    ) -> Result<Vec<AspectFact>, RuntimeError> {
        self.inner.aspects_for_payload(chart_calculation_id).await
    }

    /// Fonction natal_input_for_calculation.
    pub async fn natal_input_for_calculation(
        &self,
        chart_calculation_id: i32,
    ) -> Result<NatalChartInput, RuntimeError> {
        self.inner
            .natal_input_for_calculation(chart_calculation_id)
            .await
    }

    /// Fonction lock_idempotency.
    pub async fn lock_idempotency(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        lock_key: i64,
    ) -> Result<(), RuntimeError> {
        RuntimeRepository::lock_idempotency(tx, lock_key).await
    }

    /// Fonction calculations_for_key.
    pub async fn calculations_for_key(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        idempotency_key: &str,
    ) -> Result<Vec<ChartCalculationRow>, RuntimeError> {
        RuntimeRepository::calculations_for_key(tx, idempotency_key).await
    }

    /// Fonction persist_signals.
    pub async fn persist_signals(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
        reference_version_id: i32,
        signals: &[crate::domain::InterpretationSignalDraft],
    ) -> Result<Vec<InterpretationSignalRow>, RuntimeError> {
        Ok(RuntimeRepository::persist_signals(
            tx,
            chart_calculation_id,
            reference_version_id,
            signals,
        )
        .await?
        .into_iter()
        .map(|row| InterpretationSignalRow {
            id: row.id,
            signal_key: row.signal_key,
            theme_code: row.theme_code,
            title: row.title,
            summary: row.summary,
            priority_score: row.priority_score,
            confidence_score: row.confidence_score,
            payload_json: row.payload_json,
        })
        .collect())
    }

    /// Fonction persist_basic_payload.
    pub async fn persist_basic_payload(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        input: &NatalChartInput,
        payload_language_id: Option<i32>,
        payload: &BasicPayload,
    ) -> Result<(), RuntimeError> {
        RuntimeRepository::persist_basic_payload(tx, input, payload_language_id, payload).await
    }

    /// Fonction mark_stale_failed.
    pub async fn mark_stale_failed(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
    ) -> Result<(), RuntimeError> {
        RuntimeRepository::mark_stale_failed(tx, chart_calculation_id).await
    }

    /// Fonction insert_running_calculation.
    pub async fn insert_running_calculation(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        input: &NatalChartInput,
        options: &RuntimeOptions,
        input_hash: &str,
        idempotency_key: &str,
        next_attempt: i32,
    ) -> Result<i32, RuntimeError> {
        RuntimeRepository::insert_running_calculation(
            tx,
            input,
            options,
            input_hash,
            idempotency_key,
            next_attempt,
        )
        .await
    }

    /// Fonction heartbeat.
    pub async fn heartbeat(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
        progress_state: &str,
    ) -> Result<(), RuntimeError> {
        RuntimeRepository::heartbeat(tx, chart_calculation_id, progress_state).await
    }

    /// Fonction mark_failed.
    pub async fn mark_failed(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
        error: &RuntimeError,
    ) -> Result<(), RuntimeError> {
        RuntimeRepository::mark_failed(tx, chart_calculation_id, error).await
    }

    /// Fonction persist_facts.
    pub async fn persist_facts(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
        facts: &CalculatedChartFacts,
    ) -> Result<(), RuntimeError> {
        RuntimeRepository::persist_facts(tx, chart_calculation_id, facts).await
    }

    /// Fonction aspects_for_payload_in_tx.
    pub async fn aspects_for_payload_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
    ) -> Result<Vec<AspectFact>, RuntimeError> {
        RuntimeRepository::aspects_for_payload_in_tx(tx, chart_calculation_id).await
    }

    /// Fonction mark_completed.
    pub async fn mark_completed(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
    ) -> Result<(), RuntimeError> {
        RuntimeRepository::mark_completed(tx, chart_calculation_id).await
    }
}
