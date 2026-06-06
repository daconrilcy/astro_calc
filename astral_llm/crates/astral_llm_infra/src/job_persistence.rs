use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use astral_llm_domain::integration::JobStatus;

#[derive(Debug, Clone)]
pub struct JobRecord {
    pub job_id: Uuid,
    pub run_id: Uuid,
    pub service_code: String,
    pub tenant_id: String,
    pub api_key_id: String,
    pub user_id: Option<String>,
    pub idempotency_key: String,
    pub idempotency_payload_hash: String,
    pub request_payload_hash: String,
    pub status: JobStatus,
    pub request_json: serde_json::Value,
    pub result_json: Option<serde_json::Value>,
    pub error_json: Option<serde_json::Value>,
    pub generation_run_id: Option<Uuid>,
    pub attempt_count: i32,
    pub max_attempts: i32,
    pub submitted_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct NewJobRecord {
    pub run_id: Uuid,
    pub service_code: String,
    pub tenant_id: String,
    pub api_key_id: String,
    pub user_id: Option<String>,
    pub idempotency_key: String,
    pub idempotency_payload_hash: String,
    pub request_payload_hash: String,
    pub request_json: serde_json::Value,
    pub expires_at: Option<DateTime<Utc>>,
    pub max_attempts: i32,
}

#[derive(Debug, Clone)]
pub enum IdempotentJobClaim {
    Inserted { run_id: Uuid },
    Replay(JobRecord),
    InProgress { run_id: Uuid, status: JobStatus },
    Conflict { existing_service_code: String },
    ApiKeyMismatch { run_id: Uuid },
    PayloadMismatch { run_id: Uuid },
}

#[derive(Debug, Clone)]
pub struct IdempotencyJobIdentity {
    pub run_id: Uuid,
    pub service_code: String,
    pub api_key_id: String,
    pub idempotency_payload_hash: String,
}

pub struct JobPersistence {
    pool: PgPool,
}

impl JobPersistence {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn insert_job(&self, job: &NewJobRecord) -> Result<Uuid, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            INSERT INTO llm_jobs (
                run_id, service_code, tenant_id, api_key_id, user_id,
                idempotency_key, idempotency_payload_hash, request_payload_hash,
                status, request_json, expires_at, max_attempts
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, 'queued', $9, $10, $11
            )
            RETURNING job_id
            "#,
        )
        .bind(job.run_id)
        .bind(&job.service_code)
        .bind(&job.tenant_id)
        .bind(&job.api_key_id)
        .bind(&job.user_id)
        .bind(&job.idempotency_key)
        .bind(&job.idempotency_payload_hash)
        .bind(&job.request_payload_hash)
        .bind(&job.request_json)
        .bind(job.expires_at)
        .bind(job.max_attempts)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn claim_idempotent_insert(
        &self,
        job: &NewJobRecord,
    ) -> Result<IdempotentJobClaim, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let existing = sqlx::query_as::<_, (Uuid, String, String, String, String)>(
            "SELECT run_id, service_code, api_key_id, status, idempotency_payload_hash \
             FROM llm_jobs WHERE tenant_id = $1 AND idempotency_key = $2 \
             FOR UPDATE",
        )
        .bind(&job.tenant_id)
        .bind(&job.idempotency_key)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some((run_id, service_code, api_key_id, status_raw, existing_hash)) = existing {
            if api_key_id != job.api_key_id {
                tx.commit().await?;
                return Ok(IdempotentJobClaim::ApiKeyMismatch { run_id });
            }
            if service_code != job.service_code {
                tx.commit().await?;
                return Ok(IdempotentJobClaim::Conflict {
                    existing_service_code: service_code,
                });
            }
            if existing_hash != job.idempotency_payload_hash {
                tx.commit().await?;
                return Ok(IdempotentJobClaim::PayloadMismatch { run_id });
            }
            let Some(status) = JobStatus::parse(&status_raw) else {
                tx.commit().await?;
                return Ok(IdempotentJobClaim::InProgress {
                    run_id,
                    status: JobStatus::Queued,
                });
            };
            if status.is_terminal() {
                let record = self.fetch_job_by_run_id_tx(run_id, &mut tx).await?;
                tx.commit().await?;
                return Ok(IdempotentJobClaim::Replay(record));
            }
            tx.commit().await?;
            return Ok(IdempotentJobClaim::InProgress { run_id, status });
        }

