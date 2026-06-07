# REV-007 - Real E2E horoscope period

Status: documented, optional.

Findings checked:

- `scripts/test_horoscope_basic_next_7_days_real_e2e.ps1` exists.
- It requires `OPENAI_API_KEY`.
- It saves outputs under `output/horoscope_period_real/`.
- It is not executed by default in `scripts/docker_update_integration_stack.ps1`.

Correction policy:

- Real E2E structural or evidence failures must be fixed before promoting beyond beta.
- This test is optional for the default Docker smoke because it depends on provider credentials.
