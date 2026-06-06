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

### Mode natal simplifie (v2.4)

1. `POST /v1/calculations/natal/simplified` (`astro_simplified_natal_request_v1` → `astro_simplified_natal_response_v1`)
2. Extraire `simplified_payload.payload` + controles `llm_payload`
3. `POST /v1/readings/generate` avec `interpretation_profile_code: natal_simplified` et `astro_result.contract_version: natal_simplified_structured_v1`

Orchestration one-shot : `POST /v1/readings/natal/simplified` (LLM API, birth → calcul → lecture).

Champs `llm_payload` : `forbidden_interpretation_topics` (canonique) ; `forbidden_topics` = alias déprécié en sortie calculateur.

Smoke rapide : [`scripts/docker_simplified_natal_smoke.ps1`](../scripts/docker_simplified_natal_smoke.ps1).

Suite E2E complète (**12** cas calculateur dont 5 négatifs **422** + **7** lectures positives + **5** négatifs orchestration **400**) : [`scripts/test_natal_simplified_e2e.ps1`](../scripts/test_natal_simplified_e2e.ps1).

Recette OpenAI optionnelle (monitoring qualité, facturée) : `-UseReal -SubmitProfile -TimeoutSec 900` sur la même suite ; seuils `Assert-SimplifiedStrictOpenAiQuality` — voir [`docs/natal_simplified_forbidden_topics.md`](../docs/natal_simplified_forbidden_topics.md).

Documentation métier : [`docs/natal_simplified_reading_contract.md`](../docs/natal_simplified_reading_contract.md), [`docs/natal_simplified_forbidden_topics.md`](../docs/natal_simplified_forbidden_topics.md).

Guide débutant : [docs/GUIDE_DEBUTANT_DOCKER.md](../docs/GUIDE_DEBUTANT_DOCKER.md) §9.

Contrats calculateur supplementaires : voir `versions.json` (`astro_simplified_*`, `natal_simplified_structured_v1`, `llm_projection_natal_simplified_v1`).

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
