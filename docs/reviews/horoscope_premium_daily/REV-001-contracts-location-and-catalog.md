# REV-001 - Contracts, location and catalog

Status: closed.

Checks:

- `horoscope_premium_daily_local_request` requires `chart_calculation_id`,
  `timezone`, `location.latitude` and `location.longitude`.
- Latitude and longitude are range-limited by schema.
- `birth_data` inline is rejected by `additionalProperties: false`.
- `horoscope_services.json` carries `requires_natal_chart`,
  `requires_location`, `requires_timezone`, `requires_inline_birth_data` and
  `house_system_code`.
- `llm_integration_services.json` exposes the Premium service in `beta` only
  after `service_has_v1_orchestrator` recognizes it.

Findings:

- P1: Premium payload contract was absent from the integration validator.
  Correction: added `horoscope_premium_daily_local_request` to
  `IntegrationJobValidator` and API contract publication.

- P1: Service requirements were implicit.
  Correction: added explicit requirement fields to `horoscope_services.json`.

Result: P0/P1 fixed.
