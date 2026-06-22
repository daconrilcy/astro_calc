# astral_llm Railguard

## Purpose And Scope

- Scope: the LLM crates under `C:\dev\astral_calculation\astral_llm`, which are members of the parent workspace [../Cargo.toml](../Cargo.toml) and also mirrored by the local workspace [Cargo.toml](Cargo.toml).
- This document is the complete operational contract for future planning and implementation phases. For prompt-sized handoffs, extract or maintain a short "Execution Rules" summary of at most 10 rules from this source instead of duplicating a second divergent railguard.
- Primary sources for constraints: [../Cargo.toml](../Cargo.toml), [Cargo.toml](Cargo.toml), [README.md](README.md), [../AGENTS.md](../AGENTS.md), and the audit [../.audit/audit-1782103297.md](../.audit/audit-1782103297.md).

## Project Map

- LLM workspace members, declared in both [../Cargo.toml](../Cargo.toml) and [Cargo.toml](Cargo.toml):
  - `crates/astral_llm_domain`
  - `crates/astral_llm_application`
  - `crates/astral_llm_providers`
  - `crates/astral_llm_infra`
  - `crates/astral_llm_api`
  - `crates/astral_llm_worker`
- Composition roots:
  - [crates/astral_llm_api/src/main.rs](crates/astral_llm_api/src/main.rs)
  - [crates/astral_llm_worker/src/main.rs](crates/astral_llm_worker/src/main.rs)
- Public surfaces that matter for refactors:
  - [crates/astral_llm_domain/src/lib.rs](crates/astral_llm_domain/src/lib.rs)
  - [crates/astral_llm_application/src/lib.rs](crates/astral_llm_application/src/lib.rs)
- Test and integration entry points:
  - root `tests/`
  - the commands listed in [README.md](README.md)

## Execution Rules

- Use the parent workspace as the normal Cargo entry point; keep the nested workspace inspectable.
- Keep domain free of application, infra, API, and worker dependencies.
- Decouple application from infra through application-owned ports, traits, and DTOs.
- Treat DB rows as canonical for configurable product/reference data; keep protocol and technical invariants in code.
- Freeze only public, persisted, or externally consumed JSON contracts.
- Keep fail-fast behavior at binary boundaries; internal boot code returns typed errors.
- Prefer canonical module imports; remove deprecated root aliases after caller inventory.
- Treat provider traces as observability/debug data, not domain evidence.
- Put integration and characterization tests under root `tests/` by default.
- Split verification into mandatory compile/tests and environment-dependent smoke checks.

## Non-Negotiable Invariants

- Treat the parent repository workspace as the canonical Cargo entry point for normal commands, including `cargo run -p astral_llm_worker` and package tests. The nested [Cargo.toml](Cargo.toml) exists so `astral_llm/` remains inspectable and locally runnable, but parent [../Cargo.toml](../Cargo.toml) is the source of truth for cross-crate membership and dependency wiring. Evidence: [../Cargo.toml](../Cargo.toml), [Cargo.toml](Cargo.toml), [README.md](README.md), and [../AGENTS.md](../AGENTS.md).
- Keep the nested workspace self-consistent from `astral_llm/`. After any manifest edit, verify `cargo metadata --format-version 1 --no-deps` from this directory still succeeds, then verify the corresponding parent-workspace command from the repository root when the change affects shared dependencies or package membership. Evidence: [Cargo.toml](Cargo.toml) and the audit note that this nested workspace is intentionally inspectable from its own root.
- Do not introduce branch workflows, PR governance, or remote CI assumptions into local refactor plans. The execution context is solo, Windows-only, and local-first. Evidence: [../AGENTS.md](../AGENTS.md) and the current task context.
- By default, keep integration and characterization tests at repository root `tests/`. Do not introduce inline production tests unless explicitly justified by the slice and documented near the change. The audit verified that production `src/` files currently do not contain inline `#[cfg(test)]` modules or inline `#[test]` functions.
- Freeze JSON strictly only where the schema is genuinely public, persisted, or consumed outside the immediate orchestration boundary: API request/response payloads, worker/job envelopes, persisted run/result records, audit outputs, published contract fixtures, externally consumed reading outputs, and JSON used by downstream tools or replay/debug workflows. Internal assembly payloads, temporary orchestration DTOs, intermediate LLM composition structures, and provider-specific scratch traces are internal details and may be reshaped when behavior is protected with characterization tests. Evidence: [README.md](README.md), [../contracts/](../contracts/), and the audit.
- Treat the database as the canonical source for referential product data, model/provider capabilities, safety/catalog rules, profile mappings, thresholds, labels, and policy values. Bootstrap defaults may exist only as transitional bootstrapping or test support; runtime behavior must prefer loaded DB rows when persistence is enabled. Do not add new hard-coded canonical constants in Rust if the value can come from the DB. Do not move protocol identifiers, public Serde field names, public enum variants, compilation feature flags, typed error categories, or purely technical invariants to the DB unless they are intentionally configurable product data. Evidence: [../AGENTS.md](../AGENTS.md), [crates/astral_llm_api/src/main.rs](crates/astral_llm_api/src/main.rs), [crates/astral_llm_worker/src/main.rs](crates/astral_llm_worker/src/main.rs), and the audit finding on canonical evidence rules in [../.audit/audit-1782103297.md](../.audit/audit-1782103297.md).

