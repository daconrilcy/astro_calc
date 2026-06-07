# REV-004 - Events, scoring, projection and evidence

Findings checked:

- `period_events` are built in `astral_llm_application`, after calculator facts are received.
- Events outside `included_dates` are rejected.
- The interpretation request contains no raw transit dump.
- The projection is capped by the Basic profile limits.
- Every public day plan has evidence.

Corrections applied:

- Added `period_events`, `daily_plans`, `key_days`, `best_days`, `watch_days` and domain sections.
- Added evidence guard tests for invented and out-of-period evidence.
