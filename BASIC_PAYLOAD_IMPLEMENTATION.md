# 2026-06-19 - `astral_calculator` maintainability and feature-boundary refactor wave

- Neutralized application ports so `application::ports` no longer imports `features::{natal,simplified,horoscope}` catalog types; the shared catalog records now live in `domain`, with compatibility re-exports preserved at historical feature paths.
- Removed the last `simplified -> natal` implementation dependency by moving `validate_calculation_references` to `astrology::validation`, exposing simplified scope codes from `features::simplified::resolve`, replacing `unwrap()` fail points with explicit `RuntimeError::InvalidEngineRequest`, and removing hard-coded reference-system ids from the planetary-only payload path.
- Completed the projection-builder split: `engine/projection/builder.rs` is now limited to orchestration/shared helpers, with the section builders moved under `engine/projection/builder/`.
- Invariants: no JSON public-contract change; `application` must stay free of `crate::features::*`; `features/simplified` and `features/horoscope` must not import `features::natal::*`; simplified runtime must consume resolved reference-system ids instead of canonical numeric literals.
- Verification: `cargo check -p astral_calculator`; `cargo test -p astral_calculator --test refactor_governance_tests`; `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`; `cargo test -p astral_calculator`; `cargo test -p astral_calculator_http --test astral_calculator_http_tests`.
- Reviews: `docs/reviews/astral_calculator_refactor/REV-PROJECTION-PORTS-SIMPLIFIED-2026-06-19.md`; `docs/reviews/astral_calculator_refactor/REV-PROJECTION-PORTS-SIMPLIFIED-2026-06-19-followup-1.md`; `docs/reviews/astral_calculator_refactor_feature_boundaries/REV-PROJECTION-PORTS-SIMPLIFIED-2026-06-19.md`; `docs/reviews/astral_calculator_refactor_feature_boundaries/REV-PROJECTION-PORTS-SIMPLIFIED-2026-06-19-followup-1.md`.

# 2026-06-19 - `astral_calculator` ports builders and DB fail-fast hardening

- Replaced the last direct `infra::db` couplings in `engine::calculation_refs`, `engine::projection::profiles`, and `features::horoscope::builders` with narrow application ports implemented by the SQL repositories.
- Removed the process-wide `OnceLock` caches from `engine::calculation_refs`; CLI/env reference-system resolution now reloads canonical DB-backed mappings through the port on each call.
- Split horoscope public-request building around a dedicated `HoroscopeBuilderCatalog` port and kept the public APIs `build_horoscope_daily_calculation_request_from_public` and `build_horoscope_period_calculation_request_from_public` unchanged apart from generic trait bounds.
- Replaced the simplified service dependency on `runtime::validate_calculation_references` with the canonical `features::natal::validate` module, keeping `runtime` as a composition facade only.
- Hardened DB-bound test helpers to fail fast with explicit PostgreSQL expectations, while moving `horoscope_builders_tests` to an in-memory fake catalog so builder logic is covered without DB.
- Invariants: no new public JSON contract change; `engine/*` and `features/horoscope/builders.rs` must not import `infra::db`; no process-global cache for canonical reference mappings; DB-dependent tests must error explicitly when PostgreSQL is unavailable.
- Verification: `cargo fmt`; `cargo test -p astral_calculator --test refactor_governance_tests`; `cargo test -p astral_calculator --test horoscope_builders_tests`; `cargo test -p astral_calculator`; `cargo test -p astral_calculator_http --test astral_calculator_http_tests`.
- Reviews: `docs/reviews/astral_calculator_refactor/REV-PORTS-BUILDERS-FAILFAST-2026-06-19.md`; `docs/reviews/astral_calculator_refactor_feature_boundaries/REV-PORTS-BUILDERS-FAILFAST-2026-06-19.md`.

# 2026-06-18 - `astral_calculator` refactor maintenance waves

- Removed horoscope daily runtime fake provenance from `astral_calculator`: public pure daily calculations now emit `derived_daily_calculator_v1`, while runtime service calls compute slot transits through Swiss Ephemeris and emit `swisseph_daily_calculator_v1`.
- Added `astrology::transits` as the reusable transit/aspect primitive for horoscope period and daily assembly, so product code no longer owns its own nearest-major-aspect implementation.
- Moved house cusp geometry out of `shared::astro_math` into `astrology::house_geometry`; `shared::astro_math` is kept to numeric/zodiac primitives without domain-type imports.
- Preserved public JSON contract shapes and legacy function names; `calculate_horoscope_daily_natal` and period `*_natal` wrappers still delegate to canonical functions.
- Added governance checks preventing fake horoscope calculator sources in runtime source, preventing `shared::astro_math` from importing domain types, and requiring the new adversarial reviews to remain closed.
- Verification: `cargo check -p astral_calculator`; targeted tests documented in the reviews below.
- Reviews: `docs/reviews/astral_calculator_refactor/REV-HOROSCOPE-REAL-DAILY-adversarial.md`; `docs/reviews/astral_calculator_refactor/REV-HOROSCOPE-REAL-DAILY-followup-1.md`; `docs/reviews/astral_calculator_refactor/REV-ASTROLOGY-TRANSITS-adversarial.md`; `docs/reviews/astral_calculator_refactor/REV-ASTROLOGY-TRANSITS-followup-1.md`; `docs/reviews/astral_calculator_refactor/REV-APPLICATION-PORTS-adversarial.md`; `docs/reviews/astral_calculator_refactor/REV-SHARED-ASTRO-MATH-adversarial.md`; `docs/reviews/astral_calculator_refactor/REV-RUNTIME-REPOSITORY-SPLIT-adversarial.md`.

# 2026-06-17 - Calculator HTTP rename and gateway decoupling

- Renamed the internal calculator HTTP adapter to `astral_calculator_http` across Cargo, Docker Compose, scripts, contracts, tests and active documentation. No transitional crate, binary or Docker service alias is kept.
- Kept the HTTP route contracts unchanged: canonical inter-service calls remain under `/v1/internal/calculations/*`, with existing `/v1/calculations/*` legacy route aliases still exposed by the calculator HTTP adapter.
- Decoupled `astral_gateway` from internal calculator and LLM crates. The gateway now owns its calculator HTTP client and talks to LLM through JSON internal endpoints; LLM-side horoscope calculation-request builders, writer builders and validators remain inside `astral_llm_api` / `astral_llm_application`.
- Invariants: `astral_calculator` remains free of HTTP dependencies; `astral_gateway` must not depend on `astral_calculator`, `astral_llm_application`, `astral_llm_domain` or `astral_llm_infra`; `astral_gateway` must not embed canonical reference data from `json_db`; active surfaces must not reintroduce the removed calculator HTTP service name.
- Verification: `cargo test -p astral_calculator --test refactor_governance_tests`; `cargo test -p astral_gateway`; `cargo test -p astral_calculator_http --test astral_calculator_http_unit_regression_tests`; `cargo test -p astral_llm_api --test contracts_publish_tests`; `docker compose config`.
- Reviews: `docs/reviews/astral_calculator_refactor/REV-CALCULATOR-HTTP-RENAME-2026-06-17.md`; `docs/reviews/astral_calculator_refactor_feature_boundaries/REV-GATEWAY-DECOUPLING-2026-06-17.md`.

# 2026-06-17 - `astral_calculator` refacto structurelle par contextes metier

- Recompose `astral_calculator/src` autour des contextes explicites `bootstrap/`, `shared/`, `natal/`, `simplified/`, `horoscope/` et `engine/`.
- Deplace les anciens fichiers racine `cli.rs`, `config.rs`, `db.rs`, `time.rs`, `idempotency.rs`, `facts.rs`, `aspects.rs`, `dignities.rs`, `ephemeris.rs` et `catalog.rs` dans leurs contextes cibles, avec facades de compatibilite minimales conservees dans `lib.rs`.
- Fusionne les zones de payload natal sous `natal/payload/`:
  `build/` pour la construction,
  `rules/` pour les invariants partages,
  `validate/` pour les controles de fraicheur/reutilisation.
- Remplace l’orchestration monolithique par des services explicites:
  `natal::application::NatalCalculationService`,
  `simplified::application::SimplifiedNatalService`,
  `horoscope::application::HoroscopeService`,
  `engine::application::EngineFacadeService`.
- Scinde l’acces DB par responsabilite via `ReferenceRepository`, `CatalogRepository`, `CalculationRepository`, `ProjectionRepository` et `HoroscopeRepository`, tout en conservant le SQL existant.
- Reduit `runtime` a une facade de compatibilite mince vers les nouveaux modules.
- Nettoie les vestiges non compiles de l’ancienne topologie (`src/features`, `src/application`, `src/engine_env.rs`, ancien `runtime/payload_freshness.rs`).
- Verification executee:
  `cargo check -p astral_calculator`
  `cargo test -p astral_calculator`
  `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`

# 2026-06-16 - `astral_calculator` structural layering refactor

- Added internal architecture layers for the calculator crate without changing JSON contracts: `application/`, `domain/`, `infra/db/`, and `features/`.
- Exposed the real layered structure instead of compatibility facades: `astral_calculator::domain`, `astral_calculator::features::*`, `astral_calculator::infra::db::*`, and `astral_calculator::runtime`.
- Split the former monolithic `domain.rs` into focused domain files for natal input, chart facts, references, scoring snapshots, and payload DTOs.
- Moved SQL-facing models and runtime repository code under `infra/db/`, and moved the natal runtime service orchestration under `application/`.
- Grouped functional modules under `features/` while keeping the previous public module paths intact.
- Introduced `BasicPayloadBuilderInput` and `build_basic_payload_from` as the internal canonical payload builder entry point, with existing builder functions kept as wrappers.
- Moved the simplified catalog DB loader under `infra/db/simplified_catalog_repository` with the existing simplified facade preserved.
- Verification: `cargo test -p astral_calculator`; `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`; `cargo test -p astral_calculator_http --test astral_calculator_http_tests`; `cargo test -p astral_llm_application simplified_reading_guard`; `cargo test -p astral_llm_application simplified_reading_postprocess`.

# 2026-06-16 - `payload_rules` refactor for shared natal payload rules

