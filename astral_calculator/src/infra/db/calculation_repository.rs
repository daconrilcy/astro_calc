//! Module astral_calculator\src\infra\db\calculation_repository.rs du moteur astral_calculator.

use sqlx::{postgres::PgPool, Postgres, Transaction};

use async_trait::async_trait;

use super::models::ChartCalculationRow;
use super::runtime_queries::RuntimeQueries;
use crate::application::ports::{
    CalculationAttempt, CalculationAttemptStore, CalculationFactStore, CalculationProgressState,
    CalculationStatus, CalculationTransactionManager, PayloadStore, SignalStore,
};
use crate::domain::{
    AspectFact, BasicPayload, CalculatedChartFacts, InterpretationSignalRow, NatalChartInput,
    ObjectPositionFact, RuntimeOptions,
};
use crate::shared::error::RuntimeError;

#[derive(Clone)]
/// Structure CalculationRepository.
pub struct CalculationRepository {
    inner: RuntimeQueries,
}

#[async_trait]
impl CalculationTransactionManager for CalculationRepository {
    type Tx = Transaction<'static, Postgres>;

    async fn begin(&self) -> Result<Self::Tx, RuntimeError> {
        Ok(self.pool().begin().await?)
    }

    async fn commit(&self, tx: Self::Tx) -> Result<(), RuntimeError> {
        Ok(tx.commit().await?)
    }

    async fn lock_idempotency(&self, tx: &mut Self::Tx, lock_key: i64) -> Result<(), RuntimeError> {
        CalculationRepository::lock_idempotency(self, tx, lock_key).await
    }
}

#[async_trait]
impl PayloadStore for CalculationRepository {
    async fn existing_basic_payload(
        &self,
        chart_calculation_id: i32,
        product_code: &str,
        language_id: Option<i32>,
    ) -> Result<Option<BasicPayload>, RuntimeError> {
        CalculationRepository::existing_basic_payload(
            self,
            chart_calculation_id,
            product_code,
            language_id,
        )
        .await
    }

    async fn persist_basic_payload(
        &self,
        tx: &mut Self::Tx,
        input: &NatalChartInput,
        payload_language_id: Option<i32>,
        payload: &BasicPayload,
    ) -> Result<(), RuntimeError> {
        CalculationRepository::persist_basic_payload(self, tx, input, payload_language_id, payload)
            .await
    }
}

#[async_trait]
impl CalculationFactStore for CalculationRepository {
    async fn positions_for_payload(
        &self,
        chart_calculation_id: i32,
    ) -> Result<Vec<ObjectPositionFact>, RuntimeError> {
        CalculationRepository::positions_for_payload(self, chart_calculation_id).await
    }

    async fn aspects_for_payload(
        &self,
        chart_calculation_id: i32,
    ) -> Result<Vec<AspectFact>, RuntimeError> {
        CalculationRepository::aspects_for_payload(self, chart_calculation_id).await
    }

    async fn natal_input_for_calculation(
        &self,
        chart_calculation_id: i32,
    ) -> Result<NatalChartInput, RuntimeError> {
        CalculationRepository::natal_input_for_calculation(self, chart_calculation_id).await
    }

    async fn persist_facts(
        &self,
        tx: &mut Self::Tx,
        chart_calculation_id: i32,
        facts: &CalculatedChartFacts,
    ) -> Result<(), RuntimeError> {
        CalculationRepository::persist_facts(self, tx, chart_calculation_id, facts).await
    }

    async fn aspects_for_payload_in_tx(
        &self,
        tx: &mut Self::Tx,
        chart_calculation_id: i32,
    ) -> Result<Vec<AspectFact>, RuntimeError> {
        CalculationRepository::aspects_for_payload_in_tx(self, tx, chart_calculation_id).await
    }
}

#[async_trait]
impl SignalStore for CalculationRepository {
    async fn persist_signals(
        &self,
        tx: &mut Self::Tx,
        chart_calculation_id: i32,
        reference_version_id: i32,
        signals: &[crate::domain::InterpretationSignalDraft],
    ) -> Result<Vec<InterpretationSignalRow>, RuntimeError> {
        CalculationRepository::persist_signals(
            self,
            tx,
            chart_calculation_id,
            reference_version_id,
            signals,
        )
        .await
    }
}

#[async_trait]
impl CalculationAttemptStore for CalculationRepository {
    async fn calculations_for_key(
        &self,
        tx: &mut Self::Tx,
        idempotency_key: &str,
    ) -> Result<Vec<CalculationAttempt>, RuntimeError> {
        Ok(
            CalculationRepository::calculations_for_key(self, tx, idempotency_key)
                .await?
                .into_iter()
                .map(Into::into)
                .collect(),
        )
    }

