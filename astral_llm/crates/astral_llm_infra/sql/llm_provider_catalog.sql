-- Catalogue moteurs (providers) et modeles LLM — source de verite modifiable en base.
-- is_active = modele chargeable, usage_tier_code = profil benchmark / validation par contexte

CREATE TABLE IF NOT EXISTS llm_providers (
    id SERIAL PRIMARY KEY,
    provider_code TEXT NOT NULL UNIQUE,
    label_fr TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS llm_model_usage_tiers (
    tier_code TEXT PRIMARY KEY,
    label_fr TEXT NOT NULL,
    allows_primary_reading BOOLEAN NOT NULL DEFAULT false,
    allows_subtask BOOLEAN NOT NULL DEFAULT false,
    allows_oracle_benchmark BOOLEAN NOT NULL DEFAULT false,
    sort_order INTEGER NOT NULL DEFAULT 0
);

ALTER TABLE llm_provider_models ADD COLUMN IF NOT EXISTS provider_id INTEGER REFERENCES llm_providers(id);
ALTER TABLE llm_provider_models ADD COLUMN IF NOT EXISTS model_code TEXT;
ALTER TABLE llm_provider_models ADD COLUMN IF NOT EXISTS display_name TEXT;
ALTER TABLE llm_provider_models ADD COLUMN IF NOT EXISTS api_model_id TEXT;
ALTER TABLE llm_provider_models ADD COLUMN IF NOT EXISTS catalog_notes TEXT;
ALTER TABLE llm_provider_models ADD COLUMN IF NOT EXISTS usage_tier_code TEXT REFERENCES llm_model_usage_tiers(tier_code);
ALTER TABLE llm_provider_models ADD COLUMN IF NOT EXISTS supports_temperature BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE llm_provider_models ADD COLUMN IF NOT EXISTS reasoning_output_reserve_min INTEGER;
ALTER TABLE llm_provider_models ADD COLUMN IF NOT EXISTS reasoning_effort_subtask TEXT;
ALTER TABLE llm_provider_models ADD COLUMN IF NOT EXISTS reasoning_effort_primary TEXT;
ALTER TABLE llm_provider_models ADD COLUMN IF NOT EXISTS reasoning_effort_oracle TEXT;

UPDATE llm_provider_models
SET model_code = COALESCE(model_code, model),
    display_name = COALESCE(display_name, model),
    api_model_id = COALESCE(api_model_id, model);

CREATE TABLE IF NOT EXISTS llm_model_characteristics (
    id SERIAL PRIMARY KEY,
    model_id INTEGER NOT NULL REFERENCES llm_provider_models(id) ON DELETE CASCADE,
    max_context_tokens INTEGER,
    max_output_tokens INTEGER,
    supports_reasoning BOOLEAN NOT NULL DEFAULT false,
    supports_temperature BOOLEAN NOT NULL DEFAULT true,
    supports_streaming BOOLEAN NOT NULL DEFAULT false,
    supports_json_schema_strict BOOLEAN NOT NULL DEFAULT false,
    supports_json_object BOOLEAN NOT NULL DEFAULT false,
    structured_output_adapter TEXT,
    storage_disable_supported BOOLEAN NOT NULL DEFAULT false,
    input_price_usd_per_mtok DOUBLE PRECISION,
    output_price_usd_per_mtok DOUBLE PRECISION,
    cache_read_price_usd_per_mtok DOUBLE PRECISION,
    cache_write_price_usd_per_mtok DOUBLE PRECISION,
    reasoning_price_usd_per_mtok DOUBLE PRECISION,
    pricing_currency TEXT NOT NULL DEFAULT 'USD',
    source_kind TEXT NOT NULL DEFAULT 'seed_sql',
    source_ref TEXT,
    observed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_current BOOLEAN NOT NULL DEFAULT true
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_llm_model_characteristics_current
    ON llm_model_characteristics (model_id)
    WHERE is_current = true;

INSERT INTO llm_model_characteristics (
    model_id, max_context_tokens, max_output_tokens, supports_reasoning,
    supports_temperature, supports_streaming, supports_json_schema_strict,
    supports_json_object, structured_output_adapter, storage_disable_supported,
    pricing_currency, source_kind, source_ref, is_current
)
SELECT
    m.id, m.max_input_tokens, m.max_output_tokens, m.supports_reasoning_effort,
    COALESCE(m.supports_temperature, true), m.supports_streaming, m.supports_json_schema_strict,
    m.supports_json_object, m.structured_output_adapter, m.storage_disable_supported,
    'USD', 'backfill_llm_provider_models', 'llm_provider_catalog.sql', true
FROM llm_provider_models AS m
WHERE NOT EXISTS (
    SELECT 1
    FROM llm_model_characteristics AS c
    WHERE c.model_id = m.id
      AND c.is_current = true
);

-- Tokens reserves pour le raisonnement interne (Responses API GPT-5) avant le message assistant.
UPDATE llm_provider_models
SET reasoning_output_reserve_min = 4096
WHERE supports_reasoning_effort = true
  AND (reasoning_output_reserve_min IS NULL OR reasoning_output_reserve_min < 1);

UPDATE llm_provider_models
SET reasoning_output_reserve_min = NULL
WHERE NOT supports_reasoning_effort;

-- Efforts reasoning par contexte (litteraux API OpenAI — varient selon le modele).
UPDATE llm_provider_models
SET reasoning_effort_subtask = 'minimal',
    reasoning_effort_primary = 'low',
    reasoning_effort_oracle = 'medium'
WHERE provider = 'openai' AND model IN ('gpt-5-mini', 'gpt-5-nano');

UPDATE llm_provider_models
SET reasoning_effort_subtask = 'none',
    reasoning_effort_primary = 'low',
    reasoning_effort_oracle = 'medium'
WHERE provider = 'openai'
  AND model IN ('gpt-5.4', 'gpt-5-mini', 'gpt-5-nano', 'gpt-5.5', 'gpt-5.5-pro', 'gpt-5.1');

CREATE TABLE IF NOT EXISTS llm_generation_benchmark_usages (
    usage_code TEXT PRIMARY KEY,
    label_fr TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true
);

CREATE TABLE IF NOT EXISTS llm_generation_benchmark_usage_models (
    id SERIAL PRIMARY KEY,
    usage_code TEXT NOT NULL REFERENCES llm_generation_benchmark_usages(usage_code) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0,
    notes TEXT,
    UNIQUE (usage_code, provider, model)
);

INSERT INTO llm_model_usage_tiers (
    tier_code, label_fr, allows_primary_reading, allows_subtask, allows_oracle_benchmark, sort_order
) VALUES
    ('production_candidate', 'Candidat production Premium/Basic', true, true, false, 10),
    ('baseline', 'Baseline actuelle (gpt-4.1)', true, true, false, 20),
    ('subtask_candidate', 'Sous-taches : resume, repair, validation', false, true, false, 30),
    ('benchmark_compare', 'Comparaison reasoning (gpt-5.1)', true, true, false, 40),
    ('oracle_only', 'Oracle qualite — benchmark ponctuel uniquement', false, false, true, 50)
ON CONFLICT (tier_code) DO UPDATE SET
    label_fr = EXCLUDED.label_fr,
    allows_primary_reading = EXCLUDED.allows_primary_reading,
    allows_subtask = EXCLUDED.allows_subtask,
    allows_oracle_benchmark = EXCLUDED.allows_oracle_benchmark,
    sort_order = EXCLUDED.sort_order;

INSERT INTO llm_generation_benchmark_usages (usage_code, label_fr, sort_order) VALUES
    ('premium_chapter_orchestrated', 'Premium chapter_orchestrated', 10),
    ('premium_high_end', 'Premium haut de gamme', 20),
    ('basic_short_reading', 'Basic / lecture courte', 30),
    ('summary_synthesizer', 'SummarySynthesizer', 40),
    ('repair_validation_reformulation', 'Repair / validation / reformulation', 50),
    ('oracle_quality', 'Oracle qualite', 60)
ON CONFLICT (usage_code) DO NOTHING;

-- Rattachement des lignes existantes (provider TEXT legacy).
UPDATE llm_provider_models AS m
SET provider_id = p.id
FROM llm_providers AS p
WHERE m.provider_id IS NULL AND p.provider_code = m.provider;

INSERT INTO llm_providers (provider_code, label_fr, sort_order, is_active) VALUES
    ('fake', 'Fournisseur de test', 0, true),
    ('openai', 'OpenAI', 10, true),
    ('anthropic', 'Anthropic', 20, true),
    ('mistral', 'Mistral', 30, true)
ON CONFLICT (provider_code) DO NOTHING;

UPDATE llm_provider_models AS m
SET provider_id = p.id
FROM llm_providers AS p
WHERE m.provider_id IS NULL AND p.provider_code = m.provider;

-- OpenAI : vague 1 production + vague 2 exploration (tous is_active=true).
INSERT INTO llm_provider_models (
    provider, provider_id, model, catalog_notes, usage_tier_code,
    supports_json_schema_strict, supports_json_object, supports_reasoning_effort,
    supports_streaming, max_input_tokens, max_output_tokens,
    structured_output_adapter, storage_disable_supported, is_active
)
SELECT
    'openai', p.id, v.model, v.notes, v.tier,
    true, true, v.reasoning, v.streaming,
    v.max_in, v.max_out, 'openai_responses_text_format', true, true
FROM llm_providers p
CROSS JOIN (VALUES
    ('gpt-5.5', 'frontier actuel — qualite Premium maximale', 'production_candidate', true, true, 1050000, 128000),
    ('gpt-5.5-pro', 'oracle qualite — lent, sans streaming, benchmark ponctuel', 'oracle_only', true, false, 1050000, 128000),
    ('gpt-5.4', 'frontier abordable — Premium reasoning', 'production_candidate', true, true, 1050000, 128000),
    ('gpt-5-mini', 'mini frontier — candidat production probable', 'production_candidate', true, true, 400000, 128000),
    ('gpt-5-nano', 'nano — summary, validation, repair, Basic high-volume', 'subtask_candidate', true, true, 400000, 128000),
    ('gpt-5.1', 'reasoning precedent — comparer si gpt-5.4 instable', 'benchmark_compare', true, true, 1050000, 128000),
    ('gpt-4.1', 'baseline actuelle — non-reasoning fort', 'baseline', false, true, 1000000, 32000),
    ('gpt-4.1-mini', 'comparaison cout/latence vs gpt-5-mini', 'subtask_candidate', false, true, 1000000, 32000)
) AS v(model, notes, tier, reasoning, streaming, max_in, max_out)
WHERE p.provider_code = 'openai'
ON CONFLICT (provider, model) DO UPDATE SET
    provider_id = EXCLUDED.provider_id,
    catalog_notes = EXCLUDED.catalog_notes,
    usage_tier_code = EXCLUDED.usage_tier_code,
    supports_json_schema_strict = EXCLUDED.supports_json_schema_strict,
    supports_json_object = EXCLUDED.supports_json_object,
    supports_reasoning_effort = EXCLUDED.supports_reasoning_effort,
    supports_streaming = EXCLUDED.supports_streaming,
    max_input_tokens = EXCLUDED.max_input_tokens,
    max_output_tokens = EXCLUDED.max_output_tokens,
    structured_output_adapter = EXCLUDED.structured_output_adapter,
    storage_disable_supported = EXCLUDED.storage_disable_supported,
    is_active = EXCLUDED.is_active,
    updated_at = NOW();

UPDATE llm_provider_models
SET is_active = false, updated_at = NOW()
WHERE provider = 'openai'
  AND model NOT IN (
    'gpt-5.5', 'gpt-5.5-pro', 'gpt-5.4', 'gpt-5-mini', 'gpt-5-nano',
    'gpt-5.1', 'gpt-4.1', 'gpt-4.1-mini'
  );

-- Moteur par produit (bootstrap). Pour changer les modeles au quotidien :
--   1) config/llm_product_models.conf  2) .\scripts\set_product_llm_models.ps1  3) redemarrer astral_llm_api
CREATE TABLE IF NOT EXISTS llm_product_default_engine (
    product_code TEXT PRIMARY KEY,
    default_provider TEXT NOT NULL,
    default_model TEXT NOT NULL,
    economic_model TEXT,
    high_quality_model TEXT,
    oracle_model TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    notes TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE llm_product_default_engine ADD COLUMN IF NOT EXISTS economic_model TEXT;
ALTER TABLE llm_product_default_engine ADD COLUMN IF NOT EXISTS high_quality_model TEXT;
ALTER TABLE llm_product_default_engine ADD COLUMN IF NOT EXISTS oracle_model TEXT;

INSERT INTO llm_product_default_engine (
    product_code, default_provider, default_model, economic_model, high_quality_model, oracle_model, notes
) VALUES
    (
        'natal_prompter', 'openai', 'gpt-5-mini', 'gpt-5-nano', 'gpt-5.4', 'gpt-5.5',
        'moteur natal_prompter ; modeles par profil JSON ; override rapide via conf'
    )
ON CONFLICT (product_code) DO UPDATE SET
    default_provider = EXCLUDED.default_provider,
    default_model = EXCLUDED.default_model,
    economic_model = EXCLUDED.economic_model,
    high_quality_model = EXCLUDED.high_quality_model,
    oracle_model = EXCLUDED.oracle_model,
    notes = EXCLUDED.notes,
    is_active = EXCLUDED.is_active,
    updated_at = NOW();

-- Legacy (historique) : ces "produits" n'existent plus en runtime (migre vers natal_prompter + interpretation_profile_code).
-- On les desactive pour eviter qu'ils apparaissent comme des moteurs paralleles dans les vues ops (ex. set_product_llm_models.ps1 -Show).
UPDATE llm_product_default_engine
SET is_active = false, updated_at = NOW()
WHERE product_code IN ('natal_basic', 'natal_premium');

CREATE TABLE IF NOT EXISTS llm_product_fallback_models (
    id SERIAL PRIMARY KEY,
    product_code TEXT NOT NULL,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 10,
    is_active BOOLEAN NOT NULL DEFAULT true,
    notes TEXT,
    UNIQUE (product_code, provider, model)
);

INSERT INTO llm_product_fallback_models (product_code, provider, model, priority, notes) VALUES
    ('natal_prompter', 'openai', 'gpt-4.1', 10, 'baseline fallback natal_prompter')
ON CONFLICT (product_code, provider, model) DO UPDATE SET
    priority = EXCLUDED.priority,
    notes = EXCLUDED.notes,
    is_active = EXCLUDED.is_active;

CREATE TABLE IF NOT EXISTS llm_product_allowed_models (
    id SERIAL PRIMARY KEY,
    product_code TEXT NOT NULL,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    notes TEXT,
    UNIQUE (product_code, provider, model)
);

-- natal_prompter : modeles autorises (provider = code catalogue openai / fake)
INSERT INTO llm_product_allowed_models (product_code, provider, model, notes) VALUES
    ('natal_prompter', 'openai', 'gpt-4.1', 'baseline'),
    ('natal_prompter', 'openai', 'gpt-5-nano', 'summary ultra-low-cost'),
    ('natal_prompter', 'openai', 'gpt-5-mini', 'production probable'),
    ('natal_prompter', 'openai', 'gpt-5.4', 'qualite/prix'),
    ('natal_prompter', 'openai', 'gpt-5.5', 'qualite max raisonnable'),
    ('natal_prompter', 'openai', 'gpt-5.5-pro', 'oracle benchmark explicite'),
    ('natal_prompter', 'fake', 'fake-model', 'tests integration')
ON CONFLICT (product_code, provider, model) DO UPDATE SET
    is_active = EXCLUDED.is_active,
    notes = EXCLUDED.notes;

UPDATE llm_product_allowed_models SET is_active = false
WHERE product_code IN ('natal_basic', 'natal_premium');

INSERT INTO llm_generation_benchmark_usage_models (usage_code, provider, model, priority, notes) VALUES
    ('premium_chapter_orchestrated', 'openai', 'gpt-5-mini', 10, 'production Premium par defaut'),
    ('premium_chapter_orchestrated', 'openai', 'gpt-4.1', 20, 'baseline historique'),
    ('premium_chapter_orchestrated', 'openai', 'gpt-5.4', 30, 'qualite/prix Premium'),
    ('premium_chapter_orchestrated', 'openai', 'gpt-5.5', 40, 'qualite maximale raisonnable'),
    ('premium_high_end', 'openai', 'gpt-5.5', 10, NULL),
    ('premium_high_end', 'openai', 'gpt-5.5-pro', 20, 'oracle — runs ponctuels'),
    ('basic_short_reading', 'openai', 'gpt-5-mini', 10, NULL),
    ('basic_short_reading', 'openai', 'gpt-4.1-mini', 30, NULL),
    ('summary_synthesizer', 'openai', 'gpt-5-nano', 10, 'Premium summary prod'),
    ('summary_synthesizer', 'openai', 'gpt-5-mini', 20, NULL),
    ('repair_validation_reformulation', 'openai', 'gpt-5-nano', 10, NULL),
    ('repair_validation_reformulation', 'openai', 'gpt-5-mini', 30, NULL),
    ('oracle_quality', 'openai', 'gpt-5.5-pro', 10, 'benchmark ponctuel')
ON CONFLICT (usage_code, provider, model) DO UPDATE SET
    priority = EXCLUDED.priority,
    notes = EXCLUDED.notes;

INSERT INTO llm_provider_models (
    provider, provider_id, model, usage_tier_code,
    supports_json_schema_strict, supports_json_object, supports_reasoning_effort,
    supports_streaming, max_input_tokens, max_output_tokens,
    structured_output_adapter, storage_disable_supported, is_active
)
SELECT
    p.provider_code, p.id, v.model, 'production_candidate',
    v.strict, v.json_obj, v.reasoning, v.streaming,
    v.max_in, v.max_out, v.adapter, v.storage_off, true
FROM llm_providers p
CROSS JOIN (VALUES
    ('fake', 'fake-model', true, true, true, false, 128000, 16384, 'prompt_only', false),
    ('anthropic', 'claude-sonnet-4-20250514', true, true, false, true, 200000, 8192, 'anthropic_output_config_format', false),
    ('mistral', 'mistral-large-latest', true, true, false, true, 128000, 8192, 'mistral_response_format_json_schema', false)
) AS v(provider_code, model, strict, json_obj, reasoning, streaming, max_in, max_out, adapter, storage_off)
WHERE p.provider_code = v.provider_code
ON CONFLICT (provider, model) DO UPDATE SET
    provider_id = EXCLUDED.provider_id,
    usage_tier_code = COALESCE(llm_provider_models.usage_tier_code, EXCLUDED.usage_tier_code),
    supports_json_schema_strict = EXCLUDED.supports_json_schema_strict,
    supports_json_object = EXCLUDED.supports_json_object,
    supports_reasoning_effort = EXCLUDED.supports_reasoning_effort,
    supports_streaming = EXCLUDED.supports_streaming,
    max_input_tokens = EXCLUDED.max_input_tokens,
    max_output_tokens = EXCLUDED.max_output_tokens,
    structured_output_adapter = EXCLUDED.structured_output_adapter,
    storage_disable_supported = EXCLUDED.storage_disable_supported,
    is_active = EXCLUDED.is_active,
    updated_at = NOW();

UPDATE llm_provider_models
SET supports_reasoning_effort = false
WHERE provider = 'openai' AND model IN ('gpt-4.1', 'gpt-4.1-mini');

UPDATE llm_provider_models
SET supports_temperature = false
WHERE provider = 'openai'
  AND model IN (
    'gpt-5-mini', 'gpt-5-nano', 'gpt-5.5', 'gpt-5.5-pro',
    'gpt-5.4', 'gpt-5.1'
  );

-- gpt-5.4 / gpt-5-mini : l'API OpenAI rejette temperature (400) — seuls gpt-4.1* la supportent.
UPDATE llm_provider_models
SET supports_temperature = true
WHERE provider = 'openai'
  AND model IN ('gpt-4.1', 'gpt-4.1-mini');

UPDATE llm_model_characteristics AS c
SET
    max_context_tokens = m.max_input_tokens,
    max_output_tokens = m.max_output_tokens,
    supports_reasoning = m.supports_reasoning_effort,
    supports_temperature = COALESCE(m.supports_temperature, true),
    supports_streaming = m.supports_streaming,
    supports_json_schema_strict = m.supports_json_schema_strict,
    supports_json_object = m.supports_json_object,
    structured_output_adapter = m.structured_output_adapter,
    storage_disable_supported = m.storage_disable_supported
FROM llm_provider_models AS m
WHERE c.model_id = m.id
  AND c.is_current = true;

UPDATE llm_model_characteristics AS c
SET input_price_usd_per_mtok = v.input_price,
    output_price_usd_per_mtok = v.output_price,
    cache_read_price_usd_per_mtok = v.cache_read_price,
    cache_write_price_usd_per_mtok = v.cache_write_price,
    reasoning_price_usd_per_mtok = v.reasoning_price,
    source_kind = 'official_docs_seed',
    source_ref = v.source_ref,
    observed_at = NOW()
FROM llm_provider_models AS m
JOIN (
    VALUES
        ('openai', 'gpt-5.5', 5.0::float8, 30.0::float8, 0.5::float8, NULL::float8, NULL::float8, 'https://openai.com/api/pricing/'),
        ('openai', 'gpt-5.5-pro', 30.0::float8, 180.0::float8, NULL::float8, NULL::float8, NULL::float8, 'https://developers.openai.com/api/docs/models/compare'),
        ('openai', 'gpt-5.4', 2.5::float8, 15.0::float8, 0.25::float8, NULL::float8, NULL::float8, 'https://openai.com/api/pricing/'),
        ('openai', 'gpt-5-mini', 0.75::float8, 4.5::float8, 0.075::float8, NULL::float8, NULL::float8, 'https://openai.com/api/pricing/'),
        ('openai', 'gpt-5-nano', 0.15::float8, 0.6::float8, 0.015::float8, NULL::float8, NULL::float8, 'https://developers.openai.com/api/docs/pricing'),
        ('openai', 'gpt-5.1', 5.0::float8, 20.0::float8, NULL::float8, NULL::float8, NULL::float8, 'https://developers.openai.com/api/docs/pricing'),
        ('openai', 'gpt-4.1', 2.0::float8, 8.0::float8, 0.5::float8, NULL::float8, NULL::float8, 'https://developers.openai.com/api/docs/pricing'),
        ('openai', 'gpt-4.1-mini', 0.4::float8, 1.6::float8, 0.1::float8, NULL::float8, NULL::float8, 'https://developers.openai.com/api/docs/pricing'),
        ('anthropic', 'claude-sonnet-4-20250514', 3.0::float8, 15.0::float8, 0.3::float8, 3.75::float8, NULL::float8, 'https://platform.claude.com/docs/en/about-claude/pricing'),
        ('mistral', 'mistral-large-latest', 2.0::float8, 6.0::float8, 0.2::float8, NULL::float8, NULL::float8, 'https://mistral.ai/pricing/'),
        ('fake', 'fake-model', 0.0::float8, 0.0::float8, 0.0::float8, 0.0::float8, 0.0::float8, 'seed:fake')
) AS v(provider_code, model, input_price, output_price, cache_read_price, cache_write_price, reasoning_price, source_ref)
    ON v.provider_code = m.provider
   AND v.model = m.model
WHERE c.model_id = m.id
  AND c.is_current = true;
