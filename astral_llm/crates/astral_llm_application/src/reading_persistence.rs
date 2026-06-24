use std::error::Error as StdError;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use astral_llm_domain::chapter_orchestration::GenerationStepRecord;
use astral_llm_domain::TokenUsage;

use crate::prompt_trace::PromptTraceRecord;

mod infra_adapter;
pub use infra_adapter::shared_reading_persistence;

pub type SharedReadingPersistence = Arc<dyn ReadingPersistence>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistedRunStatus {
    Success,
    Failed,
    SafetyRejected,
    Pending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistedSafetyStatus {
    Passed,
    Rejected,
    NotChecked,
}

#[derive(Debug, Clone)]
pub struct PersistedGenerationRunRecord {
    pub id: Uuid,
    pub request_id: Option<String>,
    pub idempotency_key: Option<String>,
    pub product_code: String,
    pub user_language: String,
    pub astro_contract_version: String,
    pub output_schema_version: String,
    pub prompt_family: String,
    pub prompt_version: String,
    pub safety_policy_version: String,
    pub provider_requested: String,
    pub provider_used: Option<String>,
    pub model_requested: String,
    pub model_used: Option<String>,
    pub generation_mode: String,
    pub fallback_used: bool,
    pub selected_domains: Option<serde_json::Value>,
    pub status: PersistedRunStatus,
    pub safety_status: PersistedSafetyStatus,
    pub input_hash: String,
    pub output_hash: Option<String>,
    pub token_input: Option<i32>,
    pub token_output: Option<i32>,
    pub latency_ms: Option<i32>,
    pub error_code: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PersistedPromptTraceRecord {
    pub run_id: Uuid,
    pub chapter_code: Option<String>,
    pub step_type: Option<String>,
    pub attempt: Option<String>,
    pub prompt_family: Option<String>,
    pub prompt_version: Option<String>,
    pub message_count: i32,
    pub compiled_prompt: String,
    pub messages_json: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct PersistedTokenUsageRecord {
    pub usage_type_code: String,
    pub usage_subtype: Option<String>,
    pub token_count: i32,
    pub unit_price_usd_per_mtok: Option<f64>,
    pub estimated_cost_usd: Option<f64>,
    pub provider_metric_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExplanationCacheKeyRecord {
    pub language_code: String,
    pub key_hash: String,
}

#[derive(Debug, Clone)]
pub struct ExplanationCacheRecord {
    pub language_code: String,
    pub kind_code: String,
    pub key_hash: String,
    pub key_json: serde_json::Value,
    pub title: String,
    pub explanation: String,
    pub expression_primary: Option<String>,
    pub provider: String,
    pub model: String,
    pub prompt_version: String,
}

#[derive(Debug, Error)]
pub enum ReadingPersistenceError {
    #[error("{operation} failed: {message}")]
    Operation {
        operation: &'static str,
        message: String,
    },
}

impl ReadingPersistenceError {
    fn from_source(operation: &'static str, error: &(dyn StdError + 'static)) -> Self {
        Self::Operation {
            operation,
            message: error.to_string(),
        }
    }
}

#[async_trait]
pub trait ReadingPersistence: Send + Sync {
    async fn upsert_run(
        &self,
        record: &PersistedGenerationRunRecord,
    ) -> Result<(), ReadingPersistenceError>;

    async fn insert_prompt_trace(
        &self,
        record: &PersistedPromptTraceRecord,
    ) -> Result<(), ReadingPersistenceError>;

    async fn insert_steps(
        &self,
        run_id: Uuid,
        steps: &[GenerationStepRecord],
    ) -> Result<Vec<Uuid>, ReadingPersistenceError>;

    async fn replace_run_token_usages(
        &self,
        run_id: Uuid,
        usage_records: &[PersistedTokenUsageRecord],
    ) -> Result<(), ReadingPersistenceError>;

    async fn replace_step_token_usages(
        &self,
        step_id: Uuid,
        usage_records: &[PersistedTokenUsageRecord],
    ) -> Result<(), ReadingPersistenceError>;

    async fn lookup_natal_explanations(
        &self,
        keys: &[ExplanationCacheKeyRecord],
    ) -> Result<Vec<ExplanationCacheRecord>, ReadingPersistenceError>;

    async fn upsert_natal_explanations(
        &self,
        records: &[ExplanationCacheRecord],
    ) -> Result<(), ReadingPersistenceError>;
}

pub fn priced_usage_records(usage: &TokenUsage) -> Vec<PersistedTokenUsageRecord> {
    usage
        .items
        .iter()
        .map(|item| PersistedTokenUsageRecord {
            usage_type_code: item.usage_type.as_str().to_string(),
            usage_subtype: item.usage_subtype.clone(),
            token_count: i32::try_from(item.token_count).unwrap_or(i32::MAX),
            unit_price_usd_per_mtok: item.unit_price_usd_per_mtok,
            estimated_cost_usd: item.estimated_cost_usd,
            provider_metric_name: item.provider_metric_name.clone(),
        })
        .collect()
}

pub fn hash_json_value(value: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let digest = Sha256::digest(bytes);
    hex::encode(digest)
}

pub fn stable_json_digest(value: &serde_json::Value) -> String {
    hash_json_value(value)
}

pub fn persisted_prompt_trace_record(
    run_id: Uuid,
    trace: PromptTraceRecord,
) -> PersistedPromptTraceRecord {
    PersistedPromptTraceRecord {
        run_id,
        chapter_code: trace.chapter_code,
        step_type: trace.step_type,
        attempt: trace.attempt,
        prompt_family: trace.prompt_family,
        prompt_version: trace.prompt_version,
        message_count: trace.message_count,
        compiled_prompt: trace.compiled_prompt,
        messages_json: trace.messages_json,
    }
}