- Added an internal `astral_calculator::features::payload_rules` layer as the single source of truth for shared natal payload rules used by both payload build and runtime freshness validation.
- Moved the duplicated pure rules for `angles`, `chart_context`, `lunar_phase`, `reading_plan`, `rulership`, and canonical `house_axes` into the `payload_rules` layer (currently backed by `astral_calculator/src/features/payload_rules/`).
- Reduced the `payload` layer (currently backed by `astral_calculator/src/features/payload/`) to orchestration wrappers for those domains: it now prepares inputs, calls `payload_rules`, and assembles `BasicPayload`.
- Reduced `astral_calculator/src/runtime/payload_freshness/` to validation wrappers for those same domains: it now checks payload freshness through the same shared rules instead of recomputing them locally.
- Audited the `payload_shared` layer (currently backed by `astral_calculator/src/features/payload_shared/`) and kept only transversal helpers compiled there: aspect normalization/extraction, contract constants, and generic text/score predicates.
- Removed `payload_shared` ownership of natal visibility and canonical house-axis meaning; those domain rules now live in `payload_rules`.
- Added regression coverage for a wrap-around lunar phase mapping through the public payload builder and adversarial freshness mutations for shared chart-context and lunar-phase rules.

# 2026-06-16 - Horoscope calculator domain split

- Refactored horoscope support into `astral_calculator::features::horoscope` with focused submodules: `contracts`, `daily`, and `period`.
- Removed the root horoscope facade so the crate now exposes the actual feature path.
- Reused `astral_calculator::facts::normalize_degrees` from horoscope period logic instead of keeping duplicate math helpers inside the domain.
- Centralized horoscope RFC3339 UTC normalization and local-time-to-UTC conversion behind one shared helper layer so `builders` and period calculation no longer maintain duplicate temporal rules.

# 2026-06-16 - Swiss Ephemeris smoke test moved into `tests/`

- Moved the standalone `swe_smoke` binary into an integration test at [`tests/swiss_ephemeris_smoke_tests.rs`](tests/swiss_ephemeris_smoke_tests.rs).
- Declared the smoke check in `astral_calculator/Cargo.toml` so it runs with the `astral_calculator` test suite when `swisseph-engine` is enabled.
- Updated the calculator README to point operators to `cargo test` instead of `cargo run` for this diagnostic check.

# 2026-06-16 - Horoscope refactor review fixes

- Removed the new `horoscope -> aspects` coupling by moving shortest angular distance into `astral_calculator::facts`, where the other generic longitude helpers already live.
- Moved UTC/RFC3339 conversion helpers out of the `horoscope` domain into a crate-level internal `astral_calculator::time` module so the domain no longer owns transversal date/time utilities.
- Added explicit public regression tests for `calculate_horoscope_daily_natal` covering both the basic daily shape and the premium local slot path with `reference_datetime_utc`, local chart payload, and expected warnings.

# 2026-06-16 - Test regression hardening for astral_calculator_http

- Reduced duplication in [tests/astral_calculator_http_tests.rs](tests/astral_calculator_http_tests.rs) by reusing the shared period request fixture and transit snapshot helpers.
- Hardened the horoscope period regression checks so they locate the relevant Venus fact by content instead of relying on a fixed snapshot index.
- Kept the test coverage focused on public behavior: source provenance, context fallback, UTC normalization, and HTTP contract smoke checks.

# DB-backed reference migration - 2026-06-16

- Replaced the remaining `json_db`-backed reference lookups in `astral_calculator` with direct Postgres reads through `RuntimeRepository`.
- Added repository coverage for `astral_house_systems` and used existing SQLx row models for horoscope services, slots, time periods, scan profiles, orb weight bands, zodiacal systems, coordinate systems, house axes, and LLM projection profiles.
- Updated engine/horoscope/LLM projection tests to rely on the database-backed contract and to skip cleanly when the database is not available in the local environment.
- Kept the runtime strict-BDD: no fallback to `json_db` remains in the affected code paths.

# DB-backed reference model coverage - 2026-06-16

- Added SQLx row models in `astral_calculator/src/infra/db/models.rs` for:
  - `horoscope_services`
  - `horoscope_time_slot_profiles`
  - `astral_time_period_profiles`
  - `horoscope_scan_profiles`
  - `horoscope_orb_weight_bands`
  - `astral_zodiacal_reference_systems`
  - `astral_coordinate_reference_systems`
- Added repository read methods for the tables above so these seeded DB rows can be loaded directly from Postgres in `astral_calculator/src/infra/db/runtime_repository.rs`.
- Existing coverage already present for `astral_house_systems`, `astral_house_axis_definitions`, and `astral_llm_projection_profiles`.

# Service test UI grouped execution and observability - 2026-06-15

Refactored the local service test UI into grouped execution frames with richer
operator tooling for V2 public services.

# Token usage LLM detail, provider/model catalog normalization and test UI modal - 2026-06-15

Implemented an additive end-to-end token usage pipeline plus a normalized
provider/model catalog for LLM runtime pricing and observability.

- Added canonical token usage domain types with 4 categories:
  `input`, `output`, `cache`, `reasoning`, plus optional subtypes for cache
  `read` and `write`.
- Provider adapters now map detailed usage when the provider exposes it:
  OpenAI (`usage`, cached input, reasoning output), Anthropic
  (`input/output/cache_read/cache_write`), and Mistral
  (`prompt/completion/cached`).
- Added PostgreSQL tables `llm_token_usage_types`,
  `llm_generation_run_token_usages`, and `llm_generation_step_token_usages`.
  Legacy aggregate columns stay populated for backward compatibility.
- Normalized the provider/model catalog with `llm_providers`,
  `llm_provider_models`, and `llm_model_characteristics`, including pricing,
  limits, reasoning support, and source metadata.
- The generation runtime now prices token usage from the DB-backed model
  characteristics, persists run-level and step-level usage rows, and exposes a
  standard `token_usage` block with `summary`, `cost`, `engine`, and detailed
  items for run audit.
- `GenerateReadingResponse`, integration job envelopes, and `/v1/runs/{run_id}`
  now expose additive `token_usage` data while keeping `run_id`, `token_input`,
  `token_output`, `quality.used_provider`, and `quality.used_model`.
- The service test UI token modal now reads the enriched audit payload and shows
  input/output/cache/reasoning totals, estimated cost, and per-step usage with
  graceful `indisponible` fallbacks when a provider does not publish a metric.
- Added provider adapter regression tests for OpenAI, Anthropic, and Mistral
  detailed usage mapping, plus updated schema publication and UI fixture
  coverage.
- Added `scripts/sync_provider_model_catalog.py` and wired it into
  `scripts/lib/sync_llm_catalog.ps1` so official provider docs/API data can be
  pushed into the DB before runtime usage.

- Reworked `tests/service_test_ui/` into 4 main frames:
  input parameters, natal, horoscope (daily + period), and a non-active
  placeholder for future interpretations.
- Added automatic location resolution by default with a manual fallback switch.
- Added per-frame degraded/full mode switches and preserved explicit UI blocking
  when current backend constraints still require missing birth data.
- Reorganized service cards in stable `free -> basic -> premium` order and added
  sequential batch execution for the required groups:
  natal, horoscope daily, and horoscope period.
- Added per-service progress tracking with UI timers, step traces, JSON/result
  tabs, copy-text formatting with service name + timestamp, prompt modal, and
  token/cost modal with graceful fallback when backend data is missing.
- Added front-side audit adapters for `/api/llm/v1/runs/{run_id}` when a run id
  is exposed by the response, without making the UI depend on that backend path.
- Expanded `tests/service_test_ui/service-test-ui.test.html` to cover grouping,
  ordering, degraded-mode validation, copy formatting, usage fallback, and run
  id extraction.
- Final validation included iterative adversarial reviews focused on:
  batch-group structure, blocked CTA clarity, responsive behavior, disabled
  state handling, modal fallbacks, and runtime console health.
- Last adversarial cycle closed with no remaining actionable findings in the
  implemented UI scope.

# Horoscope period timeout stabilization - 2026-06-14

- Aligned the default/documented request timeouts for `astral_gateway`,
  `astral_calculator_http`, and `astral_llm_api` to `900000 ms` so
  `horoscope period` no longer hits a shorter outer timeout while the inner LLM
  path is still allowed to run.
- Updated the gateway inbound timeout layer to use the configured timeout plus a
  `5s` margin, matching the existing LLM API behavior.
- Kept the generic gateway LLM timeout retry for daily/general calls, but
  removed it from `render_horoscope_period` so a long period render is not
  duplicated after an HTTP timeout.
- Reduced Premium period V2 generation cost by cutting the quality retry budget
  from `2` retries to `1`, lowering the main writer output budget from `16000`
  to `12000`, and introducing a dedicated `8000` token budget for the quality
  editor path.
- Added targeted regression coverage for gateway retry semantics, gateway
  timeout margin calculation, period writer/editor token budgets, and the
  one-retry limit of the period quality loop.

# Audit PostgreSQL des prompts finalisés LLM - 2026-06-15

Ajout d’une persistance PostgreSQL des prompts finalisés réellement envoyés au LLM, exposée dans l’audit interne `GET /v1/runs/{run_id}`.

- Nouvelle table `llm_generation_prompt_traces` pour stocker plusieurs prompts par run, y compris les retries et repairs.
- Chaque trace persiste `run_id`, `chapter_code`, `step_type`, `attempt`, `prompt_family`, `prompt_version`, `message_count`, `compiled_prompt` et `messages_json`.
- Le point de stockage est centralisé dans `ProviderRouter` juste avant chaque appel provider, à partir du `ProviderGenerationRequest` final.
- Les runs parent sont désormais upsertés en base avant exécution puis finalisés après génération, afin que les prompt traces soient rattachées à un audit exploitable.
- La vue d’audit renvoyée par `/v1/runs/{run_id}` inclut maintenant `prompt_traces` en plus des `steps`.
- Les flows couverts sont : single-pass natal, orchestration par chapitres, summary/final synthesis, horoscope daily, horoscope period et leurs repairs/retries.
- La journalisation fichier existante sous `output/logs/prompts` reste active comme aide locale, mais la source d’audit principale devient PostgreSQL.

# UI de test - affichage des prompt traces LLM - 2026-06-15

Raccord de l’UI de test pour exploiter `prompt_traces` dans le bouton `Voir le prompt`.

- `tests/service_test_ui/service-test-ui.js` normalise désormais `audit.prompt_traces` en source principale, avec fallback sur les anciens champs `prompt` si l’audit multi-traces n’est pas disponible.
- Le modal prompt affiche une liste ordonnée de traces, avec metadata compacte (`step_type`, `chapter_code`, `attempt`, horodatage), puis `compiled_prompt` et `messages_json`.
- `promptAvailable` reflète maintenant la présence d’au moins une trace normalisée, au lieu d’un champ direct arbitraire.
- Les détails techniques distinguent `Prompt: disponible (n trace(s))`, `Prompt: audit indisponible` et `Prompt: non expose par le backend`.
- La couverture `tests/service_test_ui/service-test-ui.test.html` valide le prioritaire `prompt_traces`, le multi-prompts ordonné, le fallback legacy, l’audit indisponible et la conservation d’une trace partielle avec `messages_json`.

