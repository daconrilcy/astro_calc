# REV-002 - Period Resolution And Scan

Findings reviewed:

- Period resolution is still delegated to `astral_time_window`.
- `scan_profile_code = daily_noon_7_days`.
- The scan has exactly 7 snapshots and one noon snapshot per included date.
- No new worker, endpoint, jobs table or calculation engine was introduced.

Status: fixed by `horoscope_free_next_7_days_uses_daily_noon_scan`.

