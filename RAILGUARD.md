# Railguard

## Purpose And Scope

This document defines the workspace-level guardrails for `C:\dev\astral_calculation`.
It applies to the Cargo workspace declared in `Cargo.toml`, with immediate focus on `astral_calculator` and only direct regression-awareness for `astral_calculator_http`, `astral_gateway`, and `astral_llm/*`.

## Project Map

- `Cargo.toml`: workspace root and canonical member list (`astral_contracts`, `astral_calculator`, `astral_calculator_http`, `astral_gateway`, `astral_time_window`, `astral_llm/crates/*`).
- `AGENTS.md`: authoritative local execution rules, verification commands, DB-first process, and refactor invariants.
- `astral_calculator/RAILGUARD.md`: crate-level operational contract for `astral_calculator`.
- `astral_calculator/src`: calculator domain, application, astrology, feature orchestration, infra/db, runtime wiring.
- `tests/`: root integration, governance, compatibility, and contract tests registered by workspace crates.
- `tests/common/natal_catalog.rs`: non-public natal fixture catalog helper used by root tests after the production fallback removal.
- `docs/BASIC_PAYLOAD_IMPLEMENTATION.md` and `docs/reviews/astral_calculator_refactor*/`: mandatory refactor-wave documentation and adversarial review artifacts.

## Non-Negotiable Invariants

- Keep all SQLx/PostgreSQL access for `astral_calculator` under `astral_calculator/src/infra/db`. Do not move DB access into `domain`, `astrology`, `engine`, or product feature code. Evidence: `AGENTS.md` refactor rules, `astral_calculator/RAILGUARD.md`, `tests/refactor_governance_tests.rs`.
- Treat `astral_calculator/src/astrology` as the only reusable home for cross-feature astrology calculations. Do not add new metier math to `features/shared` or product-specific internals. Evidence: `AGENTS.md`, `tests/refactor_governance_tests.rs`.
- Product features in `astral_calculator` are orchestrators over contracts and shared calculations. They must not import another feature’s internals. Evidence: `AGENTS.md`, `tests/refactor_governance_tests.rs`, `.audit/audit-1782058187.md`.
- Canonical referential data must come from the database path when possible; do not add new hard-coded production reference catalogs or runtime fixture fallbacks. Evidence: `AGENTS.md`, `.audit/audit-1782058187.md`, `astral_calculator/RAILGUARD.md`.
- Natal test fixtures belong in root test support, not in production feature modules. Evidence: `tests/common/natal_catalog.rs`, `tests/payload_tests.rs`, `tests/runtime_tests.rs`, `tests/signals_tests.rs`.
- Keep behavior and scenario tests under the root `tests/` tree; do not move broad test coverage into production `src/**/*.rs`. Evidence: `AGENTS.md`, `.audit/audit-1782058187.md`, `astral_calculator/Cargo.toml`.
- The current calculator compatibility-surface slice keeps only explicit shims: `astral_calculator/src/features/mod.rs` must stay free of `payload`/`signals` aliases, `astral_calculator/src/features/natal/application/natal_calculation_service.rs` must keep standard module declarations, `astral_calculator/src/domain/mod.rs` must use explicit `pub use` exports, and in-workspace transport crates must import calculator modules through canonical paths rather than deprecated crate-root aliases. Evidence: `.audit/implementation-audit-1782059165.md`, `astral_calculator/src/features/mod.rs`, `astral_calculator/src/features/natal/application/natal_calculation_service.rs`, `astral_calculator/src/domain/mod.rs`, `astral_calculator_http/src/routes.rs`, `tests/refactor_governance_tests.rs`.
- Any `astral_calculator` refactor wave that changes structure or ownership must update `docs/BASIC_PAYLOAD_IMPLEMENTATION.md` and close the required adversarial reviews under `docs/reviews/astral_calculator_refactor/` and, when boundary-related, `docs/reviews/astral_calculator_refactor_feature_boundaries/`. Evidence: `AGENTS.md`, `tests/refactor_governance_tests.rs`.
- The 2026-06-21 natal fixture-ownership wave is only considered closed when both `docs/reviews/astral_calculator_refactor/REV-NATAL-TEST-CATALOG-FALLBACK-2026-06-21.md` and `docs/reviews/astral_calculator_refactor_feature_boundaries/REV-NATAL-TEST-CATALOG-FALLBACK-2026-06-21.md` are present and marked closed. Evidence: `.audit/implementation-audit-1782057246.md`, `docs/BASIC_PAYLOAD_IMPLEMENTATION.md`.
- This workspace is local Windows-only by default. Do not introduce Linux/macOS portability work, remote CI assumptions, PR governance steps, or branch workflows unless the user explicitly asks. Evidence: user execution context and `AGENTS.md` git rule for `main`.

