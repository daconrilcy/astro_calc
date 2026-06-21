# Railguard

## Purpose and Scope

This document defines the workspace-level guardrails for `C:\dev\astral_calculation`.
It applies to the Cargo workspace declared in `Cargo.toml`, with immediate focus on `astral_calculator` and only direct regression-awareness for `astral_calculator_http`, `astral_gateway`, and `astral_llm/*`.

## Project Map

- `Cargo.toml`: workspace root and canonical member list (`astral_contracts`, `astral_calculator`, `astral_calculator_http`, `astral_gateway`, `astral_time_window`, `astral_llm/crates/*`).
- `AGENTS.md`: local execution rules, verification commands, DB-first process, and refactor invariants.
- `astral_calculator/RAILGUARD.md`: crate-level operational contract for `astral_calculator`.
- `astral_calculator/src`: calculator domain, application, astrology, feature orchestration, infra/db, and runtime wiring.
- `astral_calculator/src/application/chart_context.rs`: shared non-natal chart-context loader used by simplified and horoscope flows.
- `tests/`: root integration, governance, compatibility, and contract tests registered by workspace crates.
- `tests/common/natal_catalog.rs`: non-public natal fixture catalog helper used by root tests after the production fallback removal.
- `docs/BASIC_PAYLOAD_IMPLEMENTATION.md` and `docs/reviews/astral_calculator_refactor*/`: required refactor-wave documentation and adversarial review artifacts.
- `.audit/audit-1782059752.md`: earlier workspace-level refactor audit for `astral_calculator`; keep it as historical evidence for boundary evolution.
- `.audit/audit-1782066159.md`: earlier follow-up audit for `astral_calculator`; keep it as historical evidence for the `builders`, `infra/db`, and deprecated-shim slices already in progress.
- `.audit/audit-1782068030.md`: current refactor audit for `astral_calculator`; use it as the primary evidence source for the active `facts_json` boundary, `infra/db` forwarding, governance-test coupling, and deprecated-shim follow-up slices.

## Non-Negotiable Invariants

