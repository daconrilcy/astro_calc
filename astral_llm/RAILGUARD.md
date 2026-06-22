# astral_llm Railguard

## Purpose And Scope

- Scope: the Rust workspace rooted at `C:\dev\astral_calculation\astral_llm`, plus parent-repository rules that still govern package commands from `C:\dev\astral_calculation`.
- This document is the operational contract for planning and implementation phases. Keep it as the single railguard for this workspace; do not create a second divergent guardrail file for the same scope.
- Evidence base: [../AGENTS.md](../AGENTS.md), [Cargo.toml](Cargo.toml), [README.md](README.md), the requested audit [../.audit/audit-1782116301.md](../.audit/audit-1782116301.md), and the follow-up implementation audit [../.audit/implementation-audit-1782115488.md](../.audit/implementation-audit-1782115488.md).
- Current refactor prompt: `Refactorer \\?\C:\dev\astral_calculation\astral_llm pour ameliorer structure, maintenabilite, evolutivite et robustesse en respectant SOLID, YAGNI, KISS et DRY.`

## Project Map

- Nested workspace members from [Cargo.toml](Cargo.toml):
  - `crates/astral_llm_domain`
  - `crates/astral_llm_application`
  - `crates/astral_llm_providers`
  - `crates/astral_llm_infra`
  - `crates/astral_llm_api`
- `astral_llm_worker` lives under `astral_llm/crates/` but is not a member of the nested [Cargo.toml](Cargo.toml) workspace. Treat any worker membership change as a separate manifest/documentation slice, not incidental fallout from application refactors. Evidence: [README.md](README.md), [crates/astral_llm_worker/Cargo.toml](crates/astral_llm_worker/Cargo.toml), and [../.audit/audit-1782114137.md](../.audit/audit-1782114137.md).
- The latest audit still confirms the same split: `cargo metadata` from `astral_llm/` reports 5 members and omits `crates/astral_llm_worker`, while the worker remains a first-class runtime in the parent workspace. Do not let future refactor plans assume nested-workspace coverage for worker commands unless the manifest is changed in a dedicated slice. Evidence: [../.audit/audit-1782116301.md](../.audit/audit-1782116301.md), [Cargo.toml](Cargo.toml), and [../Cargo.toml](../Cargo.toml).
- The parent workspace [../Cargo.toml](../Cargo.toml) still includes the worker package. Use the repository root as the preferred entrypoint when a change touches parent package wiring, Docker, or API+worker integration beyond the `astral_llm/` subtree.
- Composition roots:
  - [crates/astral_llm_api/src/main.rs](crates/astral_llm_api/src/main.rs)
  - [crates/astral_llm_worker/src/main.rs](crates/astral_llm_worker/src/main.rs)
- Public surfaces that matter for refactors:
  - [crates/astral_llm_domain/src/lib.rs](crates/astral_llm_domain/src/lib.rs)
  - [crates/astral_llm_application/src/lib.rs](crates/astral_llm_application/src/lib.rs)
- Test and integration entry points:
  - root `tests/`
  - the commands listed in [README.md](README.md)

## Non-Negotiable Invariants

