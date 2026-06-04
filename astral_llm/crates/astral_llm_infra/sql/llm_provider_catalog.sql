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
ALTER TABLE llm_provider_models ADD COLUMN IF NOT EXISTS catalog_notes TEXT;
ALTER TABLE llm_provider_models ADD COLUMN IF NOT EXISTS usage_tier_code TEXT REFERENCES llm_model_usage_tiers(tier_code);
ALTER TABLE llm_provider_models ADD COLUMN IF NOT EXISTS supports_temperature BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE llm_provider_models ADD COLUMN IF NOT EXISTS reasoning_output_reserve_min INTEGER;
ALTER TABLE llm_provider_models ADD COLUMN IF NOT EXISTS reasoning_effort_subtask TEXT;
ALTER TABLE llm_provider_models ADD COLUMN IF NOT EXISTS reasoning_effort_primary TEXT;
ALTER TABLE llm_provider_models ADD COLUMN IF NOT EXISTS reasoning_effort_oracle TEXT;

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
  AND model IN ('gpt-5.4', 'gpt-5.4-mini', 'gpt-5.4-nano', 'gpt-5.5', 'gpt-5.5-pro', 'gpt-5.1');

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
    ('gpt-5.4-mini', 'mini frontier — candidat production probable', 'production_candidate', true, true, 400000, 128000),
    ('gpt-5.4-nano', 'nano — summary, validation, repair, Basic high-volume', 'subtask_candidate', true, true, 400000, 128000),
    ('gpt-5.1', 'reasoning precedent — comparer si gpt-5.4 instable', 'benchmark_compare', true, true, 1050000, 128000),
    ('gpt-5-mini', 'low-cost production — Basic et sous-taches', 'production_candidate', true, true, 400000, 128000),
    ('gpt-5-nano', 'tres rapide — summarization, classification', 'subtask_candidate', true, true, 400000, 128000),
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
    'gpt-5.5', 'gpt-5.5-pro', 'gpt-5.4', 'gpt-5.4-mini', 'gpt-5.4-nano',
    'gpt-5.1', 'gpt-5-mini', 'gpt-5-nano', 'gpt-4.1', 'gpt-4.1-mini'
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
        'natal_premium', 'openai', 'gpt-5.4-mini', 'gpt-5-nano', 'gpt-5.4', 'gpt-5.5',
        'chapitres=gpt-5.4-mini ; summary=gpt-5-nano (economic_model) ; fallback gpt-4.1'
    ),
    ('natal_basic', 'openai', 'gpt-5.4-mini', 'gpt-5-mini', NULL, NULL, 'production Basic par defaut')
ON CONFLICT (product_code) DO UPDATE SET
    default_provider = EXCLUDED.default_provider,
    default_model = EXCLUDED.default_model,
    economic_model = EXCLUDED.economic_model,
    high_quality_model = EXCLUDED.high_quality_model,
    oracle_model = EXCLUDED.oracle_model,
    notes = EXCLUDED.notes,
    is_active = EXCLUDED.is_active,
    updated_at = NOW();

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
    ('natal_premium', 'openai', 'gpt-4.1', 10, 'baseline fallback Premium')
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

-- natal_premium : modeles autorises pour benchmark E2E Premium (provider = code catalogue openai)
INSERT INTO llm_product_allowed_models (product_code, provider, model, notes) VALUES
    ('natal_premium', 'openai', 'gpt-4.1', 'baseline'),
    ('natal_premium', 'openai', 'gpt-5-mini', 'economique / summary'),
    ('natal_premium', 'openai', 'gpt-5-nano', 'summary ultra-low-cost'),
    ('natal_premium', 'openai', 'gpt-5.4-mini', 'production probable'),
    ('natal_premium', 'openai', 'gpt-5.4', 'qualite/prix'),
    ('natal_premium', 'openai', 'gpt-5.5', 'qualite max raisonnable'),
    ('natal_premium', 'openai', 'gpt-5.5-pro', 'oracle benchmark explicite'),
    ('natal_basic', 'openai', 'gpt-4.1', NULL),
    ('natal_basic', 'openai', 'gpt-5-mini', NULL),
    ('natal_basic', 'openai', 'gpt-5.4-mini', NULL),
    ('natal_basic', 'fake', 'fake-model', NULL)
ON CONFLICT (product_code, provider, model) DO UPDATE SET
    is_active = EXCLUDED.is_active,
    notes = EXCLUDED.notes;

INSERT INTO llm_generation_benchmark_usage_models (usage_code, provider, model, priority, notes) VALUES
    ('premium_chapter_orchestrated', 'openai', 'gpt-5.4-mini', 10, 'production Premium par defaut'),
    ('premium_chapter_orchestrated', 'openai', 'gpt-4.1', 20, 'baseline historique'),
    ('premium_chapter_orchestrated', 'openai', 'gpt-5.4', 30, 'qualite/prix Premium'),
    ('premium_chapter_orchestrated', 'openai', 'gpt-5.5', 40, 'qualite maximale raisonnable'),
    ('premium_high_end', 'openai', 'gpt-5.5', 10, NULL),
    ('premium_high_end', 'openai', 'gpt-5.5-pro', 20, 'oracle — runs ponctuels'),
    ('basic_short_reading', 'openai', 'gpt-5-mini', 10, NULL),
    ('basic_short_reading', 'openai', 'gpt-5.4-mini', 20, NULL),
    ('basic_short_reading', 'openai', 'gpt-4.1-mini', 30, NULL),
    ('summary_synthesizer', 'openai', 'gpt-5-nano', 10, 'Premium summary prod'),
    ('summary_synthesizer', 'openai', 'gpt-5-mini', 20, NULL),
    ('summary_synthesizer', 'openai', 'gpt-5.4-nano', 30, NULL),
    ('repair_validation_reformulation', 'openai', 'gpt-5-nano', 10, NULL),
    ('repair_validation_reformulation', 'openai', 'gpt-5.4-nano', 20, NULL),
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
    'gpt-5.4', 'gpt-5.4-mini', 'gpt-5.4-nano', 'gpt-5.1'
  );

-- gpt-5.4 / gpt-5.4-mini : l'API OpenAI rejette temperature (400) — seuls gpt-4.1* la supportent.
UPDATE llm_provider_models
SET supports_temperature = true
WHERE provider = 'openai'
  AND model IN ('gpt-4.1', 'gpt-4.1-mini');
