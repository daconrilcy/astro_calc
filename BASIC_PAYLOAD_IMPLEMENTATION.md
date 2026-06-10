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

- `cargo test -p astral_llm_api --test horoscope_v1_tests`
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
- Overview and domain post-processing now add concrete personal anchors (`vos priorités`, `votre agenda`, owner/deadline/proof criteria) when the provider returns text that is astrologically plausible but too implicit for the natal personalization guard.

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
- Premium period public text now normalizes mechanical fragments before validation, including serialized situation hints such as `autour de vérifier`, `Appui concret :`, `est un point d'appui pour ...`, repeated `Cette énergie devient utile quand elle sert à`, and double punctuation. Domain fallback wording was also rewritten to avoid template-like `donne un angle transversal` / `gagne en valeur` phrasing.
- The mechanical text cleanup now uses case-insensitive regexes, accepts straight and typographic apostrophes, and runs as a final recursive public-string pass before validation while skipping contract/enumeration fields such as `status`, dates, evidence keys, period resolution and quality metadata.
- Domain fallback copy now limits the raw focus list to the first actionable items and writes a natural cross-domain mini-reading instead of serializing every associated situation.
- Evidence restoration fallback by index is now constrained by date for day-based arrays and by missing domain/title identity for domain sections, so valid-but-wrong provider evidence is not silently reassigned to an unrelated public block.
- Premium period fallback copy now naturalizes raw focus lists before they reach `daily_timeline`, `key_days`, `best_days`, and `watch_days`, avoiding repeated verbs such as `Vérifiez vérifier` and punctuation artifacts such as `. ,`.
- Premium period public expansion no longer appends a domain personalization sentence when the domain text already contains a personal marker, preventing duplicated `Dans ...` follow-up sentences.
- Premium prompt and editorial fallback wording now avoid the taxonomic public phrase `priorité liée à`.
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

- `cargo test -p astral_llm_api --test horoscope_v1_tests horoscope_premium_next_7_days`
- `cargo test -p astral_llm_api --test horoscope_v1_tests` (264 tests, final premium wording and personalization polish)
- `.\scripts\test_horoscope_premium_next_7_days_fake.ps1`
- `cargo test -p astral_llm_api --test contracts_publish_tests`
- `cargo test -p astral_llm_api --test integration_jobs_tests`
- `cargo test -p astral_llm_providers fake_provider_returns_full_reading_when_full_schema_has_chapter_code`
- `python scripts\import_json_db_to_postgres.py --dry-run`
- `.\scripts\test_horoscope_free_daily_all.ps1 -SkipRustChecks`
- `.\scripts\test_horoscope_premium_daily_all.ps1 -SkipRustChecks`
- `.\scripts\docker_update_integration_stack.ps1 -SkipBuild -SkipImport -SkipCatalogueSubmit`
