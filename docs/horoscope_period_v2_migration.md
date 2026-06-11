# Horoscope Period V2 Migration

## Objective

Move all `next_7_days_natal` horoscope services to `semantic_brief_v2`.

Central rule:

- Rust calculates, scores, selects, structures and validates.
- The LLM writes the public reading.
- Postprocess stays limited to technical cleanup and must not become a co-writer.

## Initial State

- The legacy period pipeline builds `horoscope_period_interpretation_request_v1`.
- Legacy request fields include public-like editorial material such as `daily_plans`, `domain_sections`, `editorial_brief`, `summary_hint`, `personalization_hint`, `focus` and `reason`.
- Legacy postprocess contains several public-text repair and personalization functions used to compensate for mechanical provider output.
- Existing V1 functions and goldens remain available as rollback and regression fixtures.

## Services Switched

- `horoscope_free_next_7_days_natal`
- `horoscope_basic_next_7_days_natal`
- `horoscope_premium_next_7_days_natal`

All three now carry `generation_mode = "semantic_brief_v2"` in `json_db/horoscope_services.json`.

## Change Log

### 2026-06-11

- Added `generation_mode` to the canonical horoscope service seed.
- Added `TargetLanguageCode` with `fr`, `en`, `es`, `de`.
- Kept temporary compatibility with legacy `target_language`.
- Added bounded `astrologer_persona` validation.
- Added `horoscope_period_writer_request_v2` contract.
- Added V2 writer request construction through `build_period_writer_request_v2`.
- Added `semantic_brief` construction from scored period evidence and events.
- Routed `HoroscopePeriodNatalOrchestrator` by service `generation_mode`.
- Added V2 writer prompt, fake writer response, response repair and postprocess path.
- Kept legacy V1 request builder and writer behavior for rollback and existing tests.
- Confirmed JSON-to-Postgres dry-run emits the `generation_mode` column and `semantic_brief_v2` rows for free/basic/premium period services.
- Adversarial review fixes:
  - Public period payload schema no longer requires legacy `target_language`; `target_language_code` and missing language both resolve to the V2 default `fr`.
  - V2 `window_candidates` are now built from atomic event facts, not from legacy humanized window objects.
  - V2 quality loop now performs a targeted editor retry for schema/evidence/date/language/artifact failures instead of relying on local public-text repair.
  - V2 `semantic_brief` now exposes exactly `daily_signal_summary`, `best_day_candidates`, `watch_day_candidates`, `key_day_candidates`, `window_candidates`, `domain_candidates`, `repeating_arcs` and `evidence`.
  - V2 repair/postprocess no longer add fallback public prose or call legacy period text reprocessing; they are limited to variant pruning, trim cleanup and strict technical fields.
  - V2 writer/editor prompts now require `target_language_code` in the writer request and fail fast instead of falling back to a hardcoded language.

## Contracts

- Public response remains `horoscope_period_response_v1`.
- New internal writer request is `horoscope_period_writer_request_v2`.
- V2 writer request is strict and includes:
  - `contract_version`
  - `service_code`
  - `generation_mode`
  - `target_language_code`
  - `period_resolution`
  - `scan_plan`
  - `detail_profile_code`
  - `semantic_brief`
  - `evidence`
  - `safety_profile`
  - `output_contract_version`

## Data Changes

- `json_db/horoscope_services.json` now includes `generation_mode`.
- The JSON-to-Postgres importer derives columns from JSON structure and data, so the new field is included in generated import SQL.

## V2 Functions Added

- `build_period_writer_request_v2`
- `validate_period_writer_request_v2_schema`
- `fake_period_writer_response_v2`
- `repair_period_response_shape_v2`
- `postprocess_period_provider_response_v2`
- `period_writer_response_with_quality_loop`
- `period_style_editor_response_v2`

## Legacy Preserved

- `build_period_interpretation_request`
- `validate_period_interpretation_request_schema`
- `fake_period_writer_response` legacy path for V1 requests
- Legacy postprocess and personalization functions for rollback only

## Tests And Validation

Executed validation commands:

- `cargo check -p astral_llm_application`
- `cargo test -p astral_llm_api --test horoscope_v1_tests` (276 passed)
- `cargo test -p astral_llm_api --test contracts_publish_tests` (3 passed, 1 ignored)
- `cargo test -p astral_llm_application` (183 unit tests + 33 integration tests + doctests passed)
- `python scripts\import_json_db_to_postgres.py --dry-run --output target\astral_json_db_import_v2.sql`

## Open Follow-Ups

- Extract the period code physically under `src/horoscope/period/` once V2 behavior is stable.
- Compare V1 and V2 outputs over free/basic/premium samples before removing legacy.
- Keep OpenAI real multilingual runs ignored by default and run them manually for certification.