## Architecture Boundaries

- `astral_calculator` owns calculator domain logic and runtime persistence; `astral_calculator_http` is HTTP transport over calculator capabilities and should stay transport-focused. Evidence: workspace member names in `Cargo.toml`, `astral_calculator_http/src/lib.rs`, `tests/refactor_governance_tests.rs`.
- `astral_gateway` must not depend directly on internal calculator or LLM crates, and must not embed canonical reference data. Evidence: `tests/refactor_governance_tests.rs`.
- `astral_llm/crates/*` are a separate bounded area; calculator refactors should not spill into them unless a direct contract/regression check requires it. Evidence: workspace member split in `Cargo.toml` and the user’s planning scope.
- Keep workspace-level governance tests and compatibility tests authoritative for boundary rules. If a structural change would invalidate them, update the tests and the relevant railguard/doc artifact in the same slice. Evidence: `tests/refactor_governance_tests.rs`, `tests/deprecated_root_alias_compat_tests.rs`.

## Rust Rules

- Preserve existing focused error boundaries; do not replace them with ad hoc panics or broad dynamic error handling in runtime paths. Evidence: `.audit/audit-1782058187.md`, `astral_calculator/RAILGUARD.md`, governance tests forbidding panic-prone horoscope paths.
- Do not introduce `unsafe` unless the user explicitly approves it and the safety contract is documented next to the code. Evidence: no workspace unsafe policy file exists; current railguards stay conservative.
- Respect the existing Cargo test registration model where root `tests/*.rs` files are wired from crate manifests. Evidence: `astral_calculator/Cargo.toml`.
- Prefer normal Rust module layout over `#[path]` indirection for in-crate modules; any remaining `#[path]` usage needs explicit justification. Evidence: `.audit/audit-1782058187.md`.

## Testing And Verification

Run the smallest relevant check first:

```powershell
cargo test -p astral_calculator --test refactor_governance_tests
```

Before finalizing broader `astral_calculator` changes, run:

```powershell
cargo test -p astral_calculator
```

Additional repository-documented checks:

- `cargo test -p astral_calculator_http --test astral_calculator_http_tests`
- `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`
- `cargo test -p astral_llm_api --test contracts_publish_tests`
- `python scripts/import_json_db_to_postgres.py` after PostgreSQL is up when DB-backed reference/bootstrap changes are involved

## Change Protocol

Before editing:

- inspect `AGENTS.md`, this file, and `astral_calculator/RAILGUARD.md` if the target is `astral_calculator`
- inspect the relevant crate manifest and the root `tests/` files covering the target behavior
- inspect `.audit/audit-1782058187.md` when the work touches current calculator boundary findings

During editing:

- keep the slice narrow and phaseable; stop when a planned invariant is only partially closed
- preserve unrelated local changes; do not reset or revert user work
- use the database-first process from `AGENTS.md` before adding any new canonical reference value

Before handing off:

- run the smallest relevant verification first, then broaden only as needed
- report skipped checks and missing environment prerequisites explicitly
- if a guardrail changes, update the relevant railguard and cite the evidence that justified the change

## Known Risks And Open Questions

- `astral_calculator/RAILGUARD.md` is currently untracked in Git. Preserve it and keep the workspace/root railguard aligned with it.
- The current audit still flags broad compatibility exports, duplicated chart-context loading, porous JSON boundaries, and repository forwarding boilerplate in `astral_calculator`. The production-owned natal catalog fallback is closed in this workspace slice and must not be reintroduced. Evidence: `.audit/audit-1782058187.md`.
- If a new fixture helper is needed for `astral_calculator`, prefer `tests/common/` over new production exports unless a runtime caller genuinely needs the data.
- The exact workspace-wide boundary for future `astral_llm` railguards is not established here; if a refactor moves into `astral_llm/*`, add a crate-level railguard there rather than stretching calculator rules by assumption.