- Keep all SQLx/PostgreSQL access for `astral_calculator` under `astral_calculator/src/infra/db`. Do not move DB access into `domain`, `astrology`, `engine`, or product feature code. Evidence: `AGENTS.md`, `astral_calculator/RAILGUARD.md`, `tests/refactor_governance_tests.rs`.
- Treat `astral_calculator/src/astrology` as the only reusable home for cross-feature astrology calculations. Do not add new metier math to `features/shared` or product-specific internals. Evidence: `AGENTS.md`, `tests/refactor_governance_tests.rs`.
- Product features in `astral_calculator` are orchestrators over contracts and shared calculations. They must not import another feature’s internals. Evidence: `AGENTS.md`, `tests/refactor_governance_tests.rs`, `.audit/audit-1782059752.md`.
- Canonical referential data must come from the database path when possible; do not add new hard-coded production reference catalogs or runtime fixture fallbacks. Evidence: `AGENTS.md`, `.audit/audit-1782059752.md`, `astral_calculator/RAILGUARD.md`.
- Natal test fixtures belong in root test support, not in production feature modules. Evidence: `tests/common/natal_catalog.rs`, `tests/payload_tests.rs`, `tests/runtime_tests.rs`, `tests/signals_tests.rs`.
- Keep behavior and scenario tests under the root `tests/` tree; do not move broad test coverage into production `src/**/*.rs`. Evidence: `AGENTS.md`, `.audit/audit-1782059752.md`, `astral_calculator/Cargo.toml`.
- The current calculator compatibility-surface slice keeps only explicit shims: `astral_calculator/src/features/mod.rs` must stay free of `payload`/`signals` aliases, `astral_calculator/src/features/natal/application/natal_calculation_service.rs` must keep standard module declarations, `astral_calculator/src/domain/mod.rs` must use explicit `pub use` exports, and in-workspace transport crates must import calculator modules through canonical paths rather than deprecated crate-root aliases. Evidence: `.audit/implementation-audit-1782059165.md`, `astral_calculator/src/features/mod.rs`, `astral_calculator/src/features/natal/application/natal_calculation_service.rs`, `astral_calculator/src/domain/mod.rs`, `astral_calculator_http/src/routes.rs`, `tests/refactor_governance_tests.rs`.
- Do not expand deprecated crate-root compatibility shims in `astral_calculator/src/lib.rs`. New code must use canonical paths under `bootstrap`, `domain`, `engine`, `features`, and `astrology`. Evidence: `astral_calculator/src/lib.rs`, `tests/refactor_governance_tests.rs`.
- Any `astral_calculator` refactor wave that changes structure or ownership must update `docs/BASIC_PAYLOAD_IMPLEMENTATION.md` and close the required adversarial reviews under `docs/reviews/astral_calculator_refactor/` and, when boundary-related, `docs/reviews/astral_calculator_refactor_feature_boundaries/`. Evidence: `AGENTS.md`, `tests/refactor_governance_tests.rs`.
- The 2026-06-21 natal fixture-ownership wave is only considered closed when both `docs/reviews/astral_calculator_refactor/REV-NATAL-TEST-CATALOG-FALLBACK-2026-06-21.md` and `docs/reviews/astral_calculator_refactor_feature_boundaries/REV-NATAL-TEST-CATALOG-FALLBACK-2026-06-21.md` are present and marked closed. Evidence: `.audit/implementation-audit-1782057246.md`, `docs/BASIC_PAYLOAD_IMPLEMENTATION.md`.
- The 2026-06-21 shared non-natal chart-context wave is only considered closed when both `docs/reviews/astral_calculator_refactor/REV-SHARED-NON-NATAL-CHART-CONTEXT-2026-06-21.md` and `docs/reviews/astral_calculator_refactor_feature_boundaries/REV-SHARED-NON-NATAL-CHART-CONTEXT-2026-06-21.md` are present and marked closed. Evidence: `.plan/plan-1782060238.md`, `docs/BASIC_PAYLOAD_IMPLEMENTATION.md`, `astral_calculator/src/application/chart_context.rs`.
- This workspace is local Windows-only by default. Do not introduce Linux/macOS portability work, remote CI assumptions, PR governance steps, or branch workflows unless the user explicitly asks. Evidence: user execution context and `AGENTS.md` git rule for `main`.

## Architecture Boundaries

