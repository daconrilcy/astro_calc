# astral_llm — Gateway LLM astrologique

Service HTTP independant du moteur `astral_calculator`. Transforme un resultat
astrologique calcule en lecture interpretative structuree.

## Demarrage

Les commandes `cargo` fonctionnent depuis **`astral_llm/`** (workspace local)
ou depuis la **racine** du depot.

La racine du depot reste le point d'entree recommande pour les changements qui
touchent le worker avec d'autres packages du depot, le Docker local, ou les
commandes documentees au niveau parent.

```powershell
# Copier et renseigner .env a la racine du depot
$env:ASTRAL_LLM_ENV = "local"
cargo run -p astral_llm_api
# ou, depuis astral_llm/ :
cd astral_llm
cargo run
```

Pour lancer ou verifier seulement le worker, utiliser la racine du depot.
Le workspace imbrique `astral_llm/` ne reference pas `astral_llm_worker`.
Utiliser aussi la racine quand la verification implique des scripts, Docker,
contrats ou packages hors `astral_llm/`.

```powershell
cargo run -p astral_llm_worker
```

En `local` / `test` : bind `127.0.0.1`, `FakeProvider` active par defaut, pas de cle provider requise.

En `production` : `ASTRAL_LLM_API_KEY` obligatoire, au moins une cle provider reelle, `ASTRAL_LLM_ENABLE_FAKE=false`, migrations SQL explicites (`ASTRAL_LLM_DB_AUTO_MIGRATE=false`). Exposition publique (`ASTRAL_LLM_PRODUCTION_MODE=public` ou `ALLOW_PUBLIC_BIND=true`) : PostgreSQL obligatoire (`ENABLE_PERSISTENCE=true`, `DATABASE_URL`).

## Bootstrap runtime

Le fail-fast reste acceptable uniquement dans les binaires `main.rs`, au moment
de convertir une erreur de demarrage en sortie process. Les helpers de
configuration, persistence, catalogues, providers et composition runtime doivent
evoluer vers des erreurs typees reutilisables, afin d'ameliorer les diagnostics
et de partager le bootstrap entre API et worker.

Lors d'une refactorisation de startup, extraire d'abord des helpers typees qui
retournent `Result<_, BootError>` ou une erreur equivalente, puis garder le
`panic!`/exit au bord binaire si le runtime local doit echouer vite.

Decision actuelle: le roadmap va vers des erreurs de bootstrap typees pour
l'API et le worker. Le fail-fast reste une politique de bord binaire, pas une
raison de garder des `panic!`/`expect()` dans les helpers reutilisables.

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

### Forcer un provider reel en local

Pour que l'UI de test horoscope sorte du chemin `fake`, il faut overrider les
defaults locaux dans `.env` ou dans l'environnement du process:

```powershell
ASTRAL_LLM_ENABLE_FAKE=false
ASTRAL_LLM_DEFAULT_PROVIDER=openai
ASTRAL_LLM_DEFAULT_MODEL=gpt-5-mini
OPENAI_API_KEY=sk-...
```

Variantes:

- `ASTRAL_LLM_DEFAULT_PROVIDER=anthropic` avec `ANTHROPIC_API_KEY`
- `ASTRAL_LLM_DEFAULT_PROVIDER=mistral` avec `MISTRAL_API_KEY`

Si tu veux bloquer tout repli silencieux pendant le debug local, ajoute aussi:

```powershell
ASTRAL_LLM_FALLBACK_ENABLED=false
ASTRAL_LLM_FALLBACK_PROVIDERS=
```

Le moteur horoscope lit aussi la politique produit `horoscope` depuis
`llm_product_default_engine`. Si cette ligne vaut `fake` / `fake-model`, elle
ecrase les variables `.env`. Mettre `config/llm_product_models.conf` a jour puis
lancer `.\scripts\set_product_llm_models.ps1` et redemarrer `astral_llm_api` /
`astral_llm_worker`.

## Crates

