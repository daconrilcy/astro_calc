use sqlx::PgPool;

use super::runtime_repository::RuntimeRepository;
use crate::domain::{BasicProductScoringProfile, EssentialDignityRuleReference};
use crate::features::natal::catalog::BasicPayloadCatalog;
use crate::shared::error::RuntimeError;

#[derive(Clone)]
pub struct CatalogRepository {
    inner: RuntimeRepository,
}

impl CatalogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            inner: RuntimeRepository::new(pool),
        }
    }

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

    pub async fn basic_product_scoring_profile(
        &self,
        product_code: &str,
        payload_contract_version: &str,
    ) -> Result<BasicProductScoringProfile, RuntimeError> {
        self.inner
            .basic_product_scoring_profile(product_code, payload_contract_version)
            .await
    }

    pub async fn essential_dignity_rule_references(
        &self,
        reference_version_id: i32,
        score_profile_id: i32,
    ) -> Result<Vec<EssentialDignityRuleReference>, RuntimeError> {
        self.inner
            .essential_dignity_rule_references(reference_version_id, score_profile_id)
            .await
    }
}
