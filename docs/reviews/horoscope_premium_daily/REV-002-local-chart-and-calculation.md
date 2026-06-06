# REV-002 - Local chart and calculation

Status: closed.

Checks:

- Premium slots are generated from local slot references and carry
  `reference_datetime_utc`.
- `local_chart` is required for each Premium slot before scoring proceeds.
- `house_system_code` is passed from the service reference profile into the
  calculator request.
- UTC date shifts are covered by the first Paris slot:
  `2026-06-06 01:00 Europe/Paris` maps to `2026-06-05T23:00:00Z`.

Findings:

- P1: Missing `local_chart` could have reached interpretation.
  Correction: `score_calculation` rejects Premium calculations missing
  Ascendant, MC or houses with `HOROSCOPE_PREMIUM_LOCAL_CHART_MISSING`.

- P1: House system could have become an implicit code default.
  Correction: tests assert `house_system_code = placidus` is sourced from
  `horoscope_services.json` through the calculation request.

Result: P0/P1 fixed.
