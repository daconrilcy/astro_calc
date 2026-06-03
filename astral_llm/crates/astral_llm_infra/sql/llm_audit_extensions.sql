-- Extensions audit / idempotence (applique avec ensure_schema en local)

CREATE EXTENSION IF NOT EXISTS pgcrypto;

ALTER TABLE llm_generation_runs ADD COLUMN IF NOT EXISTS idempotency_key TEXT;
ALTER TABLE llm_generation_runs ADD COLUMN IF NOT EXISTS user_language TEXT;
ALTER TABLE llm_generation_runs ADD COLUMN IF NOT EXISTS generation_mode TEXT;
ALTER TABLE llm_generation_runs ADD COLUMN IF NOT EXISTS fallback_used BOOLEAN DEFAULT false;
ALTER TABLE llm_generation_runs ADD COLUMN IF NOT EXISTS safety_policy_version TEXT;
ALTER TABLE llm_generation_runs ADD COLUMN IF NOT EXISTS selected_domains JSONB;

CREATE INDEX IF NOT EXISTS idx_llm_generation_runs_idempotency
    ON llm_generation_runs (idempotency_key)
    WHERE idempotency_key IS NOT NULL;

CREATE TABLE IF NOT EXISTS llm_generation_steps (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID NOT NULL REFERENCES llm_generation_runs(id) ON DELETE CASCADE,
    step_type TEXT NOT NULL,
    chapter_code TEXT,
    provider TEXT,
    model TEXT,
    status TEXT NOT NULL,
    input_tokens INTEGER,
    output_tokens INTEGER,
    latency_ms INTEGER,
    error_code TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_llm_generation_steps_run_id
    ON llm_generation_steps (run_id);

CREATE TABLE IF NOT EXISTS llm_idempotency_records (
    idempotency_key TEXT NOT NULL,
    product_code TEXT NOT NULL,
    run_id UUID NOT NULL,
    input_hash TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL,
    response_json JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (idempotency_key, product_code)
);

ALTER TABLE llm_idempotency_records ADD COLUMN IF NOT EXISTS input_hash TEXT NOT NULL DEFAULT '';

CREATE INDEX IF NOT EXISTS idx_llm_idempotency_expires
    ON llm_idempotency_records (expires_at);