        sqlx::query(
            r#"
            INSERT INTO llm_jobs (
                run_id, service_code, tenant_id, api_key_id, user_id,
                idempotency_key, idempotency_payload_hash, request_payload_hash,
                status, request_json, expires_at, max_attempts
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, 'queued', $9, $10, $11
            )
            "#,
        )
        .bind(job.run_id)
        .bind(&job.service_code)
        .bind(&job.tenant_id)
        .bind(&job.api_key_id)
        .bind(&job.user_id)
        .bind(&job.idempotency_key)
        .bind(&job.idempotency_payload_hash)
        .bind(&job.request_payload_hash)
        .bind(&job.request_json)
        .bind(job.expires_at)
        .bind(job.max_attempts)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(IdempotentJobClaim::Inserted { run_id: job.run_id })
    }

    pub async fn get_idempotency_identity(
        &self,
        tenant_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<IdempotencyJobIdentity>, sqlx::Error> {
        let row = sqlx::query_as::<_, (Uuid, String, String, String)>(
            "SELECT run_id, service_code, api_key_id, idempotency_payload_hash \
             FROM llm_jobs WHERE tenant_id = $1 AND idempotency_key = $2",
        )
        .bind(tenant_id)
        .bind(idempotency_key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(
            |(run_id, service_code, api_key_id, idempotency_payload_hash)| IdempotencyJobIdentity {
                run_id,
                service_code,
                api_key_id,
                idempotency_payload_hash,
            },
        ))
    }

    pub async fn get_job_by_run_id(
        &self,
        tenant_id: &str,
        run_id: Uuid,
    ) -> Result<Option<JobRecord>, sqlx::Error> {
        let row = sqlx::query_as::<_, JobRow>(
            r#"
            SELECT job_id, run_id, service_code, tenant_id, api_key_id, user_id,
                   idempotency_key, idempotency_payload_hash, request_payload_hash,
                   status, request_json, result_json, error_json, generation_run_id,
                   attempt_count, max_attempts, submitted_at, started_at, completed_at, expires_at
            FROM llm_jobs
            WHERE tenant_id = $1 AND run_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.and_then(|r| r.into_record()))
    }

    async fn fetch_job_by_run_id_tx(
        &self,
        run_id: Uuid,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<JobRecord, sqlx::Error> {
        let row = sqlx::query_as::<_, JobRow>(
            r#"
            SELECT job_id, run_id, service_code, tenant_id, api_key_id, user_id,
                   idempotency_key, idempotency_payload_hash, request_payload_hash,
                   status, request_json, result_json, error_json, generation_run_id,
                   attempt_count, max_attempts, submitted_at, started_at, completed_at, expires_at
            FROM llm_jobs
            WHERE run_id = $1
            "#,
        )
        .bind(run_id)
        .fetch_one(&mut **tx)
        .await?;
        row.into_record().ok_or_else(|| sqlx::Error::RowNotFound)
    }

    pub async fn claim_next_queued_job(
        &self,
        worker_id: &str,
        stale_running_secs: i64,
    ) -> Result<Option<JobRecord>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query_as::<_, JobRow>(
            r#"
            SELECT job_id, run_id, service_code, tenant_id, api_key_id, user_id,
                   idempotency_key, idempotency_payload_hash, request_payload_hash,
                   status, request_json, result_json, error_json, generation_run_id,
                   attempt_count, max_attempts, submitted_at, started_at, completed_at, expires_at
            FROM llm_jobs
            WHERE status = 'queued'
              AND (retry_after IS NULL OR retry_after <= NOW())
            ORDER BY submitted_at ASC
            FOR UPDATE SKIP LOCKED
            LIMIT 1
            "#,
        )
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            tx.commit().await?;
            return Ok(None);
        };

        let Some(mut record) = row.into_record() else {
            tx.commit().await?;
            return Ok(None);
        };

        let stale_after = Utc::now() + Duration::seconds(stale_running_secs.max(60));
        sqlx::query(
            r#"
            UPDATE llm_jobs SET
                status = 'running',
                started_at = COALESCE(started_at, NOW()),
                heartbeat_at = NOW(),
                stale_after = $2,
                attempt_count = attempt_count + 1,
                last_error_json = NULL
            WHERE job_id = $1
            "#,
        )
        .bind(record.job_id)
        .bind(stale_after)
        .execute(&mut *tx)
        .await?;

        record.status = JobStatus::Running;
        record.started_at = record.started_at.or_else(|| Some(Utc::now()));
        record.attempt_count += 1;
        let _ = worker_id;

        tx.commit().await?;
        Ok(Some(record))
    }

    pub async fn touch_heartbeat(
        &self,
        job_id: Uuid,
        stale_running_secs: i64,
    ) -> Result<(), sqlx::Error> {
        let stale_after = Utc::now() + Duration::seconds(stale_running_secs.max(60));
        sqlx::query(
            "UPDATE llm_jobs SET heartbeat_at = NOW(), stale_after = $2 WHERE job_id = $1 AND status = 'running'",
        )
        .bind(job_id)
        .bind(stale_after)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn recover_stale_running_jobs(&self) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE llm_jobs SET
                status = 'queued',
                retry_after = NOW() + INTERVAL '5 seconds',
                last_error_json = COALESCE(last_error_json, '{"code":"STALE_RECOVERY","message":"job re-queued after stale running timeout"}'::jsonb)
            WHERE status = 'running'
              AND stale_after IS NOT NULL
              AND stale_after < NOW()
              AND attempt_count < max_attempts
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn purge_expired_terminal_jobs(&self) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM llm_jobs
            WHERE expires_at IS NOT NULL
              AND expires_at < NOW()
              AND status IN (
                  'completed',
                  'failed',
                  'safety_rejected',
                  'cancelled',
                  'expired'
              )
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn mark_completed(
        &self,
        job_id: Uuid,
        generation_run_id: Option<Uuid>,
        result_json: &serde_json::Value,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE llm_jobs SET
                status = 'completed',
                completed_at = NOW(),
                generation_run_id = CASE
                    WHEN $2::uuid IS NOT NULL
                     AND EXISTS (SELECT 1 FROM llm_generation_runs WHERE id = $2)
                    THEN $2
                    ELSE NULL
                END,
                result_json = $3,
                error_json = NULL
            WHERE job_id = $1
            "#,
        )
        .bind(job_id)
        .bind(generation_run_id)
        .bind(result_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_failed(
        &self,
        job_id: Uuid,
        error_json: &serde_json::Value,
        retry: bool,
    ) -> Result<(), sqlx::Error> {
        if retry {
            sqlx::query(
                r#"
                UPDATE llm_jobs SET
                    status = 'queued',
                    retry_after = NOW() + INTERVAL '10 seconds',
                    last_error_json = $2,
                    completed_at = NULL
                WHERE job_id = $1 AND attempt_count < max_attempts
                "#,
            )
            .bind(job_id)
            .bind(error_json)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                r#"
                UPDATE llm_jobs SET
                    status = 'failed',
                    completed_at = NOW(),
                    error_json = $2,
                    last_error_json = $2
                WHERE job_id = $1
                "#,
            )
            .bind(job_id)
            .bind(error_json)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn mark_safety_rejected(
        &self,
        job_id: Uuid,
        generation_run_id: Option<Uuid>,
        result_json: &serde_json::Value,
        error_json: &serde_json::Value,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE llm_jobs SET
                status = 'safety_rejected',
                completed_at = NOW(),
                generation_run_id = CASE
                    WHEN $2::uuid IS NOT NULL
                     AND EXISTS (SELECT 1 FROM llm_generation_runs WHERE id = $2)
                    THEN $2
                    ELSE NULL
                END,
                result_json = $3,
                error_json = $4,
                last_error_json = $4
            WHERE job_id = $1
            "#,
        )
        .bind(job_id)
        .bind(generation_run_id)
        .bind(result_json)
        .bind(error_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct JobRow {
    job_id: Uuid,
    run_id: Uuid,
    service_code: String,
    tenant_id: String,
    api_key_id: String,
    user_id: Option<String>,
    idempotency_key: String,
    idempotency_payload_hash: String,
    request_payload_hash: String,
    status: String,
    request_json: serde_json::Value,
    result_json: Option<serde_json::Value>,
    error_json: Option<serde_json::Value>,
    generation_run_id: Option<Uuid>,
    attempt_count: i32,
    max_attempts: i32,
    submitted_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    expires_at: Option<DateTime<Utc>>,
}

impl JobRow {
    fn into_record(self) -> Option<JobRecord> {
        let status = JobStatus::parse(&self.status)?;
        Some(JobRecord {
            job_id: self.job_id,
            run_id: self.run_id,
            service_code: self.service_code,
            tenant_id: self.tenant_id,
            api_key_id: self.api_key_id,
            user_id: self.user_id,
            idempotency_key: self.idempotency_key,
            idempotency_payload_hash: self.idempotency_payload_hash,
            request_payload_hash: self.request_payload_hash,
            status,
            request_json: self.request_json,
            result_json: self.result_json,
            error_json: self.error_json,
            generation_run_id: self.generation_run_id,
            attempt_count: self.attempt_count,
            max_attempts: self.max_attempts,
            submitted_at: self.submitted_at,
            started_at: self.started_at,
            completed_at: self.completed_at,
            expires_at: self.expires_at,
        })
    }
}
