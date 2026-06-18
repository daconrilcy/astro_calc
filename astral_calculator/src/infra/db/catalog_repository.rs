//! Module astral_calculator\src\infra\db\catalog_repository.rs du moteur astral_calculator.

use sqlx::PgPool;

use async_trait::async_trait;

use super::runtime_queries::RuntimeQueries;
use crate::application::ports::PayloadCatalogStore;
use crate::domain::{
    BasicProductScoringProfile, EssentialDignityRuleReference, ProjectionLabelDefinition,
    ProjectionReasonDefinition,
};
use crate::features::natal::catalog::BasicPayloadCatalog;
use crate::shared::error::RuntimeError;

#[derive(Clone)]
/// Structure CatalogRepository.
pub struct CatalogRepository {
    inner: RuntimeQueries,
}

#[async_trait]
impl PayloadCatalogStore for CatalogRepository {
    async fn basic_payload_catalog(
        &self,
        product_code: &str,
        payload_contract_version: &str,
        reference_version_id: i32,
    ) -> Result<BasicPayloadCatalog, RuntimeError> {
        CatalogRepository::basic_payload_catalog(
            self,
            product_code,
            payload_contract_version,
            reference_version_id,
        )
        .await
    }

    async fn basic_product_scoring_profile(
        &self,
        product_code: &str,
        payload_contract_version: &str,
    ) -> Result<BasicProductScoringProfile, RuntimeError> {
        CatalogRepository::basic_product_scoring_profile(
            self,
            product_code,
            payload_contract_version,
        )
        .await
    }

    async fn essential_dignity_rule_references(
        &self,
        reference_version_id: i32,
        score_profile_id: i32,
    ) -> Result<Vec<EssentialDignityRuleReference>, RuntimeError> {
        CatalogRepository::essential_dignity_rule_references(
            self,
            reference_version_id,
            score_profile_id,
        )
        .await
    }

    async fn projection_reason_definitions(
        &self,
    ) -> Result<Vec<ProjectionReasonDefinition>, RuntimeError> {
        CatalogRepository::projection_reason_definitions(self).await
    }

    async fn projection_label_definitions(
        &self,
    ) -> Result<Vec<ProjectionLabelDefinition>, RuntimeError> {
        CatalogRepository::projection_label_definitions(self).await
    }
}

impl CatalogRepository {
    /// Fonction new.
    pub fn new(pool: PgPool) -> Self {
        Self {
            inner: RuntimeQueries::new(pool),
        }
    }

    /// Fonction basic_payload_catalog.
    pub async fn basic_payload_catalog(
        &self,
        product_code: &str,
        payload_contract_version: &str,
        reference_version_id: i32,
    ) -> Result<BasicPayloadCatalog, RuntimeError> {
        self.inner
            .basic_payload_catalog(product_code, payload_contract_version, reference_version_id)
            .await
    }

    /// Fonction basic_product_scoring_profile.
    pub async fn basic_product_scoring_profile(
        &self,
        product_code: &str,
        payload_contract_version: &str,
    ) -> Result<BasicProductScoringProfile, RuntimeError> {
        self.inner
            .basic_product_scoring_profile(product_code, payload_contract_version)
            .await
    }

    /// Fonction essential_dignity_rule_references.
    pub async fn essential_dignity_rule_references(
        &self,
        reference_version_id: i32,
        score_profile_id: i32,
    ) -> Result<Vec<EssentialDignityRuleReference>, RuntimeError> {
        self.inner
            .essential_dignity_rule_references(reference_version_id, score_profile_id)
            .await
    }

    /// Fonction projection_reason_definitions.
    pub async fn projection_reason_definitions(
        &self,
    ) -> Result<Vec<ProjectionReasonDefinition>, RuntimeError> {
        self.inner.projection_reason_definitions().await
    }

    /// Fonction projection_label_definitions.
    pub async fn projection_label_definitions(
        &self,
    ) -> Result<Vec<ProjectionLabelDefinition>, RuntimeError> {
        self.inner.projection_label_definitions().await
    }
}
