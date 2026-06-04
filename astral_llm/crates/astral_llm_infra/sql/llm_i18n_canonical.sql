-- i18n canonique astral_llm : langue de reponse LLM, roles astro_basis, libelles aspects (post-LLM)

CREATE TABLE IF NOT EXISTS llm_writing_locales (
    id SERIAL PRIMARY KEY,
    locale_code TEXT NOT NULL UNIQUE,
    iso_639_1 TEXT NOT NULL,
    display_name TEXT NOT NULL,
    prompt_instruction TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true
);

CREATE TABLE IF NOT EXISTS llm_astro_basis_roles (
    id SERIAL PRIMARY KEY,
    role_code TEXT NOT NULL UNIQUE,
    is_active BOOLEAN NOT NULL DEFAULT true
);

CREATE TABLE IF NOT EXISTS llm_aspect_type_labels (
    id SERIAL PRIMARY KEY,
    aspect_code TEXT NOT NULL,
    locale TEXT NOT NULL,
    label TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    UNIQUE (aspect_code, locale)
);

INSERT INTO llm_writing_locales (locale_code, iso_639_1, display_name, prompt_instruction) VALUES
    ('fr', 'fr', 'French',
     'OUTPUT_LANGUAGE: fr. Write title, body, summary fields, and any human-readable strings you generate in astro_basis (factor, label) in French. Astro DATA may be in English; never translate fact_ids.'),
    ('en', 'en', 'English',
     'OUTPUT_LANGUAGE: en. Write title, body, summary fields, and any human-readable strings you generate in astro_basis (factor, label) in English. Astro DATA may be in another language; never translate fact_ids.'),
    ('es', 'es', 'Spanish',
     'OUTPUT_LANGUAGE: es. Write title, body, summary fields, and any human-readable strings you generate in astro_basis (factor, label) in Spanish. Astro DATA may be in English; never translate fact_ids.'),
    ('de', 'de', 'German',
     'OUTPUT_LANGUAGE: de. Write title, body, summary fields, and any human-readable strings you generate in astro_basis (factor, label) in German. Astro DATA may be in English; never translate fact_ids.')
ON CONFLICT (locale_code) DO NOTHING;

INSERT INTO llm_astro_basis_roles (role_code) VALUES
    ('core'), ('supporting'), ('nuance'), ('domain_score')
ON CONFLICT (role_code) DO NOTHING;

INSERT INTO llm_aspect_type_labels (aspect_code, locale, label) VALUES
    ('conjunction', 'fr', 'conjonction'),
    ('opposition', 'fr', 'opposition'),
    ('trine', 'fr', 'trigone'),
    ('square', 'fr', 'carré'),
    ('sextile', 'fr', 'sextile'),
    ('conjunction', 'en', 'conjunction'),
    ('opposition', 'en', 'opposition'),
    ('trine', 'en', 'trine'),
    ('square', 'en', 'square'),
    ('sextile', 'en', 'sextile'),
    ('conjunction', 'es', 'conjunción'),
    ('opposition', 'es', 'oposición'),
    ('trine', 'es', 'trígono'),
    ('square', 'es', 'cuadratura'),
    ('sextile', 'es', 'sextil'),
    ('conjunction', 'de', 'Konjunktion'),
    ('opposition', 'de', 'Opposition'),
    ('trine', 'de', 'Trigon'),
    ('square', 'de', 'Quadrat'),
    ('sextile', 'de', 'Sextil')
ON CONFLICT (aspect_code, locale) DO NOTHING;
