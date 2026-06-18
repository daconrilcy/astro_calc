# Review adversariale - projection reasons v14

Date: 2026-06-18
Statut: closed

## Perimetre

Vague de rupture `natal_structured_v14` pour typer les raisons de dominantes
et d'axes, introduire le referentiel DB associe et maintenir la surface de
projection LLM publique.

## Cycle 1

### Findings

- P1 - Un oubli d'alignement entre schema, version courante et scoring profile
  aurait laisse le runtime produire un payload v14 non chargeable par le
  catalogue courant.
- P1 - Une humanisation encore basee sur les anciennes strings aurait laisse
  fuir du snake_case ou des codes combinatoires dans `supporting_factors`.
- P2 - L'absence de trace documentaire/review de la vague aurait enfreint les
  invariants de gouvernance imposes au refacto `astral_calculator`.

### Corrections

- Le schema `contracts/calculator/natal_structured_v14.schema.json`, le
  `contracts/versions.json`, les goldens et les chargements runtime ont ete
  alignes sur `natal_structured_v14`.
- Un scoring profile v14 a ete seed dans
  `json_db/astral_basic_product_scoring_profiles.json`.
- Le rendu projection utilise `render_projection_reason(...)` avec templates
  referentiels et labels generiques de dignite/signe/angle.
- La vague est documentee dans `docs/BASIC_PAYLOAD_IMPLEMENTATION.md` et la
  presente review ferme la boucle de gouvernance.

## Cycle 2

### Findings

Aucun finding ouvert.

### Verification adversariale

- Les suites `cargo test -p astral_calculator`,
  `cargo test -p astral_calculator_http --test astral_calculator_http_tests` et
  `cargo test -p astral_llm_api --test contracts_publish_tests` passent avec la
  version v14.
- Le bootstrap DB cree la nouvelle table a partir de `json_db/` via
  `scripts/import_json_db_to_postgres.py`.
- `llm_projection_natal_v1` reste stable; seul le payload brut/audit change de
  version.

## Conclusion

Aucun finding ouvert.