## Architecture Boundaries

- `astral_llm_domain` must stay free of application, infra, API, and worker dependencies. It owns domain contracts, request/response types, policies, limits, and enums. Evidence: [crates/astral_llm_domain/src/lib.rs](crates/astral_llm_domain/src/lib.rs).
- `astral_llm_application` is the orchestration layer, not a second infra layer. The audit found direct imports of `astral_llm_infra` from application code, including `generate_reading_use_case`, `integration_job_executor`, `provider_factory`, `prompt_trace`, and `raw_provider_trace`. New work must reduce that coupling, not expand it. Evidence: [../.audit/audit-1782103297.md](../.audit/audit-1782103297.md).
- Decouple application from infra through ports/traits owned by application or domain-facing modules. Repositories, persistence, provider traces, HTTP clients, config, and catalog loading should enter application use cases through narrow trait contracts or DTOs; concrete `astral_llm_infra` types belong in composition roots or adapter modules. Evidence: current direct `astral_llm_infra` coupling called out by [../.audit/audit-1782103297.md](../.audit/audit-1782103297.md).
- When a port must be implemented for an infra type that lives in a crate already depended on by `astral_llm_application`, keep the trait owned by application and implement it locally for the infra type rather than making `astral_llm_infra` depend back on application. The Phase 1 calculator slice uses `crates/astral_llm_application/src/core/calculator.rs` to implement the application-owned calculator port for `astral_llm_infra::CalculatorClient`, which preserves dependency direction and avoids a crate cycle. Evidence: [crates/astral_llm_application/src/core/calculator.rs](crates/astral_llm_application/src/core/calculator.rs), [crates/astral_llm_worker/src/main.rs](crates/astral_llm_worker/src/main.rs), and the Phase 1 plan in [../.plan/plan-1782103776.md](../.plan/plan-1782103776.md).
- Application modules must become a real import surface with stable, intentionally named modules and ports, not crate-root facade sprawl. Prefer importing from the owning module (`astral_llm_application::reading_plan::...`, `::simplified_reading::...`, etc.) when the consumer is internal or feature-specific; reserve crate-root `pub use` for externally stable use cases shared by API, worker, or tests. Evidence: the broad current exports in [crates/astral_llm_application/src/lib.rs](crates/astral_llm_application/src/lib.rs) and the audit's public-surface scorecard.
- Application features must not import sibling feature internals as shortcuts. Shared astrological or text-processing logic belongs in a stable shared module with explicit ownership, not in feature-private reuse paths. Evidence: audit notes on boundary drift plus the current feature grouping in [crates/astral_llm_application/src/lib.rs](crates/astral_llm_application/src/lib.rs).
- Composition roots (`astral_llm_api`, `astral_llm_worker`) may assemble config, providers, catalog/bootstrap data, persistence, and use cases. They must not acquire domain or application business rules of their own.
- Run and wire `astral_llm_worker` from the parent workspace by default (`cargo run -p astral_llm_worker` from `C:\dev\astral_calculation`). Do not make worker-only scripts, manifests, or assumptions that require `cd astral_llm` unless the task is explicitly about the nested workspace. Evidence: [../AGENTS.md](../AGENTS.md), [../Cargo.toml](../Cargo.toml), and [Cargo.toml](Cargo.toml).
- Shared bootstrap logic should be factored once and reused. The audit identified duplicated startup composition and fail-fast boot behavior in both entry points; keep entrypoints thin and push reusable assembly behind typed boot helpers when refactoring that area. Evidence: [crates/astral_llm_api/src/main.rs](crates/astral_llm_api/src/main.rs), [crates/astral_llm_worker/src/main.rs](crates/astral_llm_worker/src/main.rs), and [../.audit/audit-1782103297.md](../.audit/audit-1782103297.md).
- Fail-fast boot is allowed only at the binary boundary (`main.rs`) after typed bootstrap has returned a typed error. Internal boot helpers, repositories, config loaders, catalog loaders, and application assembly must return explicit error types instead of calling `panic!`, `expect()`, or `unwrap()` for expected configuration, DB, schema, or provider-bootstrap failures. Evidence: current fail-fast startup code in [crates/astral_llm_api/src/main.rs](crates/astral_llm_api/src/main.rs) and [crates/astral_llm_worker/src/main.rs](crates/astral_llm_worker/src/main.rs).
- Avoid widening crate-root facades without a concrete external consumer. The audit measured broad public surfaces in [crates/astral_llm_domain/src/lib.rs](crates/astral_llm_domain/src/lib.rs) and [crates/astral_llm_application/src/lib.rs](crates/astral_llm_application/src/lib.rs); new code should prefer module-scoped imports and turn modules into the documented import boundary instead of hiding ownership behind root re-exports.
- Deprecated or legacy crate-root aliases in `src/lib.rs` do not need compatibility preservation. Before deleting them, run a repository-wide search for impacted callers, then migrate those callers to canonical module paths and remove the aliases instead of keeping compatibility shims. Evidence: broad current exports in [crates/astral_llm_application/src/lib.rs](crates/astral_llm_application/src/lib.rs) and [crates/astral_llm_domain/src/lib.rs](crates/astral_llm_domain/src/lib.rs).
- Use idiomatic Rust module layout for new extractions: prefer `foo.rs` as a thin facade plus `foo/*.rs` submodules when a root module already exists, and do not introduce ordinary in-crate `#[path = "..."]` wiring. Evidence: current application root module declarations in [crates/astral_llm_application/src/lib.rs](crates/astral_llm_application/src/lib.rs) and the refactor planning rule to keep root module files facade-only unless explicitly justified.

