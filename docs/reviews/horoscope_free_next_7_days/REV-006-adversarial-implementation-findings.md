# REV-006 - Adversarial Implementation Findings

Findings:

- P1: `horoscope_period_response_v1` had been relaxed globally for all period
  services. This allowed a Basic/Premium response to pass schema validation
  without the full period shape. Fixed with service-conditional schema rules:
  Free compact shape, Basic timeline shape, Premium windows/strategy shape.
- P2: Free anti-leak guard codes could be hidden by the generic JSON Schema
  `additionalProperties` error. Fixed by checking Free forbidden fields before
  schema validation in `validate_period_response_evidence`.
- P2: Free accepted `watch_summary.status = active`, while the product contract
  only allows `none`, `low`, or `present`. Fixed in Free validation and schema
  conditional rules.

Regression tests added:

- `horoscope_period_schema_rejects_basic_without_timeline_shape`
- `horoscope_period_schema_rejects_premium_without_windows_shape`
- `horoscope_free_next_7_days_rejects_active_watch_summary_status`
- tightened `horoscope_free_next_7_days_rejects_basic_or_premium_leaks`

Status: fixed.

