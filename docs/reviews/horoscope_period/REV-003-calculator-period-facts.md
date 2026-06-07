# REV-003 - Calculator period facts

Findings checked:

- The calculator endpoint is separate: `POST /v1/calculations/horoscope/period/natal`.
- The calculator receives `period_resolution` and `scan_plan`.
- The calculator does not resolve `period_profile_code`.
- The calculator does not reconstruct snapshots.
- The calculator produces facts, warnings and evidence keys only; no public text or editorial scoring.

Corrections applied:

- Added `calculate_horoscope_period_natal`.
- Added period request/response structs and schema validation in `astral_calculator_api`.
