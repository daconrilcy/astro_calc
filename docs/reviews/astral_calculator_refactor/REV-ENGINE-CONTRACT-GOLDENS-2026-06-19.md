# Review adversariale - engine contract goldens (2026-06-19)

Perimetre relu:

- `tests/engine_contract_tests.rs`
- `tests/golden/llm_projection_natal_v1_paris_1990_*.json`
- `tests/golden/astro_engine_response_v1_paris_1990_rich.json`

## Findings

### EC-GOLDEN-001 - Axe de maisons reconstruit depuis le premier row seed

Statut: corrige.

Le helper de test `house_axes_from_seed` utilisait le premier membre trouve pour
un axe. Le seed contient les deux directions d'un meme axe; le test dependait
donc de l'ordre physique du JSON au lieu de reproduire la contrainte runtime.

Correction:

- selection du membre canonique avec `house_a.number < house_b.number`;
- tri final par `house_a_number`, equivalent au `ORDER BY house_a.number` de la
  query runtime.

### EC-GOLDEN-002 - Writer de golden engine trop sensible a l'ordre JSON interne

Statut: corrige.

Le writer du golden engine serialisait une `serde_json::Value`. Cela peut
masquer la forme naturelle du contrat type et produire un churn important lors
des regenerations.

Correction:

- ajout d'un helper typé `build_engine_response_sample`;
- le writer `UPDATE_ENGINE_RESPONSE_GOLDEN=1` serialise maintenant
  `AstroEngineResponse` directement;
- les assertions continuent a comparer en `Value`, donc l'ordre JSON ne devient
  pas une condition fonctionnelle du test.

## Re-review

Aucun finding ouvert.

Verification:

```powershell
cargo fmt --check
cargo test -p astral_calculator --test engine_contract_tests -- --test-threads=1
cargo test -p astral_calculator --test refactor_governance_tests
cargo test -p astral_calculator
```