- Keep `astral_llm_domain` free of application, infra, API, and worker dependencies. Evidence: [crates/astral_llm_domain/src/lib.rs](crates/astral_llm_domain/src/lib.rs).
- Treat `astral_llm_application` as the orchestration layer, not a second infra layer. The current audit still found direct `astral_llm_infra` coupling in application flows such as `generate_reading_use_case.rs`, `chapter_orchestrator.rs`, `integration_job_executor.rs`, `provider_factory.rs`, `prompt_compiler.rs`, and `summary_synthesizer.rs`. Evidence: [../.audit/audit-1782114137.md](../.audit/audit-1782114137.md).
- The current audit also shows the biggest application hotspots are still monolithic orchestration files: `text_reprocessing.rs`, `chapter_orchestrator.rs`, `horoscope/period/writer.rs`, and `generate_reading_use_case.rs`. New work must not add cross-cutting behavior to these files unless the slice is explicitly about decomposing them. Evidence: [../.audit/audit-1782116301.md](../.audit/audit-1782116301.md).
- Keep DB-backed canonical data in the database when the value is configurable product/reference data. Do not add new hard-coded canonical constants in Rust if the value can come from the DB. Evidence: [../AGENTS.md](../AGENTS.md) and the canonical-data finding in [../.audit/audit-1782114137.md](../.audit/audit-1782114137.md).
- PostgreSQL is the target source of truth for the Premium evidence catalog. `astral_llm_infra/src/evidence_canonical.rs` may remain as a local/test bootstrap or migration seed while the DB rows are incomplete, but new configurable slots, requirements, exclusions, or policies must be inserted into PostgreSQL first and then consumed from repositories/runtime loading. Evidence: [../.audit/audit-1782114137.md](../.audit/audit-1782114137.md).
- Freeze JSON only when it is public, persisted, externally consumed, contract-fixtured, or needed for replay/debug compatibility. Internal orchestration payloads may change when protected by characterization tests. Evidence: [README.md](README.md), [../contracts/](../contracts/), and [../.audit/audit-1782114137.md](../.audit/audit-1782114137.md).
- Before splitting large orchestrators, add or identify characterization coverage for every public or persisted JSON shape crossed by the slice: API reading request/response, job/idempotency envelopes, persisted run/payload/step/token rows, `RunAuditView`, prompt trace records, and raw provider trace files. Evidence: [README.md](README.md), [../contracts/](../contracts/), and [../.audit/audit-1782114137.md](../.audit/audit-1782114137.md).
- Keep fail-fast behavior at binary boundaries only. Internal boot helpers, repositories, config loaders, and application assembly should return typed errors instead of `panic!`, `expect()`, or `unwrap()` for expected failures. Evidence: [crates/astral_llm_api/src/main.rs](crates/astral_llm_api/src/main.rs), [crates/astral_llm_worker/src/main.rs](crates/astral_llm_worker/src/main.rs), and [../.audit/audit-1782114137.md](../.audit/audit-1782114137.md).
- `panic!`/`expect()` remain acceptable only at the `main.rs` boundary after typed boot context has been collected. That constraint matters because the audit still found non-trivial startup duplication and fail-fast boot paths in both API and worker entrypoints. Evidence: [../.audit/audit-1782116301.md](../.audit/audit-1782116301.md), [crates/astral_llm_api/src/main.rs](crates/astral_llm_api/src/main.rs), and [crates/astral_llm_worker/src/main.rs](crates/astral_llm_worker/src/main.rs).
- Runtime-composition refactors should move toward typed bootstrap errors. Local Windows-only runtime may still fail fast in `main.rs`, but reusable config, DB, catalog, provider, trace, and application assembly helpers must return typed `Result` values for diagnostics and API/worker reuse.
- Keep integration and characterization tests under root `tests/` by default. Evidence: the audit verified there are no inline production `#[cfg(test)]` modules or inline `#[test]` functions in `src/`.
- Do not introduce branch workflows, PR governance, or remote CI assumptions into local refactor plans. The execution context is solo, Windows-only, and local-first. Evidence: [../AGENTS.md](../AGENTS.md) and the current task context.
- Do not use `unsafe` unless a task explicitly justifies it and the justification is recorded in the implementation or review.

## Architecture Boundaries