# Horoscope real-provider local guard - 2026-06-14

Added a local-provider guard for horoscope test runs so the UI and integration
path are explicit about when a real LLM provider is expected instead of the
`fake` provider.

- Documented the exact `.env` overrides needed to force a real provider in
  local debug runs: `ASTRAL_LLM_ENABLE_FAKE=false`, `ASTRAL_LLM_DEFAULT_PROVIDER`
  set to a real provider, and the matching API key.
- Added `horoscope` to `config/llm_product_models.conf` so the product-level
  `llm_product_default_engine` row does not keep overriding the real `.env`
  provider with `fake`.
- Updated `scripts/docker_update_integration_stack.ps1` so its local fake smoke
  phase also enables the temporary horoscope fake override before calling
  `/v2/horoscope/*`.
- Added regression coverage in
  [tests/horoscope_real_provider_guard_tests.rs](tests/horoscope_real_provider_guard_tests.rs)
  for the daily and period horoscope render paths. The tests use a real-provider
  fixture and fail if the rendered response falls back to `fake` instead of the
  configured provider.
- Added one real-provider JSON repair retry for daily horoscope renders. When a
  real provider returns text that is not parseable JSON, the API retries once
  with the same real provider, same model and strict schema instead of falling
  back to local fake output.
- Daily horoscope real-provider calls now force minimal reasoning effort and
  larger output budgets (Free 4k, Basic 8k, Premium 12k tokens) so GPT-5 does
  not spend the whole output budget on reasoning and return an incomplete
  response with no assistant JSON text.
- Daily horoscope post-processing now sanitizes public slot fields so real LLM
  outputs such as `[morning]`, `slot:morning`, `slot_code`, `slot_` or `avoid_`
  are replaced/removed before the public leak validator runs.
- Free period post-processing no longer canonicalizes `key_days` wording. It
  preserves provider text when the structure is valid and keeps only technical
  repair, evidence alignment, and neutral length expansion for short Free
  outputs.
- Free period public word-count validation now counts Free-specific fields
  (`summary`, `dominant_theme`, `advice`, `watch_summary`) and expands a too
  short real-provider response with neutral guidance before hard validation.
- Gateway timeout handling now uses `ASTRAL_GATEWAY_REQUEST_TIMEOUT_MS` for the
  HTTP route timeout instead of a hard-coded 60 seconds, now aligned to the
  `900000 ms` horoscope-period timeout defaults used across gateway,
  calculator and LLM. Gateway LLM HTTP calls retry once only
  on real timeout cases (`reqwest` timeout or HTTP 408) and never retry
  validation failures such as HTTP 422.

# Horoscope contract-first simplification - 2026-06-13

Simplified horoscope post-LLM validation so daily and period generation now block
only on output-contract and integration concerns, not on editorial taxonomy or
mechanical prose heuristics.

- Daily horoscope validation keeps schema and canonical `evidence_keys`, but no
  longer rejects repeated slot bodies, copied overview phrasing, generic wording
  or missing explicit astrological vocabulary in each slot.
- Legacy period horoscope validation keeps schema, date-range, required
  sections, canonical `evidence_keys` and canonical window snapshot keys, but
  no longer blocks on repetitive daily phrasing, meta-personalization wording,
  editorial scaffolding phrases, or recalendarized strategy prose.
- Premium period blocking validation no longer rejects meta watch-window titles
  or generic best-window phrasing when the structured contract remains valid.
- Added [docs/horoscope_generation_architecture.md](docs/horoscope_generation_architecture.md)
  to freeze the target split:
  calculator facts -> persona-aware writer request -> LLM prose -> minimal
  contract validation -> non-blocking editorial audit.

Validation:

- `cargo test -p astral_llm_api --test horoscope_tests`

Adversarial follow-up:

- Removed a false-positive daily lexical gate that rejected the public English
  word `day`.
- Removed legacy period blocking on public taxonomy words such as
  `organization`, `relationship`, `energy`, `clarity`, and `integration`.
- Hardened the simplification so legacy period editorial rewrites are now
  `fake`-provider only. Real provider prose keeps its own narrative after shape
  sanitization and contract repair.
- Added a regression test proving `repair_period_response_shape` does not
  rewrite real-provider editorial prose.

# Horoscope Period V2 hard/soft validation split - 2026-06-13

## Scope

Refactored only the `SemanticBriefV2` post-generation validation path for
`horoscope period` so deterministic contract checks remain blocking while
editorial heuristics become non-blocking audit warnings.

## Behavior

- Kept the public `horoscope_period_response` contract unchanged.
- Reduced `validate_period_response_contract_gates_v2()` to hard gates only:
  schema, request/response identity, period dates, evidence keys,
  snapshot-source keys, marker overlaps, watch/domain/evidence packaging,
  Premium structure, and manifest word-count violations.
- Added a dedicated V2 hard gate for real technical leaks in public text, such
  as field names or internal identifiers like `theme_code`, `evidence_key`,
  `snapshot_key`, `scan_plan`, and similar non-publishable strings.
- Split Premium detail validation into:
  - structural blocking checks for required Premium blocks and minimum shape;
  - non-blocking audit warnings for re-calendarization in `advice` and
    `strategy`.
- Added typed V2 audit warnings with stable codes and enum severity:
  `PeriodV2QualitySeverity` and `PeriodV2QualityWarning`.
- Enriched `period_v2_editorial_audit()` with a `warnings` array while keeping
  the existing non-blocking metrics.
- Ensured `validate_period_response_quality_gates_v2()` and the V2 retry loop
  only react to hard failures; warnings never trigger
  `period_style_editor_response_v2()`.

## Validation

- `cargo test -p astral_llm_api --test horoscope_tests`

# Horoscope Period V2 semantic brief - 2026-06-11

## Payload shared invariants refactor - 2026-06-12

Refactored shared payload invariants without changing payload generation,
runtime freshness validation, or payload reuse decisions.

- Added an internal `payload_shared` module for pure,
  shared helpers only: contract constants, aspect pair normalization/extraction,
  horizon and sect mapping, canonical house-axis definitions, and small text or
  score predicates.
- Kept the `payload` module as the payload builder layer and
  `astral_calculator/src/runtime/payload_freshness/` as the persisted payload
  validator layer.
- Rewired duplicated helpers in `angles`, `signal_filters`, `aspects`,
  `chart_context`, `placements`, `house_axes`, and accidental dignity checks to
  the shared module when the rule was byte-for-byte equivalent.
- Deliberately left builder-only and validator-only orchestration logic local to
  each module to stay KISS and avoid abstracting divergent behavior.
- Added characterization coverage in
  `tests/payload_shared_characterization_tests.rs` for shared angle filtering,
  shared visibility or sect mapping, and canonical house-axis behavior through
  public payload APIs.

Validation:

- `cargo test -p astral_calculator --test payload_tests`
- `cargo test -p astral_calculator --test runtime_tests`
- `cargo test -p astral_calculator --test contract_basic_v8_tests`

## Horoscope application module split - 2026-06-12

Refactored `astral_llm_application/src/horoscope/mod.rs` into focused
`horoscope` submodules without changing public API names or response contracts.

- Kept `horoscope/mod.rs` as a lightweight module facade with compatibility
  `pub use` exports for the existing integration tests and orchestrator calls.
- Split shared concerns into local modules for service codes, types, schemas,
  reference data, text helpers, errors, writer engine defaults and
  orchestrators.
- Moved daily horoscope responsibilities under `horoscope/daily/` and period
  responsibilities under `horoscope/period/`.
- Reused existing application services for French typography and text
  reprocessing instead of creating duplicate global helpers.

Validation:

- `cargo check -p astral_llm_application`
- `cargo test -p astral_llm_api --test horoscope_tests`
- `cargo test -p astral_llm_api --test contracts_publish_tests`
- `cargo test -p astral_llm_application --test text_reprocessing_application_tests`
- `cargo test -p astral_llm_application french_typography`

## Premium 7-day V2 contract-only gates - 2026-06-12

Refactored `horoscope_premium_next_7_days_natal` V2 validation so public prose
is no longer accepted or rejected by hardcoded lexical markers.

- Added `validate_period_response_contract_gates_v2()` for schema, dates,
  evidence keys, source snapshot keys, watch coherence, Premium sections,
  window overlap and word-count validation.
- Routed `semantic_brief_v2` retries and final orchestrator validation through
  the V2 contract gate instead of the legacy evidence/public-text validator.
- Added `period_v2_editorial_audit()` in the debug envelope as non-blocking
  metadata; it is not injected into `reading.quality`.
- Added explicit V2 identity checks so the response `service_code` and
  `period_resolution` must match the writer request.
- Removed the V2 forbidden-public-pattern validator from the blocking path and
  updated tests so old lexical failures are audited, not rejected.
- Stopped V2 watch-summary postprocess from replacing `status = none` text with
  hardcoded public prose.
- Stopped V2 postprocess from replacing time/title mismatches with hardcoded
  window titles; the mismatch is now audit-only.
- Kept legacy Premium window repair behavior intact while V2 keeps provider
  prose and lets contract failures trigger retries.

Validation:

- `cargo fmt`
- `cargo test -p astral_llm_api --test horoscope_tests`

## Horoscope test helpers relocation - 2026-06-12

### Scope

Moved horoscope-specific test helper logic out of
`astral_llm_application/src/horoscope/mod.rs`.

### Behavior

- Removed `_for_test` wrapper functions from the application horoscope module.
- Exposed the underlying period writer/audit/word-count functions under their
  production names for integration-test coverage.
- Kept the prompt text aggregation helper local to `tests/horoscope_tests.rs`.

### Validation

- `cargo test -p astral_llm_api --test horoscope_tests`

## Premium 7-day V2 naturalized evidence guard - 2026-06-12

### Scope

Fixed a false negative in the Premium 7-day V2 post-safety evidence validator
after real run `cb3d7119-53be-4560-816e-e67dd4affe00` failed with
`HOROSCOPE_PERIOD_EVIDENCE_MISSING` for
`week_overview_missing_natal_personalization`.

### Behavior

- The public reading already contained astrological/natal anchors, but the
  personalization detector only accepted a narrow set of explicit UX phrases.
- The detector now accepts naturalized astrological markers such as natal,
  Lune, Soleil, Venus, Mars, Mercure, Jupiter, Saturne, carre and opposition
  as valid personalization evidence.
- The public contract remains unchanged; this only adjusts validation of
  generated Premium 7-day V2 readings.

### Validation

- Added a regression test in `tests/horoscope_tests.rs` using the same
  Jupiter/Saturn overview style seen in the failed real run.

## Premium 7-day V2 test UI catalog replacement - 2026-06-12

