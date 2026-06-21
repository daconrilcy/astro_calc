# Railguard

## Purpose and Scope

This document sets guardrails for the Rust crate `astral_calculator` inside the workspace at `C:\dev\astral_calculation`.
It applies to `astral_calculator/src`, the crate-level tests registered in `astral_calculator/Cargo.toml`, and the root `tests/` support files used by this crate.

## Project Map

- `astral_calculator/src/lib.rs`: crate root and compatibility exports.
- `astral_calculator/src/main.rs`: binary entrypoint.
- `astral_calculator/src/domain`: domain types and contracts.
- `astral_calculator/src/astrology`: reusable astrology calculations.
- `astral_calculator/src/application`: shared application ports and loading seams.
- `astral_calculator/src/application/chart_context.rs`: shared non-natal chart-context loader for simplified and horoscope flows.
- `astral_calculator/src/application/ports.rs`: compatibility facade for application ports; new capability families should be extracted to named submodules instead of growing this file.
- `astral_calculator/src/application/ports/horoscope_builder.rs`: dedicated application contracts for horoscope public-request builders.
- `astral_calculator/src/features/natal`, `astral_calculator/src/features/simplified`, `astral_calculator/src/features/horoscope`: product orchestrators.
- `astral_calculator/src/infra/db`: SQLx repositories and runtime queries.
- `astral_calculator/src/infra/db/runtime_queries/reference.rs`: residual reference-query hub; must continue shrinking by capability split.
- `astral_calculator/src/infra/db/runtime_queries/reference/systems.rs`: DB query capability for house, zodiacal, and coordinate reference systems.
- `tests/`: cross-module and contract tests wired from `astral_calculator/Cargo.toml`.
- `.audit/audit-1782061131.md`: current refactor audit used as evidence for boundaries, remaining risks, and next slices.
- `docs/BASIC_PAYLOAD_IMPLEMENTATION.md`: required slice documentation for refactor waves touching calculator boundaries, payload assembly, or fixture ownership.

## Non-Negotiable Invariants

