# REV-PORTS-BUILDERS-FAILFAST-2026-06-19 follow-up 2

Statut: closed

## Findings revus

- Medium: `tests/engine_contract_tests.rs` échouait bien explicitement sans PostgreSQL, mais retentait une connexion DB par test et transformait le fail-fast en fail-slow sur la suite complète.

## Corrections

- Mutualisation test-only du chargement des profils LLM DB via `OnceLock<Result<BTreeMap<...>, String>>`.
- Une seule tentative de connexion PostgreSQL et de chargement des profils est effectuée par process de test; les tests suivants réutilisent soit le cache chargé, soit le même message d'échec explicite.

## Verification

- `cargo fmt --check`
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator --test engine_contract_tests llm_projection_levels_share_identical_structure -- --test-threads=1`
- `cargo test -p astral_calculator --test engine_contract_tests -- --test-threads=1`

## Resultat

- L'absence de PostgreSQL continue d'échouer explicitement.
- La suite complète ne répète plus un timeout DB par test.
- Aucun finding ouvert.
