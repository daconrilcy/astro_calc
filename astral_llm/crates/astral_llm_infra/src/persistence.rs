use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use astral_llm_domain::{
    GenerateReadingResponse, GenerationErrorDetail, GenerationStepRecord, PublicTokenUsage,
    TokenCostSummary, TokenUsage, TokenUsageEngine, TokenUsageItem, TokenUsageSummary,
    TokenUsageType,
};

use crate::run_audit_view::{
    RunAuditPromptTraceView, RunAuditRow, RunAuditStepView, RunAuditView, TokenUsageItemView,
};
use crate::sql_script::execute_sql_script;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Success,
    Failed,
    SafetyRejected,
    Pending,
}

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failed => "failed",
            Self::SafetyRejected => "safety_rejected",
            Self::Pending => "pending",
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

#[derive(Debug, Clone)]
pub struct GenerationPromptTraceRecord {
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
pub struct GenerationTokenUsageRecord {
    pub usage_type_code: String,
    pub usage_subtype: Option<String>,
    pub token_count: i32,
    pub unit_price_usd_per_mtok: Option<f64>,
    pub estimated_cost_usd: Option<f64>,
    pub provider_metric_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IdempotencyHit {
    pub run_id: Uuid,
    pub status: String,
    pub input_hash: String,
    pub response: Option<GenerateReadingResponse>,
}

#[derive(Debug, Clone)]
pub enum IdempotencyClaim {
    Acquired { run_id: Uuid },
    InProgress { run_id: Uuid },
    Replay(GenerateReadingResponse),
    PayloadMismatch,
}

pub struct RunPersistence {
    pool: PgPool,
}

impl RunPersistence {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn ensure_schema(&self) -> Result<(), sqlx::Error> {
        execute_sql_script(&self.pool, include_str!("../sql/llm_generation_runs.sql")).await?;
        execute_sql_script(&self.pool, include_str!("../sql/llm_canonical.sql")).await?;
        execute_sql_script(&self.pool, include_str!("../sql/llm_provider_catalog.sql")).await?;
        execute_sql_script(&self.pool, include_str!("../sql/llm_i18n_canonical.sql")).await?;
        execute_sql_script(&self.pool, include_str!("../sql/llm_audit_extensions.sql")).await?;
        execute_sql_script(
            &self.pool,
            include_str!("../sql/llm_interpretation_profiles.sql"),
        )
        .await?;
        execute_sql_script(
            &self.pool,
            include_str!("../sql/llm_integration_services.sql"),
        )
        .await?;
        execute_sql_script(&self.pool, include_str!("../sql/llm_jobs.sql")).await?;
        Ok(())
    }