- `astral_calculator` owns calculator domain logic and runtime persistence; `astral_calculator_http` is HTTP transport over calculator capabilities and should stay transport-focused. Evidence: workspace member names in `Cargo.toml`, `astral_calculator_http/src/lib.rs`, `tests/refactor_governance_tests.rs`.
- `astral_gateway` must not depend directly on internal calculator or LLM crates, and must not embed canonical reference data. Evidence: `tests/refactor_governance_tests.rs`.
- `astral_llm/crates/*` are a separate bounded area; calculator refactors should not spill into them unless a direct contract/regression check requires it. Evidence: workspace member split in `Cargo.toml` and the user’s planning scope.
- Keep workspace-level governance tests and compatibility tests authoritative for boundary rules. If a structural change would invalidate them, update the tests and the relevant railguard/doc artifact in the same slice. Evidence: `tests/refactor_governance_tests.rs`, `tests/deprecated_root_alias_compat_tests.rs`.
- Use the latest audit to drive the next refactor slice: it still flags payload-oriented JSON context in reusable calculation paths, broad `infra/db` forwarding and query hubs, governance tests that are too coupled to incidental file structure, and deprecated crate-root aliases. Those are documented constraints for upcoming plan and dev phases, not optional cleanup. Evidence: `.audit/audit-1782068030.md`.
- New or updated governance assertions in `tests/refactor_governance_tests.rs` should prefer boundary, behavior, type-path, or explicit ownership checks over additional raw filename, line-count, or string-snapshot coupling unless no narrower check can protect the invariant. When a structural assertion remains necessary, keep it scoped to one invariant family and document why a narrower check was insufficient. Evidence: `.audit/audit-1782068030.md`, `tests/refactor_governance_tests.rs`.
- Do not add a new feature-specific internal import path when a shared seam can own the behavior. In particular, non-natal ephemeris-driven flows should converge on one typed application helper instead of each assembling chart objects, aspect definitions, house system, and calculation references independently. Evidence: `.audit/audit-1782059752.md`, `src/application/calculation_references.rs`, `src/features/simplified/service.rs`, `src/features/horoscope/application/horoscope_service.rs`.
- Keep position `facts_json` shaping centralized under `astral_calculator/src/domain/chart_facts.rs`. Reusable astrology code may call typed helpers there, but should not reintroduce local `calculated_position_facts_json` or `angle_position_facts_json` assemblers. Evidence: `astral_calculator/src/domain/chart_facts.rs`, `astral_calculator/src/astrology/ephemeris.rs`, `tests/refactor_governance_tests.rs`.
- No business-rule or payload-assembly module outside `astral_calculator/src/domain/chart_facts.rs` may become the primary owner of raw `facts_json.get(...)` key walking for position context. Consume typed `ObjectPositionFact::context()` or `PositionFactContext` helpers first, and serialize back to JSON only at the current edge helpers. Evidence: `.audit/audit-1782068030.md`, `astral_calculator/src/domain/chart_facts.rs`, `astral_calculator/src/features/natal/payload/rules/rulership.rs`, `tests/position_fact_context_tests.rs`.
- Do not add another hand-rolled ephemeris preload sequence in `simplified` or `horoscope`. Shared loading of `active_chart_objects`, `aspect_definitions`, `house_system`, and calculation references must converge through one application-owned seam before more transit-capable variants are added. Evidence: `.audit/audit-1782059752.md`, `astral_calculator/src/application/calculation_references.rs`, `astral_calculator/src/features/simplified/service.rs`, `astral_calculator/src/features/horoscope/application/horoscope_service.rs`.
- The shared non-natal chart-context seam lives in `astral_calculator/src/application/chart_context.rs` and is the only approved place to assemble `reference_version_id`, chart objects, aspect definitions, house system, and calculation references for simplified/horoscope flows. Evidence: `astral_calculator/src/application/chart_context.rs`, `astral_calculator/src/features/simplified/service.rs`, `astral_calculator/src/features/horoscope/application/horoscope_service.rs`.
- Audits of the current shared chart-context slice must judge Phase 1 against the plan's phase gate before treating later phases as missing implementation. Evidence: `.plan/plan-1782060238.md` section `11. Implementation Agent Prompt`, `astral_calculator/src/application/chart_context.rs`, `tests/calculation_reference_loader_tests.rs`.
- Keep governance checks split by invariant family under root `tests/`. New assertions should join the thematic files (`refactor_governance_tests`, `refactor_governance_runtime_tests`, `refactor_governance_review_tests`) instead of re-forming a single monolith. Evidence: `.plan/plan-1782068489.md`, `tests/refactor_governance_tests.rs`, `tests/refactor_governance_runtime_tests.rs`, `tests/refactor_governance_review_tests.rs`.

## Rust Rules

