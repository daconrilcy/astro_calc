use sqlx::PgPool;

use super::runtime_repository::RuntimeRepository;
use crate::natal::catalog::BasicPayloadCatalog;
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
}
