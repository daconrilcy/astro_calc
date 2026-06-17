# REV-b994848 - Adversarial loop

- Commit revu: `b994848 refactor(calculator): tighten boundaries and projections`
- Date: 2026-06-17
- Statut: closed

## Scope

- Frontieres `natal` / `simplified` / `horoscope` apres extraction des calculs communs sous `astrology/`.
- Compatibilite des anciens chemins publics `natal::aspects` et `natal::ephemeris`.
- Injection des repositories dans les services applicatifs sans `PgPool` dans les couches metier verrouillees.
- Contrats publics calculateur, API et schemas publies.

## Checks adversariaux

- Scan imports interdits:
  - `domain -> infra`
  - `PgPool`, `connect_from_env`, `block_on`, `run_blocking` dans `domain`, `engine`, `horoscope`, `simplified`
  - imports `crate::natal::aspects` / `crate::natal::ephemeris` depuis `simplified` ou `horoscope`
  - usage de `features/shared`
- Lecture ciblee:
  - `astrology/ephemeris.rs`
  - `natal/ephemeris.rs`
  - `runtime/mod.rs`
  - `engine/application/runtime_facade_service.rs`
  - `horoscope/application/horoscope_service.rs`
  - `simplified/application/simplified_natal_service.rs`
  - `tests/refactor_governance_tests.rs`

## Verification

- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator`
- `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`
- `cargo test -p astral_calculator_api --test astral_calculator_api_tests`
- `cargo test -p astral_llm_api --test contracts_publish_tests`
- `git show --check HEAD`

## Findings

Aucun finding ouvert.

## Notes

- Les wrappers `natal::aspects` et `natal::ephemeris` restent des re-exports de compatibilite, sans reutilisation interne par `simplified` ou `horoscope`.
- `runtime/mod.rs` reste la bordure d'assemblage qui detient `PgPool`; les services applicatifs recoivent des repositories deja construits.
- Les tests API confirment que la refacto de constructeurs ne casse pas les routes publiques calculateur.