### Scope

Replaced the public test UI/catalog entry for `horoscope_premium_next_7_days_natal`
with the explicit `Horoscope Premium 7 prochains jours V2` wording while keeping
the service code and public response contract unchanged.

### Behavior

- `json_db/llm_integration_services.json` now advertises the Premium 7-day service
  as V2 and uses `payload.target_language_code = "fr"` in its example request.
- The service test UI builds horoscope payloads with `target_language_code` and
  keeps the readable display path on `$.result.reading`; debug envelopes remain
  technical inspection material only.
- The fake Premium 7-day smoke script submits `target_language_code` while keeping
  its debug assertions as technical validation, not as UI display guidance.
- `docs/integration_api_guide.md` documents the V2 section and the UI rule to
  consume only `$.result.reading`.
- `scripts/docker_update_integration_stack.ps1` fails early if the local
  catalogue no longer exposes the Premium 7-day V2 label, unchanged public
  contracts, or the `target_language_code` example payload.

### Validation

- Added catalog assertions in `tests/integration_services_tests.rs` for the V2
  label, V2-compatible example payload, unchanged contracts, beta availability
  and sort order.

## Premium 7-day V2 editorial quality iteration - 2026-06-11

### Scope

Improved the real OpenAI `horoscope_premium_next_7_days_natal` V2 editorial path after certification run `aa283161-6fe0-4260-a824-1a4ac1a8f8d8` showed technically valid output with a functional, repetitive feel.

### Behavior

- `semantic_brief_v2` now includes internal-only `editorial_arc`, `editorial_angles`, and `section_roles` material so the writer can create a readable opening/pivot/consolidation/closure arc and give each day a distinct human angle.
- The V2 writer prompt now asks for premium editorial judgement instead of mechanical style gates: overview as trajectory, timeline as lived daily guidance, domains as transversal synthesis, windows as concrete time-bound uses, and strategy as arbitration.
- V2 postprocess/repair now applies objective, non-stylistic cleanup only: noon/afternoon windows cannot keep a morning title, `watch_summary.status = none` remains neutral with empty evidence keys when no watch carriers exist, and deterministic French fixes normalize `demi-journée` and `réorganiser`.
- V2 keeps the public response contract unchanged; the non-blocking editorial audit is computed as internal/test metadata with public word count, section counts, repeated-term observations, duplicate titles, and window/title mismatches. These metrics guide certification review but do not fail a generation.

### Validation

- Added focused tests in `tests/horoscope_tests.rs` for the editorial brief, prompt guidance, objective postprocess cleanup, and non-blocking audit metadata.

## Scope

Implemented the first active `semantic_brief_v2` path for `horoscope_premium_next_7_days_natal`.
Free and basic 7-day services remain on `legacy_v1` according to the initial Premium-only brief.

## Behavior

- `json_db/horoscope_services.json` now carries `generation_mode = "semantic_brief_v2"` only for Premium 7 days; free/basic 7 days carry `legacy_v1`.
- `HoroscopePeriodNatalOrchestrator` routes by `generation_mode`, preserving legacy V1 as rollback.
- Premium V2 builds `horoscope_period_writer_request` from atomic evidence, events and `semantic_brief` instead of public-like `daily_plans` or editorial prose.
- V2 supports `target_language_code` (`fr`, `en`, `es`, `de`) while temporarily accepting legacy `target_language`; if both diverge, `target_language_code` wins and the V2 debug envelope records `legacy_target_language_ignored`.
- V2 accepts bounded `astrologer_persona` values that cannot override safety, schema, evidence, dates or target language. The Rust payload and V2 schema expose the same persona fields only: `persona_id`, `tone`, `lexical_field`, `priority_domains`, `avoid_style` and `interpretation_style`. The writer request always includes `astrologer_persona`, using `null` when absent.
- `semantic_brief_v2` is internal writing input only: it contains period-level keywords/tone/intensity plus atomic daily/candidate material, never `semantic_brief.evidence` or public prose. Sanitized `evidence` stays top-level and candidates reference it only through `evidence_keys`.
- Premium V2 fake provider, response repair and postprocess avoid legacy co-writing functions.
- Adversarial review hardened the V2 path: Premium-only schema constants, exact semantic brief keys, atomic window candidates, strict evidence sanitization, no legacy language requirement, no V2 prompt language fallback, targeted issue-based quality retry validated by V2 gates, and no public prose added by V2 repair/postprocess.
- UI consumers must read only `$.result.reading`; `calculation`, `interpretation_request`, V2 `writer_request`, `semantic_brief`, `evidence` and quality diagnostics are debug-only.
- Detailed tracking lives in `docs/horoscope_period_v2_migration.md`.

## Validation

- `cargo check -p astral_llm_application`
- `cargo test -p astral_llm_api --test horoscope_tests`
- `cargo test -p astral_llm_api --test contracts_publish_tests`
- `cargo test -p astral_llm_application`
- `python scripts\import_json_db_to_postgres.py --dry-run --output target\astral_json_db_import_v2.sql`

## Real OpenAI certification

- Added `scripts\test_horoscope_premium_next_7_days_v2_openai.ps1` to run real Premium 7-day V2 certification from `.env` OpenAI credentials.
- The script requires `OPENAI_API_KEY`, verifies the active LLM provider is not `fake`, creates or reuses a natal `chart_calculation_id`, runs `horoscope_premium_next_7_days_natal` with `target_language_code` for `fr`, `en`, `es`, `de`, saves one request/response per language, and writes `summary.json`.
- If the running API still exposes the older public schema and rejects `target_language_code`, the script retries that language with legacy `target_language`, records `payload_mode = target_language`, and still validates that the completed V2 debug `writer_request.target_language_code` matches the requested language.
- It validates the V2 debug boundary (`interpretation_request == writer_request`), writer request contract markers, top-level evidence, no `semantic_brief.evidence`, public reading shape, non-fake provider quality metadata, 7-day timeline, windows/domains, and absence of internal technical fields in public text.
- Real OpenAI V2 writer/editor calls use compact JSON prompts, minimal reasoning and a 16k output budget; targeted retry metadata stays out of the public `reading` schema. V2 postprocess may only normalize technical consistency, such as `watch_summary.status = active` to `low` when there are watch windows but no watch days, or internal `theme`/`tone` codes inside short public label fields, without rewriting public prose.
- V2 postprocess also prunes duplicated `watch_windows` when OpenAI copies an existing `best_windows` identity (`date` + `source_snapshot_keys`) into the vigilance section; if no vigilance remains, `watch_summary` is technically reset to `none` without adding text.
- V2 public text validation no longer treats ordinary wording such as `focus`, `organization`, `clarifier`, `ajuster` or `intégrer` as a hard failure. It only rejects real internal leaks such as field names, prompt metadata, evidence key labels, snapshot key labels and semantic-brief/debug terms. Postprocess does not rewrite public prose to chase lexical variants.
- Premium V2 still prompts for the canonical `1600-2600` word target and keeps the `3200` hard limit, but final validation accepts a narrow 100-word under-target tolerance. A real output just below the target is not rejected for mechanical threshold reasons, while substantially short output still triggers quality retry/failure; Rust does not pad or complete the prose.
- Premium V2 validation treats `watch_windows` as valid vigilance carriers: `watch_summary.status = active` is accepted when either `watch_days` or `watch_windows` is populated.
- Usage: `.\scripts\test_horoscope_premium_next_7_days_v2_openai.ps1`.

# Simplified E2E fake provider restore - 2026-06-10

## Scope

Fixed the Docker-backed PostgreSQL helper used by simplified natal E2E provider switching when local `psql` is unavailable.

## Behavior

- `scripts/lib/simplified_e2e_llm_provider.ps1` now invokes `docker compose exec -T postgres psql ...` through `System.Diagnostics.ProcessStartInfo` with stdout/stderr explicitly redirected.
- SQL is written to `psql` through stdin instead of `-c <sql>`, avoiding both the PowerShell `StandardOutputEncoding` failure and Windows command-line length failures during large profile JSON restoration.

## Validation

- `Invoke-SimplifiedE2ePsql -Sql "SELECT 1;"` through the Docker fallback path.
- `Invoke-SimplifiedE2ePsql` with a 40,000-character SQL payload through the Docker fallback path.

# Premium 7-day horoscope finishing guards - 2026-06-10

## Scope

Hardened the final public text cleanup for `horoscope_premium_next_7_days_natal` after a real Premium run exposed glued French compounds and residual template wording in domain sections.

## Behavior

- French typography reprocessing now restores glued compounds such as `rendezvous`, `bouclezla`, `laissezle`, `faitesle`, `retirezvous`, `réduisezle`, `allégezle`, `phraseclé`, and common imperative forms glued to `le`, `la`, or `vous`.
- Period validation rejects those glued compounds with `HOROSCOPE_PERIOD_FRENCH_TYPOGRAPHY_FAILED` if they reach public text.
- Period typography reprocessing normalizes accidental `. .` double punctuation.
- The Premium period prompt now explicitly asks for correct French compounds and forbids glued imperative forms before generation.
- Premium `best_days` and `watch_days` fallback reasons now transform associated situations into short natural sentences instead of serializing `autour de vérifier...` lists.
- The Premium period prompt now tells the model to transform associated situations into natural marker reasons instead of copying list fragments.
- Period validation rejects mechanical public marker patterns such as `autour de vérifier`, `autour de attendre`, `: appuis concrets aide`, and `. .`.
- Premium domain sections no longer repair toward repeated `Dans ce domaine...` or `Cette énergie est utile...` templates.
- The Premium period prompt now forbids repeated domain templates such as `Dans ce domaine...`, `Cette énergie devient utile...`, and `les repères les plus utiles consistent...`.
- Domain section repair rewrites those templates into a transverse reading sentence based on the canonical domain focus.
- Weak trajectory wording such as `Le mouvement relie vos repères personnels, les appuis émotionnels et les choix à consolider` is normalized into a concrete weekly arc.
- Fake period writer output now runs through the same shape repair path as provider output, so local fake smokes cannot bypass marker/domain naturalization.
- Additional finishing pass now repairs truncated example tails such as `(par ex.` and rejects them if they survive public validation.
- French glued compound repair now covers `utilisezles`, `revenezy`, `arrêtezvous` and `joursclés`.
- Premium marker reasons are condensed to avoid repeating full associated-situation lists in `key_days` and `watch_days`.
- Premium best-day reasons avoid taxonomy wording like `point d'appui pour appuis concrets`.
- Premium best-window reasons replace generic filler with concrete uses such as confirming a resource, closing a task, requesting proof or sending a targeted message.
- Premium advice fallback now provides a denser method of use instead of short generic guidance.

## Validation

