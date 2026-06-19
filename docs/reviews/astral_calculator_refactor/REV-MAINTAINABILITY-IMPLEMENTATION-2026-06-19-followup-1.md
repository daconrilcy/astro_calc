Status: `closed`

Follow-up:
- re-review after fixing the idempotency lock handoff in natal orchestration.

Validation:
- `calculate_basic_with_catalog()` now keeps the advisory lock from `calculations_for_key()` through `insert_running_calculation()` when execution must continue.
- no reopen found in the split workflow after targeted readback and full calculator test run.

Findings:
- Aucun finding ouvert.
