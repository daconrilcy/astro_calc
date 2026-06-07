# REV-008 - Real period hardening

Status: closed.

Findings corrected:

- Real E2E accepted a period calculation with `fake_period_calculator_v1`.
- Real E2E accepted a fake or templated writer response.
- Public text could expose internal theme codes such as `organization`.
- Period public text repetition was not blocked strongly enough.
- Domain sections could reuse the same evidence set.
- UTC fields required stricter validation in tests and scripts.
- Direct calculator requests could still carry non-UTC offsets or invalid scan
  plans before reaching the runtime path.
- The public library helper `calculate_horoscope_period_natal` still exposed a
  fake period calculator source even after the API runtime used real transit
  snapshots.

Corrections:

- Runtime calculator period now recovers the stored natal input, recalculates
  transit positions for each snapshot through the existing `EphemerisEngine`,
  compares them with persisted natal positions, and emits
  `swisseph_period_calculator_v1` sources.
- Period writer uses the configured LLM provider when the default provider is
  not `fake`; fake writer is reserved for fake smoke mode.
- `theme_label` was added to the period interpretation request contract.
- Period response validation rejects technical code leaks and repetitive
  timeline text.
- Real E2E script rejects fake calculator sources, fake writer provider,
  non-UTC `_utc` fields, repeated timeline entries and shared domain evidence.
- Calculator period request normalization now rewrites all `*_utc` fields to
  real UTC and rejects duplicate snapshot keys, snapshot-count mismatches and
  snapshots outside `[start_datetime_utc, end_datetime_utc)`.
- The public calculator period helper now delegates to the non-fake derived
  fact path. The runtime path with transit snapshots emits
  `swisseph_period_calculator_v1`; the helper fallback emits
  `derived_period_calculator_v1`, never `fake_*`.

Residual note:

- Basic V1 remains `daily_noon_7_days`; no infra-day scan was added.
