# REV-GLOBAL — API d'intégration externe

Review adversariale de clôture — périmètre Phases 0–5.

| ID | Sévérité | Catégorie | Finding | Story corrective | Statut |
|----|----------|-----------|---------|------------------|--------|
| ADV-001 | Major | Rétention | TTL jobs arrondie implicitement en semaines + purge physique absente | TTL horaire via `ASTRAL_LLM_IDEMPOTENCY_TTL_HOURS`, purge `llm_jobs` terminaux expirés par API/worker | fixed |
| ADV-002 | Major | Idempotence / sécurité | Replay idempotent possible par une autre clé API du même tenant | `api_key_id` vérifié avant replay ; mismatch → `409 IDEMPOTENCY_CONFLICT` | fixed |
| ADV-003 | Major | Contrat HTTP | Service `active/beta` sans orchestrateur pouvait être enqueued puis échouer côté worker au lieu de `501` | Gate `service_has_v1_orchestrator` avant persistance | fixed |
| ADV-004 | Minor | Validation interne | `IntegrationJobValidator` ne recoupait pas `service_code` enveloppe vs service catalogue | Mismatch explicite → `INVALID_INPUT` | fixed |

## Matrice compatibilité APIs

| Mode | Endpoints | Statut |
|------|-----------|--------|
| **Intégration async V1** | `GET /v1/services`, `GET /v1/services/{code}/contract`, `POST /v1/jobs`, `GET /v1/jobs/{run_id}` | Certifié |
| Orchestration manuelle | `POST /v1/calculations/natal` + `POST /v1/readings/generate` | Legacy maintenu |
| Sync simplified | `POST /v1/readings/natal/simplified` | Legacy maintenu |
| Audit LLM | `GET /v1/runs/{id}` | Interne / ops |
| Sync générique par profil | `POST /v1/readings/natal/{profile}` | **Hors V1** — non implémenté |

## Principes respectés

- Données canoniques en base (`llm_integration_services`)
- `llm_jobs` ≠ `llm_generation_runs`
- Idempotence Stripe-like tenant-wide
- Pas de route sync générique V1
- Worker séparé du gateway HTTP

Gate REV-GLOBAL : **OK** — périmètre intégration V1 clos.
