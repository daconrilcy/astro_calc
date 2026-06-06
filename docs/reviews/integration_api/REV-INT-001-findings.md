# REV-INT-001 — Catalogue services Phase 1

| ID | Sévérité | Catégorie | Finding | Story corrective | Statut |
|----|----------|-----------|---------|------------------|--------|
| — | — | — | Aucun finding blocker | — | closed |

## Critères vérifiés

- [x] Table `llm_integration_services` avec `availability` (pas `is_active`)
- [x] Seeds : `natal_simplified` active, `*_from_payload` planned, full natal planned/active progressif
- [x] `GET /v1/services` public minimal avec `supports_async`, `supports_sync_legacy`, `supports_mercure`
- [x] `GET /v1/services/{code}/contract` avec liens schémas et notes validation
- [x] Script `manage_integration_services.ps1`
- [x] Pas de noms de modèles LLM dans catalogue public

Gate REV-INT-001 : **OK**