- `astral_llm_domain` owns domain contracts, policies, limits, request/response types, and enums. It must not depend on application, infra, API, or worker code.
- `astral_llm_application` owns use cases, planning, validation, prompt assembly, and orchestration. It must not depend on `astral_llm_infra` for business logic. The audit still classifies that dependency direction as problematic. Evidence: [../.audit/audit-1782114137.md](../.audit/audit-1782114137.md).
- Future plan/dev phases should treat `application -> infra` coupling, oversized orchestrators, and duplicated startup composition as separate slices. Do not bundle them into one refactor wave; the latest audit shows they are independent axes and should close one measurable invariant at a time. Evidence: [../.audit/audit-1782116301.md](../.audit/audit-1782116301.md) and [../.audit/audit-1782111011.md](../.audit/audit-1782111011.md).
- `astral_llm_infra` owns env/config, PostgreSQL persistence, bootstrap data, and external adapters. Business rules do not belong there.
- `astral_llm_api` and `astral_llm_worker` are composition roots. They may assemble config, persistence, providers, and use cases, but they should not own domain policy.
- `CalculatorPort` is application-owned and must stay concrete-adapter-free inside `astral_llm_application`. Concrete calculator bindings belong in runtime or adapter crates such as [crates/astral_llm_worker/src/calculator_port.rs](crates/astral_llm_worker/src/calculator_port.rs), not in [crates/astral_llm_application/src/core/calculator.rs](crates/astral_llm_application/src/core/calculator.rs). Evidence: [crates/astral_llm_application/src/core/calculator.rs](crates/astral_llm_application/src/core/calculator.rs), [crates/astral_llm_worker/src/calculator_port.rs](crates/astral_llm_worker/src/calculator_port.rs), and [../.plan/plan-1782111490.md](../.plan/plan-1782111490.md).
- Shared astrological or text-processing logic must live under explicit reusable modules, not inside a product feature by accident. Evidence: the audit still flags large cross-cutting files such as `chapter_orchestrator.rs`, `generate_reading_use_case.rs`, `horoscope/period/writer.rs`, and `text_reprocessing.rs`.
- Public crate-root exports are not a stable dumping ground. Keep new code on canonical module paths; do not add broad `pub use` surfaces unless there is a real external consumer.
- Public crate-root exports in `astral_llm_application` and `astral_llm_domain` have real consumers in `astral_llm_api`, `astral_llm_worker`, `astral_llm_providers`, `astral_llm_infra`, and root tests. Reduce them only in staged compatibility slices: migrate internal imports to explicit module paths, keep runtime-used exports, then remove only re-exports proven unused.
- Until the dedicated Phase 6 consumer-mapping slice is executed, keep `astral_llm_application/src/lib.rs` as the single crate-root export surface. Do not introduce a separate `public_api.rs` export facade as incidental cleanup. Evidence: `BASIC_PAYLOAD_IMPLEMENTATION.md` 2026-06-22 calculator-port slice and audit `../.audit/implementation-audit-1782111998.md`.
- Parent-workspace commands are required for `astral_llm_worker` and for any slice that touches packages outside `astral_llm/` or depends on parent-level runtime wiring.
- If workspace membership changes again, update [Cargo.toml](Cargo.toml), [README.md](README.md), this railguard, and run metadata checks from both roots before any behavioral edits.
- In `astral_llm_application`, `core/`, `domain/`, `infra/`, and `service/` are transitional facades, not a canonical ownership map. `domain/` currently re-exports `astral_llm_domain`, `infra/` re-exports existing application modules, and `service/` re-exports orchestrators. Do not build new architecture on those folders as if they were settled layers without a dedicated refactor decision.
- Treat `core/`, `domain/`, `infra/`, and `service/` promotion as an explicit architecture task. Until then, new production logic should live in the module that owns the behavior rather than in these facades; facades may expose stable paths only after their ownership meaning is documented.
- Current decision: these folders are not canonical import surfaces. A future architecture slice must either document a real ownership map for them or remove/replace the facade-only paths.
- Runtime consumers define the first tier of crate-root compatibility: `astral_llm_api`, `astral_llm_worker`, `astral_llm_providers`, and `astral_llm_infra`. Root tests and local-only callers may be migrated more aggressively to canonical module paths.
- Use the existing Rust module convention: root files such as `lib.rs`, `horoscope/mod.rs`, and `horoscope/period/mod.rs` should remain thin facades with `mod` declarations, narrow `pub use` exports, and boundary glue only. Do not add meaningful orchestration, parsing, persistence, or provider logic to facade files during refactor slices. Evidence: broad facade surfaces in [crates/astral_llm_application/src/lib.rs](crates/astral_llm_application/src/lib.rs) and [crates/astral_llm_application/src/horoscope/period/mod.rs](crates/astral_llm_application/src/horoscope/period/mod.rs).
- Do not introduce ordinary in-crate `#[path = "..."]` module wiring. Prefer idiomatic `foo.rs` plus `foo/*.rs` or `foo/mod.rs` plus sibling submodules matching the local directory. If `#[path]` is ever required, record why normal Rust module lookup is insufficient.
- For the current Phase 1 boundary pass, narrow the first closed slice to persistence before catalog decoupling. Evidence from local `rg` checks on 2026-06-22: `RunPersistence` is confined to `generate_reading_use_case.rs` and `provider_router.rs`, while `SharedCanonicalCatalog` still spans `generate_reading_use_case.rs`, `chapter_orchestrator.rs`, `summary_synthesizer.rs`, `final_synthesis_synthesizer.rs`, `domain_resolver.rs`, `interpretation_profile_resolver.rs`, `prompt_compiler.rs`, `request_validator.rs`, `safety_guard.rs`, `simplified_reading_guard.rs`, `writing_language.rs`, and related helpers.
- For this persistence slice, the application-owned contract lives in `crates/astral_llm_application/src/reading_persistence.rs`; infra mapping must stay in a dedicated boundary-glue submodule, currently `crates/astral_llm_application/src/reading_persistence/infra_adapter.rs`, rather than leaking infra imports into the port surface. Evidence: follow-up correction after [../.audit/implementation-audit-1782115488.md](../.audit/implementation-audit-1782115488.md).
- `crates/astral_llm_application/src/horoscope/mod.rs` may use the persistence trait DTOs only as compatibility fallout from `GenerateReadingUseCase::persistence()` returning the application port. Do not broaden the horoscope slice further during persistence-boundary work without a dedicated scope note in `BASIC_PAYLOAD_IMPLEMENTATION.md`.
- Because `astral_llm_application` already depends on `astral_llm_infra`, application-owned persistence ports for this slice may need a local adapter wrapper inside `astral_llm_application` to avoid a Cargo cycle. Treat that adapter as boundary glue only, not as a new canonical architecture layer, and keep the trait surface limited to the selected reading-path persistence operations.
- For structural refactors, close one measurable invariant per slice before moving to another axis. Record before/target metrics with local `rg` or Cargo commands, and stop if a slice would require unrelated public API widening. Evidence: [../.audit/audit-1782111011.md](../.audit/audit-1782111011.md) identifies separate axes for workspace shape, application-to-infra coupling, canonical data, hotspots, startup composition, and public exports.
- Do not change the public JSON contracts, persisted trace shapes, or compatibility fixtures in the same slice as a large module split unless the change is explicitly characterized and documented. Evidence: the audit highlights frozen API, job, persistence, prompt-trace, and raw-provider boundaries.
- The workspace-shape finding from [../.audit/audit-1782114137.md](../.audit/audit-1782114137.md) is still open: `astral_llm_worker` is on disk under `crates/` but outside the nested manifest. Treat any change to that fact as a dedicated manifest/documentation slice, not as collateral cleanup.
- The latest audit keeps that workspace-shape finding open. Any future plan that mentions worker commands, workspace membership, or metadata checks must state whether it is targeting the nested `astral_llm/` workspace, the parent repository workspace, or both. Evidence: [../.audit/audit-1782116301.md](../.audit/audit-1782116301.md), [Cargo.toml](Cargo.toml), and [../Cargo.toml](../Cargo.toml).
- The current audit also shows large orchestration files and duplicated startup composition. Do not add new cross-cutting logic to [crates/astral_llm_application/src/chapter_orchestrator.rs](crates/astral_llm_application/src/chapter_orchestrator.rs), [crates/astral_llm_application/src/generate_reading_use_case.rs](crates/astral_llm_application/src/generate_reading_use_case.rs), [crates/astral_llm_application/src/horoscope/period/writer.rs](crates/astral_llm_application/src/horoscope/period/writer.rs), [crates/astral_llm_api/src/main.rs](crates/astral_llm_api/src/main.rs), or [crates/astral_llm_worker/src/main.rs](crates/astral_llm_worker/src/main.rs) unless the slice is specifically about decomposing that file.

