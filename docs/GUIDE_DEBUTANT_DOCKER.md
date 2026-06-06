# Guide débutant — Docker, APIs et contrats Astral

Ce guide explique comment **installer**, **démarrer** et **utiliser** la stack locale Astral avec Docker Compose : calculateur astral (`astral_calculator_api`), gateway LLM (`astral_llm_api`) et PostgreSQL.

Public visé : développeur ou intégrateur qui découvre le projet et veut un parcours pas à pas, sans supposer une connaissance préalable du dépôt.

---

## Sommaire

1. [Vue d'ensemble](#1-vue-densemble)
2. [Prérequis](#2-prérequis)
3. [Architecture Docker](#3-architecture-docker)
4. [Installation pas à pas](#4-installation-pas-à-pas)
5. [Configuration (.env)](#5-configuration-env)
6. [Éphémérides (Swiss Ephemeris)](#6-éphémérides-swiss-ephemeris)
7. [Référentiel PostgreSQL](#7-référentiel-postgresql)
8. [Démarrer et arrêter la stack](#8-démarrer-et-arrêter-la-stack)
9. [Bootstrap et smoke test](#9-bootstrap-et-smoke-test)
10. [Utiliser les APIs HTTP](#10-utiliser-les-apis-http)
11. [Flux complet calculateur → LLM](#11-flux-complet-calculateur--llm)
12. [Contrats publics](#12-contrats-publics)
13. [Profils d'interprétation et modèles LLM](#13-profils-dinterprétation-et-modèles-llm)
14. [Authentification](#14-authentification)
15. [Erreurs et readiness](#15-erreurs-et-readiness)
16. [Commandes utiles](#16-commandes-utiles)
17. [Dépannage](#17-dépannage)
18. [Aller plus loin](#18-aller-plus-loin)

---

## 1. Vue d'ensemble

Le projet expose **deux services HTTP** complémentaires :

| Service | Port (hôte) | Rôle |
|---------|---------------|------|
| **astral_calculator_api** | `8080` | Calcule un thème natal (positions, maisons, aspects…) à partir de données de naissance |
| **astral_llm_api** | `8081` | Génère une **lecture textuelle** structurée à partir du résultat du calculateur |
| **PostgreSQL** | interne (`5432`) | Référentiels astrologiques, profils LLM, persistance des runs |

En **V1 certifié**, l'orchestration du thème natal **complet** reste **externe** : votre application enchaîne `POST /v1/calculations/natal` puis `POST /v1/readings/generate`.

Pour les **données de naissance partielles** (date seule, sans heure, lieu incomplet…), le produit **natal simplifié** (v2.4) ajoute un parcours **one-shot** : `POST /v1/readings/natal/simplified` calcule et génère la lecture en un seul appel. Voir le [tutoriel débutant §9](#tutoriel-débutant--natal-simplifié-v24).

Les **contrats** (JSON Schema + OpenAPI) vivent dans le dossier [`contracts/`](../contracts/). Ils décrivent les payloads échangés entre services et avec les applications tierces.

---

## 2. Prérequis

Installez sur votre machine :

| Outil | Version minimale | Usage |
|-------|------------------|-------|
| **Docker Desktop** (Windows/macOS) ou Docker Engine + Compose (Linux) | Compose v2 | Conteneurs |
| **Git** | — | Cloner le dépôt |
| **PowerShell** 5.1+ | Windows | Scripts `.ps1` du dépôt |
| **Python 3.10+** | **requis** pour l'installation pas à pas | Import du référentiel JSON → PostgreSQL (`scripts/import_json_db_to_postgres.py`) |
| **Rust** | optionnel | Développement local hors Docker |

Espace disque : prévoir **plusieurs Go** pour les images Docker et le build Rust initial.

---

## 3. Architecture Docker

```
┌─────────────────────────────────────────────────────────────┐
│  Réseau Docker : astral_net                                 │
│                                                             │
│  ┌──────────────┐    ┌─────────────────────┐               │
│  │  postgres    │◄───│ astral_calculator_api│ :8080         │
│  │  :5432       │    └─────────────────────┘               │
│  └──────┬───────┘                                           │
│         │            ┌─────────────────────┐               │
│         └───────────►│   astral_llm_api    │ :8081         │
│                      └─────────────────────┘               │
└─────────────────────────────────────────────────────────────┘
         ▲                              ▲
         │ localhost:5432 (optionnel)   │ localhost:8080 / :8081
         └──────── hôte ────────────────┘
```

### Démarrage de la stack

Une seule commande lance **les trois services** (même réseau `astral_net`, même fichier `.env`) :

```powershell
docker compose up -d --build
```

Pour ne démarrer qu'un sous-ensemble (ex. import DB seul) :

```powershell
docker compose up -d postgres
docker compose up -d postgres astral_calculator_api
```

### Accès réseau

| Depuis | URL calculateur | URL LLM |
|--------|-----------------|---------|
| Votre machine (navigateur, curl, Postman) | `http://localhost:8080` | `http://localhost:8081` |
| Un autre conteneur sur `astral_net` | `http://astral_calculator_api:8080` | `http://astral_llm_api:8081` |

---

## 4. Installation pas à pas

### Étape 1 — Cloner le dépôt

```powershell
git clone <url-du-depot> C:\dev\astral_calculation
cd C:\dev\astral_calculation
```

### Étape 2 — Créer le fichier `.env`

```powershell
Copy-Item .env.example .env
```

Éditez `.env` : au minimum, changez `POSTGRES_PASSWORD`, définissez **`DATABASE_URL`** (voir [section 5](#5-configuration-env)) et des clés API.

### Étape 3 — Préparer les éphémérides

Voir [section 6](#6-éphémérides-swiss-ephemeris). Sans fichiers `.se1`, le calculateur ne passera pas en état **ready**.

### Étape 4 — Lancer PostgreSQL seul (optionnel, pour l'import)

```powershell
docker compose up -d postgres
```

Attendez que le conteneur soit healthy :

```powershell
docker compose ps
```

### Étape 5 — Importer le référentiel en base

```powershell
python scripts/import_json_db_to_postgres.py
```

Ce script lit les fichiers JSON dans `json_db/` et crée/peuple les tables PostgreSQL.

### Étape 6 — Lancer la stack complète

```powershell
docker compose up -d --build
```

Le premier build peut prendre **10 à 30 minutes** (compilation Rust). Si le transfert de contexte dépasse quelques centaines de Mo, vérifiez que `.dockerignore` exclut bien `**/target/` (artefacts Cargo locaux).

### Étape 7 — Bootstrap et vérification

```powershell
.\scripts\docker_bootstrap.ps1
.\scripts\docker_compose_smoke.ps1
```

Si les deux scripts se terminent sans erreur, votre environnement est opérationnel.

---

## 5. Configuration (.env)

Un **seul fichier `.env`** à la racine du dépôt alimente les trois conteneurs :

- `env_file: .env` injecte les variables dans chaque service ;
- `${POSTGRES_*}` dans `docker-compose.yml` est résolu depuis ce même fichier ;
- les **overrides Compose** dans `docker-compose.yml` s'appliquent **aux conteneurs** (ex. `DATABASE_URL` avec hôte `postgres`, `ASTRAL_LLM_PROMPTS_DIR=/app/prompts`, `ASTRAL_LLM_CONTRACTS_DIR=/app/contracts/llm`, `ASTRAL_LLM_ENABLE_PERSISTENCE=true`) — ils ne remplacent pas les besoins des scripts PowerShell lancés **depuis l'hôte**.

Copiez `.env.example` vers `.env`. Variables essentielles pour Docker :

### PostgreSQL

```env
POSTGRES_DB=astral
POSTGRES_USER=postgres
POSTGRES_PASSWORD=change-me          # ← changez impérativement
POSTGRES_PORT=5432                   # utilisé avec docker-compose.dev-db-port.yml
```

### DATABASE_URL (obligatoire pour le bootstrap)

Les scripts `docker_bootstrap.ps1`, `manage_natal_interpretation_profiles.ps1` et `set_product_llm_models.ps1` **exigent** une variable `DATABASE_URL` non vide dans `.env`, même si la connexion réelle passe par `docker compose exec`.

| Situation | Valeur recommandée |
|-----------|-------------------|
| **`psql` absent** (cas le plus courant sous Windows) | N'importe quelle URL non vide suffit ; les scripts utilisent `docker compose exec -T postgres psql` |
| **`psql` installé** sur l'hôte | `postgres://USER:PASS@localhost:5432/DB` + overlay `docker-compose.dev-db-port.yml` |

**Ne pas** utiliser l'hôte `postgres` dans `DATABASE_URL` depuis l'hôte Windows — ce nom DNS n'existe que **dans** le réseau Docker.

Exemple (hôte avec port DB exposé) :

```env
DATABASE_URL=postgres://postgres:change-me@localhost:5432/astral
```

### Calculateur HTTP

```env
ASTRAL_CALCULATOR_API_KEY=ma-cle-calculateur-secrete
ASTRAL_EPHEMERIS_PATH=ephe/se-2026a  # chemin relatif à la racine du dépôt
```

Dans Docker, le chemin interne est `/app/ephe/se-2026a` (volume monté depuis `./ephe`).

### Gateway LLM

```env
ASTRAL_LLM_API_KEY=ma-cle-llm-secrete
ASTRAL_LLM_DEFAULT_PROVIDER=fake     # fake = pas d'appel OpenAI (tests locaux)
ASTRAL_LLM_DEFAULT_MODEL=fake-model
ASTRAL_LLM_ENABLE_PERSISTENCE=false  # ignoré en Docker : Compose force true
ASTRAL_LLM_REQUEST_TIMEOUT_MS=120000
```

Pour des **vraies** générations OpenAI :

```env
ASTRAL_LLM_DEFAULT_PROVIDER=openai
OPENAI_API_KEY=sk-...
ASTRAL_LLM_REQUEST_TIMEOUT_MS=900000   # recommandé pour natal_premium_plus
```

### Exposer PostgreSQL sur l'hôte

Par défaut, PostgreSQL n'est **pas** accessible depuis Windows. Pour `psql` ou DBeaver :

```powershell
docker compose -f docker-compose.yml -f docker-compose.dev-db-port.yml up -d
```

Connexion : `localhost:5432`, base `astral`, user/mot de passe du `.env`.

---

## 6. Éphémérides (Swiss Ephemeris)

Le calculateur s'appuie sur **Swiss Ephemeris**. Les fichiers binaires (`.se1`) ne sont **pas** versionnés dans Git (dossier `ephe/` ignoré).

### Structure attendue

```
ephe/
  se-2026a/
    seas_18.se1
    semo_18.se1
    sepl_18.se1
    ... (autres fichiers .se1)
```

### Obtention des fichiers

1. Téléchargez les éphémérides depuis [Astrodienst Swiss Ephemeris](https://www.astro.com/swisseph/swephinfo_e.htm) (fichiers pour la durée de validité souhaitée, ex. `se-2026a`).
2. Placez-les dans `ephe/se-2026a/` à la racine du dépôt.
3. Redémarrez le calculateur :

```powershell
docker compose restart astral_calculator_api
```

### Vérification

```powershell
curl http://localhost:8080/v1/reference/status -H "Authorization: Bearer ma-cle-calculateur-secrete"
```

Le champ `checks.ephemeris_path` doit être `true` et `status` doit être `"ready"`.

---

## 7. Référentiel PostgreSQL

Les **données canoniques** (signes, planètes, maisons, règles métier, profils LLM…) vivent en base. Le code Rust **lit** ces données ; il ne les duplique pas en dur.

### Import initial

Avec PostgreSQL démarré :

```powershell
docker compose up -d postgres
python scripts/import_json_db_to_postgres.py
```

Le script exécute **`docker compose exec -T postgres psql`** (conteneur `postgres` doit être **up**). Il lit `POSTGRES_USER` et `POSTGRES_DB` depuis `.env` — **`DATABASE_URL` n'est pas utilisé** pour la connexion.

Prérequis : Python 3.10+, stack `docker compose up -d postgres`.

### Après import

Relancez le bootstrap :

```powershell
.\scripts\docker_bootstrap.ps1
```

Le bootstrap (prérequis : stack `up`, import référentiel fait, **`DATABASE_URL` défini**, éphémérides présentes) :
1. Vérifie PostgreSQL et les healthchecks HTTP ;
2. Soumet les **profils d'interprétation** (`config/natal_interpretation_profiles/*.json`) ;
3. Applique les **modèles LLM par produit** (`config/llm_product_models.conf`) ;
4. Redémarre `astral_llm_api` pour recharger le catalogue en mémoire ;
5. Contrôle le référentiel calculateur via `/v1/reference/status`.

---

## 8. Démarrer et arrêter la stack

### Démarrer

```powershell
docker compose up -d --build
```

### Voir l'état

```powershell
docker compose ps
docker compose logs -f astral_calculator_api
docker compose logs -f astral_llm_api
```

### Arrêter (conserve les données PostgreSQL)

```powershell
docker compose down
```

### Tout supprimer y compris le volume PostgreSQL

```powershell
docker compose down -v
```

---

## 9. Bootstrap et smoke test

### Bootstrap — `scripts/docker_bootstrap.ps1`

À lancer **après** chaque `docker compose up` ou changement de profils/modèles :

```powershell
.\scripts\docker_bootstrap.ps1
```

Paramètres optionnels :

```powershell
.\scripts\docker_bootstrap.ps1 -CalculatorUrl "http://localhost:8080" -LlmUrl "http://localhost:8081"
```

### Smoke test E2E — `scripts/docker_compose_smoke.ps1`

Enchaîne un calcul natal réel puis une génération LLM avec le provider **fake** (sans coût API) :

```powershell
.\scripts\docker_compose_smoke.ps1
```

Sortie attendue : `Smoke E2E OK.` avec un `chart_calculation_id`, un `run_id` et **6 chapitres**.

Détails alignés sur [`scripts/docker_compose_smoke.ps1`](../scripts/docker_compose_smoke.ps1) :

| Élément | Valeur |
|---------|--------|
| Fixture calculateur | `contracts/integration/examples/natal_calculation_request_v1.paris_1990.json` |
| Profil LLM | `natal_basic` |
| `generation_mode` | `chapter_orchestrated` (optionnel à l'envoi — l'API l'aligne sur le profil) |
| Provider | `fake` (sans coût OpenAI) |

### E2E Premium / Premium Plus (OpenAI réel)

Test **manuel** avec appels OpenAI facturés (~4–5 min). Nécessite `OPENAI_API_KEY`, `ASTRAL_LLM_API_KEY` et `ASTRAL_LLM_REQUEST_TIMEOUT_MS=900000` dans `.env`.

```powershell
.\scripts\docker_premium_openai_e2e.ps1
```

Options utiles :

```powershell
# Revalider une sortie premium_plus existante sans regénérer
.\scripts\docker_premium_openai_e2e.ps1 -SkipBootstrap -SkipPremium -ValidateOnly `
    -PremiumPlusOutputPath output\premium_plus_reading_e2e_docker.json

# Premium seul
.\scripts\docker_premium_openai_e2e.ps1 -SkipPremiumPlus
```

Sorties par défaut : `output/premium_reading_e2e_docker.json`, `output/premium_plus_reading_e2e_docker.json` (n'écrase pas les artefacts certifiés).

Validation client : [`scripts/test_natal_premium_profile.ps1`](../scripts/test_natal_premium_profile.ps1) et [`scripts/test_natal_premium_plus_profile.ps1`](../scripts/test_natal_premium_plus_profile.ps1).

### Tutoriel débutant — natal simplifié (v2.4)

**À quoi ça sert ?** Produire une **lecture astrologique honnête** quand l'utilisateur ne fournit qu'une **date de naissance** (ou des données incomplètes). Le moteur :

- calcule ce qui est **fiable** (signes planétaires stables sur une fenêtre d'incertitude) ;
- marque les faits **ambiguës** (ex. Lune à cheval sur deux signes) ;
- **n'invente pas** Ascendant, maisons ni secte si l'entrée ne le permet pas ;
- renvoie `reading_completeness: partial` (V1).

**Deux façons d'intégrer** (voir aussi [`contracts/README.md`](../contracts/README.md)) :

| Mode | Appels | Quand l'utiliser |
|------|--------|------------------|
| **Orchestration one-shot** | `POST /v1/readings/natal/simplified` | Parcours produit « date → texte » (recommandé) |
| **Orchestration manuelle** | `POST /v1/calculations/natal/simplified` puis `POST /v1/readings/generate` | Contrôle fin du payload intermédiaire |

#### Étape 0 — Prérequis (une seule fois)

```powershell
docker compose up -d --build
python scripts/import_json_db_to_postgres.py   # tables simplified + référentiel
.\scripts\docker_bootstrap.ps1               # profils LLM dont natal_simplified
```

Vérifier que les services répondent :

```powershell
Invoke-RestMethod http://localhost:8080/health/ready
Invoke-RestMethod http://localhost:8081/health/ready
```

> **Docker vs `.env` hôte** : `docker-compose.yml` configure `ASTRAL_CALCULATOR_HOST=astral_calculator_api` **dans le conteneur** LLM. Les scripts E2E détectent l'orchestration Docker (`http://127.0.0.1:8081`) et n'exigent pas ces variables dans votre `.env` local. En développement **hors Docker** (`cargo run`), définissez `ASTRAL_CALCULATOR_HOST=127.0.0.1` et `ASTRAL_CALCULATOR_PORT=8080` dans `.env`.

> **Provider** : Compose force `ASTRAL_LLM_DEFAULT_PROVIDER=fake` dans le conteneur. La valeur `openai` dans le `.env` hôte n'affecte pas Docker ; les scripts vérifient le runtime via `GET /v1/providers`.

#### Étape 1 — Smoke rapide (1 cas, ~30 s)

```powershell
.\scripts\docker_simplified_natal_smoke.ps1
```

Équivalent à `test_natal_simplified_e2e.ps1` sur le cas `date_only`. Sortie attendue : `Calculator OK` puis `Reading OK`.

#### Étape 2 — Suite complète (recommandée après modification du code)

```powershell
docker compose up -d --build astral_llm_api   # après changement Rust LLM
docker compose up -d --build astral_calculator_api   # après changement payload calculateur (llm_payload, schémas)
.\scripts\test_natal_simplified_e2e.ps1
```

Résultat attendu : **12/12** calculateur (7 positifs + 5 négatifs **422**), **7/7** lectures positives, **5/5** négatifs orchestration **400** — phases 1, 2 et **2b** de la suite.

> **Provider E2E** : la suite active **`-ForceFake`** par défaut (bascule `natal_prompter` → fake, sans OpenAI). Recette OpenAI optionnelle : `-UseReal -SubmitProfile` (facturée, peut échouer sporadiquement malgré les retries script).

Scripts complémentaires :

```powershell
.\scripts\test_natal_simplified_calculator.ps1          # calculateur seul (422 négatifs)
.\scripts\test_natal_simplified_reading.ps1 -ForceFake  # lectures fake (7 positifs)
.\scripts\test_natal_simplified_reading.ps1 -NegativeOnly  # négatifs orchestration 400 seuls
.\scripts\test_natal_simplified_e2e.ps1 -Case date_only
.\scripts\test_natal_simplified_e2e.ps1 -NoSaveOutputs
.\scripts\test_natal_simplified_reading.ps1 -UseReal -SubmitProfile -TimeoutSec 900  # OpenAI facturé
```

Artefacts JSON (generes par defaut par `test_natal_simplified_e2e.ps1`) : `output/natal_simplified/calculator/`, `output/natal_simplified/reading/` et `output/natal_simplified/e2e_summary.json`. Desactiver : `-NoSaveOutputs`.

#### Étape 3 — Appel HTTP manuel (lecture one-shot)

Exemple **date seule** avec les clés API de votre `.env` :

```powershell
. .\scripts\lib\astral_http_auth.ps1
Import-AstralDotEnv -RepoRoot (Get-Location)
$headers = New-AstralAuthHeaders -Service llm
$body = @{
  request_contract_version = "astro_simplified_natal_request_v1"
  birth = @{ date = "1990-06-15" }
  user_language = "fr"
  audience_level = "beginner"
} | ConvertTo-Json -Depth 5

Invoke-RestMethod -Method Post -Uri "http://localhost:8081/v1/readings/natal/simplified" `
  -Headers $headers -Body $body -ContentType "application/json"
```

Réponse attendue : `reading_completeness: partial`, `reading.status: success`, chapitre `identity` (ou `ambiguous_core_identity` si Soleil ambigu), `calculation.computed_scope` selon l'entrée. Dans `calculation.llm_payload`, le champ canonique des sujets d'interprétation exclus est **`forbidden_interpretation_topics`** (`forbidden_topics` = alias déprécié). En cas de rejet garde post-génération : HTTP **422**, enveloppe orchestrée avec `calculation` + `reading.status: safety_rejected` + `violations`.

Erreurs entrée sur l'orchestration (contrat, format date, lieu incomplet, heure sans timezone) : HTTP **400** `INVALID_INPUT`, sans enveloppe orchestrée. Sur le calculateur seul (`POST /v1/calculations/natal/simplified`), les mêmes erreurs métier renvoient **422** `VALIDATION_FAILED`.

Autres exemples de requêtes : [`contracts/integration/examples/natal_simplified_examples.json`](../contracts/integration/examples/natal_simplified_examples.json).

Documentation métier : [`docs/natal_simplified_reading_contract.md`](natal_simplified_reading_contract.md), [`docs/natal_simplified_forbidden_topics.md`](natal_simplified_forbidden_topics.md).

### Smoke test E2E — natal simplifié (`docker_simplified_natal_smoke.ps1`)

Raccourci vers l'étape 1 du tutoriel ci-dessus :

```powershell
.\scripts\docker_simplified_natal_smoke.ps1
# Calcul seul :
.\scripts\docker_simplified_natal_smoke.ps1 -SkipReading
```

---

## 10. Utiliser les APIs HTTP

### Découverte des contrats

Chaque service publie son index :

```powershell
curl http://localhost:8080/v1/contracts
curl http://localhost:8081/v1/contracts
```

OpenAPI :

```powershell
curl http://localhost:8080/openapi.yaml
curl http://localhost:8081/openapi.yaml
```

JSON Schema d'une version :

```powershell
curl http://localhost:8080/v1/schemas/astro_engine_request_v1
curl http://localhost:8081/v1/schemas/generate_reading_request_v1
```

### Healthchecks

| Endpoint | Signification |
|----------|---------------|
| `GET /health/live` | Le processus HTTP répond (liveness) |
| `GET /health/ready` | DB, référentiel, éphémérides/prompts OK (readiness) |

Exemple :

```powershell
curl http://localhost:8080/health/ready
curl http://localhost:8081/health/ready
```

Si le service n'est pas prêt : **HTTP 503** avec un corps `error_response_v1` (`code: SERVICE_NOT_READY`).

### Calculateur — calcul natal

Exemple avec la fixture Paris 1990 :

```powershell
$headers = @{
  "Content-Type" = "application/json"
  "Authorization" = "Bearer ma-cle-calculateur-secrete"
}
$body = Get-Content contracts/integration/examples/natal_calculation_request_v1.paris_1990.json -Raw
Invoke-RestMethod -Method Post -Uri "http://localhost:8080/v1/calculations/natal" -Headers $headers -Body $body
```

Réponse attendue : `response_contract_version: astro_engine_response_v1`, `calculation_result.status: completed`.

### Calculateur — natal simplifié (données partielles)

Contrat : `astro_simplified_natal_request_v1` → `astro_simplified_natal_response_v1`.

```powershell
$headers = @{
  "Content-Type" = "application/json"
  "Authorization" = "Bearer $env:ASTRAL_CALCULATOR_API_KEY"
}
$body = @{
  request_contract_version = "astro_simplified_natal_request_v1"
  birth = @{ date = "1990-06-15" }
} | ConvertTo-Json -Depth 5

Invoke-RestMethod -Method Post -Uri "http://localhost:8080/v1/calculations/natal/simplified" `
  -Headers $headers -Body $body
```

Champs utiles dans la réponse : `input_precision.level`, `computed_scope`, `facts` / `ambiguous_facts`, `llm_payload.allowed_fact_codes` / `blocked_interpretation_fact_codes`, `simplified_payload.payload`.

### LLM — lecture natal simplifiée (orchestration one-shot)

Un seul appel : naissance → calculateur interne → profil `natal_simplified`.

```powershell
$headers = @{
  "Content-Type" = "application/json"
  "Authorization" = "Bearer $env:ASTRAL_LLM_API_KEY"
}
$body = @{
  request_contract_version = "astro_simplified_natal_request_v1"
  birth = @{ date = "1990-06-15" }
  user_language = "fr"
  audience_level = "beginner"
} | ConvertTo-Json -Depth 5

Invoke-RestMethod -Method Post -Uri "http://localhost:8081/v1/readings/natal/simplified" `
  -Headers $headers -Body $body
```

Prérequis gateway : client calculateur configuré (`ASTRAL_CALCULATOR_HOST` / `PORT` — automatique sous Docker Compose).

### LLM — génération de lecture (thème complet)

Avec le provider fake (local, gratuit) :

```powershell
$headers = @{
  "Content-Type" = "application/json"
  "Authorization" = "Bearer ma-cle-llm-secrete"
}
$body = @{
  product_context = @{
    product_code = "natal_prompter"
    interpretation_profile_code = "natal_basic"
    user_language = "fr"
    audience_level = "beginner"
  }
  astro_result = @{
    contract_version = "natal_structured_v13"
    chart_type = "natal"
    data = @{ domain_scores = @{ identity = 0.8 } }  # en pratique : payload du calculateur
  }
  astrologer_profile = @{
    tone = "warm"
    jargon_level = "beginner"
    wording_style = "clear"
    preferred_domains = @("identity", "relationships")
    forbidden_wording = @()
  }
  engine = @{
    provider = "fake"
    model = "fake-model"
    allow_fallback = $true
  }
  response_contract = @{
    output_schema_version = "natal_reading_v1"
    format = "structured_json"
    include_astro_sources = $true
    include_legal_disclaimer = $true
  }
} | ConvertTo-Json -Depth 10

Invoke-RestMethod -Method Post -Uri "http://localhost:8081/v1/readings/generate" -Headers $headers -Body $body
```

> **`generation_mode`** : optionnel dans `response_contract`. S'il est omis ou incorrect, l'API l'aligne automatiquement sur `interpretation_profile_code` (`InterpretationProfileResolver`).

Réponse attendue : `status: success`, `reading.schema_version: natal_reading_v1`, liste de chapitres.

### Autres endpoints utiles

| Service | Méthode | Endpoint | Description |
|---------|---------|----------|-------------|
| Calculateur | POST | `/v1/calculations/validate` | Valide un JSON sans calculer |
| Calculateur | GET | `/v1/reference/status` | État DB + éphémérides |
| LLM | POST | `/v1/readings/validate` | Valide une lecture JSON |
| Calculateur | POST | `/v1/calculations/natal/simplified` | Calcul partiel (contrats `astro_simplified_*`) |
| LLM | POST | `/v1/readings/natal/simplified` | Naissance → calcul simplifié → lecture |
| LLM | GET | `/v1/providers` | Modèles, `default_provider`, circuit breakers |
| LLM | GET | `/v1/runs/{run_id}` | Audit d'un run (si persistance active) |

---

## 11. Flux complet calculateur → LLM

C'est le **mode V1 certifié** : deux appels HTTP orchestrés par votre application.

```
  [Client]
     │
     │  POST /v1/calculations/natal
     │  (astro_engine_request_v1)
     ▼
  [astral_calculator_api]
     │
     │  astro_engine_response_v1
     │  └─ audit_payload.payload  ───┐
     │                                │
     │  POST /v1/readings/generate    │
     │  (generate_reading_request_v1) │
     ▼                                │
  [astral_llm_api] ◄──────────────────┘
     │
     │  generate_reading_response_v1
     ▼
  [Client]
```

### Règles de mapping

Document détaillé : [`contracts/integration/engine_to_reading_mapping.md`](../contracts/integration/engine_to_reading_mapping.md).

En résumé :

1. Attendre `calculation_result.status == "completed"`.
2. Copier `audit_payload.contract_version` → `astro_result.contract_version`.
3. Copier **uniquement** `audit_payload.payload` → `astro_result.data` (pas l'enveloppe complète).
4. Définir `product_context.interpretation_profile_code` (`natal_light`, `natal_basic`, `natal_premium`, `natal_premium_plus`).
5. `response_contract.generation_mode` peut être omis : l'API le dérive du profil (`single_pass` pour `natal_light`, `chapter_orchestrated` pour les autres).

### Exemples JSON prêts à l'emploi

| Fichier | Contenu |
|---------|---------|
| `contracts/integration/examples/natal_calculation_request_v1.paris_1990.json` | Requête calculateur |
| `contracts/integration/examples/natal_calculation_response_v1.paris_1990.json` | Réponse calculateur (référence) |
| `contracts/integration/examples/generate_reading_request_v1.from_engine_paris_1990.json` | Requête LLM déjà mappée |
| `contracts/integration/examples/natal_simplified_examples.json` | Matrice `input_precision` (requêtes simplified) |

### Flux natal simplifié (one-shot ou manuel)

```
  [Client]
     │
     │  POST /v1/readings/natal/simplified   (one-shot, recommandé)
     │  OU
     │  POST /v1/calculations/natal/simplified
     │       puis POST /v1/readings/generate (profil natal_simplified)
     ▼
  [astral_llm_api] ──HTTP interne──► [astral_calculator_api]
     │
     │  reading + calculation dans la réponse
     ▼
  [Client]
```

Mapping détaillé : [`contracts/integration/engine_to_reading_mapping.md`](../contracts/integration/engine_to_reading_mapping.md) (section natal simplifié).

---

## 12. Contrats publics

Index des versions actives : [`contracts/versions.json`](../contracts/versions.json).

### Calculateur (`contracts/calculator/`)

| Version | Fichier | Rôle |
|---------|---------|------|
| `astro_engine_request_v1` | `astro_engine_request_v1.schema.json` | Entrée : naissance, systèmes de référence, projection |
| `astro_engine_response_v1` | `astro_engine_response_v1.schema.json` | Sortie : résultat calcul + audit |
| `natal_structured_v13` | `natal_structured_v13.schema.json` | Payload structuré interne (dans `audit_payload`) |
| `llm_projection_natal_v1` | `llm_projection_natal_v1.schema.json` | Niveau de projection vers le LLM |
| `astro_simplified_natal_request_v1` | `astro_simplified_natal_request_v1.schema.json` | Entrée natal simplifié (date partielle…) |
| `astro_simplified_natal_response_v1` | `astro_simplified_natal_response_v1.schema.json` | Sortie calculateur simplified |
| `natal_simplified_structured_v1` | `natal_simplified_structured_v1.schema.json` | Payload structuré pour le LLM |

**Champs clés de la requête calculateur** (`astro_engine_request_v1`) :

- `calculation` : type (`natal`), zodiac (`tropical`), coordonnées (`geocentric`), maisons (`placidus`…)
- `birth` : date, heure, fuseau IANA (`Europe/Paris`), latitude/longitude
- `projection.level` : `minimal`, `standard`, `rich` (richesse du payload pour le LLM)

### LLM (`contracts/llm/`)

| Version | Fichier | Rôle |
|---------|---------|------|
| `generate_reading_request_v1` | `generate_reading_request_v1.schema.json` | Entrée gateway : contexte produit, astro, moteur LLM |
| `generate_reading_response_v1` | `generate_reading_response_v1.schema.json` | Sortie taguée : `success`, `failed`, `safety_rejected` |
| `natal_reading_v1` | `natal_reading_v1.schema.json` | Lecture finale : chapitres, summary, disclaimer |
| `chapter_provider_v1` | `chapter_provider_v1.schema.json` | Format JSON renvoyé par le LLM pour un chapitre |
| `summary_provider_v1` | `summary_provider_v1.schema.json` | Format JSON pour la synthèse / résumé |

**Réponse de génération** (`generate_reading_response_v1`) — trois cas :

```json
{ "status": "success", "run_id": "...", "reading": { "schema_version": "natal_reading_v1", ... } }
```

```json
{ "status": "failed", "run_id": "...", "error": { "code": "SCHEMA_VALIDATION_FAILED", "message": "..." } }
```

```json
{ "status": "safety_rejected", "run_id": "...", "error": { "code": "SAFETY_POLICY_VIOLATION", ... } }
```

### Commun (`contracts/common/`)

| Version | Rôle |
|---------|------|
| `error_response_v1` | Erreurs HTTP des APIs (auth, validation, readiness, rate limit…) |

Format :

```json
{
  "status": "failed",
  "error": {
    "code": "UNAUTHORIZED",
    "message": "Missing or invalid API key.",
    "details": {}
  },
  "request_id": "uuid"
}
```

---

## 13. Profils d'interprétation et modèles LLM

### Profils disponibles

Fichiers JSON dans `config/natal_interpretation_profiles/` :

| Code | Mode | Usage |
|------|------|-------|
| `natal_light` | single_pass | Lecture courte, un seul passage |
| `natal_basic` | chapter_orchestrated | 6 chapitres, séquence fixe |
| `natal_premium` | chapter_orchestrated | Chapitres riches, quality gate bloquant |
| `natal_premium_plus` | chapter_orchestrated | Lecture longue certifiée (plusieurs appels LLM) |

Soumission manuelle d'un profil :

```powershell
.\scripts\manage_natal_interpretation_profiles.ps1 -Submit -Path config\natal_interpretation_profiles\natal_basic.json
docker compose restart astral_llm_api
```

### Modèles LLM par produit

Fichier source : `config/llm_product_models.conf`

Application en base :

```powershell
.\scripts\set_product_llm_models.ps1
docker compose restart astral_llm_api
```

---

## 14. Authentification

Si `ASTRAL_CALCULATOR_API_KEY` ou `ASTRAL_LLM_API_KEY` est **non vide** dans `.env`, les routes protégées exigent :

```
Authorization: Bearer <votre-clé>
```

ou

```
X-API-Key: <votre-clé>
```

Routes **publiques** (sans clé) :

- `/health`, `/health/live`, `/health/ready`
- `/v1/contracts`, `/openapi.yaml`
- `/v1/schemas/*`

En Docker Compose, définissez **les deux clés** dans `.env` avant le premier `up`.

---

## 15. Erreurs et readiness

### Readiness (503)

`GET /health/ready` retourne **503** + `error_response_v1` si :

| Service | Causes fréquentes |
|---------|-------------------|
| Calculateur | PostgreSQL inaccessible, référentiel vide, fichiers `.se1` absents |
| LLM | PostgreSQL inaccessible, profils non chargés, prompts manquants |

### Codes d'erreur HTTP courants

| Code HTTP | Code métier | Calculateur (`:8080`) | Gateway LLM (`:8081`) |
|-----------|-------------|------------------------|------------------------|
| 401 | `UNAUTHORIZED` | Clé API manquante ou invalide | Idem |
| 400 | `INVALID_INPUT` | *(non utilisé pour validation métier)* | Entrée invalide **avant** génération (orchestration simplified, `generate`) |
| 422 | `VALIDATION_FAILED` | Payload / schéma / règles métier rejetés (natal simplified inclus) | `safety_rejected` post-génération (enveloppe `{ calculation, reading }` pour simplified) |
| 409 | `CALCULATION_IN_PROGRESS` | Idempotency calculateur | — |
| 429 | `TOO_MANY_REQUESTS` | — | Rate limit ou concurrence |
| 503 | `SERVICE_NOT_READY` | DB / éphémérides | DB / profils / prompts |
| 504 | `PROVIDER_TIMEOUT` | — | Timeout LLM provider |

**Natal simplifié — même payload invalide, code HTTP selon l'endpoint :**

- `POST /v1/calculations/natal/simplified` → **422** `VALIDATION_FAILED`
- `POST /v1/readings/natal/simplified` → **400** `INVALID_INPUT` (sans enveloppe orchestrée)

Les erreurs **métier de génération** (échec LLM après acceptation de la requête) utilisent plutôt `generate_reading_response_v1` avec `status: failed`.

### Idempotency (LLM)

Header optionnel : `Idempotency-Key: <uuid>`

- Rejeu avec même clé + même payload → réponse en cache (200)
- Même clé + payload différent → `IDEMPOTENCY_PAYLOAD_MISMATCH` (400)
- Génération en cours → 409 `{ "status": "pending", "run_id": "..." }`

---

## 16. Commandes utiles

```powershell
# Stack complète
docker compose up -d --build
.\scripts\docker_bootstrap.ps1
.\scripts\docker_compose_smoke.ps1

# Natal simplifié (fake, sans OpenAI)
.\scripts\docker_simplified_natal_smoke.ps1
.\scripts\test_natal_simplified_e2e.ps1

# Tests Rust natal simplifié
cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests
cargo test -p astral_llm_api --test astral_llm_simplified_reading_tests

# Logs
docker compose logs -f astral_llm_api

# Rebuild un seul service
docker compose up -d --build astral_calculator_api

# Tests Rust (hors Docker)
cargo test -p astral_calculator_api --test astral_calculator_api_tests
cargo test -p astral_llm_api --test contracts_publish_tests
cargo test -p astral_llm_api --test astral_llm_tests

# Développement local sans Docker (APIs sur l'hôte)
cargo run -p astral_calculator_api   # :8080
cargo run -p astral_llm_api          # :8081
```

---

## 17. Dépannage

### « Calculator reference data MISSING » au bootstrap

**Cause** : tables PostgreSQL vides ou incomplètes.

**Action** :

```powershell
docker compose up -d postgres
python scripts/import_json_db_to_postgres.py
.\scripts\docker_bootstrap.ps1
```

### `checks.ephemeris_path: false`

**Cause** : dossier `ephe/se-2026a/` absent ou sans fichiers `.se1`.

**Action** : installer Swiss Ephemeris (section 6), puis `docker compose restart astral_calculator_api`.

### `astral_llm_api /health/ready` indisponible

**Causes** :
- Profils non soumis → relancer `docker_bootstrap.ps1`
- PostgreSQL down → `docker compose ps`

### Build Docker très long

Normal au premier lancement (compilation Rust). Les builds suivants utilisent le cache Docker.

### Port déjà utilisé (8080 / 8081)

Modifiez le mapping dans `docker-compose.yml` :

```yaml
ports:
  - "18080:8080"   # exemple
```

Puis adaptez les URLs dans les scripts (`-CalculatorUrl`, `-LlmUrl`).

### Génération OpenAI qui timeout

Augmentez dans `.env` :

```env
ASTRAL_LLM_REQUEST_TIMEOUT_MS=900000
```

Redémarrez : `docker compose restart astral_llm_api`.

### Natal simplifié — HTTP 500 / `REFERENCE_DATA_MISSING`

**Cause** : tables `astral_simplified_*` ou scopes absents (import DB non fait).

**Action** :

```powershell
python scripts/import_json_db_to_postgres.py
docker compose restart astral_calculator_api astral_llm_api
.\scripts\test_natal_simplified_calculator.ps1 -Case date_only
```

### Natal simplifié — `PRODUCT_POLICY_VIOLATION domain_count`

**Cause** : binaire `astral_llm_api` obsolète (profil `natal_simplified` limite à **1** domaine).

**Action** :

```powershell
docker compose up -d --build astral_llm_api
.\scripts\test_natal_simplified_reading.ps1 -Case date_only
```

### Natal simplifié — tests lecture échouent sans fake

**Cause** : gateway configuré sur OpenAI alors que les scripts E2E attendent le provider **fake** (gratuit).

**Action** : utiliser Docker Compose (`ASTRAL_LLM_DEFAULT_PROVIDER=fake` dans le conteneur) ou `-UseReal` avec `OPENAI_API_KEY`. Vérifier : `GET /v1/providers` → `default_provider: fake`.

### `DATABASE_URL absent` au bootstrap (étape 3)

**Cause** : `manage_natal_interpretation_profiles.ps1` ou `set_product_llm_models.ps1` lance une erreur si `DATABASE_URL` est vide.

**Action** : ajouter dans `.env` :

```env
DATABASE_URL=postgres://postgres:change-me@localhost:5432/astral
```

(mot de passe aligné sur `POSTGRES_PASSWORD`). Relancer `.\scripts\docker_bootstrap.ps1`.

### `psql` + host `postgres` introuvable depuis l'hôte

**Cause** : `DATABASE_URL` pointe vers `@postgres:5432` alors que `psql` est installé localement.

**Action** : remplacer par `@localhost:5432` et utiliser `docker-compose.dev-db-port.yml`, **ou** désinstaller/retirer `psql` du PATH pour forcer `docker compose exec`.

---

## 18. Aller plus loin

| Document | Contenu |
|----------|---------|
| [`contracts/README.md`](../contracts/README.md) | Index contrats et modes d'intégration |
| [`AGENTS.md`](../AGENTS.md) | Règles workspace, commandes développeur |
| [`BASIC_PAYLOAD_IMPLEMENTATION.md`](../BASIC_PAYLOAD_IMPLEMENTATION.md) | Détails moteur calculateur et API HTTP |
| [`Astral_llm_implementation.md`](../Astral_llm_implementation.md) | Pipeline LLM, quality gates, profils Premium Plus |
| [`docs/natal_simplified_reading_contract.md`](natal_simplified_reading_contract.md) | Contrat produit lecture simplifiée |
| [`docs/natal_simplified_forbidden_topics.md`](natal_simplified_forbidden_topics.md) | Sujets interdits et contrôles anti-hallucination |

Pour certifier le **natal simplifié** (fake, sans coût OpenAI) :

```powershell
.\scripts\test_natal_simplified_e2e.ps1
```

Pour certifier une lecture Premium Plus de bout en bout (hors Docker ou avec stack Docker) :

```powershell
# Stack Docker + OpenAI réel (recommandé)
.\scripts\docker_premium_openai_e2e.ps1

# Ou scripts unitaires (API locale ou Docker sur :8081)
.\scripts\test_natal_premium_profile.ps1
.\scripts\test_natal_premium_plus_profile.ps1
```

(Requiert `OPENAI_API_KEY` et `ASTRAL_LLM_REQUEST_TIMEOUT_MS=900000`.)

---

*Dernière mise à jour : juin 2026 — stack Docker Compose V1 + natal simplifié v2.4 (`POST /v1/readings/natal/simplified`).*
