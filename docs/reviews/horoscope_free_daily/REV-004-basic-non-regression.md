# REV-004 — Non-regression Basic

## Checklist

- [x] Basic conserve `service_code = horoscope_basic_daily_natal_3_slots`.
- [x] Basic conserve `payload_contract = horoscope_basic_daily_natal_request_v1`.
- [x] Basic conserve trois slots publics.
- [x] Basic conserve les labels `Matin`, `Apres-midi`, `Soir`.
- [x] Basic conserve les guards inter-slots.
- [x] Basic n'utilise pas la shape Free.
- [x] Les goldens Basic n'ont pas ete regeneres opportunistement.

## Verification

- `horoscope_basic_daily_does_not_use_free_summary_shape`
- `horoscope_response_schema_accepts_basic_shape`
- `horoscope_interpretation_request_matches_golden`
- `horoscope_response_golden_passes_schema_and_evidence_guard`
