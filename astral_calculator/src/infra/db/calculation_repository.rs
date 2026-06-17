use sqlx::{postgres::PgPool, Postgres, Transaction};

use super::models::{ChartCalculationRow, InterpretationSignalRow};
use super::runtime_repository::RuntimeRepository;
use crate::domain::{
    AspectFact, BasicPayload, CalculatedChartFacts, NatalChartInput, ObjectPositionFact,
    RuntimeOptions,
};
use crate::shared::error::RuntimeError;

#[derive(Clone)]
pub struct CalculationRepository {
    inner: RuntimeRepository,
}

impl CalculationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            inner: RuntimeRepository::new(pool),
        }
    }

    pub fn pool(&self) -> &PgPool {
        self.inner.pool()
    }

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

    pub async fn positions_for_payload(
        &self,
        chart_calculation_id: i32,
    ) -> Result<Vec<ObjectPositionFact>, RuntimeError> {
        self.inner.positions_for_payload(chart_calculation_id).await
    }

    pub async fn aspects_for_payload(
        &self,
        chart_calculation_id: i32,
    ) -> Result<Vec<AspectFact>, RuntimeError> {
        self.inner.aspects_for_payload(chart_calculation_id).await
    }

    pub async fn natal_input_for_calculation(
        &self,
        chart_calculation_id: i32,
    ) -> Result<NatalChartInput, RuntimeError> {
        self.inner.natal_input_for_calculation(chart_calculation_id).await
    }

    pub async fn lock_idempotency(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        lock_key: i64,
    ) -> Result<(), RuntimeError> {
        RuntimeRepository::lock_idempotency(tx, lock_key).await
    }

    pub async fn calculations_for_key(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        idempotency_key: &str,
    ) -> Result<Vec<ChartCalculationRow>, RuntimeError> {
        RuntimeRepository::calculations_for_key(tx, idempotency_key).await
    }

    pub async fn persist_signals(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
        reference_version_id: i32,
        signals: &[crate::domain::InterpretationSignalDraft],
    ) -> Result<Vec<InterpretationSignalRow>, RuntimeError> {
        RuntimeRepository::persist_signals(tx, chart_calculation_id, reference_version_id, signals)
            .await
    }

    pub async fn persist_basic_payload(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        input: &NatalChartInput,
        payload_language_id: Option<i32>,
        payload: &BasicPayload,
    ) -> Result<(), RuntimeError> {
        RuntimeRepository::persist_basic_payload(tx, input, payload_language_id, payload).await
    }

    pub async fn mark_stale_failed(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
    ) -> Result<(), RuntimeError> {
        RuntimeRepository::mark_stale_failed(tx, chart_calculation_id).await
    }

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

    pub async fn heartbeat(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
        progress_state: &str,
    ) -> Result<(), RuntimeError> {
        RuntimeRepository::heartbeat(tx, chart_calculation_id, progress_state).await
    }

    pub async fn mark_failed(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
        error: &RuntimeError,
    ) -> Result<(), RuntimeError> {
        RuntimeRepository::mark_failed(tx, chart_calculation_id, error).await
    }

    pub async fn persist_facts(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
        facts: &CalculatedChartFacts,
    ) -> Result<(), RuntimeError> {
        RuntimeRepository::persist_facts(tx, chart_calculation_id, facts).await
    }

    pub async fn aspects_for_payload_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
    ) -> Result<Vec<AspectFact>, RuntimeError> {
        RuntimeRepository::aspects_for_payload_in_tx(tx, chart_calculation_id).await
    }

    pub async fn mark_completed(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
    ) -> Result<(), RuntimeError> {
        RuntimeRepository::mark_completed(tx, chart_calculation_id).await
    }
}
