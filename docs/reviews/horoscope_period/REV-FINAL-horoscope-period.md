# REV-FINAL - Horoscope period

Final adversarial review status: closed for V1 period implementation with real
E2E hardening.

Checked:

- Contracts, seeds, calculator route, LLM orchestration, worker routing, scripts and docs are aligned.
- Period resolution remains delegated to `astral_time_window`.
- `scan_plan` is generated once in the application and consumed by the calculator.
- Public response is a period response, not a daily response.
- Evidence guards reject missing, invented or out-of-period evidence.
- Docker update runs the grouped period test suite after daily horoscope smokes.
- Runtime period calculation no longer emits fake calculator sources.
- Real E2E rejects fake calculator/writer output, repeated timeline text,
  technical code leaks and non-normalized UTC fields.

Residual limits accepted for V1:

- `daily_noon_7_days` is an intentional Basic approximation.
- Real provider E2E is optional and credential-gated, but strict when run.
