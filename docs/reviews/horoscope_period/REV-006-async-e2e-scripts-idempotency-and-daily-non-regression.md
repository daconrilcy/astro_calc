# REV-006 - Async E2E, scripts, idempotency and daily non-regression

Findings checked:

- The service uses existing `POST /v1/jobs`, worker, polling and idempotency.
- No new job endpoint, worker, table or idempotency mechanism was added.
- `service_has_v1_orchestrator` recognizes the period service.
- Daily Free, Basic and Premium smokes remain separate non-regression checks.
- `scripts/test_horoscope_period_all.ps1` groups Rust checks and fake smoke.

Corrections applied:

- Added period routing in the async execution path now carried by `IntegrationJobExecutor`.
- Added period smoke scripts and Docker update integration.