## Rust-Specific Rules

- Keep blocking I/O, environment loading, and database wiring out of domain and pure application helpers. The audit shows these concerns still leak into application and boot code; future changes should move them behind ports or composition-root wiring.
- Prefer typed errors and explicit boot results over `panic!` and `expect()` in new code paths. Existing entrypoints may convert typed boot errors into process failure, but libraries and internal boot code must preserve error categories so tests and callers can distinguish invalid config, missing `DATABASE_URL`, DB connection failure, schema mismatch, catalog load failure, and provider bootstrap failure. Evidence: [../.audit/audit-1782103297.md](../.audit/audit-1782103297.md), [crates/astral_llm_api/src/main.rs](crates/astral_llm_api/src/main.rs), and [crates/astral_llm_worker/src/main.rs](crates/astral_llm_worker/src/main.rs).
- Treat provider traces as observability/debug data, not domain evidence or public contract by default. Trace structures may help reproduce provider behavior, prompt routing, and raw payloads, but application decisions must use validated domain/application data rather than parsing trace payloads as canonical inputs. Evidence: trace-related modules exported in [crates/astral_llm_application/src/lib.rs](crates/astral_llm_application/src/lib.rs) and the audit notes on `prompt_trace` / `raw_provider_trace`.
- Do not add new `pub use` items at crate root unless they are intended as stable API. The current root exports are already broad and contribute to accidental coupling.
- Governance tests must protect architecture boundaries and behavior, not freeze file shape by accident. Brittle string-based, path-shape, or line-count assertions may be relaxed or replaced when they block legitimate refactoring; prefer type-level checks, module-boundary tests, behavior-level characterization tests, forbidden-dependency checks, and public-contract drift tests.
- Treat `astral_llm_worker` as a distinct runtime even when it shares setup with the API. Shared code should live in reusable helpers, not duplicated startup sequences.
- Keep the workspace Windows-first and local-only unless a task explicitly changes scope. Do not optimize for Linux/macOS portability by default.
- Do not use `unsafe` unless a task explicitly justifies it and the justification is recorded in the implementation or review.

## Testing And Verification

- Mandatory compile/test commands should not require secrets, providers, or a running DB unless the touched code specifically needs that integration path:
  - `cargo test -p astral_llm_api --test astral_llm_tests`
  - `cargo test -p astral_llm_api --test astral_llm_injection_tests`
  - `cargo test -p astral_llm_api --test prompt_golden_tests`
  - `cargo test -p astral_llm_api --test astral_llm_editorial_fixtures`
  - `cargo test -p astral_llm_api --test astral_llm_load_tests`
  - `cargo test -p astral_llm_application`
  - `cargo test -p astral_llm_domain`
  - `cargo test -p astral_llm_infra`
