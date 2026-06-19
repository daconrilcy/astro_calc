# Review maintainability audit - 2026-06-19

## Scope

- `astral_calculator/src/application/ports.rs`
- `astral_calculator/src/features/natal/application/natal_calculation_service.rs`
- `astral_calculator/src/engine/application/runtime_facade_service.rs`
- `astral_calculator/src/runtime/mod.rs`
- `astral_calculator/src/shared/astro_math.rs`
- `astral_calculator/src/domain/chart_facts.rs`
- size scan of `astral_calculator/src/**`

## Findings

### P1 - `NatalCalculationService` still centralizes too many responsibilities

References:

- `astral_calculator/src/features/natal/application/natal_calculation_service.rs:67`
- `astral_calculator/src/features/natal/application/natal_calculation_service.rs:133`
- `astral_calculator/src/features/natal/application/natal_calculation_service.rs:205`

`calculate_basic_with_catalog()` still handles, in one method, reference loading, reference validation, idempotency, stale-run recovery, payload cache reuse, ephemeris execution, signal aggregation, payload building, and persistence completion. This keeps the behavior correct, but it leaves one orchestration method as the mandatory edit point for nearly every natal change.

Impact:

- Violates SRP in practice: one method changes for persistence rules, catalog evolution, ephemeris lifecycle, or payload assembly.
- Raises regression risk because cross-cutting behavior is coupled to one transactional flow.
- Makes testing expensive: many scenarios require exercising the whole saga instead of isolated policies.

Recommended refactor:

- Extract a `NatalReferenceSnapshotLoader` for all pre-transaction reference/catalog loading and validation.
- Extract a `NatalReusePolicy` for completed/running/stale decision logic.
- Extract a `NatalPersistenceWorkflow` for transaction state transitions and heartbeats.
- Keep `NatalCalculationService` as a thin orchestrator over these collaborators.

### P1 - Application ports remain too broad and feature-mixed

References:

- `astral_calculator/src/application/ports.rs:89`
- `astral_calculator/src/application/ports.rs:165`
- `astral_calculator/src/application/ports.rs:215`

The crate no longer depends on `infra` from domain, but some application ports are still "god traits". `ReferenceCatalog` mixes lookup concerns for engine request resolution, natal payload references, horoscope references, and language lookup. `NatalCalculationStore` mixes transaction management, idempotency state, fact persistence, payload reads, payload writes, and signal persistence.

Impact:

- Violates ISP: consumers depend on methods outside their real use-case boundary.
- Slows evolution because any adapter change touches oversized traits and wide mocks.
- Encourages services to stay large since one trait already exposes the whole workflow surface.

Recommended refactor:

- Split `ReferenceCatalog` into narrower ports such as `ReferenceSystemResolver`, `NatalReferenceStore`, and `LocalizationCatalog`.
- Split `NatalCalculationStore` into `CalculationTransactionManager`, `CalculationAttemptStore`, `CalculationFactStore`, `PayloadStore`, and `SignalStore`.
- Make each feature service depend only on the minimum port set it actually consumes.

### P2 - Astrological math still lives under `shared`, against the target architecture

References:

- `astral_calculator/src/shared/astro_math.rs:1`
- `astral_calculator/src/astrology/aspects.rs:6`
- `astral_calculator/src/astrology/ephemeris.rs:83`
- `astral_calculator/src/astrology/house_geometry.rs:4`
- `astral_calculator/src/astrology/transits.rs:5`

The current code explicitly describes `shared::astro_math` as reusable astrological calculations, and `astrology/*` imports it directly. That contradicts the workspace rule that business astrology calculations must live under `astrology/`, not `shared`.

Impact:

- Keeps the canonical home of astrology primitives ambiguous.
- Makes future extraction harder because callers already learn the wrong path.
- Weakens architectural governance: the code works, but not on the declared canonical boundary.

Recommended refactor:

- Move these functions into `astrology::math` or split them into `astrology::angles` and `astrology::zodiac`.
- Keep temporary re-exports from `shared::astro_math` only as migration wrappers if needed.
- Add a governance test preventing new `crate::shared::astro_math` imports from `astrology/` and `features/`.

### P2 - `runtime` still exposes a legacy mixed surface instead of a narrow composition boundary

References:

- `astral_calculator/src/runtime/mod.rs:3`
- `astral_calculator/src/runtime/mod.rs:5`
- `astral_calculator/src/runtime/mod.rs:15`
- `astral_calculator/src/engine/application/runtime_facade_service.rs:13`

`runtime/mod.rs` is still a mixed bag of concrete PostgreSQL composition, feature validator re-exports, and even DB parsing helpers. In parallel, `EngineFacadeService` depends directly on concrete feature request/response types and concrete feature services.

Impact:

- The public internal surface remains harder to reason about than necessary.
- Composition concerns and legacy compatibility helpers stay coupled in one module.
- Replacing one feature service or exposing a new runtime path still requires editing a broad top-level boundary.

Recommended refactor:

- Restrict `runtime` to composition and top-level runtime builders.
- Move validator re-exports behind explicit compatibility modules, or delete them once consumers migrate.
- Introduce narrower engine-facing traits for `natal`, `simplified`, and `horoscope` use cases so `EngineFacadeService` orchestrates capabilities, not concrete service types.

### P2 - Domain facts still rely on opaque JSON blobs for core behavior

References:

- `astral_calculator/src/domain/chart_facts.rs:24`
- `astral_calculator/src/domain/chart_facts.rs:60`
- `astral_calculator/src/domain/chart_facts.rs:82`
- `astral_calculator/src/features/natal/application/natal_calculation_service.rs:286`

Several key domain records still carry `serde_json::Value` bags (`facts_json`, `calculation_notes_json`, `payload_json`) instead of typed substructures. This keeps contracts flexible, but it also hides invariants in string keys and optional runtime probing.

Impact:

- Weakens compile-time guarantees and discoverability.
- Increases accidental duplication of key names and JSON-shape assumptions.
- Makes refactors noisier because behavior can break without type-level guidance.

Recommended refactor:

- Introduce typed structs for the stable subdomains already relied on by runtime logic, starting with angle/object visibility context inside `facts_json`.
- Keep free-form JSON only for genuinely extensible metadata that has no stable consumer.
- Add mapper-level conversion so DB persistence remains unchanged while domain contracts become explicit.

## Recommended order

1. Split `NatalCalculationService` and the oversized application ports together. These two debts reinforce each other.
2. Move astrological math into `astrology/` and lock the boundary with governance tests.
3. Reduce the `runtime` compatibility surface and stop re-exporting lower-level helpers by default.
4. Replace the most-used JSON blobs in domain facts with typed structures.

## Status

Audit only. No code change applied in this review.
