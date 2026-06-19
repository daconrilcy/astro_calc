# Review maintainability audit - 2026-06-19 - pass 2

## Scope

- `astral_calculator/src/**`
- `tests/refactor_governance_tests.rs`
- previous refactor review artifacts under `docs/reviews/astral_calculator_refactor/`
- current governance verification: `cargo test -p astral_calculator --test refactor_governance_tests`

## Result

The main architectural boundaries are currently guarded and the governance suite passes:

- no `domain -> infra` dependency found by governance tests;
- business layers do not use `PgPool`, `connect_from_env`, `block_on`, or `run_blocking`;
- `astrology/` and product features no longer import `shared::astro_math`;
- `runtime/mod.rs` is kept as a wiring facade;
- `EngineFacadeService` depends on capability traits rather than concrete feature services;
- `RuntimeRepository` has been reduced to a residual helper.

Remaining refactor opportunities are therefore maintainability improvements, not immediate blocking defects.

## Findings

### P2 - Compatibility aliases keep old public paths attractive

References:

- `astral_calculator/src/lib.rs:29`
- `astral_calculator/src/lib.rs:44`
- `astral_calculator/src/lib.rs:59`

`lib.rs` still exposes historical aliases such as `catalog`, `db`, `dignities`, `ephemeris`, and `facts`. This is intentionally compatible, but it keeps non-canonical paths discoverable for new code, especially `facts -> shared::astro_math` after the internal migration to `astrology::*`.

Impact:

- Weakens the canonical module story for new contributors.
- Can reintroduce ambiguous imports without failing current governance tests.
- Keeps public API cleanup coupled to internal refactor decisions.

Recommended refactor:

- Mark legacy aliases as deprecated with explicit replacement paths where possible.
- Add a governance test forbidding internal crate usage of these aliases.
- Plan removal only after external consumers are migrated.

### P2 - Natal workflow is split logically but still physically concentrated

References:

- `astral_calculator/src/features/natal/application/natal_calculation_service.rs:184`
- `astral_calculator/src/features/natal/application/natal_calculation_service.rs:267`
- `astral_calculator/src/features/natal/application/natal_calculation_service.rs:399`
- `astral_calculator/src/features/natal/application/natal_calculation_service.rs:528`

The previous "god method" has been split into `NatalReferenceSnapshotLoader`, `NatalReusePolicy`, and `NatalCalculationWorkflow`, which is a clear improvement. However, all collaborators remain private in the same 579-line file and still share the same broad generic bounds.

Impact:

- Focused unit testing remains awkward; most behavior is still exercised through the top-level service.
- The file remains a high-conflict edit point for reference loading, idempotency, reuse, ephemeris execution, and persistence.
- The generic bounds duplicate across the internal collaborators, increasing noise and making future splits harder.

Recommended refactor:

- Move `snapshot_loader`, `reuse_policy`, and `workflow` into named submodules under `features/natal/application/`.
- Introduce local type aliases or narrower helper traits for repeated bounds only where it reduces real noise.
- Keep the public `NatalCalculationService` as the composition/orchestration entry point.

### P2 - Persisted status and progress states are still stringly typed

References:

- `astral_calculator/src/features/natal/application/natal_calculation_service.rs:310`
- `astral_calculator/src/features/natal/application/natal_calculation_service.rs:379`
- `astral_calculator/src/features/natal/application/natal_calculation_service.rs:454`
- `astral_calculator/src/features/natal/application/natal_calculation_service.rs:478`
- `astral_calculator/src/features/natal/application/natal_calculation_service.rs:501`

The natal calculation lifecycle compares and writes raw strings for statuses and heartbeat progress states: `completed`, `running`, `calculating_facts`, `aggregating_signals`, `building_payload`.

Impact:

- Typos are not caught at compile time.
- Status evolution requires hunting string literals across service and repository code.
- Repository and domain contracts are less explicit than they need to be.

Recommended refactor:

- Introduce small enums or newtypes for calculation status and progress state.
- Keep DB serialization/deserialization at the infra boundary.
- Add mapper tests for unknown DB values to fail explicitly.

### P2 - Core facts still carry partially typed JSON bags

References:

- `astral_calculator/src/domain/chart_facts.rs:50`
- `astral_calculator/src/domain/chart_facts.rs:60`
- `astral_calculator/src/domain/chart_facts.rs:115`
- `astral_calculator/src/domain/chart_facts.rs:137`

`ObjectPositionFact` now has typed access for `object_context` and `angle_context`, but the stable runtime surface still starts from `facts_json`. `AspectFact::calculation_notes_json` and `InterpretationSignalDraft::payload_json` remain fully opaque.

Impact:

- Stable keys remain invisible to Rust types and can drift silently.
- Reuse and validation logic still depends on optional JSON probing.
- New consumers can accidentally duplicate key names instead of reusing typed accessors.

Recommended refactor:

- Expand typed wrappers for `visibility_context` first, because it is already a runtime invariant.
- Add typed note/payload structs only for keys that have stable consumers.
- Keep arbitrary JSON only for genuinely extensible metadata.

### P3 - Some canonical reference mappings remain code-local

References:

- `astral_calculator/src/features/natal/payload/rules/house_axes.rs:9`
- `astral_calculator/src/features/natal/payload/rules/house_axes.rs:25`
- `astral_calculator/src/features/simplified/resolve.rs:26`
- `astral_calculator/src/features/simplified/resolve.rs:30`
- `astral_calculator/src/features/simplified/resolve.rs:90`
- `astral_calculator/src/features/simplified/resolve.rs:239`

House-axis mappings, simplified calculation scope codes, feature exclusions, limitation codes, and the `world_civil_date_window` policy selector are still embedded in code. Some of these may be protocol constants, but several look like canonical reference values that already belong in DB-backed catalogs.

Impact:

- Product/catalog evolution still requires code changes.
- Risk of divergence from seed data increases as profiles evolve.
- Violates the "base before code" rule where these are reference-data concepts rather than algorithmic constants.

Recommended refactor:

- Classify each literal as either protocol constant, algorithmic invariant, or DB reference value.
- Move DB reference values into existing catalog/repository paths.
- Add targeted governance only for values that are confirmed canonical DB data.

## Recommended order

1. Type lifecycle status/progress states, because it is low risk and reduces runtime ambiguity.
2. Split the natal application collaborators into files once the status types are in place.
3. Type `visibility_context` before broader JSON cleanup.
4. Deprecate legacy public aliases and prevent internal usage.
5. Audit code-local mappings against DB catalogs before moving them.

## Verification

- `cargo test -p astral_calculator --test refactor_governance_tests` passed: 41 tests.

## Status

Audit only. No production code change applied.
