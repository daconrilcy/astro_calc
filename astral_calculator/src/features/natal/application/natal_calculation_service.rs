//! Module astral_calculator\src\features\natal\application\natal_calculation_service.rs du moteur astral_calculator.

use std::sync::Arc;

use super::reuse_policy::{NatalReusePolicy, NatalReuseResolution};
use super::snapshot_loader::NatalReferenceSnapshotLoader;
use super::workflow::NatalCalculationWorkflow;
use crate::application::ports::{
    CalculationAttemptStore, CalculationFactStore, CalculationTransactionManager,
    LocalizationCatalog, NatalReferenceStore, PayloadCatalogStore, PayloadStore,
    ReferenceSystemResolver, SignalStore,
};
use crate::astrology::ephemeris::EphemerisEngine;
use crate::domain::{BasicPayload, NatalChartInput, RuntimeOptions};
use crate::features::natal::application::NatalCalculationCapability;
use crate::features::natal::catalog::BasicPayloadCatalog;
use crate::shared::error::RuntimeError;
use crate::shared::idempotency::{advisory_lock_key, idempotency_key, input_hash};
use async_trait::async_trait;

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
    C: CalculationTransactionManager
        + CalculationAttemptStore
        + CalculationFactStore
        + PayloadStore
        + SignalStore,
    P: PayloadCatalogStore,
    R: NatalReferenceStore + LocalizationCatalog + ReferenceSystemResolver,
    E: EphemerisEngine,
{
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

    pub fn options(&self) -> &RuntimeOptions {
        &self.options
    }

    pub async fn calculate_basic(
        &self,
        input: NatalChartInput,
    ) -> Result<BasicPayload, RuntimeError> {
        let (payload, _) = self.calculate_basic_with_catalog(input).await?;
        Ok(payload)
    }

    pub async fn calculate_basic_with_catalog(
        &self,
        input: NatalChartInput,
    ) -> Result<(BasicPayload, BasicPayloadCatalog), RuntimeError> {
        let product_code = input.product_code().to_string();
        let payload_language_id = self.references.language_id_for_code("en").await?;
        let input_hash = input_hash(&input)?;
        let idempotency_key = idempotency_key(&input, &self.options)?;
        let lock_key = advisory_lock_key(&idempotency_key);
        let snapshot = NatalReferenceSnapshotLoader::new(&self.catalogs, &self.references)
            .load(&input, &product_code)
            .await?;

        let mut tx = self.calculations.begin().await?;
        self.calculations
            .lock_idempotency(&mut tx, lock_key)
            .await?;
        let existing = self
            .calculations
            .calculations_for_key(&mut tx, &idempotency_key)
            .await?;
        let reuse_policy = NatalReusePolicy::new(
            &self.calculations,
            &snapshot,
            &product_code,
            self.options.stale_after_seconds,
        );
        let tx = match reuse_policy
            .resolve(&input, payload_language_id, &idempotency_key, &existing, tx)
            .await?
        {
            NatalReuseResolution::Return(payload) => return Ok((payload, snapshot.catalog)),
            NatalReuseResolution::Proceed(tx) => tx,
        };

        NatalCalculationWorkflow::new(
            &self.calculations,
            &self.ephemeris,
            &self.options,
            &snapshot,
        )
        .execute(
            tx,
            input,
            payload_language_id,
            input_hash,
            idempotency_key,
            existing,
        )
        .await
        .map(|payload| (payload, snapshot.catalog))
    }
}

#[async_trait]
impl<C, P, R, E> NatalCalculationCapability for NatalCalculationService<C, P, R, E>
where
    C: CalculationTransactionManager
        + CalculationAttemptStore
        + CalculationFactStore
        + PayloadStore
        + SignalStore
        + Send
        + Sync,
    P: PayloadCatalogStore + Send + Sync,
    R: NatalReferenceStore + LocalizationCatalog + ReferenceSystemResolver + Send + Sync,
    E: EphemerisEngine + Send + Sync,
{
    fn options(&self) -> &RuntimeOptions {
        &self.options
    }

    async fn calculate_basic(&self, input: NatalChartInput) -> Result<BasicPayload, RuntimeError> {
        NatalCalculationService::calculate_basic(self, input).await
    }

    async fn calculate_basic_with_catalog(
        &self,
        input: NatalChartInput,
    ) -> Result<(BasicPayload, BasicPayloadCatalog), RuntimeError> {
        NatalCalculationService::calculate_basic_with_catalog(self, input).await
    }
}
