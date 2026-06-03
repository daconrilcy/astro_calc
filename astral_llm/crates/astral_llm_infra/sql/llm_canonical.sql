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
