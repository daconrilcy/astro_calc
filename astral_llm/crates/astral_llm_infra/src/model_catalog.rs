use astral_llm_domain::model_capability::ModelCapability;

use crate::provider_catalog::{row_to_capability, ProviderCatalogRepository};

pub use crate::provider_catalog::load_active_provider_codes;

pub async fn load_model_capabilities(pool: &sqlx::PgPool) -> Vec<ModelCapability> {
    let repo = ProviderCatalogRepository::new(pool.clone());
    let rows = repo.list_models(None, false).await;
    let Ok(rows) = rows else {
        return Vec::new();
    };

    rows.iter().filter_map(row_to_capability).collect()
}
