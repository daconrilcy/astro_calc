use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use astral_llm_domain::GenerationErrorDetail;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Success,
    Failed,
    SafetyRejected,
}

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failed => "failed",
            Self::SafetyRejected => "safety_rejected",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafetyStatus {
    Passed,
    Rejected,
    NotChecked,
}

impl SafetyStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Rejected => "rejected",
            Self::NotChecked => "not_checked",
        }
    }
}

#[derive(Debug, Clone)]
pub struct GenerationRunRecord {
    pub id: Uuid,
    pub request_id: Option<String>,
    pub product_code: String,
    pub astro_contract_version: String,
    pub output_schema_version: String,
    pub prompt_family: String,
    pub prompt_version: String,
    pub provider_requested: String,
    pub provider_used: Option<String>,
    pub model_requested: String,
    pub model_used: Option<String>,
    pub status: RunStatus,
    pub safety_status: SafetyStatus,
    pub input_hash: String,
    pub output_hash: Option<String>,
    pub token_input: Option<i32>,
    pub token_output: Option<i32>,
    pub latency_ms: Option<i32>,
    pub error_code: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub struct RunPersistence {
    pool: PgPool,
}

impl RunPersistence {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn ensure_schema(&self) -> Result<(), sqlx::Error> {
        sqlx::query(include_str!("../sql/llm_generation_runs.sql"))
            .execute(&self.pool)
            .await?;
        sqlx::query(include_str!("../sql/llm_canonical.sql"))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn insert_run(&self, record: &GenerationRunRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO llm_generation_runs (
                id, request_id, product_code, astro_contract_version, output_schema_version,
                prompt_family, prompt_version, provider_requested, provider_used,
                model_requested, model_used, status, safety_status, input_hash, output_hash,
                token_input, token_output, latency_ms, error_code, created_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15,
                $16, $17, $18, $19, $20
            )
            "#,
        )
        .bind(record.id)
        .bind(&record.request_id)
        .bind(&record.product_code)
        .bind(&record.astro_contract_version)
        .bind(&record.output_schema_version)
        .bind(&record.prompt_family)
        .bind(&record.prompt_version)
        .bind(&record.provider_requested)
        .bind(&record.provider_used)
        .bind(&record.model_requested)
        .bind(&record.model_used)
        .bind(record.status.as_str())
        .bind(record.safety_status.as_str())
        .bind(&record.input_hash)
        .bind(&record.output_hash)
        .bind(record.token_input)
        .bind(record.token_output)
        .bind(record.latency_ms)
        .bind(&record.error_code)
        .bind(record.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

pub fn hash_json(value: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let digest = Sha256::digest(bytes);
    hex::encode(digest)
}

pub fn error_code(error: &GenerationErrorDetail) -> String {
    error.code.as_str().to_string()
}
