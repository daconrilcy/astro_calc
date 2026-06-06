# REV-007 - Free and Basic non-regression

Status: closed.

Checks:

- Free rejects public `timeline`, `best_slots` and `watch_slots`.
- Basic rejects Premium response shape.
- Premium does not reuse Basic shape.
- Free keeps no public slots; Basic keeps three public slots.

Findings:

- No P0/P1 remaining after test coverage.

Evidence:

- `horoscope_basic_free_non_regression_after_premium_routing`
- `horoscope_basic_free_non_regression_after_premium_validators`
- schema rejection tests for Free/Basic/Premium shape mixing

Result: closed.