- `src/infra/db` is the only home for SQLx and PostgreSQL access code. Do not move DB access into `domain`, `astrology`, or product feature code. Evidence: `astral_calculator/src/infra/db` layout and `.audit/audit-1782061131.md`.
- Domain and reusable astrology logic must not depend on infrastructure. Evidence: `.audit/audit-1782061131.md` reports no `domain -> infra` or `astrology -> infra` dependency inversion.
- Product features are orchestrators, not calculation libraries: `natal`, `simplified`, and `horoscope` validate input, load references and repositories, call shared calculations, then assemble output. Evidence: `.audit/audit-1782061131.md` and `astral_calculator/src/features`.
- Reusable astronomy and astrology calculations belong under `src/astrology`; do not add new cross-feature math to `features/shared` or to product-specific internals. Evidence: `AGENTS.md` and `.audit/audit-1782061131.md`.
- Public behavior tests stay under root `tests/`; do not move broad behavioral coverage into inline unit tests inside production `src/**/*.rs`. Current production source must contain no `#[cfg(test)]` modules and no `#[test]` functions. Evidence: `.audit/audit-1782061131.md`, `tests/refactor_governance_tests.rs::calculator_production_source_does_not_contain_inline_tests`.
- Canonical referential data should come from the database when possible, not from hard-coded production fixtures. Evidence: production payload builders require an explicit `BasicPayloadCatalog` input in `src/features/natal/payload/build/mod.rs`, while the legacy fixture builder was removed from `src/features/natal/catalog.rs`.
- Canonical feature surface is narrower than before: `src/features/mod.rs` no longer exposes `payload` or `signals` aliases, and `src/features/natal/application/natal_calculation_service.rs` now uses normal `mod` declarations for its private submodules. Evidence: `astral_calculator/src/features/mod.rs`, `astral_calculator/src/features/natal/application/natal_calculation_service.rs`.
- Root payload and signal regression tests now import canonical paths directly from `astral_calculator::features::natal::payload::build` and `astral_calculator::features::natal::signals`; do not reintroduce `features::payload` or `features::signals` as public aliases. Evidence: `tests/payload_tests.rs`, `tests/payload_shared_characterization_tests.rs`, `tests/signals_tests.rs`.
- Basic payload reference data is DB-backed. Aspect definitions, aspect families, accidental dignity conditions, sect affinities, lunar phases, product scoring profiles, accidental scoring params, polarity bands, and essential dignity weights must be loaded through repositories or runtime catalogs instead of reintroduced as Rust constants. Evidence: `docs/BASIC_PAYLOAD_IMPLEMENTATION.md` and the `json_db/astral_*.json` seeds.
- If runtime reference loading fails because PostgreSQL is missing a relation or column documented in `json_db`, fix the database sync/import path, not Rust fallbacks. Evidence: `docs/BASIC_PAYLOAD_IMPLEMENTATION.md` names `scripts/import_json_db_to_postgres.py`, `scripts/patch_astral_aspects_default_orb_deg.py`, and `scripts/patch_astral_aspect_families_expected_count.py`.
- Natal fixture catalogs used by integration tests belong under root test support, not in `src/features/natal/catalog.rs`. Evidence: `tests/common/natal_catalog.rs`, `tests/payload_tests.rs`, `tests/runtime_tests.rs`, `tests/signals_tests.rs`.
- The 2026-06-21 Phase 1 slice removing the production `test_catalog()` fallback is closed only when `docs/BASIC_PAYLOAD_IMPLEMENTATION.md` links both paired review artifacts and they are marked `Statut: closed` / `Aucun finding ouvert`. Evidence: `.audit/audit-1782061131.md`, `docs/reviews/astral_calculator_refactor/REV-NATAL-TEST-CATALOG-FALLBACK-2026-06-21.md`, and `docs/reviews/astral_calculator_refactor_feature_boundaries/REV-NATAL-TEST-CATALOG-FALLBACK-2026-06-21.md`.
- The 2026-06-21 shared non-natal chart-context slice is closed only when `docs/BASIC_PAYLOAD_IMPLEMENTATION.md` links both paired review artifacts and they are marked `Statut: closed` / `Aucun finding ouvert`. Evidence: `.plan/plan-1782060238.md`, `docs/reviews/astral_calculator_refactor/REV-SHARED-NON-NATAL-CHART-CONTEXT-2026-06-21.md`, and `docs/reviews/astral_calculator_refactor_feature_boundaries/REV-SHARED-NON-NATAL-CHART-CONTEXT-2026-06-21.md`.
- The 2026-06-21 shared transient non-natal execution slice is closed only when `docs/BASIC_PAYLOAD_IMPLEMENTATION.md` links both paired review artifacts and they are marked `Statut: closed` / `Aucun finding ouvert`. Evidence: `astral_calculator/src/application/transient_chart.rs`, `docs/reviews/astral_calculator_refactor/REV-SHARED-TRANSIENT-CHART-2026-06-21.md`, and `docs/reviews/astral_calculator_refactor_feature_boundaries/REV-SHARED-TRANSIENT-CHART-2026-06-21.md`.
- The 2026-06-21 Phase 2 `included_days` follow-up is closed only when production source remains free of inline tests and the repository-edge JSON decode remains contextualized. Evidence: `tests/refactor_governance_tests.rs::calculator_production_source_does_not_contain_inline_tests`, `tests/refactor_governance_tests.rs::horoscope_repository_keeps_included_days_decode_contextualized_at_adapter_edge`, `docs/reviews/astral_calculator_refactor/REV-TYPED-INCLUDED-DAYS-2026-06-21-followup-1.md`, and `docs/reviews/astral_calculator_refactor_feature_boundaries/REV-TYPED-INCLUDED-DAYS-2026-06-21-followup-1.md`.
- The 2026-06-21 horoscope-builder port extraction is closed only while the horoscope builder DTOs and `HoroscopeBuilderCatalog` live in `src/application/ports/horoscope_builder.rs`, `src/application/ports.rs` re-exports them for compatibility, and the paired reviews remain closed. Evidence: `astral_calculator/src/application/ports/horoscope_builder.rs`, `docs/reviews/astral_calculator_refactor/REV-HOROSCOPE-BUILDER-PORTS-2026-06-21.md`, and `docs/reviews/astral_calculator_refactor_feature_boundaries/REV-HOROSCOPE-BUILDER-PORTS-2026-06-21.md`.
- The 2026-06-21 reference-system query split is closed only while the house, zodiacal, and coordinate reference-system SQL remains under `src/infra/db/runtime_queries/reference/systems.rs` and the paired reviews remain closed. Evidence: `astral_calculator/src/infra/db/runtime_queries/reference/systems.rs`, `docs/reviews/astral_calculator_refactor/REV-REFERENCE-SYSTEM-QUERY-SPLIT-2026-06-21.md`, and `docs/reviews/astral_calculator_refactor_feature_boundaries/REV-REFERENCE-SYSTEM-QUERY-SPLIT-2026-06-21.md`.
- Legacy compatibility exports may remain only as explicit shims; new code must use canonical paths. `src/domain/mod.rs` must keep explicit `pub use` exports instead of wildcard re-exports, and workspace consumers such as `astral_calculator_http` must import `bootstrap::{db,env}` and `astrology::ephemeris` directly instead of deprecated crate-root aliases. Evidence: `.audit/implementation-audit-1782059165.md`, `src/domain/mod.rs`, `../astral_calculator_http/src/routes.rs`, `../astral_calculator_http/src/state.rs`, `tests/refactor_governance_tests.rs`.
- Deprecated crate-root aliases in `astral_calculator/src/lib.rs` are compatibility shims only. New code must not add new root aliases, and refactors should retire unused shims rather than broadening the public surface. Evidence: `astral_calculator/src/lib.rs` and `.audit/audit-1782061131.md`, which still reports aliases for `aspects`, `catalog`, `cli`, `config`, `db`, `dignities`, `ephemeris`, `facts`, and `idempotency`.
- The shared non-natal chart-context seam lives in `astral_calculator/src/application/chart_context.rs` and is the only approved place to assemble `reference_version_id`, chart objects, aspect definitions, house system, and calculation references for simplified/horoscope flows. Evidence: `astral_calculator/src/application/chart_context.rs`, `astral_calculator/src/features/simplified/service.rs`, `astral_calculator/src/features/horoscope/application/horoscope_service.rs`, and `.audit/audit-1782061131.md`.
- Simplified and horoscope must not each hand-roll a second transient chart execution loop when the shared chart-context seam fits the case. Evidence: `.audit/audit-1782061131.md` identifies duplicated transit orchestration in `src/features/simplified/service.rs` and `src/features/horoscope/application/horoscope_service.rs`.
- `astral_calculator/src/application/transient_chart.rs` is the only approved seam for non-natal transient execution that mutates a baseline `NatalChartInput`, swaps the UTC instant, applies the transient `product_code`, and calls `EphemerisEngine::calculate_chart`. Simplified and horoscope must call this seam rather than invoking `.calculate_chart(` directly in their services. Evidence: `astral_calculator/src/application/transient_chart.rs`, `tests/transient_chart_tests.rs`, and `tests/refactor_governance_tests.rs`.
- `astral_calculator/src/application/ports.rs` is a compatibility facade and temporary aggregation point, not a permanent catch-all. New capability families should land in a dedicated `application` submodule or extraction slice instead of extending the omnibus file further. The horoscope-builder family is the reference pattern: concrete contracts in `src/application/ports/horoscope_builder.rs`, re-exported by `ports.rs` for source compatibility. Evidence: `.audit/audit-1782061131.md`, `src/application/ports.rs`, `src/application/ports/horoscope_builder.rs`, `tests/refactor_governance_tests.rs`.
- `astral_calculator/src/infra/db/runtime_queries/reference.rs` must shrink by capability split rather than absorb more unrelated SQL. Query additions in this area should go to a capability-focused submodule or be coupled to an explicit split slice. The reference-system queries now live in `src/infra/db/runtime_queries/reference/systems.rs`; do not move them back into the hub. Evidence: `.audit/audit-1782061131.md`, `src/infra/db/runtime_queries/reference.rs`, `src/infra/db/runtime_queries/reference/systems.rs`.
- The Phase 2 `included_days` typing slice must stop at the application boundary: `HoroscopePeriodProfile` may expose only typed day codes, any JSON decode must happen once inside `src/infra/db/horoscope_repository.rs`, and `src/features/horoscope/builders.rs` must not call `serde_json::from_value::<Vec<String>>` directly. The repository decode must stay contextualized with `RuntimeError::InvalidRuntimeTable` and `period_profile_code`. Evidence: `.plan/plan-1782061555.md` Phase 2, `.audit/implementation-audit-1782062109.md`, `src/application/ports/horoscope_builder.rs`, `src/infra/db/horoscope_repository.rs`, `src/features/horoscope/builders.rs`, `tests/refactor_governance_tests.rs`.
- `unsafe` should be avoided unless a change has an explicit documented justification and safety contract next to the code. Evidence: no unsafe policy file exists in this crate, so the default remains conservative.

