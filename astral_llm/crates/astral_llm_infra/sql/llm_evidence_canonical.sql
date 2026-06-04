-- Referentiels evidence Premium (couche interpretive)

CREATE TABLE IF NOT EXISTS llm_evidence_kind_definitions (
    id SERIAL PRIMARY KEY,
    kind_code TEXT NOT NULL UNIQUE,
    kind_family TEXT NOT NULL,
    label_fr TEXT,
    default_usage TEXT NOT NULL DEFAULT 'interpretive_basis',
    is_active BOOLEAN NOT NULL DEFAULT true
);

CREATE TABLE IF NOT EXISTS llm_chapter_evidence_slots (
    id SERIAL PRIMARY KEY,
    chapter_code TEXT NOT NULL,
    slot_role TEXT NOT NULL,
    kind_code TEXT,
    object_code TEXT,
    house_number INTEGER,
    domain_code TEXT,
    priority INTEGER NOT NULL DEFAULT 100,
    min_weight REAL NOT NULL DEFAULT 0.0,
    max_items INTEGER NOT NULL DEFAULT 1,
    required_if_available BOOLEAN NOT NULL DEFAULT false,
    is_active BOOLEAN NOT NULL DEFAULT true
);

CREATE TABLE IF NOT EXISTS llm_premium_evidence_policies (
    id SERIAL PRIMARY KEY,
    product_code TEXT NOT NULL UNIQUE,
    min_evidence_per_chapter INTEGER NOT NULL DEFAULT 4,
    min_distinct_kind_families INTEGER NOT NULL DEFAULT 2,
    min_non_placement_if_available INTEGER NOT NULL DEFAULT 1,
    max_core_overlap_ratio REAL NOT NULL DEFAULT 0.60,
    domain_score_counts_in_minimum BOOLEAN NOT NULL DEFAULT false,
    max_core_evidence INTEGER NOT NULL DEFAULT 3,
    max_supporting_evidence INTEGER NOT NULL DEFAULT 4,
    max_nuance_evidence INTEGER NOT NULL DEFAULT 2,
    max_avoid_repeating INTEGER NOT NULL DEFAULT 5,
    max_supporting_semantic_chapters INTEGER NOT NULL DEFAULT 3,
    is_active BOOLEAN NOT NULL DEFAULT true
);

ALTER TABLE llm_premium_evidence_policies
    ADD COLUMN IF NOT EXISTS max_supporting_semantic_chapters INTEGER NOT NULL DEFAULT 3;

CREATE TABLE IF NOT EXISTS llm_evidence_requirements (
    id SERIAL PRIMARY KEY,
    requirement_code TEXT NOT NULL UNIQUE,
    chapter_code TEXT NOT NULL,
    accepted_kind_codes TEXT[] NOT NULL DEFAULT '{}',
    accepted_object_codes TEXT[] NOT NULL DEFAULT '{}',
    accepted_house_numbers INTEGER[] NOT NULL DEFAULT '{}',
    min_count INTEGER NOT NULL DEFAULT 1,
    required_if_available BOOLEAN NOT NULL DEFAULT true,
    severity TEXT NOT NULL DEFAULT 'blocking',
    is_active BOOLEAN NOT NULL DEFAULT true
);

INSERT INTO llm_evidence_kind_definitions (kind_code, kind_family, label_fr, default_usage) VALUES
    ('placement', 'placement', 'Placement', 'interpretive_basis'),
    ('angle', 'placement', 'Angle', 'interpretive_basis'),
    ('aspect', 'aspect', 'Aspect', 'interpretive_basis'),
    ('house_ruler', 'rulership', 'Maitre de maison', 'interpretive_basis'),
    ('essential_dignity', 'dignity', 'Dignite essentielle', 'interpretive_basis'),
    ('accidental_dignity', 'dignity', 'Dignite accidentelle', 'interpretive_basis'),
    ('planetary_condition', 'condition', 'Condition planetaire', 'interpretive_basis'),
    ('sect_condition', 'condition', 'Secte', 'interpretive_basis'),
    ('lunar_phase', 'condition', 'Phase lunaire', 'interpretive_basis'),
    ('dominant_planet', 'balance', 'Planete dominante', 'interpretive_basis'),
    ('element_balance', 'balance', 'Balance elementaire', 'interpretive_basis'),
    ('modality_balance', 'balance', 'Balance modale', 'interpretive_basis'),
    ('house_emphasis', 'pattern', 'Emphase maison', 'interpretive_basis'),
    ('house_axis', 'pattern', 'Axe maison', 'interpretive_basis'),
    ('domain_score', 'domain_score', 'Score domaine', 'domain_selection'),
    ('other', 'other', 'Autre', 'interpretive_basis')
ON CONFLICT (kind_code) DO NOTHING;

INSERT INTO llm_premium_evidence_policies (
    product_code, min_evidence_per_chapter, min_distinct_kind_families,
    min_non_placement_if_available, max_core_overlap_ratio, domain_score_counts_in_minimum,
    max_core_evidence, max_supporting_evidence, max_nuance_evidence, max_avoid_repeating,
    max_supporting_semantic_chapters
) VALUES
    ('natal_premium', 4, 2, 1, 0.60, false, 3, 4, 2, 5, 3)
