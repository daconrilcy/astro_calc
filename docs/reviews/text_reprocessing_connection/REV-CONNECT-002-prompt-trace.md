# REV-CONNECT-002 - Prompt trace

## Review

Finding P2 corrige: `prompt_trace` avait son propre formatter `<<< role >>>`.

Correction: `format_compiled_messages` delegue a `reprocess_prompt_trace`, qui utilise `SERVICE_PROMPT_TRACE` et `FormatTrace`.

## Verification

- Fixture: `tests/fixtures/text_reprocessing_migration/prompt_trace/baseline.json`
- Test: `text_reprocessing_adapter_prompt_trace_matches_legacy_format`
