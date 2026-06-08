# REV-CONNECT-001 - Shared

## Review

Aucun P0/P1/P2 ouvert.

`reprocess_shared_text` centralise sanitation + typographie pour texte partage. Les helpers bas niveau restent conserves car le module central les utilise.

## Verification

- Fixture: `tests/fixtures/text_reprocessing_migration/shared/baseline.json`
- Test: `text_reprocessing_adapter_shared_matches_pipeline_contract`