- `cargo test -p astral_llm_api --test horoscope_tests`
- `cargo test -p astral_llm_application --test text_reprocessing_application_tests`
- `cargo test -p astral_llm_application french_typography`
- `.\scripts\test_horoscope_basic_next_7_days_fake.ps1`
- `.\scripts\test_horoscope_premium_next_7_days_fake.ps1`

# LLM model alias cleanup - 2026-06-10

## Scope

Replaced deprecated OpenAI model aliases across tracked code, configuration, tests, contracts, scripts, and documentation.

## Behavior

- The legacy 5.4 mini alias is now `gpt-5-mini`.
- The legacy 5.4 nano alias is now `gpt-5-nano`.
- Product defaults, natal interpretation profiles, provider catalogue seeds, Docker defaults, benchmark scripts, and test expectations now use the updated aliases.

## Validation

- No tracked occurrences of the legacy mini/nano 5.4 aliases.

# Premium 7-day horoscope editorial direction - 2026-06-10

Refactored `horoscope_premium_next_7_days_natal` writer guidance to reduce mechanical prose.

## Follow-up: narrative arcs and usage domains

- Added `json_db/horoscope_period_editorial_arcs.json` so repeated period themes receive distinct narrative functions, especially recurring `integration` signals: friction, reality test, recovery, and closure.
- Added `json_db/horoscope_period_public_themes.json` so public labels and domains use usage-oriented wording such as `Engagements et limites`, `Échanges à cadrer`, `Appuis concrets`, and `Énergie mentale`.
- Watch windows now reuse the editorial arc for repeated themes, preventing duplicated `title` + `watch_point` pairs.
- Premium tests now assert unique `reader_situation` values, usage-oriented public domains, and non-duplicated watch-window prompts.

## Changes

- Added a Premium-only `editorial_brief` to the period interpretation request. It gives each of the 7 dates a human role, narrative function, reader situation, action mode, contrast with the previous day, and angle to avoid reusing.
- Added canonical editorial role data in `json_db/horoscope_period_editorial_roles.json`, consumed by the runtime instead of hardcoded role mappings.
- Replaced the Premium period prompt's lexical anti-repetition instructions with editorial orchestration: distinct day functions, usable windows, non-duplicative domains, and strategy synthesis.
- Adjusted Premium period generation temperature from `0.4` to `0.55` while keeping structured output validation.
- Removed the period post-processing layer that rewrote public labels and canonicalized key-day/window wording; the runtime now keeps only structural repair, technical key restoration, and overlap pruning, while the text reprocessing adapter no longer applies `HumanizeLabels` to `horoscope_period`.
- Updated `premium_rich` word targets from `2200-3200` to `1600-2600`, with the hard limit still at `3200`, to reduce forced filler when source signals are not dense enough.
- Added regression coverage in `tests/horoscope_tests.rs`.

# Premium 7-day horoscope readability - 2026-06-09

## Follow-up: LLM-owned marker wording and evidence basis

- Premium `key_days`, `best_days`, and `watch_days` wording is preserved from the provider when valid; marker dates and `evidence_keys` stay canonical from the request so provider-invented keys cannot pass through repair.
- The Premium prompt now asks the LLM to make marked days understandable inside the matching `daily_timeline` entry, without code-side prose rewriting.
- `evidence_summary` is explicitly treated as the section of evidence keys used to support the interpretation; repair keeps canonical dates and keys while preserving provider labels when valid.
- Premium `domain_sections` now keep canonical evidence keys and re-check natal/personal anchoring after cleanup so compacting cannot remove the required personalization.
- Domain section validation relies on canonical `evidence_keys`; it no longer rejects natural domain prose only because it avoids a narrow personalization keyword list.
- Mechanical wording such as `devient plus lisible` is now rejected by validation so the provider regenerates naturally instead of being rewritten by code.
- Premium validation keeps checking that public evidence keys are canonical; repetition avoidance is handled by prompt constraints, not by code-side prose or key rewriting.
- The Premium period prompt now explicitly bans mechanical wording such as structural signal language, “thème ... lisible”, “relief principal”, “timeline”, and trajectory phrasing.
- Formatting and typographic reprocessing remains limited to cleanup/guard behavior; it does not rewrite provider style into canned prose.

## Tests

- Added regression coverage in `tests/horoscope_tests.rs`.

## Scope

Improved the editorial structure of `horoscope_premium_next_7_days_natal`.

## Behavior

- Premium day markers now explain their role as structural, supportive, or vigilance markers instead of repeating a generic “repere utile” sentence.
- The Premium period prompt now enforces a clearer reading flow: overview, short period markers, daily timeline, domains, hourly windows, then strategy.
- Premium advice and strategy are synthesis sections only; they must not introduce new explicit dates after the timeline and windows have already listed the dated details.
- Domain section post-processing restores canonical evidence by domain, title, or original section order when the model renames a domain or returns empty evidence arrays.
- Overview and domain post-processing now add concrete personal anchors (`vos priorités`, `votre agenda`, owner/deadline/proof criteria) when the provider returns text that is astrologically plausible but too implicit for the natal personalization guard.

## Tests

- Added regression coverage in `tests/horoscope_tests.rs`.

# E2E real stabilization - 2026-06-08

# Premium daily horoscope editorial hardening - 2026-06-09

## Scope

Improved the real LLM output quality for `horoscope_premium_daily_local_2h_slots` and cleaned the local readable rendering.

## Behavior

- Strengthened the Premium daily writer prompt against repeated slot reasons, repeated mechanical phrasing, and public leakage of domain codes in titles or texts.
- Added post-generation repair for duplicated `best_slots` / `watch_slots` reasons by reusing the matching timeline sentence when available.
- Added validation that rejects repeated slot summary reasons if repair cannot make them distinct.
- The service test UI no longer renders the structural `domain` code as reader-facing metadata.

## Tests

- Added regression coverage in `tests/horoscope_tests.rs`.
- Extended `tests/service_test_ui/service-test-ui.test.html`.

# Service test UI horoscope slot labels - 2026-06-09

## Scope

Fixed the local service test UI rendering for horoscope payload arrays that carry both a human title and a public `slot_label`.

## Behavior

- `slot_label`, `day_label`, `date`, and `domain` are now preserved as visible metadata when they are not already used as the section title.
- Premium daily horoscope `best_slots`, `watch_slots`, and `timeline` entries now show their 2-hour labels in the readable tab.
- The JSON contract and engine output are unchanged.

## Tests

Added focused coverage in `tests/service_test_ui/service-test-ui.test.html`.

# LLM text rendering dash normalization - 2026-06-09

## Scope

Added a central LLM text reprocessing operation that normalizes em dashes (`—`) to ASCII hyphens (`-`) across rendered public text.

## Behavior

- Added `normalize_dashes` to the text reprocessing operation contract.
- Applied dash normalization in the central pipeline before French typography and length/repetition processors.
- Wired the operation into the shared, horoscope, natal theme, natal simplified, and calculator projection rendering adapters.
- Added an adapter-level guard so rendering adapters include dash normalization even when a caller passes a narrowed operation list.
- Exposed dash-normalized fields in simplified reading post-processing and execution audit records.
- Preserved technical string fields such as codes, ids, keys, and roles.

## Tests

Added focused regression coverage in `tests/text_reprocessing_application_tests.rs`.

## Scope

Stabilized the real end-to-end scenarios present in `scripts/` for horoscope and natal reading flows.

## Horoscope period payloads

- Aligned the OpenAI provider response schema with the public response shape used by the service.
- Removed variant-specific fields from both `properties` and `required` in the provider schema.
- Pruned variant-specific fields during post-processing:
  - free responses keep free-only fields;
  - basic responses remove free-only and premium-only fields;
  - premium responses remove free-only fields and keep premium period fields.
- Re-applied word-count enforcement after text reprocessing and pruning so premium period payloads remain inside their configured bounds.
- Restored canonical request evidence keys when a provider returns empty `evidence_keys` arrays for period markers or domain sections.
- Treated blank evidence keys as absent so canonical request fallbacks are used.
- Re-ran period repetition normalization after provider text reprocessing and word-bound enforcement.

## Natal readings

- Removed repeated symbolic disclaimer boilerplate from generated chapter bodies during natal theme text reprocessing.
- Kept the legal disclaimer field untouched.
- Kept meaningful interpretation sentences when removing boilerplate fragments.
- Ignored repeated structural astrology anchors such as `milieu du ciel`, house labels, and planet-placement trigrams in the repeated-trigram quality counter, while keeping editorial phrase repetition blocking.
- Limited structural astrology trigram exemptions to numbered houses and real zodiac placements so non-placement repetitions such as `Mars en tension` still count.
- Kept useful chapter sentences such as `Cette lecture symbolique met en lumière...` and generic exploratory hypotheses.

## Tests

Added focused regression coverage in:

- `tests/horoscope_tests.rs`
- `tests/text_reprocessing_application_tests.rs`
- `astral_llm/crates/astral_llm_application/src/text_trigrams.rs`

Validated with the real E2E scripts and supporting Rust test suites.

# Local real service test UI - 2026-06-09

## Scope

Added a developer-only static UI for manually exercising the real local calculator and LLM integration services from a birth input.

## Location

- UI files: `tests/service_test_ui/`
- Launcher/proxy: `scripts/start_service_test_ui.ps1`
- Shutdown behavior: `scripts/start_service_test_ui.ps1` now polls `HttpListener.GetContextAsync()` every 250 ms and exits cleanly when the listener stops, so the local UI stops quickly even when idle.
- Browser logic test page: `tests/service_test_ui/service-test-ui.test.html`

Start it with:

```powershell
.\scripts\start_service_test_ui.ps1
```

Then open `http://localhost:8099/`.

## Runtime prerequisites

- Docker stack running with calculator `:8080`, LLM API `:8081`, PostgreSQL and `astral_llm_worker`.
- Integration services submitted in DB with `.\scripts\manage_integration_services.ps1 -Submit`.
- `.env` configured for the local stack.
- `OPENAI_API_KEY` is required for real provider-backed generations. Fake-provider local smoke flows still depend on the backend configuration.
- The local proxy loads `ASTRAL_LLM_API_KEY` and `ASTRAL_CALCULATOR_API_KEY` from `.env` and injects them server-side when the UI fields are empty. If needed, the UI fields can override those values for a browser session.
- After catalogue changes, run `.\scripts\manage_integration_services.ps1 -Submit` so the running DB exposes the updated service list, including `natal_premium`.

## Behavior

