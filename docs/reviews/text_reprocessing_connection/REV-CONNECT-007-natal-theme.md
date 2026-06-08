# REV-CONNECT-007 - Natal theme

## Review

Aucun P0/P1/P2 ouvert.

La lecture finale orchestree passe dans `reprocess_natal_theme` apres assemblage des chapitres, enrichissement `astro_basis`, validations de coherence et construction de la qualite.

Correction adversariale: les operations destructrices de longueur/repetition ne sont pas appliquees au reading final; elles restent gerees par `TokenBudget` et `ReadingQualityValidator`.

## Verification

- Fixture: `tests/fixtures/text_reprocessing_migration/natal_theme/baseline.json`
- `cargo test -p astral_llm_application text_reprocessing`
