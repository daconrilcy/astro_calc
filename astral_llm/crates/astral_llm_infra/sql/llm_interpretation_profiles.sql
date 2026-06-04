-- Profils d'interpretation natal (source canonique pour natal_prompter)
-- Soumission ops : scripts/manage_natal_interpretation_profiles.ps1

CREATE TABLE IF NOT EXISTS llm_interpretation_profiles (
    profile_code TEXT PRIMARY KEY,
    product_code TEXT NOT NULL DEFAULT 'natal_prompter',
    schema_version TEXT NOT NULL DEFAULT 'v1',
    profile_json JSONB NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_llm_interpretation_profiles_active
    ON llm_interpretation_profiles (product_code, is_active);