- For the current refactor backlog, the latest audit explicitly called out the remaining dependency-direction problem, oversized orchestrators, broad root exports, and canonical-data hard-coding. Use this as the acceptance filter before editing application or infra internals. Evidence: [../.audit/audit-1782103297.md](../.audit/audit-1782103297.md).
- Environment-dependent smoke commands require the relevant `.env`, `DATABASE_URL`, secrets, external provider access, or local services. Run them only when the task touches startup, provider routing, persistence, workers, or real-provider behavior, and report missing prerequisites explicitly:
  - `cargo run -p astral_llm_api`
  - `cargo run -p astral_llm_worker`
  - `cargo test -p astral_llm_providers --test provider_real_smoke -- --ignored`
- For manifest or workspace-shape changes, run `cargo metadata --format-version 1 --no-deps` from `astral_llm/` first.
- Keep `cargo fmt --check` green from `astral_llm/` after refactor slices that touch shared modules, manifests, or cross-crate wiring; this loop verified the calculator-port slice with that command successfully.
- For parent-workspace behavior, run package commands from `C:\dev\astral_calculation`; this is mandatory for worker changes unless the task specifically targets the nested workspace.
- For refactors touching startup, verify both entrypoints still compile from `astral_llm/` with `cargo test -p astral_llm_application --no-run` and `cargo test -p astral_llm_worker --no-run`.
- For the calculator port slice, verify the boundary with:
  - `cargo test -p astral_llm_application --test integration_job_executor_tests`
  - `cargo test -p astral_llm_application --test horoscope_application_builders_tests`
  - `cargo test -p astral_llm_worker --no-run`
  - `rg -n "CalculatorClient|astral_llm_infra" astral_llm/crates/astral_llm_application/src/integration_job_executor.rs astral_llm/crates/astral_llm_application/src/horoscope/orchestrators.rs`

## Change Protocol

- Before structural or boundary-affecting edits, inspect:
  - [../AGENTS.md](../AGENTS.md)
  - [../Cargo.toml](../Cargo.toml)
  - [Cargo.toml](Cargo.toml)
  - [README.md](README.md)
  - [../.audit/audit-1782103297.md](../.audit/audit-1782103297.md)
  - the public crate surfaces in [crates/astral_llm_domain/src/lib.rs](crates/astral_llm_domain/src/lib.rs) and [crates/astral_llm_application/src/lib.rs](crates/astral_llm_application/src/lib.rs)
- Refactor order for structural changes:
  1. Confirm the domain contract or data shape.
  2. Introduce or narrow application ports and services.
  3. Move infra/bootstrap details behind composition-root wiring.
  4. Update API or worker wiring last.
- If a change alters a public schema, runtime contract, or workspace member layout, document the compatibility impact in the task plan before implementation.
- For boundary-only slices such as the Phase 1 calculator port change, keep the workspace/member layout, runtime entrypoints, and README crate inventory unchanged unless the active plan explicitly includes those files. Evidence: the implementation audit in [../.audit/implementation-audit-1782104288.md](../.audit/implementation-audit-1782104288.md) flagged worker/runtime and manifest drift as out-of-scope for the calculator-port slice.
- For refactor waves under the repository-wide `astral_calculator`/LLM cleanup rules, update [../BASIC_PAYLOAD_IMPLEMENTATION.md](../BASIC_PAYLOAD_IMPLEMENTATION.md) with a short implementation summary, invariants, verification commands, and review links, and place required adversarial reviews under [../docs/reviews/](../docs/reviews/) when the task scope calls for them. Evidence: [../AGENTS.md](../AGENTS.md).

## Open Questions Handling Protocol

- Open questions are decision gates, not blockers by default.
- If an open question affects public contracts, persistence, workspace layout, dependency direction, startup behavior, or runtime behavior, resolve it before implementation.
- If an open question affects only internal structure, naming, module layout, or cleanup sequencing, proceed with a documented default assumption.
- Every implementation plan that crosses an unresolved question must record:
  - Question
  - Scope
  - Why it matters
  - Default if unresolved
  - Evidence searched
  - Decision
  - Files affected
  - Rollback risk
- If evidence is insufficient, use the safest local default: preserve public and persisted contracts, remove deprecated aliases if no concrete caller is found, prefer typed internal DTOs over raw JSON for non-public orchestration data, prefer application-owned ports over direct infra imports, prefer DB-loaded canonical data over hard-coded Rust constants, and avoid adding crate-root `pub use` exports.
- Required decision record shape:
  - Open Question:
  - Why it matters:
  - Default rule if unresolved:
  - How to resolve:
  - Decision owner / evidence required:
  - Status:

