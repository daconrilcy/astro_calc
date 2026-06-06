-- Jobs async integration API (file d'attente worker)

CREATE TABLE IF NOT EXISTS llm_jobs (
    job_id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id                    UUID UNIQUE NOT NULL,

    service_code              TEXT NOT NULL REFERENCES llm_integration_services(service_code),

    tenant_id                 TEXT NOT NULL DEFAULT 'default',
    api_key_id                TEXT NOT NULL,
    user_id                   TEXT NULL,

    idempotency_key           TEXT NOT NULL,
    idempotency_payload_hash  TEXT NOT NULL,
    request_payload_hash      TEXT NOT NULL,

    status                    TEXT NOT NULL CHECK (status IN (
        'queued',
        'running',
        'completed',
        'failed',
        'safety_rejected',
        'cancelled',
        'expired'
    )),

    request_json              JSONB NOT NULL,
    result_json               JSONB NULL,
    error_json                JSONB NULL,
    last_error_json           JSONB NULL,

    generation_run_id         UUID NULL REFERENCES llm_generation_runs(id),

    attempt_count             INT NOT NULL DEFAULT 0,
    max_attempts              INT NOT NULL DEFAULT 3,
    retry_after               TIMESTAMPTZ NULL,

    submitted_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at                TIMESTAMPTZ NULL,
    heartbeat_at              TIMESTAMPTZ NULL,
    completed_at              TIMESTAMPTZ NULL,
    stale_after               TIMESTAMPTZ NULL,
    expires_at                TIMESTAMPTZ NULL,

    UNIQUE (tenant_id, idempotency_key)
);

CREATE INDEX IF NOT EXISTS idx_llm_jobs_worker_queue
    ON llm_jobs (status, retry_after, submitted_at)
    WHERE status = 'queued';

CREATE INDEX IF NOT EXISTS idx_llm_jobs_running_stale
    ON llm_jobs (status, stale_after)
    WHERE status = 'running';

CREATE INDEX IF NOT EXISTS idx_llm_jobs_tenant_run
    ON llm_jobs (tenant_id, run_id);