- The UI loads `GET /v1/services` through the local proxy and displays only `active` and `beta` services.
- `natal_premium` is listed as a beta real full-natal service and uses a rich engine projection.
- `natal_premium` chapter profile was strengthened to reduce under-length failures: target `360` words per chapter, minimum `260`, body structure `4 x 60-110` words, with explicit expansion focus on `relationships`, `career`, `communication_mind`, `family_roots`, `money`, `family`, and `growth_path`. The default premium E2E fixtures now request 7 domains and include `communication_mind` plus `family_roots`.
- It resolves city/country through `/api/geocode`, backed by Nominatim/OpenStreetMap.
- It submits real jobs through `POST /v1/jobs` with a unique `Idempotency-Key`, then polls `GET /v1/jobs/{run_id}`.
- For horoscope services, it first calls the calculator natal endpoint to obtain `chart_calculation_id`, then submits the horoscope job.
- Each service result has a readable view and a raw formatted JSON view.
- Horoscope daily keeps internal `watch_point` codes in the interpretation request, but public Basic `reading.slots[].watch_point` and Premium `reading.timeline[].watch_point` now use `public_watch_point` labels from `json_db/horoscope_theme_advice_axes.json` so the readable UI does not expose internal identifiers.

## Geocoding limits

Nominatim usage is for local developer testing only. The proxy sets an identifying User-Agent/Referer, keeps an in-memory cache, and enforces a minimum one-second delay between external geocoding calls. It must not be used for bulk geocoding or as a production geocoding backend.

## Known limits

- `planned`, `disabled`, and `deprecated` services are hidden in this V1.
- Full natal and horoscope flows require a birth time; the UI disables those buttons when the time is missing.
- Long premium runs can take several minutes and may consume provider quota.
- Results depend on the worker because the UI uses the public async `/v1/jobs` integration path.

# Horoscope period fake smoke routing - 2026-06-09

## Scope

Fixed the local fake smoke path for `horoscope_premium_next_7_days_natal`, which could time out while the worker retried a real OpenAI-backed job.

## Behavior

- The horoscope writer now resolves engine defaults from the canonical `horoscope` product policy before calling the LLM provider.
- The canonical SQL seed declares `horoscope` in `llm_product_generation_policies`, allowing `llm_product_default_engine` overrides to apply to horoscope services.
- Period fake smoke wrappers temporarily switch only the `horoscope` product to `fake/fake-model`, restart the LLM API and worker so the catalog is reloaded, then restore the previous product default.
- Standalone basic/premium period fake scripts perform the same temporary switch unless their wrapper passes `-AssumeFakeProviderConfigured`.
- Fake smoke polling now prints the last job status JSON before raising a timeout, so retrying worker failures expose their real error code.

## Validation

- `cargo test -p astral_llm_api --test horoscope_tests horoscope_premium_next_7_days`
- `docker compose up -d --build astral_llm_api astral_llm_worker`
- `.\scripts\test_horoscope_premium_next_7_days_all.ps1 -SkipRustChecks`
- `.\scripts\test_horoscope_period_all.ps1 -SkipRustChecks`

# Horoscope Premium 7 days generation refactor - 2026-06-10

## Scope

Refactored the deterministic generation model for `horoscope_premium_next_7_days_natal` to make the Premium period reading less mechanical and to prevent internal editorial scaffolding from leaking into public windows.

## Behavior

- `build_period_watch_windows` now returns no watch windows when there is no true vigilance event. Neutral/context events are no longer recycled into artificial low-risk windows.
- Watch window titles and watch points always come from public theme labels (`horoscope_period_public_themes.json`) instead of editorial arc templates.
- Premium daily planning caps repeated use of the same theme at 3 days when alternatives exist.
- `period_event_score` is now a selection score without repetition bonus. Repetition density remains available separately as `theme_density_score` for period-level analysis.
- Period events now keep internal metadata (`fact_type`, `transiting_object`, `natal_target`, `natal_house`) so selection can avoid presenting retreat-heavy houses such as 8/12 as ordinary best-day candidates.
- `build_period_best_windows` prefers distinct themes and distinct dates before using a fill pass.
- Premium validation rejects meta watch-window wording such as `nouvelle facette`, `répéter le même conseil`, `fonction narrative`, and `changer l'usage`.
- The Premium prompt now states that `editorial_brief` is internal guidance and must not be copied directly into public text.
- The period interpretation request schema now requires the Premium selection metadata emitted on each `period_event`: `theme_density_score`, `fact_type`, `transiting_object`, `natal_target`, `natal_house`, `natal_focus_hint`, and `personalization_hint`.
- Editorial arc/role seed wording no longer carries copyable meta phrases such as `Nouvelle facette` or `changer l'usage` in `editorial_brief`.
- Premium validation now also rejects those editorial meta phrases across the full public response, while keeping the dedicated window error for `watch_windows`.
- Period public validation now rejects French elision errors such as `d’réaccorder` with `HOROSCOPE_PERIOD_FRENCH_TYPOGRAPHY_FAILED`.
- Period public validation rejects mechanical personalization fragments such as `vos repères personnels liés à`.
- The Premium writer prompt now requires secondary same-day signals to remain short nuances; the daily text and advice must stay aligned with the main `daily_plan` theme.
- `scripts/test_integration_jobs_e2e.ps1` now guards the local async smoke against accidental real-provider usage: it expects `default_provider=fake` unless `-AllowRealProvider` is passed, and reports OpenAI rate limits as external provider failures.
- `scripts/docker_update_integration_stack.ps1` now wraps the integration jobs smoke with a temporary fake-provider override for both `natal_prompter` defaults and the `natal_simplified` interpretation profile, restarts API/worker so async jobs use fake, then restores the configured product/profile models before continuing the remaining smokes.
- `scripts/docker_update_integration_stack.ps1` no longer exposes the `-RunRealHoroscopePeriodE2E` path. The update wrapper only runs deterministic fake/local smoke suites.
- Integration and horoscope fake smoke scripts now assert the completed job quality provider is `fake`, including idempotent replay for the integration jobs smoke.
- `FakeProvider` now treats a provider schema containing full reading fields (`summary` + `chapters`) as a full `natal_reading_v1` request even when a `chapter_code` is present for prompt context. This prevents the async `natal_simplified` smoke from returning a chapter-only JSON that fails schema validation.
- `scripts/lib/horoscope_e2e_fake_provider.ps1` now also enables fake at the Docker environment level for API and worker during horoscope fake smokes. Daily horoscope smoke suites wrap their fake jobs with this helper, so `docker_update_integration_stack.ps1` can continue through the daily and period fake suites without consuming OpenAI quota.
- Premium period provider responses now run a final evidence realignment pass before validation. Public `evidence_keys` and window `source_snapshot_keys` for daily timeline, day markers, domain sections, best/watch windows, watch summary, strategy, and evidence summary are restored only from the canonical interpretation request, preventing real LLM omissions from failing with `HOROSCOPE_PERIOD_EVIDENCE_MISSING` while still rejecting invented raw response evidence.
- Historical note: Premium period public text previously normalized mechanical fragments before validation. The active period path now avoids semantic or taxonomic rewrites after provider output and keeps only structural repair, technical leak checks, typography, and length/contract guards.
- The mechanical text cleanup now uses case-insensitive regexes, accepts straight and typographic apostrophes, and runs as a final recursive public-string pass before validation while skipping contract/enumeration fields such as `status`, dates, evidence keys, period resolution and quality metadata.
- Domain fallback copy now limits the raw focus list to the first actionable items and writes a natural cross-domain mini-reading instead of serializing every associated situation.
- Evidence restoration fallback by index is now constrained by date for day-based arrays and by missing domain/title identity for domain sections, so valid-but-wrong provider evidence is not silently reassigned to an unrelated public block.
- Premium period fallback copy now naturalizes raw focus lists before they reach `daily_timeline`, `key_days`, `best_days`, and `watch_days`, avoiding repeated verbs such as `Vérifiez vérifier` and punctuation artifacts such as `. ,`.
- Premium period public expansion no longer appends a domain personalization sentence when the domain text already contains a personal marker, preventing duplicated `Dans ...` follow-up sentences.
- Historical note: Premium prompt and editorial fallback wording previously removed a taxonomic public phrase from deterministic copy.
- Premium period final cleanup now directly reapplies glued French compound repair on every public string, including `utilisezles` in overview fields.
- French typography cleanup now also repairs real-run glued forms such as `aprèsmidi`, `qu’estce`, `mesurezl`, and `demipromesses`.
- Premium period cleanup rewrites the latest real-run polish fragments: `La journée dynamique...`, `revint`, raw `Stabiliser Tester limites...` trajectories, abstract `Le mouvement part de vos repères...` trajectories, and `Dans X, Le plus utile...` domain appendices.
- Premium prompt now explicitly warns against those malformed formulations before generation.
- Premium domain fallback wording no longer uses the repeated `Le plus utile est...` cadence.
- Premium domain personalization fallback no longer emits `<domain> donne une direction claire`; generic domain filler is replaced instead of appended, support sentences vary by domain, validation blocks the exact generic fallback plus broken `consiste à de` phrasing, and typography cleanup rewrites `allègerez` to `allégez`.
- Premium period marker wording now separates opportunity wording for `best_days` from risk wording for `watch_days`; repaired best/watch marker openings vary by date to avoid repeated card phrasing; `best_days` rejects `Avant de promettre davantage`, daily personalization fallback uses a concrete proof/owner/deadline criterion, domain fallback endings no longer start with the domain title, typography cleanup rewrites `allége la charge` to `allégez la charge`, and the service test UI displays `Stratégie`.
- Premium period final personalization hardening now runs after public-string normalization and restores accepted, concrete overview/daily anchors without using meta `repères personnels` wording, preventing `HOROSCOPE_PERIOD_EVIDENCE_MISSING` failures caused by `week_overview_missing_natal_personalization` or too few personalized daily entries.
- Premium domain personalization now appends only a short personal tail when the domain already contains a concrete support sentence, preventing duplicated `Le bon appui est...` / `Le geste à garder est...` domain wording.
- Premium domain public normalization now deduplicates repeated support sentences inside a domain section after every repair pass, making the personalization hardening idempotent.
- Premium best-day fallback wording now uses colon-led action phrases instead of malformed assemblies such as `consolider nommer` or `rendre concret tenir`; validation rejects those fragments if a provider returns them.
- Premium public cleanup now repairs real-run grammar fragments such as `Soleil dynamique un`, `et suspendre la discussion`, and compact unpunctuated trajectories before validation.
- Raw LLM provider outputs are now stored before post-processing in unique files under `output/logs/raw_llm_outputs/{run_id}/...` by default outside production. Set `ASTRAL_LLM_STORE_RAW_PROVIDER_OUTPUTS=false` in `.env` to disable, or override `ASTRAL_LLM_RAW_PROVIDER_OUTPUT_DIR`. These are dev audit artifacts and may contain uncleaned generated text.