## Known Decisions, Risks, And Open Questions

### Known Decisions

- Deprecated or legacy crate-root aliases are not protected compatibility surfaces. They may be deleted after repository-wide caller migration.
- Reference-data volatility priority is:
  1. projection/scoring catalogs
  2. system lookup data
  3. natal references
- JSON is frozen only when it is public, persisted, externally consumed, included in contract fixtures, or required for replay/debug compatibility. Internal orchestration JSON may be replaced by typed DTOs.
- DB-backed canonical data is preferred for configurable product/reference data, while protocol identifiers, public enum variants, Serde field names, compile feature flags, typed error categories, and purely technical invariants remain in code unless intentionally product-configurable.

### Known Risks

- `astral_llm_application` still imports `astral_llm_infra` directly in multiple files. Treat this as technical debt to reduce, not as an accepted architecture pattern.
- The current calculator-port slice verified that `integration_job_executor.rs` and `horoscope/orchestrators.rs` no longer reference `CalculatorClient` or `astral_llm_infra` directly; keep future calculator-boundary work at least as strict as that `rg` check.
- Some governance tests may accidentally freeze file shape, strings, paths, or line counts. When such tests block legitimate refactoring, replace them with stronger behavior-level, type-level, public-contract, or dependency-boundary checks.
- Startup logic is duplicated between API and worker entrypoints. When touching boot code, extract typed shared bootstrap helpers and keep `main.rs` thin.
- The latest audit still shows oversized orchestration files in application, broad crate-root exports, and hard-coded canonical data in infra. Before splitting them, decide whether the next slice is dependency-direction cleanup, orchestration decomposition, or DB-backed canonical data migration; when choosing a data slice, use the volatility priority above.
- The current workspace manifest already includes the crates needed by the nested workspace, but this should be re-verified after each manifest edit because workspace metadata regressions are high-impact.

### Decision Gates Before Implementation

- Public JSON contract classification: must be resolved before modifying any JSON shape, Serde struct, fixture, persisted payload, API response, worker envelope, audit output, or replay/debug artifact. Default: protect only API-facing, persisted, worker/job, contract-fixtured, downstream-tool, or replay/debug JSON; otherwise treat as internal and refactorable with characterization tests.
- Deprecated crate-root aliases: must be resolved before deleting aliases from `src/lib.rs`. Resolution method: repository-wide caller search. Default: delete aliases if only tests or internal callers remain, and migrate callers to canonical module paths.
- Inherent repository methods vs application traits: must be resolved before removing inherent repository methods if such methods exist in the touched slice. Resolution method: search all direct callers. Default: standardize on application traits if no concrete direct caller requires inherent methods.

### Refactor-Time Decisions

- JSON-value orientation in horoscope internals: resolve when touching `crates/astral_llm_application/src/horoscope/**` or `crates/astral_llm_application/src/service/horoscope/**`. Default: move toward typed internal intermediates unless evidence shows upstream public contract, persistence, provider, or replay/debug coupling.
- Application ports to introduce first: resolve per refactor slice. Default: introduce the narrowest trait needed by the current use case.
- Governance brittleness: resolve when a test blocks a legitimate refactor. Default: replace string/path/line-count assertions with behavior-level, type-level, public-contract, or forbidden-dependency checks.

### Strategic Priorities

- Start repository/query extraction with projection/scoring catalogs unless the current task explicitly targets another data family.
- Use system lookup data as the second extraction priority.
- Keep natal references canonical and stable with stricter validation and less churn-oriented abstraction.

### Example Decision Record

- Question: Can horoscope request/composition internals move from `serde_json::Value` assembly to typed internal intermediates?
- Scope: only horoscope application internals unless public API payloads, persisted outputs, provider payloads, or replay/debug artifacts are affected.
- Why it matters: typed intermediates reduce accidental schema drift and make validation more reliable.
- Default if unresolved: use typed internal DTOs for orchestration and convert to JSON only at public, persistence, provider, or replay/debug boundaries.
- Evidence searched: API response structs, worker envelope usage, contract fixtures, golden tests, persisted result records, provider payload requirements, and downstream tool consumers.
- Decision: proceed with typed internal intermediates if no external contract coupling is found.
- Files affected: `crates/astral_llm_application/src/horoscope/**`, `crates/astral_llm_application/src/service/horoscope/**`, and related tests under root `tests/`.
- Status: unresolved until classified in the implementation plan for the slice.
