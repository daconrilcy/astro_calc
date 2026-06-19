Status: `closed`

Scope:
- natal orchestration split into loader/reuse/workflow;
- narrow application ports introduced with repository compatibility;
- astrology math moved to canonical `astrology::angles` and `astrology::zodiac`;
- runtime helper re-exports moved under `runtime::compat`;
- typed `PositionFactContext` added over persisted `facts_json`.

Findings:
- Finding 1: the first implementation released the idempotency-locked transaction between reuse analysis and `insert_running_calculation`, reopening a race window for duplicate `running` rows under concurrent requests.
- Correction: `NatalReusePolicy` now returns either a final payload or the still-open transaction, and `NatalCalculationWorkflow` continues on the same `tx` without releasing the advisory lock.
- Aucun finding ouvert.

Checks:
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator --test position_fact_context_tests`
- `cargo test -p astral_calculator`
- `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`
