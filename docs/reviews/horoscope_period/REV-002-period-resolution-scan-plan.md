# REV-002 - Period resolution and scan plan

Findings checked:

- `astral_time_window` is the only resolver for `next_7_days`.
- The horoscope module consumes a resolved period and does not recode the window rules.
- `anchor_date` is interpreted as a local civil date in `timezone`.
- `period_resolution` includes local and UTC boundaries, `end_exclusive` and 7 `included_dates`.
- `scan_plan` contains `snapshot_count`, unique `snapshot_key` values and one noon snapshot per included date.

Corrections applied:

- Added scan-plan validation for duplicate keys and snapshots outside the period.
- Added tests for exclusive end, UTC date shift and included dates.
