# astral_llm — Gateway LLM astrologique

Service Rust independant du moteur de calcul. Il transforme un resultat astrologique
deja calcule en lecture interpretative structuree, securisee et interchangeable entre
fournisseurs LLM.

## Architecture

```txt
astral_llm/
  crates/
    astral_llm_domain/       — contrats (request/response, safety, providers)
    astral_llm_application/  — use cases, prompt compiler, router, validation
    astral_llm_providers/    — adapters OpenAI, Anthropic, Mistral, Fake
    astral_llm_infra/        — config, secrets, persistence, telemetry
    astral_llm_api/          — serveur HTTP Axum
  prompts/
    natal_basic/v1/
    natal_premium/v1/
  sql/
    llm_generation_runs.sql
```

## Commandes

```bash
# Build
cargo build -p astral_llm_api

# Lancer le service (FakeProvider par defaut)
cargo run -p astral_llm_api

# Tests
cargo test -p astral_llm_api --test astral_llm_tests
cargo test -p astral_llm_application
cargo test -p astral_llm_providers
```

## Variables d'environnement

| Variable | Defaut | Description |
|---|---|---|
| `ASTRAL_LLM_HOST` | `0.0.0.0` | Bind host |
| `ASTRAL_LLM_PORT` | `8081` | Bind port |
| `ASTRAL_LLM_PROMPTS_DIR` | `astral_llm/prompts` | Repertoire des prompts versionnes |
| `ASTRAL_LLM_DEFAULT_PROVIDER` | `openai` | Provider si absent de la requete |
| `ASTRAL_LLM_DEFAULT_MODEL` | `gpt-4.1` | Modele si absent de la requete |
| `ASTRAL_LLM_FALLBACK_PROVIDERS` | `openai,mistral,anthropic` | Chaine de fallback (OpenAI toujours en tete) |
| `ASTRAL_LLM_ENABLE_FAKE` | `false` | Active le FakeProvider (dev/tests) |
| `ASTRAL_LLM_ENABLE_PERSISTENCE` | `false` | Active l'audit PostgreSQL |
| `OPENAI_API_KEY` | — | Cle OpenAI (`.env` obligatoire en prod) |
| `OPENAI_BASE_URL` | `https://api.openai.com` | URL API OpenAI |
| `OPENAI_DEFAULT_MODEL` | — | Alias de repli pour `ASTRAL_LLM_DEFAULT_MODEL` |
| `ANTHROPIC_API_KEY` | — | Cle Anthropic |
| `ANTHROPIC_BASE_URL` | `https://api.anthropic.com` | URL API Anthropic |
| `MISTRAL_API_KEY` | — | Cle Mistral |
| `MISTRAL_BASE_URL` | `https://api.mistral.ai` | URL API Mistral |
| `DATABASE_URL` | — | PostgreSQL pour persistence |

Toutes les variables sont declarees dans `.env` / `.env.example` a la racine du depot.

## Securite (post-review)

- Auth API via `ASTRAL_LLM_API_KEY` (`Authorization: Bearer` ou `X-API-Key`), `/health` exempt
- Bind local par defaut (`127.0.0.1`)
- Limites : body 2 Mo, astro JSON 512 Ko, max 12 domaines/chapitres
- Timeout requete HTTP + timeout provider (`engine.timeout_ms` / `ASTRAL_LLM_REQUEST_TIMEOUT_MS`)
- Payload astro encapsule + scan anti-injection
- Erreurs provider sanitisees cote client
- FakeProvider desactive par defaut ; echec au demarrage si aucune cle reelle
- URLs provider allowlist HTTPS (anti-SSRF)
- Referentiels canoniques en base : `llm_astrological_domains`, `llm_safety_content_patterns`, `llm_product_prompt_profiles`

## Endpoints

| Methode | Route | Description |
|---|---|---|
| GET | `/health` | Sante du service |
| POST | `/v1/readings/generate` | Generation d'une lecture |
| POST | `/v1/readings/validate` | Validation JSON contre schema |
| GET | `/v1/providers` | Capacites des providers disponibles |
| GET | `/v1/schemas/{version}` | Schema JSON (ex. `natal_reading_v1`) |

## Flux de generation

```txt
Request -> SafetyGuard (pre) -> PromptCompiler -> ProviderRouter -> LLM
       -> SchemaRegistry validate -> SafetyGuard (post) -> Response
```

Modes :
- `single_pass` : un appel LLM pour toute la lecture (Basic)
- `chapter_orchestrated` : un appel par chapitre avec budget tokens (Premium)

## Securite

Hierarchie stricte :
```txt
Mandatory platform rules > Safety rules > Product contract > Astrologer profile > Request > Astro data
```

L'override `safety_policy` ne peut que renforcer les regles obligatoires.

## Persistence

Tables creees automatiquement si `ASTRAL_LLM_ENABLE_PERSISTENCE=true` :
- `llm_generation_runs` — audit (hashes, latence, provider, status)
- `llm_generation_payloads` — payloads sanitises (optionnel)

Les prompts complets ne sont pas logues par defaut (donnees personnelles).

## Phases implementees

1. Contrats domain (request/response/errors)
2. Serveur Axum + FakeProvider
3. PromptCompiler (prompts versionnes)
4. SchemaRegistry (`natal_reading_v1` via schemars)
5. SafetyResolver (mandatory + product + override)
6. SafetyGuard (pre/post validation)
7. ProviderTrait + ProviderRouter + fallback
8. OpenAI adapter (Responses API)
9. Mistral adapter (json_schema + safe_prompt)
10. Anthropic adapter (output_config.format)
11. Mode chapter_orchestrated
12. Persistence PostgreSQL
