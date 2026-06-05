use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use astral_llm_domain::{GenerateReadingResponse, GenerationErrorDetail, GenerationStepRecord};

use crate::run_audit_view::{RunAuditRow, RunAuditStepView, RunAuditView};
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

    pub async fn insert_run(&self, record: &GenerationRunRecord) -> Result<(), sqlx::Error> {
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

    pub async fn insert_steps(
        &self,
        run_id: Uuid,
        steps: &[GenerationStepRecord],
    ) -> Result<(), sqlx::Error> {
        for step in steps {
            sqlx::query(
                r#"
                INSERT INTO llm_generation_steps (
                    run_id, step_type, chapter_code, provider, model, status,
                    input_tokens, output_tokens, latency_ms, error_code
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
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
            SELECT step_type, chapter_code, provider, model, status,
                   input_tokens, output_tokens, latency_ms, error_code, created_at
            FROM llm_generation_steps
            WHERE run_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(Some(run.into_view(steps)))
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
