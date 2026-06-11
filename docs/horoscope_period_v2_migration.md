# Horoscope Period V2 Migration

## Objective

Move `horoscope_premium_next_7_days_natal` to `semantic_brief_v2`.
Free and basic 7-day services remain on `legacy_v1` according to the initial Premium-only brief.

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

- `horoscope_premium_next_7_days_natal`

Only Premium 7 days carries `generation_mode = "semantic_brief_v2"` in `json_db/horoscope_services.json`.
`horoscope_free_next_7_days_natal` and `horoscope_basic_next_7_days_natal` intentionally carry `legacy_v1`.

## Change Log

### 2026-06-11

- Added `generation_mode` to the canonical horoscope service seed.
- Added `TargetLanguageCode` with `fr`, `en`, `es`, `de`.
- Kept temporary compatibility with legacy `target_language`; when both fields are provided and diverge, `target_language_code` wins and V2 debug output records `legacy_target_language_ignored`.
- Added bounded `astrologer_persona` validation aligned with the V2 schema. Public payloads no longer accept legacy persona knobs such as `directiveness` or `metaphor_level`.
- Added `horoscope_period_writer_request_v2` contract.
- Added V2 writer request construction through `build_period_writer_request_v2`.
- Added `semantic_brief` construction from scored period evidence and events.
- Routed `HoroscopePeriodNatalOrchestrator` by service `generation_mode`.
- Added V2 writer prompt, fake writer response, response repair and postprocess path.
- Kept legacy V1 request builder and writer behavior for rollback and existing tests.
- Confirmed JSON-to-Postgres dry-run emits the `generation_mode` column; Premium 7 days is `semantic_brief_v2`, while free/basic 7 days remain `legacy_v1`.
- Adversarial review fixes:
  - Public period payload schema no longer requires legacy `target_language`; `target_language_code` and missing language both resolve to the V2 default `fr`.
  - V2 `window_candidates` are now built from atomic event facts, not from legacy humanized window objects.
  - V2 quality loop now performs a targeted editor retry for schema/evidence/date/language/artifact failures instead of relying on local public-text repair.
  - V2 `semantic_brief` now exposes only atomic writing material: `period_arc_keywords`, `dominant_keywords`, `week_tone_codes`, `week_intensity`, `daily_signal_summary`, `best_day_candidates`, `watch_day_candidates`, `key_day_candidates`, `window_candidates`, `domain_candidates` and `repeating_arcs`.
  - `semantic_brief.evidence` is forbidden. Sanitized `evidence` is top-level only and candidates reference it only through `evidence_keys`.
  - V2 repair/postprocess no longer add fallback public prose or call legacy period text reprocessing; they are limited to variant pruning, trim cleanup and strict technical fields.
  - V2 writer/editor prompts now require `target_language_code` in the writer request and fail fast instead of falling back to a hardcoded language.
  - `astrologer_persona` is always present in the V2 writer request and is `null` when absent from the public payload.
  - V2 editor retry validates through V2 schema/evidence/public-text gates only; it no longer calls the legacy period public payload validator after targeted editing.
  - `validate_semantic_brief_is_atomic` now rejects legacy/public prose keys, long strings, sentence-like keywords, unknown evidence references, dates outside the period, snapshot keys outside the scan plan and duplicate top-level evidence keys.
  - The V2 contract is Premium-only: `service_code = horoscope_premium_next_7_days_natal`, `generation_mode = semantic_brief_v2`, and `detail_profile_code = premium_rich`.

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
  - `astrologer_persona`
  - `output_contract_version`

`semantic_brief_v2` is an internal writing input, not a public or UI contract.
It may contain technical codes and short keywords, but no public prose and no full evidence objects.
Top-level `evidence` is sanitized to stable technical fields only: `evidence_key`, `date`, `snapshot_key`, `fact_type`, `transiting_object`, `aspect`, `natal_target`, `natal_house`, `theme_code`, `tone_code` and `score`.

The UI must consume only `$.result.reading`.
It must not consume `calculation`, `interpretation_request`, `writer_request`, `semantic_brief`, `evidence` or `quality_checks`.
In V2 debug output, `writer_request` is an alias of `interpretation_request`.
If legacy `target_language` is ignored because `target_language_code` is also present and different, the debug envelope contains `debug.language_compatibility.legacy_target_language_ignored = true`.

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

These V2 functions are implemented as shared period capability, but only Premium 7 days is routed to them by canonical `generation_mode`.

## Legacy Preserved

- `build_period_interpretation_request`
- `validate_period_interpretation_request_schema`
- `fake_period_writer_response` legacy path for V1 requests
- Legacy postprocess and personalization functions for rollback only
- Free/basic 7-day services use this legacy path intentionally.

## Tests And Validation

Executed validation commands:

- `cargo check -p astral_llm_application`
- `cargo test -p astral_llm_api --test horoscope_v1_tests` (286 passed)
- `cargo test -p astral_llm_api --test contracts_publish_tests` (3 passed, 1 ignored)
- `cargo test -p astral_llm_application` (183 unit tests + 33 integration tests + doctests passed)
- `python scripts\import_json_db_to_postgres.py --dry-run --output target\astral_json_db_import_v2.sql`

## Open Follow-Ups

- Continue extracting period code under `src/horoscope/period/` after the initial `PeriodGenerationMode` split. Target modules: `public_request.rs`, `calculation_request.rs`, `evidence.rs`, `scoring.rs`, `semantic_brief.rs`, `writer_request.rs`, `writer.rs`, `response_repair.rs`, `postprocess.rs`, `validators.rs`, `quality.rs` and `legacy_v1.rs`.
- Compare V1 and V2 outputs over Premium samples before removing legacy rollback.
- Keep OpenAI real multilingual runs ignored by default and run one real Premium V2 generation per `fr`, `en`, `es`, `de` after fake validation for certification.
