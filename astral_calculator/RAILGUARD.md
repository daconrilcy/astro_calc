# Railguard

## Purpose And Scope

This document sets guardrails for the Rust crate `astral_calculator` inside the workspace at `C:\dev\astral_calculation`.
It applies to the crate source under `astral_calculator/src`, the crate-level tests registered in `astral_calculator/Cargo.toml`, and the root `tests/` support files that the crate depends on.

## Project Map

- `astral_calculator/src/main.rs`: binary entrypoint and CLI wiring.
- `astral_calculator/src/lib.rs`: crate root and legacy compatibility exports.
- `astral_calculator/src/bootstrap`: environment, DB, and CLI bootstrap helpers.
- `astral_calculator/src/domain`: domain types and contracts.
- `astral_calculator/src/astrology`: reusable astrology calculations and validation.
- `astral_calculator/src/application`: shared application ports and reference loading.
- `astral_calculator/src/features/natal`, `astral_calculator/src/features/simplified`, `astral_calculator/src/features/horoscope`: product orchestrators.
- `astral_calculator/src/infra/db`: SQLx repositories and runtime queries.
- `tests/`: cross-module and contract tests registered from `astral_calculator/Cargo.toml`.

## Non-Negotiable Invariants

- `src/infra/db` is the only home for SQLx and PostgreSQL access code. Evidence: workspace audit and crate layout in `astral_calculator/src/infra/db`, plus the refactor audit noting `domain -> infra` must not exist.
- Domain and reusable astrology logic must not depend on infrastructure. Evidence: audit confirms no `domain -> infra` or `astrology -> infra` dependency inversion, with reusable calculations under `astral_calculator/src/astrology`.
- Product features are orchestrators, not calculation libraries: `natal`, `simplified`, and `horoscope` validate input, load references/repositories, call shared calculations, then assemble output. Evidence: audit `.\.audit\audit-1782055817.md` and the crate layout under `astral_calculator/src/features`.
- Public JSON contracts and root test registrations are intentionally externalized; do not move behavioral coverage into inline unit tests inside production `src/**/*.rs`. Evidence: the audit found 0 inline `#[cfg(test)]` modules and 0 inline `#[test]` functions in production code, with behavior concentrated under `tests/`.
- Canonical referential data should come from the database when possible, not from hard-coded production fixtures. Evidence: production payload builders now require an explicit `BasicPayloadCatalog` input in `src/features/natal/payload/build/mod.rs`, while the legacy fixture builder was removed from `src/features/natal/catalog.rs`.
- Natal fixture catalogs used by integration tests belong under root test support, not in `src/features/natal/catalog.rs`. Evidence: `tests/common/natal_catalog.rs`, `tests/payload_tests.rs`, `tests/runtime_tests.rs`, `tests/signals_tests.rs`.
- The 2026-06-21 Phase 1 slice removing the production `test_catalog()` fallback is not closed until its paired review artifacts exist under both `docs/reviews/astral_calculator_refactor/` and `docs/reviews/astral_calculator_refactor_feature_boundaries/`. Evidence: `.audit/implementation-audit-1782057246.md`, `docs/BASIC_PAYLOAD_IMPLEMENTATION.md`.
- Legacy compatibility exports may remain only as explicit shims; new code must use canonical paths. Evidence: audit highlights `src/lib.rs`, `src/domain/mod.rs`, `src/features/mod.rs`, and `src/features/natal/application/natal_calculation_service.rs` as overly broad compatibility surfaces.
- `unsafe` should be avoided unless a change has an explicit documented justification and safety contract next to the code. Evidence: no unsafe policy file exists in this crate, so the default must stay conservative.

## Architecture Boundaries

- Keep SQL/runtime wiring in `bootstrap` and `infra/db`; do not introduce DB connection helpers into `domain`, `astrology`, or product feature modules. Evidence: `astral_calculator/src/bootstrap`, `astral_calculator/src/infra/db`, and the audit on dependency direction.
- Keep reusable astrology math in `astral_calculator/src/astrology`; do not add new cross-feature math into `features/shared` or into a product feature namespace. Evidence: audit and crate map.
- Do not reach from `simplified` or `horoscope` into natal internals such as natal-only aspects or ephemeris wrappers. Evidence: audit notes the boundary rule and the existing shared module `astral_calculator/src/astrology`.
- Do not add new runtime fallback paths that silently substitute test fixtures or hard-coded catalogs for DB-backed inputs. Evidence: `src/features/natal/payload/build/mod.rs` now requires an explicit `BasicPayloadCatalog`, and fixture ownership lives in `tests/common/natal_catalog.rs`.
- Keep compatibility wrappers isolated. New code should import canonical modules, not the compatibility aliases from `lib.rs` or broad re-exports from `domain/mod.rs` and `features/mod.rs`. Evidence: audit public-surface findings.

## Rust Rules

- Preserve the crate’s current error model and boundary style; do not replace focused error types with ad hoc panics or broad dynamic error handling without a clear reason.
- Keep async/blocking boundaries explicit. Do not add `connect_from_env`, `PgPool`, `block_on`, or `run_blocking` into metier modules. Evidence: workspace rules and the refactor audit.
- Keep feature flags intentional. The manifest currently exposes `swisseph-engine` and `test-utils`; do not widen feature behavior without updating the relevant tests in `tests/`.
- Treat crate-root compatibility exports as deprecated surface. If a new path is required, add it through canonical modules first and keep any shim narrowly scoped.

## Testing And Verification

Run the smallest relevant check first:

```powershell
cargo test -p astral_calculator --test refactor_governance_tests
```

Before finalizing broader refactors, run:

```powershell
cargo test -p astral_calculator
```

Additional targeted checks already documented for this crate:

- `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`
- `cargo test -p astral_calculator --test runtime_tests`
- `cargo test -p astral_calculator --test payload_tests`
- `cargo test -p astral_calculator --test engine_contract_tests -- --test-threads=1`
- `cargo test -p astral_calculator --test runtime_identity_bootstrap_tests`
- `cargo test -p astral_calculator --test natal_reuse_policy_tests`
- `cargo test -p astral_calculator --test calculation_reference_loader_tests`

## Change Protocol

Before editing:

- inspect `astral_calculator/Cargo.toml` to confirm test registration and feature flags.
- inspect `astral_calculator/src/lib.rs`, `astral_calculator/src/domain/mod.rs`, and `astral_calculator/src/features/mod.rs` before changing public paths.
- inspect the relevant product module and the matching root test under `tests/`.
- read the current audit note in `.\.audit\audit-1782055817.md` when a refactor touches boundaries, catalogs, or orchestration.

During editing:

- keep changes within the crate scope unless the change is explicitly about shared workspace tests or docs.
- prefer narrow refactors that preserve JSON contracts and the existing test layout.
- document each refactor wave in `docs/BASIC_PAYLOAD_IMPLEMENTATION.md` when the work changes structure or ownership.
- if a rule here must change, update this railguard in the same work and cite the file or test that invalidated the old rule.

Before handing off:

- run the smallest relevant test first, then broaden if the change touches multiple layers.
- report any skipped verification and why it was skipped.
- call out any compatibility shim, fallback, or contract change explicitly.

## Known Risks And Open Questions

- The fallback from production payload assembly to `test_catalog()` is closed in the current slice: future edits must keep fixture ownership under `tests/common/natal_catalog.rs` and must not reintroduce `test_catalog()` into `src/`.
- Some compatibility exports may still be consumed by real callers outside the crate. Confirm usage before narrowing them further.
- The shared application seam for chart-context loading is still an open refactor target in the audit; future work should avoid duplicating reference-loading logic across product flows.
