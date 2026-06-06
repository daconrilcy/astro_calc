# REV-INT-004 — Mercure Phase 4

| ID | Sévérité | Catégorie | Finding | Story corrective | Statut |
|----|----------|-----------|---------|------------------|--------|
| — | — | — | Aucun finding blocker | — | closed |

## Critères vérifiés

- [x] Service Mercure dans `docker-compose.yml`
- [x] Publisher worker : topic `tenants/{tenant_id}/jobs/{run_id}`
- [x] Événement minimal `{ run_id, status, poll_url }`
- [x] `supports_mercure: true` sur `natal_simplified` dans seed
- [x] Publication optionnelle (`ASTRAL_LLM_MERCURE_URL` non défini = no-op)

Gate REV-INT-004 : **OK**