    async fn mark_stale_failed(
        &self,
        tx: &mut Self::Tx,
        chart_calculation_id: i32,
    ) -> Result<(), RuntimeError> {
        CalculationRepository::mark_stale_failed(self, tx, chart_calculation_id).await
    }

    async fn insert_running_calculation(
        &self,
        tx: &mut Self::Tx,
        input: &NatalChartInput,
        options: &RuntimeOptions,
        input_hash: &str,
        idempotency_key: &str,
        next_attempt: i32,
    ) -> Result<i32, RuntimeError> {
        CalculationRepository::insert_running_calculation(
            self,
            tx,
            input,
            options,
            input_hash,
            idempotency_key,
            next_attempt,
        )
        .await
    }

    async fn heartbeat(
        &self,
        tx: &mut Self::Tx,
        chart_calculation_id: i32,
        progress_state: CalculationProgressState,
    ) -> Result<(), RuntimeError> {
        CalculationRepository::heartbeat(self, tx, chart_calculation_id, progress_state).await
    }

    async fn mark_failed(
        &self,
        tx: &mut Self::Tx,
        chart_calculation_id: i32,
        error: &RuntimeError,
    ) -> Result<(), RuntimeError> {
        CalculationRepository::mark_failed(self, tx, chart_calculation_id, error).await
    }

    async fn mark_completed(
        &self,
        tx: &mut Self::Tx,
        chart_calculation_id: i32,
    ) -> Result<(), RuntimeError> {
        CalculationRepository::mark_completed(self, tx, chart_calculation_id).await
    }
}

impl From<ChartCalculationRow> for CalculationAttempt {
    fn from(row: ChartCalculationRow) -> Self {
        Self {
            id: row.id,
            status: CalculationStatus::from_db_str(&row.status),
            execution_attempt: row.execution_attempt,
            heartbeat_at: row.heartbeat_at,
            stale_after_seconds: row.stale_after_seconds,
        }
    }
}

impl CalculationRepository {
    /// Fonction new.
    pub fn new(pool: PgPool) -> Self {
        Self {
            inner: RuntimeQueries::new(pool),
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
        RuntimeQueries::lock_idempotency(tx, lock_key).await
    }

    /// Fonction calculations_for_key.
    pub async fn calculations_for_key(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        idempotency_key: &str,
    ) -> Result<Vec<ChartCalculationRow>, RuntimeError> {
        RuntimeQueries::calculations_for_key(tx, idempotency_key).await
    }

    /// Fonction persist_signals.
    pub async fn persist_signals(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
        reference_version_id: i32,
        signals: &[crate::domain::InterpretationSignalDraft],
    ) -> Result<Vec<InterpretationSignalRow>, RuntimeError> {
        Ok(
            RuntimeQueries::persist_signals(
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
            .collect(),
        )
    }

    /// Fonction persist_basic_payload.
    pub async fn persist_basic_payload(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        input: &NatalChartInput,
        payload_language_id: Option<i32>,
        payload: &BasicPayload,
    ) -> Result<(), RuntimeError> {
        RuntimeQueries::persist_basic_payload(tx, input, payload_language_id, payload).await
    }

    /// Fonction mark_stale_failed.
    pub async fn mark_stale_failed(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
    ) -> Result<(), RuntimeError> {
        RuntimeQueries::mark_stale_failed(tx, chart_calculation_id).await
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
        RuntimeQueries::insert_running_calculation(
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
        progress_state: CalculationProgressState,
    ) -> Result<(), RuntimeError> {
        RuntimeQueries::heartbeat(tx, chart_calculation_id, progress_state).await
    }

    /// Fonction mark_failed.
    pub async fn mark_failed(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
        error: &RuntimeError,
    ) -> Result<(), RuntimeError> {
        RuntimeQueries::mark_failed(tx, chart_calculation_id, error).await
    }

    /// Fonction persist_facts.
    pub async fn persist_facts(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
        facts: &CalculatedChartFacts,
    ) -> Result<(), RuntimeError> {
        RuntimeQueries::persist_facts(tx, chart_calculation_id, facts).await
    }

    /// Fonction aspects_for_payload_in_tx.
    pub async fn aspects_for_payload_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
    ) -> Result<Vec<AspectFact>, RuntimeError> {
        RuntimeQueries::aspects_for_payload_in_tx(tx, chart_calculation_id).await
    }

    /// Fonction mark_completed.
    pub async fn mark_completed(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
    ) -> Result<(), RuntimeError> {
        RuntimeQueries::mark_completed(tx, chart_calculation_id).await
    }
}
