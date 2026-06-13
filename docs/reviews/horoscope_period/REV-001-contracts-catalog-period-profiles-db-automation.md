# REV-001 - Contracts, catalogue, period profiles and DB automation

Findings checked:

- The public service is `horoscope_basic_next_7_days_natal`.
- Public payload uses `horoscope_period_natal_request` and does not allow profile overrides.
- `horoscope_services.json` carries `period_profile_code`, `detail_profile_code` and `scan_profile_code`.
- `horoscope_detail_profiles.json` and `horoscope_scan_profiles.json` are imported by the generic `json_db` importer.
- `llm_integration_services.json` publishes the service as `beta`.
- `scripts/docker_update_integration_stack.ps1` keeps the sequence import DB -> submit catalogue -> restart -> readiness -> smoke.

Corrections applied:

- Added period contracts and calculator contracts.
- Added DB seeds for detail and scan profiles.
- Added Docker update smoke integration through `scripts/test_horoscope_period_all.ps1`.