## Frozen JSON And Trace Boundaries

- API reading contracts are frozen unless a compatibility decision is recorded: `GenerateReadingRequest`, `GenerateReadingResponse`, `NatalReadingResponse`, `SafetyRejectedResponse`, `GenerationFailedResponse`, `QualityMetadata`, `AstroBasisItem`, and public `token_usage`.
- Published contract schemas, fixtures, and externally consumed outputs under [../contracts/](../contracts/) and root `tests/` are frozen until their expected outputs are intentionally regenerated.
- Job and idempotency contracts are frozen at their persistence/API boundary: logical job payload hashes, submitted job envelopes, job status responses, and `llm_idempotency_records.response_json` replay values.
- Run persistence shapes are frozen at schema and audit boundaries: `llm_generation_runs`, `llm_generation_payloads`, `llm_generation_steps`, run/step token usage tables, and `RunAuditView`.
- Prompt trace persistence is frozen at `llm_generation_prompt_traces`: `chapter_code`, `step_type`, `attempt`, `prompt_family`, `prompt_version`, `message_count`, `compiled_prompt`, and `messages_json`. The `messages_json` shape is a JSON array of provider messages with stable `role` and `content` fields.
- Raw provider trace file shape is a debug/replay boundary. Preserve `trace_id`, `created_at_epoch_ms`, `run_id`, `request_id`, `product_code`, `chapter_code`, requested/used model fields, provider, fallback flag, `raw_text`, `parsed_json`, provider metadata, and usage unless a migration note explains the change.
- Internal orchestration DTOs, temporary `serde_json::Value` assembly, prompt scratch payloads, and provider-specific intermediate structs are not frozen by default. They may be replaced by typed DTOs when the public/persisted boundaries above remain stable and characterization tests cover the refactor.

