# REV-CONNECT-003 - Calculator projection

## Review

Aucun P0/P1/P2 ouvert dans `astral_llm_application`: aucun point runtime direct de projection calculateur n'a ete trouve dans cette crate.

Correction: helper `reprocess_calculator_projection` disponible et teste pour les payloads exposant des codes publics.

## Verification

- Fixture: `tests/fixtures/text_reprocessing_migration/calculator_projection/baseline.json`
- Test: `text_reprocessing_adapter_calculator_projection_keeps_normalized_json_stable`
