-- Referentiels canoniques astral_llm (source de verite en base)

CREATE TABLE IF NOT EXISTS llm_astrological_domains (
    id SERIAL PRIMARY KEY,
    domain_code TEXT NOT NULL UNIQUE,
    label_fr TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true
);

CREATE TABLE IF NOT EXISTS llm_safety_content_patterns (
    id SERIAL PRIMARY KEY,
    pattern_type TEXT NOT NULL,
    locale TEXT NOT NULL DEFAULT 'fr',
    pattern TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true
);

CREATE TABLE IF NOT EXISTS llm_product_prompt_profiles (
    id SERIAL PRIMARY KEY,
    product_code TEXT NOT NULL UNIQUE,
    prompt_family TEXT NOT NULL,
    prompt_version TEXT NOT NULL DEFAULT 'v1',
    is_active BOOLEAN NOT NULL DEFAULT true
);

CREATE TABLE IF NOT EXISTS llm_service_limits (
    id SERIAL PRIMARY KEY,
    limit_code TEXT NOT NULL UNIQUE,
    limit_value INTEGER NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true
);

CREATE TABLE IF NOT EXISTS llm_provider_models (
    id SERIAL PRIMARY KEY,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    supports_json_schema_strict BOOLEAN NOT NULL DEFAULT false,
    supports_json_object BOOLEAN NOT NULL DEFAULT false,
    supports_reasoning_effort BOOLEAN NOT NULL DEFAULT false,
    supports_streaming BOOLEAN NOT NULL DEFAULT false,
    max_input_tokens INTEGER NOT NULL,
    max_output_tokens INTEGER NOT NULL,
    structured_output_adapter TEXT NOT NULL,
    storage_disable_supported BOOLEAN NOT NULL DEFAULT false,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (provider, model)
);

CREATE TABLE IF NOT EXISTS llm_product_generation_policies (
    id SERIAL PRIMARY KEY,
    product_code TEXT NOT NULL UNIQUE,
    max_domains INTEGER NOT NULL,
    max_chapters INTEGER NOT NULL,
    max_output_tokens INTEGER NOT NULL,
    max_reasoning_effort TEXT NOT NULL DEFAULT 'medium',
    allow_chapter_orchestrated BOOLEAN NOT NULL DEFAULT false,
    is_active BOOLEAN NOT NULL DEFAULT true
);

-- Seeds bootstrap (idempotent)
INSERT INTO llm_astrological_domains (domain_code, label_fr, sort_order) VALUES
    ('identity', 'Identite', 1),
    ('emotional_life', 'Vie emotionnelle', 2),
    ('relationships', 'Relations', 3),
    ('career', 'Carriere', 4),
    ('money', 'Argent', 5),
    ('family', 'Famille', 6),
    ('inner_conflicts', 'Conflits interieurs', 7),
    ('talents', 'Talents', 8),
    ('growth_path', 'Chemin de croissance', 9)
ON CONFLICT (domain_code) DO NOTHING;

INSERT INTO llm_product_prompt_profiles (product_code, prompt_family, prompt_version) VALUES
    ('natal_basic', 'natal_basic', 'v1'),
    ('natal_premium', 'natal_premium', 'v1')
ON CONFLICT (product_code) DO NOTHING;

INSERT INTO llm_product_generation_policies (
    product_code, max_domains, max_chapters, max_output_tokens, max_reasoning_effort, allow_chapter_orchestrated
) VALUES
    ('natal_basic', 6, 6, 8000, 'medium', false),
    ('natal_premium', 12, 12, 16000, 'high', true)
ON CONFLICT (product_code) DO NOTHING;

INSERT INTO llm_provider_models (
    provider, model, supports_json_schema_strict, supports_json_object, supports_reasoning_effort,
    supports_streaming, max_input_tokens, max_output_tokens, structured_output_adapter, storage_disable_supported
) VALUES
    ('fake', 'fake-model', true, true, true, false, 128000, 16384, 'prompt_only', true),
    ('openai', 'gpt-4.1', true, true, false, true, 128000, 16384, 'openai_responses_text_format', true),
    ('openai', 'gpt-4o-mini', true, true, false, true, 128000, 16384, 'openai_responses_text_format', true),
    ('anthropic', 'claude-sonnet-4-20250514', true, true, false, true, 200000, 8192, 'anthropic_output_config_format', false),
    ('mistral', 'mistral-large-latest', true, true, false, true, 128000, 8192, 'mistral_response_format_json_schema', false)
ON CONFLICT (provider, model) DO NOTHING;

UPDATE llm_provider_models
SET supports_reasoning_effort = false
WHERE provider = 'openai' AND model = 'gpt-4.1';

INSERT INTO llm_safety_content_patterns (pattern_type, locale, pattern) VALUES
    ('injection', 'en', 'ignore previous'),
    ('injection', 'en', 'ignore safety'),
    ('injection', 'fr', 'ignore les instructions'),
    ('injection', 'fr', 'oublie tes regles'),
    ('medical', 'fr', 'diagnostic medical'),
    ('medical', 'en', 'medical diagnosis'),
    ('death', 'fr', 'vous allez mourir'),
    ('death', 'en', 'you will die'),
    ('deterministic', 'fr', 'destin inevitable'),
    ('deterministic', 'en', 'certainly will happen'),
    ('symbolic', 'fr', 'symbolique'),
    ('symbolic', 'fr', 'interpretation'),
    ('symbolic', 'en', 'symbolic'),
    ('symbolic', 'en', 'interpretation');
