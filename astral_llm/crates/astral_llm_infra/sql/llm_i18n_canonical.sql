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

CREATE TABLE IF NOT EXISTS llm_element_balance_labels (
    id SERIAL PRIMARY KEY,
    element_code TEXT NOT NULL,
    locale TEXT NOT NULL,
    display_label TEXT NOT NULL,
    interpretive_label TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    UNIQUE (element_code, locale)
);

CREATE TABLE IF NOT EXISTS llm_modality_balance_labels (
    id SERIAL PRIMARY KEY,
    modality_code TEXT NOT NULL,
    locale TEXT NOT NULL,
    display_label TEXT NOT NULL,
    interpretive_label TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    UNIQUE (modality_code, locale)
);

CREATE TABLE IF NOT EXISTS llm_sect_labels (
    id SERIAL PRIMARY KEY,
    sect_code TEXT NOT NULL,
    locale TEXT NOT NULL,
    display_label TEXT NOT NULL,
    interpretive_label TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    UNIQUE (sect_code, locale)
);

CREATE TABLE IF NOT EXISTS llm_house_theme_labels (
    id SERIAL PRIMARY KEY,
    house_number SMALLINT NOT NULL,
    locale TEXT NOT NULL,
    display_label TEXT NOT NULL,
    interpretive_label TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    UNIQUE (house_number, locale)
);

INSERT INTO llm_element_balance_labels (element_code, locale, display_label, interpretive_label) VALUES
    ('fire', 'fr', 'Dominante élément Feu', 'Dominante Feu : élan, intuition et mouvement'),
    ('earth', 'fr', 'Dominante élément Terre', 'Dominante Terre : stabilité, réalisme et construction'),
    ('air', 'fr', 'Dominante élément Air', 'Dominante Air : idées, échanges et perspective'),
    ('water', 'fr', 'Dominante élément Eau', 'Dominante Eau : sensibilité, mémoire et lien intérieur')
ON CONFLICT (element_code, locale) DO NOTHING;

INSERT INTO llm_modality_balance_labels (modality_code, locale, display_label, interpretive_label) VALUES
    ('cardinal', 'fr', 'Dominante cardinale', 'Dominante cardinale : initiative et ouverture des cycles'),
    ('fixed', 'fr', 'Dominante fixe', 'Dominante fixe : persévérance et ancrage'),
    ('mutable', 'fr', 'Dominante mutable', 'Dominante mutable : adaptabilité et transition')
ON CONFLICT (modality_code, locale) DO NOTHING;

INSERT INTO llm_sect_labels (sect_code, locale, display_label, interpretive_label) VALUES
    ('day', 'fr', 'Thème diurne', 'Thème diurne : visibilité et rayonnement'),
    ('night', 'fr', 'Thème nocturne', 'Thème nocturne : intériorité et réceptivité')
ON CONFLICT (sect_code, locale) DO NOTHING;

INSERT INTO llm_house_theme_labels (house_number, locale, display_label, interpretive_label) VALUES
    (2, 'fr', 'Emphase de la maison 2', 'Emphase de la maison 2 : ressources, valeur et sécurité'),
    (3, 'fr', 'Emphase de la maison 3', 'Emphase de la maison 3 : communication et environnement proche'),
    (4, 'fr', 'Emphase de la maison 4', 'Emphase de la maison 4 : racines, foyer et mémoire'),
    (10, 'fr', 'Emphase de la maison 10', 'Emphase de la maison 10 : vocation et reconnaissance')
ON CONFLICT (house_number, locale) DO NOTHING;

CREATE TABLE IF NOT EXISTS llm_house_axis_labels (
    id SERIAL PRIMARY KEY,
    axis_code TEXT NOT NULL,
    locale TEXT NOT NULL,
    display_label TEXT NOT NULL,
    interpretive_label TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    UNIQUE (axis_code, locale)
);

INSERT INTO llm_house_axis_labels (axis_code, locale, display_label, interpretive_label) VALUES
    ('private_public', 'fr', 'Axe vie privée / vie publique',
     'Axe vie privée / vie publique : tension entre foyer intérieur, exposition et rôle social'),
    ('self_relationship', 'fr', 'Axe identité / relation',
     'Axe identité / relation : équilibre entre affirmation personnelle et rencontre de l''autre'),
    ('resources_sharing', 'fr', 'Axe ressources personnelles / ressources partagées',
     'Axe ressources personnelles / ressources partagées : circulation entre sécurité propre, confiance et engagement commun'),
    ('local_distant', 'fr', 'Axe proche / lointain',
     'Axe proche / lointain : tension entre environnement immédiat et horizons élargis'),
    ('creation_collective', 'fr', 'Axe création personnelle / collectif',
     'Axe création personnelle / collectif : tension entre expression individuelle et idéaux partagés'),
    ('control_surrender', 'fr', 'Axe maîtrise / lâcher-prise',
     'Axe maîtrise / lâcher-prise : tension entre ordre quotidien et besoin de relâchement intérieur'),
    ('private_public', 'en', 'Private / public life axis',
     'Private / public life axis: tension between inner home, visibility and social role'),
    ('self_relationship', 'en', 'Identity / relationship axis',
     'Identity / relationship axis: balance between personal assertion and encounter with others'),
    ('resources_sharing', 'en', 'Personal / shared resources axis',
     'Personal / shared resources axis: flow between self-security, trust and shared commitment'),
    ('local_distant', 'en', 'Near / distant axis',
     'Near / distant axis: tension between immediate environment and wider horizons'),
    ('creation_collective', 'en', 'Personal creation / collective axis',
     'Personal creation / collective axis: tension between individual expression and shared ideals'),
    ('control_surrender', 'en', 'Control / surrender axis',
     'Control / surrender axis: tension between daily order and inner release')
ON CONFLICT (axis_code, locale) DO NOTHING;
