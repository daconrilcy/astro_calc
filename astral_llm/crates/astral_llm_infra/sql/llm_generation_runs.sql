CREATE TABLE IF NOT EXISTS llm_generation_runs (
    id UUID PRIMARY KEY,
    request_id TEXT,
    product_code TEXT NOT NULL,
    astro_contract_version TEXT NOT NULL,
    output_schema_version TEXT NOT NULL,
    prompt_family TEXT NOT NULL,
    prompt_version TEXT NOT NULL,
    provider_requested TEXT NOT NULL,
    provider_used TEXT,
    model_requested TEXT NOT NULL,
    model_used TEXT,
    status TEXT NOT NULL,
    safety_status TEXT NOT NULL,
    input_hash TEXT NOT NULL,
    output_hash TEXT,
    token_input INTEGER,
    token_output INTEGER,
    latency_ms INTEGER,
    error_code TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_llm_generation_runs_created_at
    ON llm_generation_runs (created_at DESC);

CREATE INDEX IF NOT EXISTS idx_llm_generation_runs_product_code
    ON llm_generation_runs (product_code);

CREATE TABLE IF NOT EXISTS llm_generation_payloads (
    run_id UUID PRIMARY KEY REFERENCES llm_generation_runs(id) ON DELETE CASCADE,
    sanitized_request_json JSONB,
    sanitized_response_json JSONB,
    prompt_hash TEXT,
    astro_facts_hash TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE llm_generation_payloads
    ADD COLUMN IF NOT EXISTS sanitized_request_json JSONB;
ALTER TABLE llm_generation_payloads
    ADD COLUMN IF NOT EXISTS sanitized_response_json JSONB;
ALTER TABLE llm_generation_payloads
    ADD COLUMN IF NOT EXISTS prompt_hash TEXT;
ALTER TABLE llm_generation_payloads
    ADD COLUMN IF NOT EXISTS astro_facts_hash TEXT;
ALTER TABLE llm_generation_payloads
    ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

CREATE TABLE IF NOT EXISTS llm_generation_prompt_traces (
    id BIGSERIAL PRIMARY KEY,
    run_id UUID NOT NULL REFERENCES llm_generation_runs(id) ON DELETE CASCADE,
    chapter_code TEXT,
    step_type TEXT,
    attempt TEXT,
    prompt_family TEXT,
    prompt_version TEXT,
    message_count INTEGER NOT NULL,
    compiled_prompt TEXT NOT NULL,
    messages_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_llm_generation_prompt_traces_run_created
    ON llm_generation_prompt_traces (run_id, created_at ASC, id ASC);
