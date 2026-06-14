# REV-007 - Adversarial follow-up findings

Scope: second adversarial cycle after the initial Free implementation.

Findings fixed:

- Interpretation request schema had relaxed `domain_sections` globally to allow
  Free empty sections. Fixed with service-specific schema conditions: Free has
  no `daily_plans` or `domain_sections`; Basic/Premium still require 7
  `daily_plans` and 2 to 5 `domain_sections`.
- Free `free_compact` disabled public daily timeline, but the writer request
  still carried 7 `daily_plans`. Fixed by consuming `include_daily_timeline`
  when building the interpretation request.
- Response schema allowed `watch_summary.status = present` globally. Fixed with
  a polymorphic root `watch_summary` and service-specific narrowing so `present`
  is Free-only.
- Free response repair and validation did not enforce the canonical key-day
  title. Fixed by normalizing `key_days[].title` sur un repère neutre.
- `week_overview` leaks in Free were rejected with a generic code. Fixed with
  `HOROSCOPE_PERIOD_FREE_WEEK_OVERVIEW_LEAK`.

Regression coverage:

- `horoscope_free_next_7_days_interpretation_is_free_compact`
- `horoscope_free_next_7_days_rejects_basic_or_premium_leaks`
- `horoscope_free_next_7_days_rejects_basic_key_day_title`
- `horoscope_free_next_7_days_repair_normalizes_key_day_title`
- `horoscope_period_interpretation_schema_rejects_basic_without_domain_sections`
- `horoscope_period_schema_rejects_basic_present_watch_summary_status`

Status: fixed.