## Architecture Boundaries

- `astral_calculator` owns calculator domain logic and runtime persistence; `astral_calculator_http` is HTTP transport over calculator capabilities and should stay transport-focused. Evidence: workspace member names in `Cargo.toml`, `astral_calculator_http/src/lib.rs`, `tests/refactor_governance_tests.rs`.
- `astral_gateway` must not depend directly on internal calculator or LLM crates, and must not embed canonical reference data. Evidence: `tests/refactor_governance_tests.rs`.
- `astral_llm/crates/*` are a separate bounded area; calculator refactors should not spill into them unless a direct contract or regression check requires it. Evidence: workspace member split in `Cargo.toml` and the user’s planning scope.
- Keep workspace-level governance tests and compatibility tests authoritative for boundary rules. If a structural change would invalidate them, update the tests and the relevant railguard or doc artifact in the same slice. Evidence: `tests/refactor_governance_tests.rs`, `tests/deprecated_root_alias_compat_tests.rs`.
- Do not add new compatibility facades when a canonical module path already exists. The audit shows `src/lib.rs`, `src/domain/mod.rs`, and `src/features/mod.rs` still expose broad alias surfaces; future work should shrink, not expand, those surfaces.
- The shared non-natal chart-context seam lives in `astral_calculator/src/application/chart_context.rs` and is the only approved place to assemble `reference_version_id`, chart objects, aspect definitions, house system, and calculation references for simplified/horoscope flows. Evidence: `astral_calculator/src/application/chart_context.rs`, `astral_calculator/src/features/simplified/service.rs`, `astral_calculator/src/features/horoscope/application/horoscope_service.rs`.
- Review or audit work for the shared chart-context seam must check the Phase 1 gate first; later plan phases (`included_days`, `application/ports.rs`, infra query split, shim retirement) are separate slices unless the workspace claims they were implemented in the same pass. Evidence: `.plan/plan-1782060238.md`, `docs/BASIC_PAYLOAD_IMPLEMENTATION.md`.
- Keep dependency direction explicit while splitting hot files: `application` contracts may depend on `domain` records, `features/*` services may depend on `application` seams, and `infra/db` may implement `application` traits, but `application` must not import `infra/db` or feature internals just to reduce file size. Evidence: `tests/refactor_governance_tests.rs` and `.audit/audit-1782061131.md`.

