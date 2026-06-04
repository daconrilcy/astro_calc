//! Matrice benchmark usages ↔ modeles (referentiel `llm_generation_benchmark_*`).

use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BenchmarkUsageRow {
    pub usage_code: String,
    pub label_fr: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BenchmarkUsageModelRow {
    pub usage_code: String,
    pub provider: String,
    pub model: String,
    pub priority: i32,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct BenchmarkCatalog {
    pub usages: Vec<BenchmarkUsageRow>,
    pub usage_models: Vec<BenchmarkUsageModelRow>,
}

pub async fn load_benchmark_catalog(pool: &PgPool) -> BenchmarkCatalog {
    let usages = sqlx::query_as(
        "SELECT usage_code, label_fr, sort_order \
         FROM llm_generation_benchmark_usages WHERE is_active = true ORDER BY sort_order",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let usage_models = sqlx::query_as(
        "SELECT usage_code, provider, model, priority, notes \
         FROM llm_generation_benchmark_usage_models ORDER BY usage_code, priority",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    BenchmarkCatalog {
        usages,
        usage_models,
    }
}

impl BenchmarkCatalog {
    pub fn models_for_usage(&self, usage_code: &str) -> Vec<&BenchmarkUsageModelRow> {
        self.usage_models
            .iter()
            .filter(|m| m.usage_code == usage_code)
            .collect()
    }
}