## Rust-Specific Rules

- Keep blocking I/O, environment loading, and database wiring out of domain and pure application helpers.
- Prefer typed errors and explicit boot results over `panic!` and `expect()` in new code paths.
- For startup code, prefer `BootError`-style enums or structured error types that preserve the failing subsystem: config validation, persistence/schema, canonical catalog, provider secrets/catalog, tracing, HTTP bind, worker job loop, or external service clients.
- `panic!`, `expect()`, and process exit are acceptable only at binary boundaries after the typed error has enough context for diagnosis. Do not bury expected startup failures inside infra loaders or application assembly.
- `astral_llm_infra::config::AppConfig` must surface environment parsing failures through typed results. Invalid base URLs and bind addresses belong to config-loading errors inside infra; only `astral_llm_api/src/main.rs` and `astral_llm_worker/src/main.rs` may convert them into process-fatal startup failures.
- Treat provider traces as observability/debug data, not domain evidence or public contract by default.
- Keep trace-setting assembly centralized in the runtime/composition root rather than scattered through application helpers.
- Avoid widening crate-root facades without a concrete consumer. The audit measured broad public surfaces in [crates/astral_llm_domain/src/lib.rs](crates/astral_llm_domain/src/lib.rs) and [crates/astral_llm_application/src/lib.rs](crates/astral_llm_application/src/lib.rs).
- Do not add new `pub use` items at crate root unless they are intended as stable API.
- When narrowing crate-root exports, verify direct consumers first across `astral_llm/crates` and root `tests/`. Root tests are allowed to be migrated to module paths, but API, worker, infra, and provider consumers require a compatibility plan.
- Keep the workspace Windows-first and local-only unless a task explicitly changes scope.

## Testing And Verification

- Workspace-shape checks:
  - `cargo metadata --format-version 1 --no-deps` from `astral_llm/`
  - `cargo metadata --format-version 1 --no-deps` from the repository root when parent manifests, Docker, or API+worker integration are in scope
  - `cargo test -p astral_llm_worker --no-run` from the repository root after worker or parent-workspace wiring changes