## Rust Rules

- Preserve existing focused error boundaries; do not replace them with ad hoc panics or broad dynamic error handling in runtime paths. Evidence: `.audit/audit-1782061131.md` and `tests/refactor_governance_tests.rs`.
- Keep async and blocking boundaries explicit. Do not add `connect_from_env`, `PgPool`, `block_on`, or `run_blocking` into metier modules. Evidence: `AGENTS.md`.
- Respect the existing Cargo test registration model where root `tests/*.rs` files are wired from crate manifests. Evidence: `astral_calculator/Cargo.toml`.
- Prefer normal Rust module layout over `#[path]` indirection for in-crate modules; any new `#[path]` usage needs explicit justification. Evidence: `src/features/natal/application/mod.rs`, `src/features/natal/application/natal_calculation_service.rs`, `tests/refactor_governance_tests.rs`.
- Keep public aliases and compatibility shims additive only when a test or external caller requires them; otherwise prefer canonical modules and remove dead shims in the same slice. Evidence: `astral_calculator/src/lib.rs` and `.audit/audit-1782061131.md`.
- When a root module file coexists with a same-named directory, treat the root file as a facade. Keep meaningful contracts or SQL in named submodules, and use narrow `pub use` re-exports only where source compatibility requires them. Evidence: `src/application/ports.rs` plus `src/application/ports/horoscope_builder.rs`, and `src/infra/db/runtime_queries/reference.rs` plus `src/infra/db/runtime_queries/reference/systems.rs`.

