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

CREATE TABLE IF NOT EXISTS llm_token_usage_types (
    usage_type_code TEXT PRIMARY KEY,
    label_fr TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true
);

INSERT INTO llm_token_usage_types (usage_type_code, label_fr, sort_order, is_active) VALUES
    ('input', 'Tokens entree', 10, true),
    ('output', 'Tokens sortie', 20, true),
    ('cache', 'Tokens cache', 30, true),
    ('reasoning', 'Tokens raisonnement', 40, true)
ON CONFLICT (usage_type_code) DO UPDATE SET
    label_fr = EXCLUDED.label_fr,
    sort_order = EXCLUDED.sort_order,
    is_active = EXCLUDED.is_active;

CREATE TABLE IF NOT EXISTS llm_generation_run_token_usages (
    id BIGSERIAL PRIMARY KEY,
    run_id UUID NOT NULL REFERENCES llm_generation_runs(id) ON DELETE CASCADE,
    usage_type_code TEXT NOT NULL REFERENCES llm_token_usage_types(usage_type_code),
    usage_subtype TEXT,
    token_count INTEGER NOT NULL,
    unit_price_usd_per_mtok DOUBLE PRECISION,
    estimated_cost_usd DOUBLE PRECISION,
    provider_metric_name TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_llm_generation_run_token_usages_run_id
    ON llm_generation_run_token_usages (run_id, usage_type_code);

CREATE TABLE IF NOT EXISTS llm_generation_step_token_usages (
    id BIGSERIAL PRIMARY KEY,
    step_id UUID NOT NULL REFERENCES llm_generation_steps(id) ON DELETE CASCADE,
    usage_type_code TEXT NOT NULL REFERENCES llm_token_usage_types(usage_type_code),
    usage_subtype TEXT,
    token_count INTEGER NOT NULL,
    unit_price_usd_per_mtok DOUBLE PRECISION,
    estimated_cost_usd DOUBLE PRECISION,
    provider_metric_name TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_llm_generation_step_token_usages_step_id
    ON llm_generation_step_token_usages (step_id, usage_type_code);

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