- Preserve existing focused error boundaries; do not replace them with ad hoc panics or broad dynamic error handling in runtime paths. Evidence: `.audit/audit-1782058187.md` and `tests/refactor_governance_tests.rs`.
- Keep async and blocking boundaries explicit. Do not add `connect_from_env`, `PgPool`, `block_on`, or `run_blocking` into metier modules. Evidence: `AGENTS.md`.
- Respect the existing Cargo test registration model where root `tests/*.rs` files are wired from crate manifests. Evidence: `astral_calculator/Cargo.toml`.
- Prefer normal Rust module layout over `#[path]` indirection for in-crate modules; any new `#[path]` usage needs explicit justification. Evidence: `src/features/natal/application/mod.rs`, `src/features/natal/application/natal_calculation_service.rs`, `tests/refactor_governance_tests.rs`.
- Avoid introducing new `serde_json::Value` carriers in domain/application code when a typed record can own the data. The latest audit still identifies JSON-shaped state in `domain` and `application` as a refactor risk. Evidence: `.audit/audit-1782068030.md`, `.audit/audit-1782059752.md`.
- Do not widen `HoroscopePeriodProfile` or horoscope period builders with additional raw JSON fields. If period-profile data changes, prefer a typed application record first and keep JSON serialization at the adapter boundary. Evidence: `.audit/audit-1782059752.md`, `astral_calculator/src/application/ports.rs`, `astral_calculator/src/features/horoscope/builders.rs`.
- Any reduction of inherent repository wrappers in `astral_calculator/src/infra/db/reference_repository.rs` or `astral_calculator/src/infra/db/calculation_repository.rs` must start from a direct-caller inventory and remove one coherent wrapper family at a time. Retained wrappers need caller evidence; do not delete or keep them speculatively. Evidence: `.audit/audit-1782068030.md`, `.audit/audit-1782066159.md`, `astral_calculator/src/infra/db/reference_repository.rs`, `astral_calculator/src/infra/db/calculation_repository.rs`, `astral_calculator_http/src/reference_status.rs`, and `astral_calculator/src/runtime/mod.rs`.

## Testing and Verification

Run the smallest relevant check first:

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
- `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`
- `cargo test -p astral_calculator_http --test astral_calculator_http_tests`
- `cargo test -p astral_llm_api --test contracts_publish_tests`
- `python scripts/import_json_db_to_postgres.py` after PostgreSQL is up when DB-backed reference or bootstrap changes are involved

## Change Protocol

Before editing:

- inspect `AGENTS.md`, this file, and `astral_calculator/src/lib.rs` if the target is `astral_calculator`
- inspect `astral_calculator/Cargo.toml` to confirm test registration and feature flags
- inspect the root `tests/` files covering the target behavior
- read `.audit/audit-1782059752.md` when the work touches current calculator boundary findings
- read `docs/BASIC_PAYLOAD_IMPLEMENTATION.md` when a change affects payload assembly, fixture ownership, reference loading, or review-tracked refactor waves

During editing:

- keep the slice narrow and phaseable; stop when a planned invariant is only partially closed
- preserve unrelated local changes; do not reset or revert user work
- use the database-first process from `AGENTS.md` before adding any new canonical reference value
- if a refactor wave changes calculator boundaries or fixture or catalog ownership, update the paired adversarial reviews under `docs/reviews/astral_calculator_refactor/` and `docs/reviews/astral_calculator_refactor_feature_boundaries/` in the same slice
- do not use `#[path]` as a workaround for boundary or module-layout drift; fix the module layout instead

Before handing off:

- run the smallest relevant verification first, then broaden only as needed
- report skipped checks and missing environment prerequisites explicitly
- if a guardrail changes, update the relevant railguard and cite the evidence that justified the change

## Known Risks and Open Questions

- `astral_calculator/RAILGUARD.md` is currently untracked in Git. Preserve it and keep the workspace and crate railguards aligned.
- The current audit still flags porous `facts_json` boundaries, broad `infra/db` forwarding, governance assertions that are too coupled to incidental structure, and deprecated crate-root aliases in `astral_calculator`. The production-owned natal catalog fallback is closed in this workspace slice and must not be reintroduced.
- If a new fixture helper is needed for `astral_calculator`, prefer `tests/common/` over new production exports unless a runtime caller genuinely needs the data.
- The exact workspace-wide boundary for future `astral_llm` railguards is not established here; if a refactor moves into `astral_llm/*`, add a crate-level railguard there rather than stretching calculator rules by assumption.
