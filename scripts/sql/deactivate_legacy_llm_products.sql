-- Desactive les anciens product_code LLM (remplaces par natal_prompter + profils).
-- Idempotent : aligne llm_canonical.sql / llm_provider_catalog.sql.

UPDATE llm_product_prompt_profiles
SET is_active = false
WHERE product_code IN ('natal_basic', 'natal_premium');

UPDATE llm_product_generation_policies
SET is_active = false
WHERE product_code IN ('natal_basic', 'natal_premium');

UPDATE llm_product_default_engine
SET is_active = false, updated_at = NOW()
WHERE product_code IN ('natal_basic', 'natal_premium');

UPDATE llm_product_allowed_models
SET is_active = false
WHERE product_code IN ('natal_basic', 'natal_premium');
