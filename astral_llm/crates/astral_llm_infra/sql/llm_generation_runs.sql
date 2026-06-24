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

CREATE TABLE IF NOT EXISTS llm_natal_fact_explanations (
    id BIGSERIAL PRIMARY KEY,
    language TEXT NOT NULL,
    kind_code TEXT NOT NULL,
    key_hash TEXT NOT NULL,
    key_json JSONB NOT NULL,
    title TEXT NOT NULL,
    explanation TEXT NOT NULL,
    expression_primary TEXT,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    prompt_version TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_llm_natal_fact_explanations_language_hash UNIQUE (language, key_hash)
);

CREATE INDEX IF NOT EXISTS idx_llm_natal_fact_explanations_kind
    ON llm_natal_fact_explanations (language, kind_code);

CREATE TABLE IF NOT EXISTS llm_natal_explanation_facts (
    id BIGSERIAL PRIMARY KEY,
    kind_code TEXT NOT NULL,
    key_hash TEXT NOT NULL,
    key_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_llm_natal_explanation_facts_hash UNIQUE (key_hash)
);

CREATE INDEX IF NOT EXISTS idx_llm_natal_explanation_facts_kind
    ON llm_natal_explanation_facts (kind_code);

CREATE TABLE IF NOT EXISTS llm_natal_explanation_translations (
    id BIGSERIAL PRIMARY KEY,
    fact_id BIGINT NOT NULL REFERENCES llm_natal_explanation_facts(id) ON DELETE CASCADE,
    language_code TEXT NOT NULL,
    title TEXT NOT NULL,
    explanation TEXT NOT NULL,
    expression_primary TEXT,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    prompt_version TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_llm_natal_explanation_translations_language
        CHECK (language_code IN ('fr', 'en', 'es', 'de')),
    CONSTRAINT uq_llm_natal_explanation_translations_fact_language
        UNIQUE (fact_id, language_code)
);

CREATE INDEX IF NOT EXISTS idx_llm_natal_explanation_translations_language
    ON llm_natal_explanation_translations (language_code);

INSERT INTO llm_natal_explanation_facts (
    kind_code, key_hash, key_json, created_at, updated_at
)
SELECT DISTINCT ON (key_hash)
    kind_code, key_hash, key_json, created_at, updated_at
FROM llm_natal_fact_explanations
WHERE language IN ('fr', 'en', 'es', 'de')
ORDER BY key_hash, updated_at DESC
ON CONFLICT (key_hash) DO NOTHING;

INSERT INTO llm_natal_explanation_translations (
    fact_id, language_code, title, explanation, expression_primary,
    provider, model, prompt_version, created_at, updated_at
)
SELECT
    facts.id,
    legacy.language,
    legacy.title,
    legacy.explanation,
    legacy.expression_primary,
    legacy.provider,
    legacy.model,
    legacy.prompt_version,
    legacy.created_at,
    legacy.updated_at
FROM llm_natal_fact_explanations legacy
JOIN llm_natal_explanation_facts facts ON facts.key_hash = legacy.key_hash
WHERE legacy.language IN ('fr', 'en', 'es', 'de')
ON CONFLICT (fact_id, language_code) DO NOTHING;
