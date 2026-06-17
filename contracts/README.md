# Contrats publics Astral

> **Guide débutant** (Docker, APIs, contrats pas à pas) : [docs/GUIDE_DEBUTANT_DOCKER.md](../docs/GUIDE_DEBUTANT_DOCKER.md)

Surface consommable par les applications tierces. **Ce repertoire n est pas une source metier** : les JSON Schema sont derives ou verifies depuis le code Rust ; les OpenAPI decrivent les endpoints HTTP.

## Reseau Docker (Compose local)

Applications sur le reseau `astral_net` :

```txt
http://astral_calculator_api:8080
http://astral_llm_api:8081
http://astral_gateway:8082
```

Depuis l hote : `http://localhost:8080`, `http://localhost:8081` et `http://localhost:8082`.

## Modes d integration

### Mode public recommande — gateway V2

1. `POST /v2/natal/simplified/{free|basic|premium}`
2. `POST /v2/natal/full/{free|basic|premium}`
3. `POST /v2/horoscope/daily/{free|basic|premium}`
4. `POST /v2/horoscope/period/{free|basic|premium}`

La gateway V2 porte la facade publique d'orchestration. Elle appelle `astral_calculator` pour le calcul et `astral_llm` pour la generation, sans exposer les contrats techniques intermediaires.

### Surface calculateur interne — V1

`astral_calculator_api` est une API HTTP technique du calculateur. Les appels
inter-services doivent utiliser les routes canoniques
`/v1/internal/calculations/*`. Les anciennes routes `/v1/calculations/*`
restent disponibles comme aliases legacy compatibles pour l'outillage local et
les scripts existants.

### Mode integration async V1

Catalogue + jobs async pour applications tierces :

1. `GET /v1/services` — catalogue (`active` + `beta`, `?include=planned` optionnel)
2. `GET /v1/services/{code}/contract` — contrat payload métier
3. `POST /v1/jobs` + header `Idempotency-Key` — soumission async (`status: queued`)
4. `GET /v1/jobs/{run_id}` — poll jusqu'à statut terminal

Contrat normatif : [`docs/integration_api_contract.md`](../docs/integration_api_contract.md). Guide : [`docs/integration_api_guide.md`](../docs/integration_api_guide.md).

### Surfaces sync legacy — supprimees du runtime courant

`POST /v1/readings/generate` et `POST /v1/readings/natal/simplified` ont ete retires de `astral_llm_api`.

Les anciens artefacts sync associes sont en cours d'extinction et ne font plus partie du parcours d'integration documente.

Smoke E2E async : [`scripts/test_integration_jobs_e2e.ps1`](../scripts/test_integration_jobs_e2e.ps1) (natal_simplified + worker). Full natal : [`scripts/test_natal_from_birth_e2e.ps1`](../scripts/test_natal_from_birth_e2e.ps1).

Bootstrap catalogue en base : `.\scripts\manage_integration_services.ps1 -Submit` (après import profils).

Worker Docker : service `astral_llm_worker`. Mercure optionnel : `http://localhost:3000` (topic `tenants/{tenant_id}/jobs/{run_id}`).

Schémas : `integration_*_v1.schema.json` dans `contracts/llm/`.

### Mode futur — orchestration interne (hors perimetre V1)

`POST /v1/natal/readings/from-birth` — non implemente.

Le calculateur conserve `POST /v1/calculations/natal/simplified` comme alias
legacy du contrat de calcul partiel. Le chemin canonique inter-services est
`POST /v1/internal/calculations/natal/simplified`. Les champs `llm_payload`
exposent `forbidden_interpretation_topics` comme nom canonique
(`forbidden_topics` = alias de sortie calculateur conserve pour compatibilite
descendante de donnees).

Documentation métier : [`docs/natal_simplified_reading_contract.md`](../docs/natal_simplified_reading_contract.md), [`docs/natal_simplified_forbidden_topics.md`](../docs/natal_simplified_forbidden_topics.md).

Guide débutant : [docs/GUIDE_DEBUTANT_DOCKER.md](../docs/GUIDE_DEBUTANT_DOCKER.md) §9.

## Decouverte des contrats

| Service | Endpoint |
|---------|----------|
| Calculateur | `GET /v1/contracts` |
| LLM | `GET /v1/contracts` |

Chaque reponse inclut les liens vers `/v1/schemas/{version}` et `/openapi.yaml`.

## Versions actives

Voir [versions.json](versions.json).

## Erreurs

Format commun : [common/error_response_v1.schema.json](common/error_response_v1.schema.json).

Readiness : `GET /health/ready` retourne **503** + `error_response_v1` (`SERVICE_NOT_READY`) si le service n est pas pret.

Auth : si `ASTRAL_*_API_KEY` est defini, les routes protegees exigent `Authorization: Bearer <key>` ou `X-API-Key`. En Docker Compose, definir les deux cles dans `.env` avant `docker compose up`.
