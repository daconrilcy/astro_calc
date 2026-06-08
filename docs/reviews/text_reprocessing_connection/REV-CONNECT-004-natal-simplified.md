# REV-CONNECT-004 - Natal simplified

## Review

Finding P2 corrige: sanitation et typographie etaient implementees localement dans `simplified_reading_postprocess`.

Correction: `sanitize_reading_text_fields` et `restore_french_typography_fields` sont devenus wrappers vers `reprocess_natal_simplified`.

Note: `normalize_simplified_interpretive_roles` reste conserve comme logique structurelle de contrat, car le remplacer par `HumanizeLabels` modifierait aussi des labels et casserait la parite stricte.

## Verification

- `cargo test -p astral_llm_application simplified_reading_postprocess`
- `cargo test -p astral_llm_api --test astral_llm_simplified_reading_tests`
