# Contrats publics Astral

> **Guide débutant** (Docker, APIs, contrats pas à pas) : [docs/GUIDE_DEBUTANT_DOCKER.md](../docs/GUIDE_DEBUTANT_DOCKER.md)

Surface consommable par les applications tierces. **Ce repertoire n est pas une source metier** : les JSON Schema sont derives ou verifies depuis le code Rust ; les OpenAPI decrivent les endpoints HTTP.

## Reseau Docker (Compose local)

Applications sur le reseau `astral_net` :

```txt
http://astral_calculator_api:8080
http://astral_llm_api:8081
```

Depuis l hote : `http://localhost:8080` et `http://localhost:8081`.

## Modes d integration

### Mode V1 certifie — orchestration externe

1. `POST /v1/calculations/natal` (contrat `astro_engine_request_v1`)
2. Extraire `audit_payload.payload` de la reponse `astro_engine_response_v1`
3. `POST /v1/readings/generate` (contrat `generate_reading_request_v1`)

`response_contract.generation_mode` est **optionnel** : s'il est omis ou incorrect, l'API l'aligne sur `interpretation_profile_code` (`InterpretationProfileResolver`).

Smoke E2E fake : [`scripts/docker_compose_smoke.ps1`](../scripts/docker_compose_smoke.ps1) (`natal_basic`, provider `fake`).

Voir [integration/engine_to_reading_mapping.md](integration/engine_to_reading_mapping.md).

### Mode futur — orchestration interne (hors perimetre V1)

`POST /v1/natal/readings/from-birth` — non implemente.

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