| Crate | Role |
|---|---|
| `astral_llm_domain` | Contrats request/response, policies, capabilities |
| `astral_llm_application` | Use cases, safety, prompts, router, normalizer |
| `astral_llm_providers` | Adapters OpenAI, Anthropic, Mistral, Fake |
| `astral_llm_infra` | Config `.env`, validation boot, persistence, referentiel |
| `astral_llm_api` | Serveur Axum |
| `astral_llm_worker` | Worker jobs, membre du workspace parent ; verifier depuis la racine du depot |

## Referentiel canonique

PostgreSQL est la source canonique des donnees produit et referentielles:
policies, profils, modeles, libelles, services d'integration et catalogue
evidence Premium. Les bootstraps Rust existants servent de pont local/test ou
de fallback temporaire, mais ne doivent pas devenir la source de verite pour de
nouvelles donnees configurables.

Le catalogue evidence Premium est en migration: une partie est chargee depuis la
DB, mais `astral_llm_infra/src/evidence_canonical.rs` contient encore un
bootstrap complet utilise quand les lignes DB sont absentes. Les prochains
changements doivent migrer slots, requirements, exclusions et policies vers
PostgreSQL avant de consommer ces valeurs dans le code.

Decision actuelle: `evidence_canonical.rs` n'est pas une source canonique
permanente. Il reste acceptable comme bootstrap local/test et comme seed de
migration tant que PostgreSQL ne couvre pas tout le catalogue. Toute nouvelle
donnee evidence configurable doit d'abord exister en DB.

## Surfaces publiques

Les exports racine de `astral_llm_application` et `astral_llm_domain` sont
consommes par l'API, le worker, les providers, l'infra et les tests racine. Les
reductions de surface doivent donc etre progressives: migrer d'abord les imports
internes vers des chemins de modules explicites, conserver les exports runtime
utilises, puis supprimer seulement les re-exports prouves inutilises.

## Contrats JSON et traces

Avant de decouper les grands orchestrateurs, proteger par tests de
caracterisation les formes JSON publiques ou persistantes:

- API lecture: `GenerateReadingRequest`, `GenerateReadingResponse`,
  `NatalReadingResponse`, erreurs safety/failed et `token_usage`.
- Contrats publies et fixtures sous `contracts/` et `tests/`.
- Enveloppes jobs/idempotence: payload logique de job, statut, reponse rejouee
  depuis `llm_idempotency_records.response_json`.
- Persistance runs: `llm_generation_runs`, `llm_generation_payloads`,
  `llm_generation_steps`, usages token et `RunAuditView`.
- Prompt trace: `llm_generation_prompt_traces` avec `chapter_code`,
  `step_type`, `attempt`, `prompt_family`, `prompt_version`, `message_count`,
  `compiled_prompt` et `messages_json` au format tableau de messages
  `{ role, content }`.
- Raw provider trace: fichiers JSON de debug contenant `trace_id`, timestamps,
  identifiants run/request, provider/model, `raw_text`, `parsed_json`,
  metadata provider et usage.

Les DTOs d'orchestration internes et payloads temporaires peuvent changer, a
condition de conserver ces frontieres et d'ajouter des tests comportementaux ou
golden adaptes avant l'extraction.

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

Avant une slice de roadmap, etablir une baseline avec les commandes les plus
proches du chemin touche:

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

Commandes ciblees par type de changement:

```powershell
# Manifest/workspace
cargo metadata --format-version 1 --no-deps
cd astral_llm
cargo metadata --format-version 1 --no-deps

# Worker ou integration API+worker: privilegier la racine
cargo test -p astral_llm_api --test integration_jobs_tests
cargo test -p astral_llm_api --test integration_services_tests
cargo test -p astral_llm_worker --no-run

# Contrats publics et schemas
cargo test -p astral_llm_api --test contracts_publish_tests

# Evidence/premium/orchestration lecture
cargo test -p astral_llm_api --test astral_llm_evidence_planner_tests
cargo test -p astral_llm_api --test astral_llm_editorial_fixtures
cargo test -p astral_llm_api --test astral_llm_load_tests
```

Documentation detaillee : `Astral_llm_implementation.md`
