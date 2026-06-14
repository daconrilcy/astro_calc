# REV-CONNECT-008-followup - cycle adversarial supplementaire

## Scope

Relecture complete apres branchement progressif des services et premieres corrections.

## Findings

### P2 - Erreur d'adaptation natal_simplified silencieuse

`sanitize_reading_text_fields` et `restore_french_typography_fields` utilisaient `unwrap_or_default` sur le resultat de l'adapter. Une erreur de serialisation/deserialisation aurait laisse le service continuer avec un audit vide, sans signal clair.

Correction: les wrappers echouent explicitement avec `expect` si l'adapter `text_reprocessing` ne peut pas traiter le contrat typed.

Test: `text_reprocessing_adapter_natal_simplified_preserves_technical_fields`.

### P2 - Couverture insuffisante des champs techniques natal_simplified

Les tests couvraient la sanitation/typographie generale, mais pas la preservation simultanee de `code`, `fact_id` et `interpretive_role` au niveau adapter.

Correction: ajout d'un test dedie qui verifie que les champs publics `summary`, `body`, `label`, `factor` sont retraitables tandis que les champs techniques restent intacts.

Test: `text_reprocessing_adapter_natal_simplified_preserves_technical_fields`.

### P2 - Sanitation publique horoscope_period encore implementee localement

La review statique a montre que `sanitize_period_public_string` portait encore les remplacements de codes techniques, les corrections de fragments francais et les frontieres de phrases dans `horoscope/mod.rs`.

Correction: deplacement de cette logique dans `ScriptSanitizerProcessor` pour `SERVICE_HOROSCOPE_PERIOD`; `sanitize_period_public_string` ne fait plus que deleguer a `reprocess_horoscope_period`.

Tests: `text_reprocessing_horoscope_period_sanitizes_public_technical_leaks`, `cargo test -p astral_llm_api --test horoscope_tests`.

## Residual findings

Aucun P0/P1/P2 ouvert sur le branchement `text_reprocessing`.
