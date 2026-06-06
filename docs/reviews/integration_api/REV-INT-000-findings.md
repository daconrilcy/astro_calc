# REV-INT-000 — Gate contrat Phase 0

Review adversariale du contrat d'intégration V1 avant implémentation.

| ID | Sévérité | Catégorie | Finding | Story corrective | Statut |
|----|----------|-----------|---------|------------------|--------|
| — | — | — | Aucun finding blocker | — | closed |

## Critères vérifiés

- [x] Statuts publics : sémantique terminale / non terminale documentée (`queued` uniquement, pas `pending`)
- [x] Matrice HTTP POST /v1/jobs : tous codes avec exemples JSON
- [x] Idempotence : hash différent **et** service_code différent → 409
- [x] Hashing : canonicalisation JSON + champs job logique documentés
- [x] Replay idempotent `completed` : 200 avec `result`
- [x] `integration_job_request_v1` : `payload` opaque
- [x] Exemple idempotence même clé / autre service_code → 409
- [x] Distinction envelope vs payload métier
- [x] Aucune route sync générique dans contrat V1
- [x] Rétention TTL / purge / 404 post-purge
- [x] Schémas JSON publiés sous `contracts/llm/integration_*_v1.schema.json`

Gate REV-INT-000 : **OK**