ON CONFLICT (product_code) DO NOTHING;

UPDATE llm_premium_evidence_policies
SET max_supporting_semantic_chapters = 3
WHERE product_code = 'natal_premium' AND max_supporting_semantic_chapters IS DISTINCT FROM 3;

INSERT INTO llm_chapter_evidence_slots (chapter_code, slot_role, kind_code, object_code, house_number, priority, max_items, required_if_available) VALUES
    ('identity', 'core', 'angle', 'ascendant', 1, 10, 1, true),
    ('identity', 'core', 'house_ruler', NULL, 1, 20, 1, true),
    ('identity', 'supporting', 'aspect', NULL, NULL, 40, 2, false),
    ('identity', 'nuance', 'essential_dignity', NULL, NULL, 50, 1, false),
    ('emotional_life', 'core', 'placement', 'moon', NULL, 10, 1, true),
    ('emotional_life', 'core', 'aspect', NULL, NULL, 20, 2, true),
    ('emotional_life', 'supporting', 'placement', NULL, 4, 30, 1, true),
    ('emotional_life', 'supporting', 'house_ruler', NULL, 4, 40, 1, true),
    ('emotional_life', 'nuance', 'lunar_phase', NULL, NULL, 50, 1, false),
    ('relationships', 'core', 'placement', 'venus', NULL, 10, 1, true),
    ('relationships', 'core', 'placement', NULL, 7, 20, 1, true),
    ('relationships', 'core', 'house_ruler', 'descendant', NULL, 30, 1, true),
    ('relationships', 'supporting', 'aspect', NULL, NULL, 40, 2, false),
    ('relationships', 'nuance', 'placement', 'moon', NULL, 50, 1, false),
    ('career', 'core', 'angle', 'mc', 10, 10, 1, true),
    ('career', 'core', 'placement', NULL, 10, 20, 1, true),
    ('career', 'core', 'house_ruler', 'mc', NULL, 30, 1, true),
    ('career', 'supporting', 'placement', 'saturn', NULL, 40, 1, false),
    ('career', 'supporting', 'placement', 'jupiter', NULL, 50, 1, false),
    ('career', 'supporting', 'placement', NULL, 2, 60, 1, false),
    ('career', 'supporting', 'placement', NULL, 6, 70, 1, false),
    ('growth_path', 'core', 'placement', 'north_node', NULL, 10, 1, false),
    ('growth_path', 'core', 'placement', 'saturn', NULL, 20, 1, false),
    ('growth_path', 'supporting', 'aspect', NULL, NULL, 30, 2, false),
    ('growth_path', 'supporting', 'placement', NULL, 8, 40, 1, false),
    ('growth_path', 'supporting', 'placement', NULL, 9, 50, 1, false),
    ('growth_path', 'supporting', 'placement', NULL, 12, 60, 1, false);

INSERT INTO llm_evidence_requirements (
    requirement_code, chapter_code, accepted_kind_codes, accepted_object_codes,
    accepted_house_numbers, min_count, required_if_available, severity
) VALUES
    ('career_mc_or_h10', 'career', ARRAY['angle','placement'], ARRAY['mc'], ARRAY[10], 1, true, 'blocking'),
    ('relationships_venus_or_h7', 'relationships', ARRAY['placement','house_ruler'], ARRAY['venus'], ARRAY[7], 1, true, 'blocking'),
    ('emotional_moon_aspects', 'emotional_life', ARRAY['aspect'], ARRAY['moon'], ARRAY[]::integer[], 1, true, 'blocking'),
    ('identity_asc_ruler', 'identity', ARRAY['angle','house_ruler'], ARRAY['ascendant'], ARRAY[1], 1, true, 'blocking'),
    ('global_aspect_when_available', 'identity', ARRAY['aspect'], ARRAY[]::text[], ARRAY[]::integer[], 1, true, 'warning'),
    ('career_ruler_10', 'career', ARRAY['house_ruler'], ARRAY['mc'], ARRAY[10], 1, true, 'blocking'),
    ('relationships_ruler_7', 'relationships', ARRAY['house_ruler'], ARRAY['descendant'], ARRAY[7], 1, true, 'blocking'),
    ('relationships_relational_aspect', 'relationships', ARRAY['aspect'], ARRAY['venus','descendant'], ARRAY[]::integer[], 1, true, 'warning'),
    ('growth_path_nodal', 'growth_path', ARRAY['placement'], ARRAY['north_node','south_node'], ARRAY[]::integer[], 1, true, 'blocking'),
    ('growth_path_structuring_aspect', 'growth_path', ARRAY['aspect'], ARRAY['saturn','north_node','south_node'], ARRAY[]::integer[], 1, true, 'warning'),
    ('growth_path_transformation_house', 'growth_path', ARRAY['placement'], ARRAY[]::text[], ARRAY[8,9,12], 1, true, 'warning')
ON CONFLICT (requirement_code) DO NOTHING;
