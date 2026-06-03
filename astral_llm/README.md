# astral_llm — Gateway LLM astrologique

Service HTTP independant du moteur `astral_calculator`. Transforme un resultat
astrologique calcule en lecture interpretative structuree.

## Demarrage

Les commandes `cargo` fonctionnent depuis **`astral_llm/`** (workspace local) ou depuis la **racine** du depot (`cargo -p astral_llm_api …`).

```powershell
# Copier et renseigner .env a la racine du depot
$env:ASTRAL_LLM_ENV = "local"
cargo run -p astral_llm_api
# ou, depuis astral_llm/ :
cd astral_llm
cargo run
```

En `local` / `test` : bind `127.0.0.1`, `FakeProvider` active par defaut, pas de cle provider requise.

En `production` : `ASTRAL_LLM_API_KEY` obligatoire, au moins une cle provider reelle, `ASTRAL_LLM_ENABLE_FAKE=false`, migrations SQL explicites (`ASTRAL_LLM_DB_AUTO_MIGRATE=false`). Exposition publique (`ASTRAL_LLM_PRODUCTION_MODE=public` ou `ALLOW_PUBLIC_BIND=true`) : PostgreSQL obligatoire (`ENABLE_PERSISTENCE=true`, `DATABASE_URL`).

## Variables principales

| Variable | Defaut (local) | Role |
|---|---|---|
| `ASTRAL_LLM_ENV` | `local` | Profil `local` \| `test` \| `production` |
| `ASTRAL_LLM_HOST` | `127.0.0.1` | Interface d'ecoute |
| `ASTRAL_LLM_ENABLE_FAKE` | `true` (local) | Provider de test sans API externe |
| `ASTRAL_LLM_DEFAULT_PROVIDER` | `fake` (local) | Provider par defaut |
| `ASTRAL_LLM_FALLBACK_PROVIDERS` | *(vide)* | Chaine explicite ; OpenAI n'est plus impose |
| `ASTRAL_LLM_ALLOW_PUBLIC_BIND` | `false` | Requis pour `0.0.0.0` en production |
| `ASTRAL_LLM_DB_AUTO_MIGRATE` | `true` (local) | Interdit en production |

## Crates

| Crate | Role |
|---|---|
| `astral_llm_domain` | Contrats request/response, policies, capabilities |
| `astral_llm_application` | Use cases, safety, prompts, router, normalizer |
| `astral_llm_providers` | Adapters OpenAI, Anthropic, Mistral, Fake |
| `astral_llm_infra` | Config `.env`, validation boot, persistence, referentiel |
| `astral_llm_api` | Serveur Axum |

## Securite

- Auth via `ASTRAL_LLM_API_KEY` en production (header `Authorization: Bearer` ou `X-API-Key`)
- Bind local par defaut (`127.0.0.1:8081`)
- Normalisation du payload astro avant prompt (pas de JSON moteur brut)
- Merge safety non affaiblissant ; rejet safety standardise sans fallback
- OpenAI : `store: false` sur l'API Responses

## Idempotence et limites

- Header `Idempotency-Key` (ou champ `idempotency_key`) pour eviter les doubles generations
- `ASTRAL_LLM_MAX_CONCURRENT_REQUESTS` : limite globale (429 si depasse)
- `ASTRAL_LLM_MAX_CONCURRENT_REQUESTS_PER_KEY` / `ASTRAL_LLM_MAX_REQUESTS_PER_MINUTE_PER_KEY` : quotas par cle API
- `ASTRAL_LLM_MAX_PREMIUM_RUNS_PER_KEY` : quota Premium concurrent par cle
- Circuit breaker provider : etat dans `GET /v1/providers`

## Tests

```powershell
cargo test -p astral_llm_api --test astral_llm_tests
cargo test -p astral_llm_api --test astral_llm_injection_tests
cargo test -p astral_llm_api --test prompt_golden_tests
cargo test -p astral_llm_api --test astral_llm_editorial_fixtures
cargo test -p astral_llm_api --test astral_llm_load_tests
cargo test -p astral_llm_providers --test provider_real_smoke -- --ignored
cargo test -p astral_llm_application
cargo test -p astral_llm_domain
cargo test -p astral_llm_infra
```

Documentation detaillee : `Astral_llm_implementation.md`