## Validation

- `cargo test -p astral_llm_api --test horoscope_tests horoscope_premium_next_7_days`
- `cargo test -p astral_llm_api --test horoscope_tests` (264 tests, final premium wording and personalization polish)
- `.\scripts\test_horoscope_premium_next_7_days_fake.ps1`
- `cargo test -p astral_llm_api --test contracts_publish_tests`
- `cargo test -p astral_llm_api --test integration_jobs_tests`
- `cargo test -p astral_llm_providers fake_provider_returns_full_reading_when_full_schema_has_chapter_code`
- `python scripts\import_json_db_to_postgres.py --dry-run`
- `.\scripts\test_horoscope_free_daily_all.ps1 -SkipRustChecks`
- `.\scripts\test_horoscope_premium_daily_all.ps1 -SkipRustChecks`
- `.\scripts\docker_update_integration_stack.ps1 -SkipBuild -SkipImport -SkipCatalogueSubmit`
# 2026-06-13

## Observabilite des jobs safety_rejected

- Enrichissement de `error.details` sur `GET /v1/jobs/{run_id}` pour les jobs `safety_rejected`.
- Conservation de `category`, `rule_id` et `violations` dans `llm_jobs.error_json` au lieu de ne renvoyer que `code` et `message`.
- Documentation du champ optionnel `error.details` dans `contracts/llm/integration_job_status_v1.schema.json`.
- Ajout de tests unitaires sur `job_error_from_reading` pour verrouiller la serialisation des erreurs `safety_rejected` et `failed`.

## Hardening du cadrage symbolique natal

- Ajout d'un helper applicatif `ensure_symbolic_framing_text` pour injecter un cadrage interpretatif court quand un chapitre natal n'en contient pas explicitement.

## UI debug natal V2

- Ajout d'un panneau "Inspection calcul natal" dans `tests/service_test_ui/`, affiche avant la sous-section des services natals.
- Le panneau permet de lancer une inspection `simplified` ou `full` avec choix du tier public (`free`, `basic`, `premium`).
- Chaque inspection appelle un endpoint dedie `.../inspect` qui s'arrete avant le LLM, renvoie le `calculation` et expose la requete `GenerateReadingRequest` construite sans lancer `generate_reading`.
- Application du helper avant la validation safety des chapitres et de la synthese finale afin d'eviter les `SAFETY_REJECTED` fragiles du type `missing symbolic/interpretive framing`.
- Ajout de tests de non-regression sur un texte de type `growth_path` trop affirmatif et sur l'idempotence quand le cadrage existe deja.

## Compatibilite OpenAI reasoning.effort

- Correction defensive dans l'adapter OpenAI : `ReasoningEffort::None` est converti en `minimal` au lieu de serialiser `reasoning.effort = "none"`.
- Cela evite les echecs `400 unsupported_value` sur des modeles comme `gpt-5-mini`, qui n'acceptent plus `none`.

## Premium evidence planner

- Correction du planner Premium pour autoriser une requirement `blocking` a reintroduire une evidence pourtant marquee dans `prior_avoid` quand c'est necessaire pour satisfaire le contrat du chapitre.
- Nettoyage de `avoid_repeating` apres selection afin qu'aucune evidence active ne reste simultanement dans la liste d'evitement.
- Ajout d'un test de non-regression sur `emotional_life` pour garantir qu'un `moon aspect` disponible est bien inclus quand `emotional_moon_aspects` est requis.

## Compatibilite payload horoscope UI de test

- Correction du generateur `tests/service_test_ui/service-test-ui.js` pour n'envoyer `target_language_code` que pour `horoscope_premium_next_7_days_natal`.
- Les autres payloads horoscope de l'UI de test reviennent a `target_language`, conforme aux contrats publics legacy encore valides en schema.
- Mise a jour du test HTML embarque pour verrouiller la distinction entre periode Premium V2 et services horoscope legacy.

## Cadrage amont du premium daily local

- Durcissement du writer `horoscope_premium_daily_local_2h_slots` en amont de generation: prompt exigeant un JSON compact minified, des champs courts et une densite strictement bornee par section.
- Ajout de `maxLength` internes dans le schema provider strict pour `summary`, `advice`, `premium_timeline_slot`, `premium_slot_summary` et `domain_section`, sans modifier le contrat JSON public.
- Ajout de tests de non-regression sur ces contraintes amont afin d'eviter les reponses tronquees par debordement de longueur.

## Cadrage amont du period Free

- Durcissement du writer `horoscope_free_next_7_days_natal` en amont de generation: prompt exigeant un JSON compact minified et une lecture plus courte, explicitement bornee section par section.
- Ajout de `minLength` / `maxLength` internes dans le schema provider strict pour `summary`, `dominant_theme`, `key_days`, `advice`, `watch_summary` et `evidence_summary`, sans modifier le contrat JSON public.
- Passage du `reasoning_effort` a `minimal` sur le flux Free period legacy pour eviter qu'un budget de sortie soit absorbe par le raisonnement avant emission du JSON.
- Prune final du flux Free legacy dans le writer et l'orchestrateur pour empecher les enrichisseurs communs Basic/Premium de reintroduire `daily_timeline`, `week_overview`, `domain_sections` ou d'autres champs interdits.

## Cadrage amont du period Basic

- Durcissement du writer `horoscope_basic_next_7_days_natal` en amont de generation: prompt exigeant un JSON compact minified, 7 entrees quotidiennes denses mais courtes, 2 a 3 domaines et une synthese bornee.
- Ajout de contraintes internes `minLength` / `maxLength` et de cardinalites plus strictes dans le schema provider Basic pour eviter les sorties coupees par `max_output_tokens`, sans modifier le contrat JSON public.
- Passage du `reasoning_effort` a `minimal` sur le flux Basic period legacy afin que le budget provider soit consacre au JSON final.
- Normalisation deterministe du fragment public mecanique `verifiez verifier` avant validation finale, pour eviter un rejet post-generation sur une formule casse tout en conservant le hard gate anti-fuite technique.

## Stabilisation des builds Docker Rust

- Verrouillage des caches BuildKit Cargo dans `docker/astral_calculator_http/Dockerfile`, `docker/astral_llm_api/Dockerfile` et `docker/astral_llm_worker/Dockerfile`.
- Ajout de `sharing=locked` et de `id=` explicites sur les mounts `registry`, `git` et `target` pour eviter les corruptions de cache concurrentes du type `.cargo-ok already exists`.
## 2026-06-13/14 - Gateway V2 consolidation

Etat final retenu :

- `astral_contracts` porte la taxonomie publique typed et les contrats communs
- `astral_gateway` porte l'orchestration publique V2 pour `natal` et `horoscope`
- `astral_calculator` est proprietaire exclusif du calcul, y compris les builders horoscope
- `astral_llm` est proprietaire exclusif du rendu LLM, avec endpoints internes de rendu pour la gateway
- le catalogue `GET /v1/services` expose `api_surface` et positionne explicitement la gateway V2 comme point d'entree public recommande
- `IntegrationJobExecutor` remplace le reliquat de dispatch central historique

Extinction legacy retenue :

- les routes sync publiques `POST /v1/readings/generate` et `POST /v1/readings/natal/simplified` sont supprimees du runtime
- `supports_sync_legacy` et `endpoints.submit_sync_legacy` ne sont plus publies dans la facade publique V1
- les scripts et suites de compatibilite sync ont ete supprimes
- le mapping manuel sync et les exemples associes ont ete retires

Outillage courant :

- les scripts premium utiles restent sous `scripts/` et ciblent `POST /v1/internal/readings/render`
- `docker-compose.legacy-cutover.yml` ne sert plus qu'au shim `product_code` legacy restant

Validation de cloture :

- `cargo test -p astral_gateway`
- `cargo test -p astral_llm_api`
- `cargo test -p astral_llm_api --test contracts_publish_tests`
- `cargo test -p astral_llm_api --test integration_jobs_tests`
- `cargo test -p astral_llm_api --test integration_services_tests`
- `cargo test -p astral_llm_worker`

## 2026-06-14 - Strategie finale de tests factorises

Etat final retenu :

- tous les tests Rust du workspace vivent sous `tests/`
- les crates declarent explicitement leurs suites via `[[test]]` dans leurs `Cargo.toml`
- aucun bloc `#[cfg(test)]` ni module `mod tests` ne reste dans `astral_*/src` ou `astral_llm/crates/*/src`
- un test de gouvernance (`tests/inline_tests_governance_tests.rs`) verrouille cette regle

Couverture ajoutee ou consolidee :

- `tests/contracts_registry_tests.rs` couvre la taxonomie typed `astral_contracts`, les descripteurs horoscope et la presence des schemas communs/publics
- `tests/horoscope_builders_tests.rs` couvre les builders factorises du calculateur pour daily/period, les validations d'entree et le scan plan
- `tests/gateway_route_surface_tests.rs` couvre la surface publique `astral_gateway` V2 et verifie l'absence des anciennes routes sync runtime
- `tests/integration_job_executor_tests.rs` couvre la matrice de support des services factorises cote `IntegrationJobExecutor`
- `tests/astral_calculator_http_unit_regression_tests.rs` et `tests/astral_llm_api_unit_regression_tests.rs` reprennent les regressions unitaires minimales anciennement inline
- `tests/chapter_quality_repair_tests.rs` et `tests/interpretive_evidence_tests.rs` preservent les comportements publics encore utiles apres extraction
- `tests/reading_quality_validator_tests.rs` restaure les gates qualite premium/premium_plus et la normalisation defensive des codes de chapitre renvoyes par les providers
- `tests/openai_provider_adapter_tests.rs` verrouille l'extraction `output_text` de l'adapter OpenAI, l'erreur actionable sur reponse reasoning-only et la downgrade `reasoning_effort none -> minimal`

Validation de cloture executee pour cette refonte :

- `cargo test -p astral_contracts --test contracts_registry_tests --test inline_tests_governance_tests`
- `cargo test -p astral_calculator --test horoscope_builders_tests`
- `cargo test -p astral_gateway --test gateway_route_surface_tests`
- `cargo test -p astral_llm_application --test integration_job_executor_tests --test chapter_quality_repair_tests`
- `cargo test -p astral_llm_domain --test interpretive_evidence_tests`
- `cargo test -p astral_calculator_http --test astral_calculator_http_unit_regression_tests`
- `cargo test -p astral_llm_api --test astral_llm_api_unit_regression_tests`
- `cargo test -p astral_gateway`
- `cargo test -p astral_calculator`
- `cargo test -p astral_calculator_http --test astral_calculator_http_tests`
- `cargo test -p astral_llm_api --test astral_llm_tests`
- `cargo test -p astral_llm_api --test integration_services_tests`
- `cargo test -p astral_llm_api --test integration_jobs_tests`
- `cargo test -p astral_llm_api --test contracts_publish_tests`
- `cargo test -p astral_llm_api --test horoscope_tests`
- `cargo test -p astral_llm_api --test astral_llm_simplified_reading_tests`
- `cargo test -p astral_llm_application simplified_reading_guard`
- `cargo test -p astral_llm_application simplified_reading_postprocess`
- `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`