- Core compile/test commands from [README.md](README.md):
  - `cargo test -p astral_llm_application`
  - `cargo test -p astral_llm_domain`
  - `cargo test -p astral_llm_infra`
  - `cargo test -p astral_llm_api --test astral_llm_tests`
  - `cargo test -p astral_llm_api --test astral_llm_injection_tests`
  - `cargo test -p astral_llm_api --test prompt_golden_tests`
  - `cargo test -p astral_llm_api --test astral_llm_editorial_fixtures`
  - `cargo test -p astral_llm_api --test astral_llm_load_tests`
  - `cargo test -p astral_llm_providers --test provider_real_smoke -- --ignored`
- Config-loading slices should verify typed env parsing directly with `cargo test -p astral_llm_infra --test app_config_env_tests`, then re-run `cargo test -p astral_llm_api --test astral_llm_tests` and `cargo test -p astral_llm_worker --no-run` to confirm the binary edges still compile and route startup failures at the process boundary.
- Parent-workspace commands from [../AGENTS.md](../AGENTS.md) still apply when the touched slice includes parent-level package wiring or integration outside `astral_llm/`.
- For boundary work, start with focused `rg` checks against the touched modules before broad test runs. The current audit used this pattern to confirm the remaining `application -> infra` coupling.
- Persistence-port slices should verify the selected closed metric with `rg -n "RunPersistence" astral_llm/crates/astral_llm_application/src/generate_reading_use_case.rs astral_llm/crates/astral_llm_application/src/chapter_orchestrator.rs astral_llm/crates/astral_llm_application/src/provider_router.rs` and `rg -n "astral_llm_infra::" astral_llm/crates/astral_llm_application/src/reading_persistence.rs`, then run `cargo test -p astral_llm_application`, `cargo test -p astral_llm_api --test astral_llm_tests`, `cargo test -p astral_llm_api --test integration_jobs_tests`, and `cargo test -p astral_llm_worker --no-run`.
- Persistence-port slices that remap prompt traces or token-usage DTOs must also keep at least one non-DB regression test under root `tests/` that exercises the application port directly. Evidence: `tests/reading_persistence_tests.rs` covers retry-attempt suffixing and persisted prompt-trace JSON shape without PostgreSQL.
- Calculator-boundary slices should verify both trait ownership and runtime adapter placement with `rg -n "CalculatorPort|CalculatorClient|impl CalculatorPort" astral_llm/crates tests`, then run `cargo test -p astral_llm_application --test integration_job_executor_tests`, `cargo test -p astral_llm_worker --no-run`, and `cargo test -p astral_llm_api --test integration_jobs_tests`.
- Before taking a roadmap slice, validate the current baseline with commands relevant to the touched path. At minimum:
  - manifest/workspace: `cargo metadata --format-version 1 --no-deps` from the repository root and from `astral_llm/`
  - API routes/contracts: `cargo test -p astral_llm_api --test astral_llm_tests`, `cargo test -p astral_llm_api --test contracts_publish_tests`
  - worker or API+worker integration: `cargo test -p astral_llm_worker --no-run`, `cargo test -p astral_llm_api --test integration_jobs_tests`, `cargo test -p astral_llm_api --test integration_services_tests`
  - prompt/orchestration changes: `cargo test -p astral_llm_api --test prompt_golden_tests`, `cargo test -p astral_llm_api --test astral_llm_editorial_fixtures`
  - evidence/premium catalog changes: `cargo test -p astral_llm_api --test astral_llm_evidence_planner_tests`, `cargo test -p astral_llm_api --test astral_llm_load_tests`
  - app/domain boundary changes: `cargo test -p astral_llm_application`, `cargo test -p astral_llm_domain`
