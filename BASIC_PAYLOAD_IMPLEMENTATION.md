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
- Updated `premium_rich` word targets from `2200-3200` to `1600-2600`, with the hard limit still at `3200`, to reduce forced filler when source signals are not dense enough.
- Added regression coverage in `tests/horoscope_v1_tests.rs`.

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

- Added regression coverage in `tests/horoscope_v1_tests.rs`.

## Scope

Improved the editorial structure of `horoscope_premium_next_7_days_natal`.

## Behavior

- Premium day markers now explain their role as structural, supportive, or vigilance markers instead of repeating a generic “repere utile” sentence.
- The Premium period prompt now enforces a clearer reading flow: overview, short period markers, daily timeline, domains, hourly windows, then strategy.
- Premium advice and strategy are synthesis sections only; they must not introduce new explicit dates after the timeline and windows have already listed the dated details.
- Domain section post-processing restores canonical evidence by domain, title, or original section order when the model renames a domain or returns empty evidence arrays.
- Overview and domain post-processing now add an explicit `repères personnels` anchor when the provider returns text that is astrologically plausible but too implicit for the natal personalization guard.

## Tests

- Added regression coverage in `tests/horoscope_v1_tests.rs`.

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

- Added regression coverage in `tests/horoscope_v1_tests.rs`.
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

- `tests/horoscope_v1_tests.rs`
- `tests/text_reprocessing_application_tests.rs`
- `astral_llm/crates/astral_llm_application/src/text_trigrams.rs`

Validated with the real E2E scripts and supporting Rust test suites.

# Local real service test UI - 2026-06-09

## Scope

Added a developer-only static UI for manually exercising the real local calculator and LLM integration services from a birth input.

## Location

- UI files: `tests/service_test_ui/`
- Launcher/proxy: `scripts/start_service_test_ui.ps1`
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

- `cargo test -p astral_llm_api --test horoscope_v1_tests horoscope_premium_next_7_days`
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

## Validation

- `cargo test -p astral_llm_api --test horoscope_v1_tests horoscope_premium_next_7_days`
- `cargo test -p astral_llm_api --test horoscope_v1_tests`
- `cargo test -p astral_llm_api --test contracts_publish_tests`
- `python scripts\import_json_db_to_postgres.py --dry-run`