    /// Verifie que les tables attendues existent (production sans auto-migrate).
    pub async fn verify_schema(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT 1 FROM llm_generation_runs LIMIT 0")
            .execute(&self.pool)
            .await?;
        sqlx::query("SELECT 1 FROM llm_idempotency_records LIMIT 0")
            .execute(&self.pool)
            .await?;
        sqlx::query("SELECT 1 FROM llm_generation_steps LIMIT 0")
            .execute(&self.pool)
            .await?;
        sqlx::query("SELECT 1 FROM llm_integration_services LIMIT 0")
            .execute(&self.pool)
            .await?;
        sqlx::query("SELECT 1 FROM llm_jobs LIMIT 0")
            .execute(&self.pool)
            .await?;
        sqlx::query("SELECT 1 FROM llm_generation_prompt_traces LIMIT 0")
            .execute(&self.pool)
            .await?;
        sqlx::query("SELECT 1 FROM llm_generation_run_token_usages LIMIT 0")
            .execute(&self.pool)
            .await?;
        sqlx::query("SELECT 1 FROM llm_generation_step_token_usages LIMIT 0")
            .execute(&self.pool)
            .await?;
        sqlx::query("SELECT 1 FROM llm_model_characteristics LIMIT 0")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn insert_payloads(
        &self,
        run_id: Uuid,
        sanitized_request: &serde_json::Value,
        sanitized_response: &serde_json::Value,
        prompt_hash: &str,
        astro_facts_hash: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO llm_generation_payloads (
                run_id, sanitized_request_json, sanitized_response_json,
                prompt_hash, astro_facts_hash, created_at
            ) VALUES ($1, $2, $3, $4, $5, NOW())
            ON CONFLICT (run_id) DO UPDATE SET
                sanitized_request_json = EXCLUDED.sanitized_request_json,
                sanitized_response_json = EXCLUDED.sanitized_response_json,
                prompt_hash = EXCLUDED.prompt_hash,
                astro_facts_hash = EXCLUDED.astro_facts_hash
            "#,
        )
        .bind(run_id)
        .bind(sanitized_request)
        .bind(sanitized_response)
        .bind(prompt_hash)
        .bind(astro_facts_hash)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn upsert_run(&self, record: &GenerationRunRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO llm_generation_runs (
                id, request_id, idempotency_key, product_code, user_language,
                astro_contract_version, output_schema_version, prompt_family, prompt_version,
                safety_policy_version, provider_requested, provider_used, model_requested, model_used,
                generation_mode, fallback_used, selected_domains, status, safety_status,
                input_hash, output_hash, token_input, token_output, latency_ms, error_code, created_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17,
                $18, $19, $20, $21, $22, $23, $24, $25, $26
            )
            ON CONFLICT (id) DO UPDATE SET
                request_id = EXCLUDED.request_id,
                idempotency_key = EXCLUDED.idempotency_key,
                product_code = EXCLUDED.product_code,
                user_language = EXCLUDED.user_language,
                astro_contract_version = EXCLUDED.astro_contract_version,
                output_schema_version = EXCLUDED.output_schema_version,
                prompt_family = EXCLUDED.prompt_family,
                prompt_version = EXCLUDED.prompt_version,
                safety_policy_version = EXCLUDED.safety_policy_version,
                provider_requested = EXCLUDED.provider_requested,
                provider_used = EXCLUDED.provider_used,
                model_requested = EXCLUDED.model_requested,
                model_used = EXCLUDED.model_used,
                generation_mode = EXCLUDED.generation_mode,
                fallback_used = EXCLUDED.fallback_used,
                selected_domains = EXCLUDED.selected_domains,
                status = EXCLUDED.status,
                safety_status = EXCLUDED.safety_status,
                input_hash = EXCLUDED.input_hash,
                output_hash = EXCLUDED.output_hash,
                token_input = EXCLUDED.token_input,
                token_output = EXCLUDED.token_output,
                latency_ms = EXCLUDED.latency_ms,
                error_code = EXCLUDED.error_code
            "#,
        )
        .bind(record.id)
        .bind(&record.request_id)
        .bind(&record.idempotency_key)
        .bind(&record.product_code)
        .bind(&record.user_language)
        .bind(&record.astro_contract_version)
        .bind(&record.output_schema_version)
        .bind(&record.prompt_family)
        .bind(&record.prompt_version)
        .bind(&record.safety_policy_version)
        .bind(&record.provider_requested)
        .bind(&record.provider_used)
        .bind(&record.model_requested)
        .bind(&record.model_used)
        .bind(&record.generation_mode)
        .bind(record.fallback_used)
        .bind(&record.selected_domains)
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

    pub async fn insert_prompt_trace(
        &self,
        record: &GenerationPromptTraceRecord,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO llm_generation_prompt_traces (
                run_id, chapter_code, step_type, attempt, prompt_family, prompt_version,
                message_count, compiled_prompt, messages_json
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(record.run_id)
        .bind(&record.chapter_code)
        .bind(&record.step_type)
        .bind(&record.attempt)
        .bind(&record.prompt_family)
        .bind(&record.prompt_version)
        .bind(record.message_count)
        .bind(&record.compiled_prompt)
        .bind(&record.messages_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_steps(
        &self,
        run_id: Uuid,
        steps: &[GenerationStepRecord],
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        let mut step_ids = Vec::with_capacity(steps.len());
        for step in steps {
            let step_id: Uuid = sqlx::query_scalar(
                r#"
                INSERT INTO llm_generation_steps (
                    run_id, step_type, chapter_code, provider, model, status,
                    input_tokens, output_tokens, latency_ms, error_code
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                RETURNING id
                "#,
            )
            .bind(run_id)
            .bind(&step.step_type)
            .bind(&step.chapter_code)
            .bind(&step.provider)
            .bind(&step.model)
            .bind(step.status.as_str())
            .bind(step.input_tokens.map(|v| v as i32))
            .bind(step.output_tokens.map(|v| v as i32))
            .bind(step.latency_ms.map(|v| v as i32))
            .bind(&step.error_code)
            .fetch_one(&self.pool)
            .await?;
            step_ids.push(step_id);
        }
        Ok(step_ids)
    }

    pub async fn replace_run_token_usages(
        &self,
        run_id: Uuid,
        usages: &[GenerationTokenUsageRecord],
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM llm_generation_run_token_usages WHERE run_id = $1")
            .bind(run_id)
            .execute(&self.pool)
            .await?;
        for usage in usages {
            sqlx::query(
                r#"
                INSERT INTO llm_generation_run_token_usages (
                    run_id, usage_type_code, usage_subtype, token_count,
                    unit_price_usd_per_mtok, estimated_cost_usd, provider_metric_name
                ) VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
            )
            .bind(run_id)
            .bind(&usage.usage_type_code)
            .bind(&usage.usage_subtype)
            .bind(usage.token_count)
            .bind(usage.unit_price_usd_per_mtok)
            .bind(usage.estimated_cost_usd)
            .bind(&usage.provider_metric_name)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn replace_step_token_usages(
        &self,
        step_id: Uuid,
        usages: &[GenerationTokenUsageRecord],
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM llm_generation_step_token_usages WHERE step_id = $1")
            .bind(step_id)
            .execute(&self.pool)
            .await?;
        for usage in usages {
            sqlx::query(
                r#"
                INSERT INTO llm_generation_step_token_usages (
                    step_id, usage_type_code, usage_subtype, token_count,
                    unit_price_usd_per_mtok, estimated_cost_usd, provider_metric_name
                ) VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
            )
            .bind(step_id)
            .bind(&usage.usage_type_code)
            .bind(&usage.usage_subtype)
            .bind(usage.token_count)
            .bind(usage.unit_price_usd_per_mtok)
            .bind(usage.estimated_cost_usd)
            .bind(&usage.provider_metric_name)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn claim_idempotency(
        &self,
        key: &str,
        product_code: &str,
        run_id: Uuid,
        input_hash: &str,
        ttl_hours: i64,
    ) -> Result<IdempotencyClaim, sqlx::Error> {
        let expires = Utc::now() + Duration::hours(ttl_hours);
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query_as::<_, (Uuid, String, String, Option<serde_json::Value>)>(
            "SELECT run_id, status, input_hash, response_json FROM llm_idempotency_records \
             WHERE idempotency_key = $1 AND product_code = $2 AND expires_at > NOW() \
             FOR UPDATE",
        )
        .bind(key)
        .bind(product_code)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some((existing_run_id, status, existing_hash, response_json)) = row {
            if existing_hash != input_hash {
                tx.rollback().await?;
                return Ok(IdempotencyClaim::PayloadMismatch);
            }

            let claim = match status.as_str() {
                "completed" => {
                    let response = response_json.and_then(|v| serde_json::from_value(v).ok());
                    if let Some(response) = response {
                        IdempotencyClaim::Replay(response)
                    } else {
                        IdempotencyClaim::InProgress {
                            run_id: existing_run_id,
                        }
                    }
                }
                "pending" => IdempotencyClaim::InProgress {
                    run_id: existing_run_id,
                },
                "failed" | "safety_rejected" => {
                    sqlx::query(
                        "UPDATE llm_idempotency_records SET status = 'pending', run_id = $3, \
                         response_json = NULL, expires_at = $4 \
                         WHERE idempotency_key = $1 AND product_code = $2",
                    )
                    .bind(key)
                    .bind(product_code)
                    .bind(run_id)
                    .bind(expires)
                    .execute(&mut *tx)
                    .await?;
                    IdempotencyClaim::Acquired { run_id }
                }
                _ => IdempotencyClaim::InProgress {
                    run_id: existing_run_id,
                },
            };
            tx.commit().await?;
            return Ok(claim);
        }

        sqlx::query(
            r#"
            INSERT INTO llm_idempotency_records (
                idempotency_key, product_code, run_id, input_hash, status, expires_at
            ) VALUES ($1, $2, $3, $4, 'pending', $5)
            "#,
        )
        .bind(key)
        .bind(product_code)
        .bind(run_id)
        .bind(input_hash)
        .bind(expires)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(IdempotencyClaim::Acquired { run_id })
    }

    pub async fn find_idempotency(
        &self,
        key: &str,
        product_code: &str,
    ) -> Result<Option<IdempotencyHit>, sqlx::Error> {
        let row = sqlx::query_as::<_, (Uuid, String, String, Option<serde_json::Value>)>(
            "SELECT run_id, status, input_hash, response_json FROM llm_idempotency_records \
             WHERE idempotency_key = $1 AND product_code = $2 AND expires_at > NOW()",
        )
        .bind(key)
        .bind(product_code)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(run_id, status, input_hash, response_json)| {
            let response = response_json.and_then(|v| serde_json::from_value(v).ok());
            IdempotencyHit {
                run_id,
                status,
                input_hash,
                response,
            }
        }))
    }

    pub async fn delete_idempotency_record(
        &self,
        key: &str,
        product_code: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "DELETE FROM llm_idempotency_records WHERE idempotency_key = $1 AND product_code = $2",
        )
        .bind(key)
        .bind(product_code)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn finalize_idempotency(
        &self,
        key: &str,
        product_code: &str,
        status: &str,
        response: Option<&GenerateReadingResponse>,
    ) -> Result<(), sqlx::Error> {
        let json = response.map(|r| serde_json::to_value(r).unwrap_or(serde_json::json!({})));
        sqlx::query(
            "UPDATE llm_idempotency_records SET status = $3, response_json = $4 \
             WHERE idempotency_key = $1 AND product_code = $2",
        )
        .bind(key)
        .bind(product_code)
        .bind(status)
        .bind(json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_run_audit(&self, run_id: Uuid) -> Result<Option<RunAuditView>, sqlx::Error> {
        let run = sqlx::query_as::<_, RunAuditRow>(
            r#"
            SELECT id, request_id, idempotency_key, product_code, user_language, generation_mode,
                   provider_requested, provider_used, model_requested, model_used,
                   status, safety_status, error_code, latency_ms, token_input, token_output,
                   selected_domains, fallback_used, created_at
            FROM llm_generation_runs
            WHERE id = $1
            "#,
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(run) = run else {
            return Ok(None);
        };

        let steps = sqlx::query_as::<_, RunAuditStepView>(
            r#"
            SELECT id, step_type, chapter_code, provider, model, status,
                   input_tokens, output_tokens, latency_ms, error_code, created_at
            FROM llm_generation_steps
            WHERE run_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        let run_usage_rows = sqlx::query_as::<_, TokenUsageItemView>(
            r#"
            SELECT usage_type_code, usage_subtype, token_count,
                   unit_price_usd_per_mtok, estimated_cost_usd, provider_metric_name
            FROM llm_generation_run_token_usages
            WHERE run_id = $1
            ORDER BY id ASC
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        let mut steps = steps;
        for step in &mut steps {
            let rows = sqlx::query_as::<_, TokenUsageItemView>(
                r#"
                SELECT usage_type_code, usage_subtype, token_count,
                       unit_price_usd_per_mtok, estimated_cost_usd, provider_metric_name
                FROM llm_generation_step_token_usages
                WHERE step_id = $1
                ORDER BY id ASC
                "#,
            )
            .bind(step.id)
            .fetch_all(&self.pool)
            .await?;
            step.token_usage = build_public_usage(
                &step.provider,
                &step.model,
                None,
                rows,
            );
        }

        let prompt_traces = sqlx::query_as::<_, RunAuditPromptTraceView>(
            r#"
            SELECT chapter_code, step_type, attempt, prompt_family, prompt_version,
                   message_count, compiled_prompt, messages_json, created_at
            FROM llm_generation_prompt_traces
            WHERE run_id = $1
            ORDER BY created_at ASC, id ASC
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        let run_provider = run
            .provider_used
            .clone()
            .unwrap_or_else(|| run.provider_requested.clone());
        let run_model = run
            .model_used
            .clone()
            .unwrap_or_else(|| run.model_requested.clone());
        let token_usage = build_public_usage(&run_provider, &run_model, None, run_usage_rows);

        Ok(Some(run.into_view(steps, prompt_traces, token_usage)))
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

fn build_public_usage(
    provider: &str,
    model: &str,
    pricing_source: Option<String>,
    rows: Vec<TokenUsageItemView>,
) -> Option<PublicTokenUsage> {
    if rows.is_empty() {
        return None;
    }

    let details: Vec<TokenUsageItem> = rows
        .into_iter()
        .filter_map(|row| {
            let usage_type = match row.usage_type_code.as_str() {
                "input" => TokenUsageType::Input,
                "output" => TokenUsageType::Output,
                "cache" => TokenUsageType::Cache,
                "reasoning" => TokenUsageType::Reasoning,
                _ => return None,
            };
            Some(TokenUsageItem {
                usage_type,
                usage_subtype: row.usage_subtype,
                token_count: row.token_count.max(0) as u32,
                provider_metric_name: row.provider_metric_name,
                unit_price_usd_per_mtok: row.unit_price_usd_per_mtok,
                estimated_cost_usd: row.estimated_cost_usd,
            })
        })
        .collect();

    if details.is_empty() {
        return None;
    }

    let usage = TokenUsage {
        items: details.clone(),
    };
    Some(PublicTokenUsage {
        summary: TokenUsageSummary {
            input_tokens: usage.tokens_for(TokenUsageType::Input),
            output_tokens: usage.tokens_for(TokenUsageType::Output),
            cache_tokens: usage.tokens_for(TokenUsageType::Cache),
            reasoning_tokens: usage.tokens_for(TokenUsageType::Reasoning),
        },
        cost: TokenCostSummary {
            currency: "USD".into(),
            estimated_total: details
                .iter()
                .filter_map(|item| item.estimated_cost_usd)
                .reduce(|a, b| a + b),
            input_cost: usage.cost_for(TokenUsageType::Input),
            output_cost: usage.cost_for(TokenUsageType::Output),
            cache_cost: usage.cost_for(TokenUsageType::Cache),
            reasoning_cost: usage.cost_for(TokenUsageType::Reasoning),
        },
        engine: TokenUsageEngine {
            provider: provider.to_string(),
            model: model.to_string(),
            pricing_source,
        },
        details,
    })
}