- Current observed verification for the 2026-06-22 Phase 1 role/locale slice: `cargo test -p astral_llm_api --test astral_llm_i18n_tests`, `cargo test -p astral_llm_api --test astral_llm_astro_basis_tests`, and `cargo test -p astral_llm_application` all pass after the catalog-plumbing updates and the fake-provider test-fixture hardening. Evidence: local verification run on 2026-06-22 against those three commands.
- Role/locale boundary slices must keep `astro_basis_validator.rs` and `writing_language.rs` free of direct `bootstrap_*` calls. Canonical roles and writing-locale instructions must come from the injected `SharedCanonicalCatalog`, with only a generic language-string fallback when the catalog has no matching locale. Evidence: `crates/astral_llm_application/src/astro_basis_validator.rs`, `crates/astral_llm_application/src/writing_language.rs`, and audit `../.audit/audit-1782116301.md`.
- The fake-provider test seam is part of that contract: when `fake-model` is used in natal chapter-orchestrated tests, generated chapter bodies must stay above the current premium chapter word floor, otherwise `tests/astral_llm_astro_basis_tests.rs` and related orchestration checks fail for provider-fixture reasons instead of boundary regressions. Evidence: `crates/astral_llm_providers/src/fake_provider.rs`, `tests/astral_llm_astro_basis_tests.rs`, and implementation audit `../.audit/implementation-audit-1782117191.md`.

## Change Protocol

- Before structural or boundary-affecting edits, inspect:
  - [../AGENTS.md](../AGENTS.md)
  - [../Cargo.toml](../Cargo.toml)
  - [Cargo.toml](Cargo.toml)
  - [README.md](README.md)
  - [../.audit/audit-1782114137.md](../.audit/audit-1782114137.md)
  - [crates/astral_llm_domain/src/lib.rs](crates/astral_llm_domain/src/lib.rs)
  - [crates/astral_llm_application/src/lib.rs](crates/astral_llm_application/src/lib.rs)
- Refactor order for structural changes:
  1. Confirm the domain contract or data shape.
  2. Introduce or narrow application ports and services.
  3. Move infra/bootstrap details behind composition-root wiring.
  4. Update API or worker wiring last.
- If a change alters a public schema, runtime contract, or workspace member layout, document the compatibility impact in the task plan before implementation.
- Keep this railguard updated when a new refactor wave lands. The repository rules require each wave to be documented in `BASIC_PAYLOAD_IMPLEMENTATION.md` and reviewed adversarially before being treated as closed.
- Workspace-shape waves count even when they only touch `Cargo.toml`, `Cargo.lock`, README, or this railguard. If the nested `astral_llm/` workspace membership, commands, or scope narrative changes, add the slice to `../BASIC_PAYLOAD_IMPLEMENTATION.md` before closing the loop.

## Known Risks And Open Questions

- The worker crate sits inside the `astral_llm/` directory tree but outside the nested workspace manifest. The remaining risk is accidental scope drift when a boundary refactor also edits nested-workspace manifests or docs without an explicit manifest slice.
- Audit `audit-1782114137.md` still shows oversized orchestration files, broad crate-root exports, direct `application -> infra` imports, hard-coded canonical evidence data, and duplicated startup composition. Until those are reduced, avoid adding new cross-cutting behavior to the same modules.
- `astral_llm_application/src/lib.rs` still exposes a broad public surface; any export cleanup should be staged after the consumer graph is mapped, not as a mechanical first step.
- Startup logic is duplicated between API and worker entrypoints. When touching boot code, extract typed shared bootstrap helpers and keep `main.rs` thin; fail fast only after typed boot errors reach the binary boundary.
- Catalog decoupling remains a larger follow-up slice than persistence decoupling. Unless a future pass also retargets the catalog-dependent helpers listed above, do not claim that the reading path is fully free of infra-owned catalog types.
- The Phase 1 verification gate is currently green only because the fake-provider chapter fixture now respects premium length and repetition quality floors. If future profile thresholds tighten again, re-validate `tests/astral_llm_astro_basis_tests.rs` before attributing failures to catalog-boundary work.
- This role/locale slice depends on catalog completeness at runtime. If `SharedCanonicalCatalog` reaches application validation without `astro_basis_roles` or `writing_locales`, the result will now follow catalog state plus generic language fallback instead of silently re-bootstraping canonical data inside application code.
- Heavy functional suites were not run while updating this document. Before structural refactors, validate the touched slice against the root integration tests closest to it using the baseline command matrix above, and report skipped suites with the missing prerequisite or reason.
