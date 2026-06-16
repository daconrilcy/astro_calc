use astral_llm_domain::PublicTokenUsage;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub(crate) struct RunAuditRow {
    pub(crate) id: Uuid,
    pub(crate) request_id: Option<String>,
    pub(crate) idempotency_key: Option<String>,
    pub(crate) product_code: String,
    pub(crate) user_language: Option<String>,
    pub(crate) generation_mode: Option<String>,
    pub(crate) provider_requested: String,
    pub(crate) provider_used: Option<String>,
    pub(crate) model_requested: String,
    pub(crate) model_used: Option<String>,
    pub(crate) status: String,
    pub(crate) safety_status: String,
    pub(crate) error_code: Option<String>,
    pub(crate) latency_ms: Option<i32>,
    pub(crate) token_input: Option<i32>,
    pub(crate) token_output: Option<i32>,
    pub(crate) selected_domains: Option<serde_json::Value>,
    pub(crate) fallback_used: Option<bool>,
    pub(crate) created_at: DateTime<Utc>,
}

impl RunAuditRow {
    pub(crate) fn into_view(
        self,
        steps: Vec<RunAuditStepView>,
        prompt_traces: Vec<RunAuditPromptTraceView>,
        token_usage: Option<PublicTokenUsage>,
    ) -> RunAuditView {
        RunAuditView {
            run_id: self.id,
            request_id: self.request_id,
            idempotency_key: self.idempotency_key,
            product_code: self.product_code,
            user_language: self.user_language,
            generation_mode: self.generation_mode,
            provider_requested: self.provider_requested,
            provider_used: self.provider_used,
            model_requested: self.model_requested,
            model_used: self.model_used,
            status: self.status,
            safety_status: self.safety_status,
            error_code: self.error_code,
            latency_ms: self.latency_ms,
            token_input: self.token_input,
            token_output: self.token_output,
            selected_domains: self.selected_domains,
            fallback_used: self.fallback_used,
            created_at: self.created_at,
            token_usage,
            steps,
            prompt_traces,
        }
    }
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct RunAuditStepView {
    pub id: Uuid,
    pub step_type: String,
    pub chapter_code: Option<String>,
    pub provider: String,
    pub model: String,
    pub status: String,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub latency_ms: Option<i32>,
    pub error_code: Option<String>,
    pub created_at: DateTime<Utc>,
    #[sqlx(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_usage: Option<PublicTokenUsage>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct TokenUsageItemView {
    pub usage_type_code: String,
    pub usage_subtype: Option<String>,
    pub token_count: i32,
    pub unit_price_usd_per_mtok: Option<f64>,
    pub estimated_cost_usd: Option<f64>,
    pub provider_metric_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct RunAuditPromptTraceView {
    pub chapter_code: Option<String>,
    pub step_type: Option<String>,
    pub attempt: Option<String>,
    pub prompt_family: Option<String>,
    pub prompt_version: Option<String>,
    pub message_count: i32,
    pub compiled_prompt: String,
    pub messages_json: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunAuditView {
    pub run_id: Uuid,
    pub request_id: Option<String>,
    pub idempotency_key: Option<String>,
    pub product_code: String,
    pub user_language: Option<String>,
    pub generation_mode: Option<String>,
    pub provider_requested: String,
    pub provider_used: Option<String>,
    pub model_requested: String,
    pub model_used: Option<String>,
    pub status: String,
    pub safety_status: String,
    pub error_code: Option<String>,
    pub latency_ms: Option<i32>,
    pub token_input: Option<i32>,
    pub token_output: Option<i32>,
    pub selected_domains: Option<serde_json::Value>,
    pub fallback_used: Option<bool>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_usage: Option<PublicTokenUsage>,
    pub steps: Vec<RunAuditStepView>,
    pub prompt_traces: Vec<RunAuditPromptTraceView>,
}