Note :

- la fixture `tests/golden/astro_engine_response_v1_paris_1990_rich.json` a ete regeneree via le mecanisme de test prevu (`UPDATE_ENGINE_RESPONSE_GOLDEN=1`) pour realigner le golden publie sur l'enveloppe moteur courante

## 2026-06-14 - Docker update stack et service test UI

Etat final retenu :

- `docker-compose.yml` expose maintenant `astral_gateway` sur `:8082` avec healthcheck `GET /health/ready`
- `scripts/docker_update_integration_stack.ps1` pilote la stack refactoree complete : compose explicite, bootstrap DB, reseed catalogue, sync profils/modeles, restart runtime, tests Rust et smokes V2
- `scripts/docker_update_integration_stack.ps1` nettoie maintenant automatiquement `tmp_target/` en fin d'execution reussie pour recuperer l'espace disque occupe par les artefacts Cargo locaux. Le nettoyage n'est pas lance en cas d'echec, afin de conserver les artefacts utiles au diagnostic.
- `scripts/start_service_test_ui.ps1` expose des proxys `gateway`, `llm` et `calculator`, et reste lancable sous PowerShell 7 comme sous `powershell.exe`
- `tests/service_test_ui/*` cible la surface publique `astral_gateway` V2 et garde `GET /api/llm/v1/services` comme diagnostic async interne

Parcours automatiques retenus :

- bootstrap DB systematique via `python scripts/import_json_db_to_postgres.py`
- soumission systematique du catalogue via `scripts/manage_integration_services.ps1 -Submit`
- sync systematique du catalogue LLM via `Sync-AstralLlmCatalog`
- tests Rust par defaut :
  - `cargo test -p astral_contracts --test contracts_registry_tests --test inline_tests_governance_tests`
  - `cargo test -p astral_gateway`
  - `cargo test -p astral_llm_application --test integration_job_executor_tests --test chapter_quality_repair_tests`
  - `cargo test -p astral_llm_api --test contracts_publish_tests`
  - `cargo test -p astral_llm_api --test integration_services_tests`
  - `cargo test -p astral_llm_api --test integration_jobs_tests`
  - `cargo test -p astral_calculator_http --test astral_calculator_http_tests`
- smokes HTTP publics :
  - `POST /v2/natal/simplified/free`
  - `POST /v2/natal/full/basic`
  - `POST /v2/horoscope/daily/free`
  - `POST /v2/horoscope/period/free`
  - verification `404` sur `POST /v1/readings/generate` et `POST /v1/readings/natal/simplified`

Exclus du parcours automatique car moteur LLM reel :

- `provider_real_smoke`
- `*_real_e2e.ps1`
- `generate_premium*_e2e.ps1`
- `test_natal_premium*_profile.ps1`
- toute suite necessitant `OPENAI_API_KEY`
- tout appel direct certifiant `POST /v1/internal/readings/render`

## 2026-06-14 - Horoscope period artifact cleanup

- le postprocess 0horoscope period0 retire maintenant les suffixes d'artefacts provider sur les champs publics avant validation, notamment les fragments 0</structured_reading>0 et les auto-commentaires de sortie malformee
- le postprocess period relance ensuite le pipeline partage de retraitement texte FR pour restaurer la typographie publique sans toucher aux champs techniques comme 0evidence_keys0

- 2026-06-14 : les prompts systeme du flux horoscope period interdisent maintenant explicitement tout meta-commentaire sur le resultat, le JSON, le schema, les erreurs, les timeouts, les troncatures ou le processus de generation, afin de reduire les injections de type auto-commentaire provider.

- 2026-06-14 : le post-processing horoscope period reutilise maintenant la normalisation existante des tirets cadratins pour remplacer les caracteres 0—0 par 0-0 dans les champs publics avant validation finale.

- 2026-06-14 : le champ public 0week_overview.trajectory0 du flux horoscope period est maintenant durci contre la recopie brute de phases internes (mise_en_mouvement) et les residus d'edition (} } (removed)), avec fallback automatique vers une trajectoire publique propre si le contenu reste suspect.

# 2026-06-15 - Correction modals tokens/couts horoscope daily/period

- UI de tests: ajout du support de `debug.run_id` pour retrouver les audits LLM horoscope.
- Gateway horoscope: generation d'un `run_id` stable par requete, propagation vers l'API LLM et exposition dans `debug`.
- API LLM horoscope: reutilisation de `debug_run_id` pour persister les audits `daily` et `period` sous le meme identifiant que celui expose au gateway.
- API LLM horoscope: persistance des `GenerationStepRecord` et des `token_usages` pricies pour les writers `daily`, `period` et les retries de reparation/qualite, afin que `/v1/runs/{run_id}` expose enfin `steps`, `token_usage` et les compteurs/couts attendus par les modales UI.

# 2026-06-15 - Correction build Docker multi-services avec target-dir Cargo

- Cause: le workspace force `tmp_target` dans `.cargo/config.toml`, alors que les Dockerfiles copiaient encore les binaires depuis `/app/target/release`.
- Correction: alignement des mounts cache Docker BuildKit et des chemins `cp` sur `/app/tmp_target/release` pour `astral_calculator_http`, `astral_gateway`, `astral_llm_api` et `astral_llm_worker`.
## 2026-06-17 - Refacto feature boundaries W0-W2

Resume court:
- creation du module canonique `astral_calculator/src/astrology/` pour les calculs communs `aspects` et `ephemeris`;
- conservation des anciens chemins publics `natal::aspects` et `natal::ephemeris` via wrappers de compatibilite;
- migration des nouveaux appels internes vers `crate::astrology::*` et `EphemerisEngine::calculate_chart`;
- sortie de `PgPool` des services metier `engine`, `simplified` et `horoscope` via un builder runtime en bordure.

Invariants de couche:
- `natal`, `simplified` et `horoscope` restent des orchestrateurs produit;
- les calculs astrologiques reutilisables vivent sous `astrology/`, pas sous une feature produit;
- aucune dependance `domain -> infra`;
- aucun `PgPool`, `connect_from_env`, `block_on` ou `run_blocking` dans les couches metier verrouillees par test;
- aucun import `crate::natal::aspects` ou `crate::natal::ephemeris` depuis `simplified` ou `horoscope`.

Commandes de verification:
- `cargo fmt`
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator`
- `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`

Reviews:
- `docs/reviews/astral_calculator_refactor_feature_boundaries/REV-W00-plan.md`
- `docs/reviews/astral_calculator_refactor_feature_boundaries/REV-W00-adversarial.md`
- `docs/reviews/astral_calculator_refactor_feature_boundaries/REV-W00-followup-1.md`
- `docs/reviews/astral_calculator_refactor_feature_boundaries/REV-FINAL.md`
- `docs/reviews/astral_calculator_refactor_feature_boundaries/REV-b994848-adversarial-loop.md`
- `docs/reviews/astral_calculator_refactor/REV-b994848-adversarial-loop.md`

## 2026-06-17 - Review adversariale du commit b994848

Resume court:
- review adversariale du dernier commit `b994848 refactor(calculator): tighten boundaries and projections`;
- aucun finding ouvert apres scans d'imports, lecture ciblee des services/runtime/wrappers et verification des contrats;
- aucune correction code requise dans cette boucle.

Commandes de verification:
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator`
- `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`
- `cargo test -p astral_calculator_http --test astral_calculator_http_tests`
- `cargo test -p astral_llm_api --test contracts_publish_tests`
- `git show --check HEAD`

## 2026-06-19 - Maintainability refactor: natal orchestration, astrology math, runtime compat, typed position context

Resume court:
- decomposition de `NatalCalculationService::calculate_basic_with_catalog()` en `NatalReferenceSnapshotLoader`, `NatalReusePolicy` et `NatalCalculationWorkflow` sans changer le contrat JSON public;
- introduction de ports applicatifs plus fins (`ReferenceSystemResolver`, `NatalReferenceStore`, `LocalizationCatalog`, `CalculationTransactionManager`, `CalculationAttemptStore`, `CalculationFactStore`, `PayloadStore`, `SignalStore`) avec compatibilite via composition de traits et sans modifier les repositories PostgreSQL concrets;
- migration des primitives `shared::astro_math` vers `astrology::angles` et `astrology::zodiac`, puis bascule des consommateurs internes vers les chemins canoniques;
- reduction de `runtime` a un facade de wiring PostgreSQL, avec les anciens helpers de validation deplaces sous `runtime::compat`;
- ajout d'un contexte typé pour `ObjectPositionFact::facts_json` (`ObjectContext`, `AngleContext`, `PositionFactContext`) sans migration SQL, en conservant le JSON brut inconnu.

Invariants de couche:
- `runtime::build_runtime_service` reste le point unique de composition PostgreSQL concrete;
- les nouveaux services applicatifs consomment des ports fins et non le trait large historique;
- `shared::astro_math` reste un wrapper de compatibilite sans logique propre;
- les calculs astrologiques reutilisables vivent sous `crate::astrology::*`;
- le contexte typé ne supprime ni ne reformatte `facts_json`, il ajoute seulement une couche de lecture stable cote Rust.

Commandes de verification:
- `cargo fmt`
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator --test position_fact_context_tests`
- `cargo test -p astral_calculator`
- `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`

Reviews:
- `docs/reviews/astral_calculator_refactor/REV-MAINTAINABILITY-IMPLEMENTATION-2026-06-19.md`
- `docs/reviews/astral_calculator_refactor/REV-MAINTAINABILITY-IMPLEMENTATION-2026-06-19-followup-1.md`
- `docs/reviews/astral_calculator_refactor/REV-MAINTAINABILITY-IMPLEMENTATION-2026-06-19-followup-2.md`
- `docs/reviews/astral_calculator_refactor_feature_boundaries/REV-MAINTAINABILITY-IMPLEMENTATION-2026-06-19.md`
- `docs/reviews/astral_calculator_refactor_feature_boundaries/REV-MAINTAINABILITY-IMPLEMENTATION-2026-06-19-followup-1.md`
- `docs/reviews/astral_calculator_refactor_feature_boundaries/REV-MAINTAINABILITY-IMPLEMENTATION-2026-06-19-followup-2.md`
