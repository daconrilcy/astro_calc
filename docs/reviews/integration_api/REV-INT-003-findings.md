# REV-INT-003 — Full natal Phase 3

| ID | Sévérité | Catégorie | Finding | Story corrective | Statut |
|----|----------|-----------|---------|------------------|--------|
| — | — | — | Aucun finding blocker | — | closed |

## Critères vérifiés

- [x] `engine_reading.rs` : mapping `astro_engine_response_v1` → `generate_reading_request_v1`
- [x] `CalculatorClient.calculate_natal` pour orchestration unified
- [x] `profile_code` catalogue utilisé (pas confondu avec `service_code`)
- [x] Activation `natal_basic` en catalogue pour E2E from-birth
- [x] Script `test_natal_from_birth_e2e.ps1`
- [x] Exemples contrats `contracts/integration/examples/`

Gate REV-INT-003 : **OK**