## Testing and Verification

Run the smallest relevant check first:

```powershell
cargo test -p astral_calculator --test calculation_reference_loader_tests
```

Then validate the shared transient seam with:

```powershell
cargo test -p astral_calculator --test transient_chart_tests
```

Then validate boundary regressions with:

```powershell
cargo test -p astral_calculator --test refactor_governance_tests
```

Before finalizing broader `astral_calculator` changes, run:

```powershell
cargo test -p astral_calculator
```

Additional repository-documented checks:

- `cargo test -p astral_calculator --test deprecated_root_alias_compat_tests`
- `cargo test -p astral_calculator --test runtime_identity_bootstrap_tests`
- `cargo test -p astral_calculator --test calculation_reference_loader_tests`
- `cargo test -p astral_calculator --test natal_reuse_policy_tests`
- `cargo test -p astral_calculator --test horoscope_builders_tests`
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`
- `cargo test -p astral_calculator_http --test astral_calculator_http_tests`
- `cargo test -p astral_llm_api --test contracts_publish_tests`
- `python scripts/import_json_db_to_postgres.py` after PostgreSQL is up when DB-backed reference or bootstrap changes are involved

## Change Protocol

Before editing:

- inspect `AGENTS.md`, this file, and `astral_calculator/src/lib.rs` if the target is `astral_calculator`
- inspect `astral_calculator/Cargo.toml` to confirm test registration and feature flags
- inspect the root `tests/` files covering the target behavior
- read `.audit/audit-1782061131.md` when the work touches current calculator boundary findings
- read `docs/BASIC_PAYLOAD_IMPLEMENTATION.md` when a change affects payload assembly, fixture ownership, reference loading, or review-tracked refactor waves

During editing:

- keep the slice narrow and phaseable; stop when a planned invariant is only partially closed
- preserve unrelated local changes; do not reset or revert user work
- use the database-first process from `AGENTS.md` before adding any new canonical reference value
- if a refactor wave changes calculator boundaries or fixture or catalog ownership, update the paired adversarial reviews under `docs/reviews/astral_calculator_refactor/` and `docs/reviews/astral_calculator_refactor_feature_boundaries/` in the same slice

Before handing off:

- run the smallest relevant verification first, then broaden only as needed
- report skipped checks and missing environment prerequisites explicitly
- if a guardrail changes, update the relevant railguard and cite the evidence that justified the change

## Known Risks and Open Questions

- `astral_calculator/RAILGUARD.md` is currently untracked in Git. Preserve it and keep the workspace and crate railguards aligned.
- The current audit still flags broad compatibility exports, duplicated chart-context loading, porous JSON boundaries, and repository forwarding boilerplate in `astral_calculator`. The production-owned natal catalog fallback is closed in this workspace slice and must not be reintroduced.
- If a new fixture helper is needed for `astral_calculator`, prefer `tests/common/` over new production exports unless a runtime caller genuinely needs the data.
- The exact workspace-wide boundary for future `astral_llm` railguards is not established here; if a refactor moves into `astral_llm/*`, add a crate-level railguard there rather than stretching calculator rules by assumption.
