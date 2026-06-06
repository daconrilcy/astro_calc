# REV-INT-002 — Jobs async Phase 2

| ID | Sévérité | Catégorie | Finding | Story corrective | Statut |
|----|----------|-----------|---------|------------------|--------|
| — | — | — | Aucun finding blocker | — | closed |

## Critères vérifiés

- [x] Table `llm_jobs` séparée de `llm_generation_runs`, FK `generation_run_id`
- [x] Index worker queue (`status=queued`) et stale running
- [x] `POST /v1/jobs` / `GET /v1/jobs/{run_id}` avec idempotence tenant-wide
- [x] `IntegrationJobValidator` : enveloppe → payload_contract → gate profil from_payload
- [x] Worker `astral_llm_worker` + `UnifiedReadingOrchestrator`
- [x] `natal_simplified` exécutable ; services `planned` → 404
- [x] Statut initial `queued` partout (DB, API, E2E)
- [x] Gate from_payload : mismatch profil → 422
- [x] Script `test_integration_jobs_e2e.ps1`

Gate REV-INT-002 : **OK**
