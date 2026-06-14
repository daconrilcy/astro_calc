# astral_llm — état courant

`astral_llm_api` est le service de rendu LLM du workspace Astral.

Responsabilités courantes :

- rendre une lecture à partir d'un payload astro déjà calculé
- exposer la surface d'intégration async V1 (`/v1/services`, `/v1/jobs`)
- exposer des endpoints internes de rendu consommés par `astral_gateway`
- appliquer validation, quality gates, safety, post-process et audit

Le service ne porte plus de routes sync publiques legacy.

## Endpoints actifs

Public / intégration :

- `GET /health`, `GET /health/live`, `GET /health/ready`
- `GET /v1/contracts`
- `GET /openapi.yaml`
- `GET /v1/schemas/{schema_version}`
- `GET /v1/services`
- `GET /v1/services/{service_code}/contract`
- `POST /v1/jobs`
- `GET /v1/jobs/{run_id}`
- `POST /v1/readings/validate`
- `GET /v1/runs/{run_id}` pour l'audit ops

Interne :

- `POST /v1/internal/readings/render`
- `POST /v1/internal/horoscope/daily/render`
- `POST /v1/internal/horoscope/period/render`

Routes retirees :

- `POST /v1/readings/generate`
- `POST /v1/readings/natal/simplified`

## Architecture

Séparation actuelle :

- `astral_gateway` orchestre les parcours publics V2
- `astral_calculator` possède le calcul
- `astral_llm` possède le rendu LLM

Flux natal V2 :

1. validation du contrat public dans la gateway
2. appel calculateur
3. construction d'une `GenerateReadingRequest`
4. appel interne `POST /v1/internal/readings/render`
5. quality gates, safety, post-process
6. mapping vers la réponse publique V2

Flux horoscope V2 :

1. validation du contrat public dans la gateway
2. appel calculateur horoscope
3. appel interne writer daily ou period
4. mapping vers la réponse publique V2

## Scripts courants

Principaux scripts encore utiles :

- `.\scripts\docker_bootstrap.ps1`
- `.\scripts\docker_compose_smoke.ps1`
- `.\scripts\test_integration_jobs_e2e.ps1`
- `.\scripts\test_natal_from_birth_e2e.ps1`
- `.\scripts\test_natal_simplified_calculator.ps1`
- `.\scripts\generate_premium_reading_e2e.ps1`
- `.\scripts\test_natal_premium_profile.ps1`
- `.\scripts\generate_premium_plus_reading_e2e.ps1`
- `.\scripts\test_natal_premium_plus_profile.ps1`
- `.\scripts\docker_premium_openai_e2e.ps1`

Les scripts premium utilisent maintenant l'endpoint interne `POST /v1/internal/readings/render`.

## Contrats et docs de référence

- guide de démarrage : [docs/GUIDE_DEBUTANT_DOCKER.md](C:/dev/astral_calculation/docs/GUIDE_DEBUTANT_DOCKER.md)
- contrats publiés : [contracts/README.md](C:/dev/astral_calculation/contracts/README.md)
- contrat d'intégration V1 : [docs/integration_api_contract.md](C:/dev/astral_calculation/docs/integration_api_contract.md)
- guide d'intégration : [docs/integration_api_guide.md](C:/dev/astral_calculation/docs/integration_api_guide.md)
- contrat métier natal simplifié : [docs/natal_simplified_reading_contract.md](C:/dev/astral_calculation/docs/natal_simplified_reading_contract.md)

## Validation minimale

```powershell
cargo test -p astral_llm_api
cargo test -p astral_gateway
cargo test -p astral_llm_worker
```
