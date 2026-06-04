# astral_llm — Gateway LLM astrologique

Service Rust independant du moteur de calcul (`astral_calculator`). Il transforme un resultat
astrologique deja calcule en lecture interpretative structuree, securisee et interchangeable entre
fournisseurs LLM.

## Etat du projet

| Label | Statut | Signification |
|---|---|---|
| **V1-technical-freeze** | **VALIDE** | Gel architecture/securite : perimetre fige, P1–P4 implementes |
| **astral_llm V1-production-public OpenAI** | **VALIDEE** | Gateway + orchestration Premium certifies sur OpenAI |
| **Premium interpretatif riche OpenAI** | **VALIDÉ PRODUIT** | Evidence Planner clos ; E2E rich OpenAI OK (ex. run `0619a1e8`) |
| **Chantier Evidence Planner** | **CLOS** | Plus de correction structurelle prevue ; maintenance bugs seulement |
| **Mistral / Anthropic** | **Adapters presents, non certifies** | Smoke ignore ; phase optimisation quand cles disponibles |

```txt
astral_llm V1-production-public OpenAI     : VALIDEE
Premium interpretatif riche OpenAI         : VALIDÉ PRODUIT
Chantier Evidence Planner                  : CLOS (2026-06-04)
E2E OpenAI rich (6 steps generated)        : OK — scripts/generate_premium_reading_e2e.ps1
Mistral / Anthropic                        : adapters presents, non certifies
```

**Reference E2E produit (2026-06-04)** : run `0619a1e8-4069-4f89-b6ea-db14f32f38ea` — ~47 s, 6 steps `generated`, libelles maîtrise humanises, cap Soleil supporting (3 chapitres), prompts dans `output/logs/prompts/0619a1e8-.../`.

Le gel **V1-technical-freeze** ne doit plus faire l'objet d'une refonte architecture. La suite n'est plus une **correction** du planner, mais une **phase d'optimisation** (voir ci-dessous).

Le gateway n'est **pas** un simple proxy LLM : la validation metier se fait **avant** tout appel provider.

### Perimetre gele (ne pas refondre sauf bug constate)

```txt
ConfigValidator, ProviderRouter, FallbackPolicy, ModelCapabilityRegistry,
AstroPayloadNormalizer, SafetyGuard, Idempotency flow, Rate limiting,
Circuit breaker, ChapterOrchestrator
```

### Limites editoriales connues (non bloquantes)

- **Amorces parfois « promptees »** : formulations type « En développant… », « En prenant en compte… » (effet secondaire des consignes `ChapterWritingGuidance` + diversite d'ouvertures). Acceptable en prod ; affinage style en optimisation.
- **Densite des prompts chapitre** : structure tres controlee (4 paragraphes, liste `fact_id`, anti-trigrammes) — securise `astro_basis` et la diversite, peut donner une prose un peu scolaire. A equilibrer en phase optimisation, pas en rouvrant le planner.

### Phase d'optimisation (suite logique, hors Evidence Planner)

1. **OpenAI** : comparer cout / latence / qualite par modele sur le meme golden E2E ; changer les modeles via `config/llm_product_models.conf` + `set_product_llm_models.ps1`.
2. **Mistral / Anthropic** : `cargo test -p astral_llm_providers --test provider_real_smoke -- --ignored` puis E2E Premium quand cles disponibles.
3. **Referentiel evidence** : enrichir progressivement les slots (noeuds, phases lunaires, dignites mineures, patterns d'aspects) via tables canoniques — pas de constantes en code.
4. **Style redactionnel** : allegement cible des consignes prompt / guidance pour une prose moins « structuree par contraintes », sans casser les garde-fous qualite.

### Checklist avant V1-production-public

**Automatisee (CI / local, sans cles provider) :**

```bash
cargo test -p astral_llm_api --test astral_llm_tests
cargo test -p astral_llm_api --test astral_llm_injection_tests
cargo test -p astral_llm_api --test prompt_golden_tests
cargo test -p astral_llm_api --test astral_llm_editorial_fixtures
cargo test -p astral_llm_api --test astral_llm_astro_basis_tests
cargo test -p astral_llm_api --test astral_llm_load_tests
cargo test -p astral_llm_application
cargo test -p astral_llm_infra
cargo test -p astral_llm_domain
```

**Manuelle (cles + reseau / PostgreSQL) :**

```bash
cargo test -p astral_llm_providers --test provider_real_smoke -- --ignored
cargo test -p astral_llm_api --test astral_llm_load_tests -- --ignored  # idempotence DB
```

**Validation E2E Premium (apres smoke providers) :**

| Parametre | Valeur |
|---|---|
| Produit | `natal_premium` |
| Mode | `chapter_orchestrated` |
| Provider | OpenAI d'abord, puis Mistral ou Anthropic |
| Langue | `fr` |
| Audience | `beginner` |

Criteres de passage : JSON valide, `astro_basis` valide (≥1 fact interpretatif par chapitre Premium, pas de `domain_score` seul), synthese finale personnalisee (pas de placeholder pipeline), pas de conseil medical/juridique/financier, pas de fatalisme, pas de repetition excessive, pas de liste froide de faits, disclaimer present, qualite Premium non rejetee (`READING_QUALITY_FAILED`), steps persistes (chapitres + `summary`), idempotence rejoue la reponse, `GET /v1/providers` expose `circuit_breakers`. Gate Premium : `product_code=natal_premium` bloquant meme si `single_pass` est force par erreur.

Le payload astro Premium doit inclure des placements/aspects (via `planets`, `positions` ou `llm_projection_natal_v1`) — un jeu `domain_scores` seul est rejete.

## Validation produit (implementee — V1-technical-freeze)

### P1 — Smoke tests providers reels

```bash
cargo test -p astral_llm_providers --test provider_real_smoke -- --ignored
```

- OpenAI / Mistral / Anthropic : JSON schema minimal `{"ok": true}`
- Rejet cle API invalide (HTTP/API/Config + message auth)
- Fichier : `astral_llm_providers/tests/provider_real_smoke.rs` (remplace l'ancien `openai_smoke.rs`)

### P2 — Fixtures redactionnelles

```bash
cargo test -p astral_llm_api --test astral_llm_editorial_fixtures
```

Fixtures : `tests/fixtures/astral_llm/editorial/`

- `natal_basic_beginner_fr`
- `natal_premium_psychological_fr`
- `natal_premium_traditional_en`

Checks : lisibilite, non-repetition, cadrage interpretatif, jargon, fatalisme, conseils interdits, `astro_basis`.

### P3 — Qualite Premium bloquante

`ReadingQualityValidator::validate_for_product` + `requires_blocking_quality_gate()` :

- **Basic** (`natal_basic` + `single_pass`) : warnings non bloquants (`tracing`)
- **Premium** : gate bloquante si `chapter_orchestrated` **ou** `product_code=natal_premium` (meme en `single_pass` mal configure)
- Code erreur : `READING_QUALITY_FAILED`
- Seuils : longueur chapitre (≥40 mots), cadrage interpretatif, repetition (trigrammes), `astro_basis`, determinisme, disclaimer
- `EditorialValidator` : fatalisme, conseils interdits, jargon beginner (fixtures)

### P3b — Astro basis Premium (interpretatif obligatoire)

`AstroFactUsage` distingue `domain_selection` (scores de domaine) et `interpretive_basis` (placements, aspects, angles, dignites, maitres).

- **Basic** : `domain_score` autorise seul (`min_interpretive_astro_basis_refs_per_chapter = 0`)
- **Premium** : ≥1 fact interpretatif valide par chapitre (`min_interpretive_astro_basis_refs_per_chapter = 1`) ; `domain_score` seul → `SCHEMA_VALIDATION_FAILED`
- `PromptCompiler` : en mode chapitre, ne fournit au LLM que les facts du domaine + facts globaux (soleil, lune, ascendant, aspects majeurs)
- Libelles affichables : tables `llm_astro_object_labels` / `llm_zodiac_sign_labels` (locale `fr`/`en`) ; `AstroLabelHumanizer` applique les libelles aux facts normalises et ecrase les `astro_basis[].label` renvoyes par le LLM (ex. `Soleil en Capricorne en maison 2`)
- Disclaimer legal : `default_legal_disclaimer` (accents FR : interprétation, médical, …)
- Tests : `cargo test -p astral_llm_api --test astral_llm_astro_basis_tests` ; `cargo test -p astral_llm_api --test astral_llm_evidence_planner_tests` ; `cargo test -p astral_llm_api --test astral_llm_evidence_coherence_tests`

### P3c — SummarySynthesizer (mode chapter_orchestrated)

Etape finale apres tous les chapitres :

```txt
Chapter outputs -> SummarySynthesizer -> summary.title + summary.short_text -> validation qualite
```

- Schema provider : `summary_provider_v1`
- Placeholders interdits : « Synthese produite par… », « generation chapitre par chapitre », mention du pipeline
- Step auditee : `summary` dans `ExecutionAudit` (tokens `input_tokens` / `output_tokens` remontés depuis `route.response.usage`)
- Run : `token_input` / `token_output` = somme des steps (chapitres + summary) via `ExecutionAudit::aggregate_token_usage`

### P4 — Tests de charge locaux

```bash
cargo test -p astral_llm_api --test astral_llm_load_tests
cargo test -p astral_llm_api --test astral_llm_load_tests -- --ignored  # idempotence DB
```

- Semaphore global, RPM / concurrent par cle, quota Premium
- Circuit breaker sous echecs transitoires
- Idempotence concurrente (PostgreSQL, `#[ignore]` sans `DATABASE_URL`)

## Architecture

```txt
astral_llm/
  crates/
    astral_llm_domain/       — contrats (request/response, safety, policies, capabilities)
    astral_llm_application/  — use cases, orchestration, validation, router
    astral_llm_providers/    — adapters OpenAI, Anthropic, Mistral, Fake
    astral_llm_infra/        — config, secrets, persistence, referentiel, redaction
    astral_llm_api/          — serveur HTTP Axum (lib + binaire ; auth, rate limits, routes)
  prompts/
    natal_basic/v1/          — system.md, task.md, format.md, safety.md
    natal_premium/v1/
  crates/astral_llm_infra/sql/
    llm_generation_runs.sql  — runs, payloads, index
    llm_canonical.sql        — referentiels LLM (domains, safety, policies, models)
    llm_evidence_canonical.sql — kinds, slots chapitre, policies Premium, requirements
    llm_audit_extensions.sql — steps, idempotence
```

### Chaine gateway complete (Premium chapter_orchestrated)

```txt
RequestValidator
  -> ProductPolicyValidator
  -> ModelCapabilityRegistry
  -> AstroPayloadNormalizer
  -> InterpretiveEvidenceBuilder
  -> ChapterEvidencePlanner
  -> EvidenceDiversityValidator::validate_packs (pre-LLM, Premium)
  -> SafetyResolver / SafetyGuard (pre requete)
  -> PromptCompiler (ChapterEvidencePack par chapitre — pas la reserve globale)
  -> prompt_trace (fichier + tracing : prompt compile envoye au provider)
  -> ProviderSchemaCompiler
  -> ProviderRouter (+ ProviderCircuitBreaker)
  -> LLM
  -> post-traitement chapitre (par chapitre) :
     AstroBasisRoleNormalizer → ChapterEvidenceBasisEnricher (CORE + SUPPORTING sauf identity)
     → AstroBasisRoleNormalizer → AstroLabelHumanizer
     → AstroBasisValidator → ChapterEvidenceCoherence
     (repair LLM repair_evidence si orphelins body / incoherence non enrichissable)
  -> EvidenceDiversityValidator::validate_reading (post-LLM, Premium)
  -> repair_opening_duplicates (jusqu'a 6 tours, tous chapitres en violation)
  -> SummarySynthesizer
  -> ResponseValidator (lecture complete + summary)
  -> SafetyGuard (post)
  -> ReadingQualityValidator (bloquant Premium / natal_premium)
  -> Persistence / audit (evidence_metrics dans steps si Premium)
```

Codes erreur dedies :

- `PREMIUM_EVIDENCE_DIVERSITY_FAILED` — pool/payload insuffisant ou packs trop repetitifs
- `ASTRO_BASIS_INVALID` — fact_id hors pack ; `ChapterEvidenceCoherence` : planete citee dans le `body` sans `astro_basis`, ou slots pack encore absents apres enrichisseur. Repair LLM `repair_evidence` une fois. **Avant** coherence : `ChapterEvidenceBasisEnricher` injecte les CORE manquants (tous chapitres) et les SUPPORTING manquants (**sauf** `identity`) — evite un 2e appel LLM quand seuls des `fact_id` manquent
- `READING_QUALITY_FAILED` — qualite redactionnelle

Fixtures E2E :

- `request-premium-minimal.json` — test negatif (trio asc/sun/moon)
- `request-premium-rich.json` — golden `natal_payload_v13_paris_1990` pour E2E OpenAI

SQL : [`astral_llm/crates/astral_llm_infra/sql/llm_evidence_canonical.sql`](astral_llm/crates/astral_llm_infra/sql/llm_evidence_canonical.sql) ; i18n : [`llm_i18n_canonical.sql`](astral_llm/crates/astral_llm_infra/sql/llm_i18n_canonical.sql) (`llm_writing_locales` fr/en/es/de, `llm_astro_basis_roles`, `llm_aspect_type_labels`)

**Langue de reponse LLM** : `OUTPUT_LANGUAGE` injecte dans les instructions systeme (`WritingLanguageDirective`) selon `product_context.user_language`. Le bloc `--- BEGIN ASTRO DATA ---` envoye au modele utilise des libelles humanises (`AstroPayloadNormalizer::to_chapter_evidence_pack_block` + `AstroLabelHumanizer::label_for_fact_id`). Post-LLM : `AstroBasisRoleNormalizer` (2 passages autour de `ChapterEvidenceBasisEnricher`) puis `AstroLabelHumanizer` sur `astro_basis` (label, factor). Roles : correspondance exacte `fact_id` puis alias `object_code` **dans la meme famille** (`evidence_fact_parse::fact_id_role_bucket` : ex. `signal:object_position:sun` ≠ `placement:sun:*`).

**ChapterEvidencePlanner** (`chapter_evidence_planner.rs`, catalogue `evidence_canonical.rs` / SQL) :

- `semantic_fact_key` sur chaque `InterpretiveEvidence` ; overlap, `avoid_repeating`, exclusions via `PriorChapterUsage` (cles semantiques, pas `fact_id` bruts).
- Extracteur : `ascendant_ruler`, `mc_ruler`, `descendant_ruler` (payload v13), `dominant_house_rulers` → `house_ruler` avec `source_house_number` sur les angles (`astro_fact_extractor.rs`).
- Slot `relationships` : `house_ruler` + `object_code` **`descendant`** ; requirement bloquant `relationships_ruler_7` ; `chapter_excludes_candidate` interdit `ruler:angle:mc:*` dans ce pack.
- Slot `career` : `house_ruler` + `mc` ; requirement `career_ruler_10`.
- Identity : `chapter_excludes_candidate` exclut tout fait Soleil ; pack sans soleil (reserve career).
- `inject_blocking_requirements`, `fill_minimums` (familles aspect / house_ruler / dignite), validation adaptative (warning si pool pauvre).

**Post-traitement basis** (`chapter_evidence_basis_enricher.rs`) :

- **CORE** manquants : injectes pour **tous** les chapitres (y compris `identity`).
- **SUPPORTING** manquants : injectes pour tous les chapitres **sauf** `identity` (aligne tests `does_not_append_supporting_from_pack`, `appends_supporting_for_career_coherence`).
- Declenche **avant** `ChapterEvidenceCoherence::validate_premium` dans `generate_one_chapter`.

**Requirements chapitre** (`llm_evidence_requirements`) : audit dans `EvidenceMetrics.requirement_audit`. Codes : `career_ruler_10`, `relationships_ruler_7`, `relationships_relational_aspect`, `growth_path_nodal`, `growth_path_structuring_aspect`, `growth_path_transformation_house`.

**Qualite redactionnelle** :

- `ChapterWritingGuidance` : structure 4 §, anti-trigrammes, `openings_to_avoid_from_prior`, liste **Mandatory astro_basis** (tous fact_id core + supporting du pack).
- `ReadingOpeningDiversityValidator` + `text_trigrams` : amorces chapitre (5 mots) / paragraphe (4 mots) ; prefixes generiques FR non bloquants (`GENERIC_PARA_OPENING_PREFIXES_FR`).
- `repair_opening_duplicates` : boucle jusqu'a 6 rounds, regenere **chaque** chapitre en violation (attempt `repair_opening`).
- Autres repairs : repetition intra-chapitre, `min_words`, `repair_evidence` (si coherence echoue malgre enrichisseur).

**E2E premium** (`scripts/generate_premium_reading_e2e.ps1`, `request-premium-rich.json`) : runs de reference `54d2634c`, `627c9ada` — ~38–43 s, 6 steps `generated`, 6 fichiers `*_primary.txt` (pas de `*_repair_*`).

Tests :

```bash
cargo test -p astral_llm_application
cargo test -p astral_llm_api --test astral_llm_evidence_planner_tests
cargo test -p astral_llm_api --test astral_llm_evidence_coherence_tests
cargo test -p astral_calculator --test payload_tests basic_payload_exposes_rulership
```

### Middleware HTTP (ordre entrant)

Tower execute la **derniere** couche ajoutee en premier :

```txt
TraceLayer
  -> require_api_key
  -> api_key_rate_limit (concurrent + RPM par cle)
  -> concurrency_limit (semaphore global)
  -> TimeoutLayer -> RequestBodyLimitLayer
  -> handler
```

L'authentification precede les quotas pour eviter qu'une requete non authentifiee consomme des slots.

Pour `generation_mode = chapter_orchestrated`, un quota supplementaire `MAX_PREMIUM_RUNS_PER_KEY` s'applique sur `POST /v1/readings/generate` (apres verification idempotence / rejeu).

## Commandes

```bash
# Build
cargo build -p astral_llm_api

# Lancer (local : FakeProvider par defaut)
ASTRAL_LLM_ENV=local cargo run -p astral_llm_api

# Tests integration (racine tests/)
cargo test -p astral_llm_api --test astral_llm_tests
cargo test -p astral_llm_api --test astral_llm_injection_tests
cargo test -p astral_llm_api --test prompt_golden_tests
cargo test -p astral_llm_api --test astral_llm_editorial_fixtures
cargo test -p astral_llm_api --test astral_llm_load_tests

# Smoke providers (manuel)
cargo test -p astral_llm_providers --test provider_real_smoke -- --ignored

# Tests unitaires crates
cargo test -p astral_llm_application
cargo test -p astral_llm_infra
cargo test -p astral_llm_domain
```

## Variables d'environnement

| Variable | Defaut (local) | Description |
|---|---|---|
| `ASTRAL_LLM_ENV` | `local` | Profil `local` / `test` / `production` |
| `ASTRAL_LLM_PRODUCTION_MODE` | `internal` | `internal` ou `public` (niveau d'exposition prod) |
| `ASTRAL_LLM_HOST` | `127.0.0.1` | Bind host |
| `ASTRAL_LLM_PORT` | `8081` | Bind port |
| `ASTRAL_LLM_API_KEY` | — | Auth gateway ; obligatoire en production |
| `ASTRAL_LLM_PROMPTS_DIR` | `astral_llm/prompts` | Repertoire des prompts versionnes |
| `ASTRAL_LLM_DEFAULT_PROVIDER` | `fake` (local) | Provider si absent de la requete |
| `ASTRAL_LLM_DEFAULT_MODEL` | `fake-model` (local) | Modele si absent de la requete |
| `ASTRAL_LLM_ENABLE_FAKE` | `true` (local) | FakeProvider (interdit en production) |
| `ASTRAL_LLM_FALLBACK_ENABLED` | `true` | Active le fallback transitoire |
| `ASTRAL_LLM_FALLBACK_PROVIDERS` | *(vide)* | Chaine explicite ; aucune priorite OpenAI imposee |
| `ASTRAL_LLM_FALLBACK_MAX_RETRIES` | `1` | Retries par provider avant fallback suivant |
| `ASTRAL_LLM_ALLOW_CROSS_PROVIDER_FALLBACK` | `false` | Fallback inter-vendors (privacy) |
| `ASTRAL_LLM_ENABLE_PERSISTENCE` | `false` | Audit PostgreSQL + idempotence |
| `ASTRAL_LLM_DB_AUTO_MIGRATE` | `true` (local) | **Interdit** en production |
| `ASTRAL_LLM_STORE_SANITIZED_PAYLOADS` | `false` | Persiste payloads rediges dans `llm_generation_payloads` |
| `ASTRAL_LLM_ALLOW_PUBLIC_BIND` | `false` | Requis pour ecouter sur `0.0.0.0` en production |
| `ASTRAL_LLM_MAX_CONCURRENT_REQUESTS` | `32` | Semaphore global (429 si sature) |
| `ASTRAL_LLM_MAX_CONCURRENT_REQUESTS_PER_KEY` | `8` | Concurrent par cle API |
| `ASTRAL_LLM_MAX_REQUESTS_PER_MINUTE_PER_KEY` | `120` | RPM par cle API |
| `ASTRAL_LLM_MAX_PREMIUM_RUNS_PER_KEY` | `4` | Premium concurrent par cle |
| `ASTRAL_LLM_IDEMPOTENCY_TTL_HOURS` | `24` | TTL enregistrements idempotence |
| `ASTRAL_LLM_CIRCUIT_BREAKER_FAILURES` | `5` | Echecs consecutifs avant circuit ouvert |
| `ASTRAL_LLM_CIRCUIT_BREAKER_OPEN_SECS` | `60` | Duree circuit ouvert |
| `ASTRAL_LLM_MAX_BODY_BYTES` | `2097152` | Taille max body HTTP |
| `ASTRAL_LLM_MAX_ASTRO_JSON_BYTES` | `524288` | Taille max JSON astro |
| `ASTRAL_LLM_MAX_DOMAIN_COUNT` | `12` | Plafond domaines par requete |
| `ASTRAL_LLM_REQUEST_TIMEOUT_MS` | `120000` | Timeout requete HTTP (+ marge layer) |
| `OPENAI_API_KEY` | — | Cle OpenAI |
| `OPENAI_BASE_URL` | `https://api.openai.com` | URL API OpenAI |
| `OPENAI_DEFAULT_MODEL` | — | Alias repli pour `ASTRAL_LLM_DEFAULT_MODEL` |
| `ANTHROPIC_API_KEY` | — | Cle Anthropic |
| `ANTHROPIC_BASE_URL` | `https://api.anthropic.com` | URL API Anthropic |
| `MISTRAL_API_KEY` | — | Cle Mistral |
| `MISTRAL_BASE_URL` | `https://api.mistral.ai` | URL API Mistral |
| `DATABASE_URL` | — | PostgreSQL (obligatoire si persistence ou prod publique) |

Declaration complete : `.env` / `.env.example` a la racine du depot.

## Profils d'environnement

### `local` / `test`

- Bind `127.0.0.1` par defaut
- `FakeProvider` autorise sans cle provider externe
- Auto-migrate PostgreSQL possible (`ENABLE_PERSISTENCE` + `DB_AUTO_MIGRATE`)
- Idempotence active uniquement si persistence active (sinon pas de deduplication durable)

### `production` — mode `internal`

- `ASTRAL_LLM_API_KEY` obligatoire
- Au moins une cle provider reelle (OpenAI, Anthropic ou Mistral)
- `ASTRAL_LLM_ENABLE_FAKE=false`
- `ASTRAL_LLM_DB_AUTO_MIGRATE=false` ; migrations SQL appliquees **hors runtime**
- `verify_schema()` au boot si persistence sans auto-migrate
- Persistence **optionnelle** (V1 interne / reseau restreint)

### `production` — exposition publique

Declenchee si `ASTRAL_LLM_PRODUCTION_MODE=public` **ou** `ASTRAL_LLM_ALLOW_PUBLIC_BIND=true` (defaut `public` si bind public sans mode explicite).

Regles supplementaires (`ConfigValidator`) :

- `ASTRAL_LLM_ENABLE_PERSISTENCE=true` **obligatoire**
- `DATABASE_URL` **obligatoire**
- Idempotence robuste et audit steps dependent de PostgreSQL

Critere d'acceptation boot : `ENV=production` + exposition publique + `ENABLE_PERSISTENCE=false` => **refus au demarrage**.

## Securite et privacy

### ConfigValidator (boot)

- Production : API key, cle provider, pas de fake, pas d'auto-migrate
- Bind `0.0.0.0` : `ALLOW_PUBLIC_BIND=true`
- Exposition publique : persistence + `DATABASE_URL`
- Limites rate > 0

### Auth et rate limiting

- Auth : `Authorization: Bearer` ou `X-API-Key` (comparaison constant-time)
- Limite **globale** : ne remplace pas les quotas **par cle API**
- 429 standardise (`too_many_requests`)

### Donnees sensibles

- `AstroPayloadNormalizer` : faits astro normalises ; pas de JSON moteur brut dans le prompt
- `PrivacyPolicy` : `redact_birth_data_before_llm`, `disable_provider_storage` (OpenAI `store: false`)
- Persistance : hashes + redaction (`birth_date`, coordonnees, `custom_instructions` => `[REDACTED]`)
- Test golden : `prompt_golden` / `prompt_golden_tests` — le prompt compile ne doit pas contenir PII ni chaines d'injection

### Hierarchie safety

```txt
Mandatory platform rules > Safety rules > Product contract > Astrologer profile > Request > Astro data
```

L'override `safety_policy` ne peut que **renforcer** les regles obligatoires. Pas de fallback sur rejet safety.

Reponse safety standardisee : `status`, `error.code`, `category`, `rule_id`, `violations`.

## Modeles LLM par produit

Configuration operationnelle (sans toucher au code Rust) :

| Etape | Commande |
|---|---|
| 1. Editer | `config/llm_product_models.conf` — une ligne par produit : `product_code` `chapter_model` `summary_model` `[provider]` |
| 2. Appliquer | `.\scripts\set_product_llm_models.ps1` |
| 3. Redemarrer | `astral_llm_api` (catalogue recharge au boot) |
| Verifier | `.\scripts\set_product_llm_models.ps1 -Show` |

Exemple fichier :

```text
natal_premium	gpt-5.4-mini	gpt-5-nano	openai
```

Exemple CLI (sans modifier le fichier) :

```powershell
.\scripts\set_product_llm_models.ps1 -Product natal_premium -Chapters gpt-5.4-mini -Summary gpt-5-nano
```

Comportement runtime :

- **Chapitres** : colonne SQL `default_model` (si `engine.model` absent de la requete).
- **Summary** : colonne SQL `economic_model` (si `engine.model` absent).
- **Test ponctuel** (sans changer la base) : `engine.model`, `engine.summary_model`, ou `generate_premium_reading_e2e.ps1 -Model` / `-SummaryModel`.

Valeurs Premium actuelles (2026-06-04) : chapitres `gpt-5.4-mini`, summary `gpt-5-nano`.

### Referentiels canoniques (base)

Tables (ou bootstrap si DB vide) :

- `llm_astrological_domains`
- `llm_safety_content_patterns`
- `llm_product_prompt_profiles`
- `llm_providers` — moteurs LLM (`provider_code`, `is_active`) ; liste modifiable
- `llm_provider_models` — modeles par moteur (`is_active` = utilisable en production) ; jointure `provider_id` → `llm_providers`
- `ProviderCatalogRepository` (infra) : `list_providers`, `add_provider`, `delete_provider`, `set_provider_active`, `list_models`, `add_model`, `delete_model`, `set_model_active`
- Au boot : `load_active_provider_codes` + `load_model_capabilities` → `ModelCapabilityRegistry::from_db_catalog`
- Avant prompt : `validate_engine_in_catalog` (moteur actif + modele actif dans le catalogue)
- `llm_model_usage_tiers` — profils : `production_candidate`, `baseline`, `subtask_candidate`, `benchmark_compare`, `oracle_only`
- `llm_generation_benchmark_usages` + `llm_generation_benchmark_usage_models` — matrice usage ↔ modeles recommandes
- Seeds OpenAI (vague 1 + vague 2) : tous actifs ; tiers voir SQL ; E2E Premium : `scripts/benchmark_premium_e2e_models.ps1` (5 runs, `-MaxOutputTokens 4096` par defaut, `-IncludeOracle` pour gpt-5.5-pro avec `engine.allow_oracle_benchmark`)
- Apres benchmark : `scripts/summarize_benchmark_runs.ps1` lit le JSONL resume, appelle `GET /v1/runs/{run_id}`, estime le cout (grille OpenAI de reference dans le script) et exporte `benchmark_metrics_<stamp>.csv` + JSONL enrichi (colonnes `manual_*` a remplir a la main)
- Modeles reasoning : `reasoning_output_reserve_min`, `reasoning_effort_subtask` / `_primary` / `_oracle` (litteraux API par modele, ex. gpt-5-mini subtask=`minimal`, gpt-5.4+ subtask=`none`) ; module `reasoning_generation`
- OpenAI Responses API (GPT-5) : `openai_adapter` agrege les blocs `output[].type=message`
- Validation contexte : `PrimaryReading` (chapitres), `Subtask` (summary/repair), `OracleBenchmark` (oracle explicite)
- `llm_product_allowed_models` — modeles autorises par `product_code` (ex. `natal_premium` + gpt-5.4-mini). Liste vide en politique = pas de filtre modele
- `llm_product_default_engine` — `default_model` (chapitres), `economic_model` (summary) ; voir section **Modeles LLM par produit**
- `llm_product_generation_policies`

Les valeurs metier ne sont pas dupliquees en constantes Rust lorsqu'elles existent en base.

## Providers LLM

| Provider | Structured output | Notes |
|---|---|---|
| OpenAI | Responses API, `text.format` + `json_schema` strict | `store: false` |
| Mistral | `response_format: json_schema` | `safe_prompt` si safety native |
| Anthropic | `output_config.format` | Selon capacites modele (`ModelCapabilityRegistry`) |
| Fake | JSON fixe | Local / tests uniquement |

`ProviderSchemaCompiler` adapte le schema canonique au format attendu par chaque provider.
`ProviderCircuitBreaker` : `closed` / `open` / `half_open` — etat visible dans `GET /v1/providers` (`circuit_breakers`).

## Orchestration Premium

- **DomainResolver** : domaines avant LLM (scores astro, preferred, politique produit)
- **ReadingPlanBuilder** : validation du plan chapitres
- **ChapterOrchestrator** : un appel LLM par chapitre ; **summary** via `resolve_subtask_engine` (`economic_model` si la requete ne fixe pas `engine.model`) ; statuts `generated`, `repaired`, `failed`, etc. ; **retry automatique** si chapitre sous `min_words` (2 tentatives, `max_words` non bloquant) ; retry repetition (trigrammes, 3 tentatives) ; **anti-repetition en amont** : `chapter_structure.md`, `ChapterWritingGuidance` (4 paragraphes, phrases des chapitres precedents, amorces interdites) ; score trigrammes sans mots grammaticaux (`text_trigrams`) ; safety par chapitre
- **ExecutionAudit** : steps dans `llm_generation_steps`
- **Token budget** : plafonds par chapitre / global

Modes `response_contract.generation_mode` :

- `single_pass` — Basic, un appel LLM
- `chapter_orchestrated` — Premium, orchestration multi-chapitres

## Idempotence

- Header `Idempotency-Key` ou champ `idempotency_key`
- Table `llm_idempotency_records` (PostgreSQL)
- Flux transactionnel : `SELECT ... FOR UPDATE` puis insert/update dans `claim_idempotency` (evite la course double-insert)
- Finalisation : `finalize_idempotency` apres generation
- Protection : `input_hash` sur payload **redige** (`redact_request_for_storage`), rejeu reponse terminale, reclaim apres `failed` / `safety_rejected`, conflit si payload different (`IDEMPOTENCY_PAYLOAD_MISMATCH`)
- `run_id` API = `run_id` reponse (pas de divergence)
- **Production publique** : cle idempotence **obligatoire** ; echec du store => **503** (fail closed, pas de generation sans verrou)

Sans persistence, l'idempotence **n'est pas** durable entre processus.

## Persistence

### Local / test

Tables creees automatiquement **uniquement** si :

- `ASTRAL_LLM_ENABLE_PERSISTENCE=true`
- `ASTRAL_LLM_DB_AUTO_MIGRATE=true`

### Production

- `ASTRAL_LLM_DB_AUTO_MIGRATE=true` **interdit**
- Migrations appliquees **avant** demarrage (`llm_generation_runs.sql`, `llm_canonical.sql`, `llm_audit_extensions.sql`)
- Boot : `verify_schema()` si auto-migrate desactive

### Tables

| Table | Role |
|---|---|
| `llm_generation_runs` | Audit : hashes, latence, providers, status, safety |
| `llm_generation_steps` | Steps d'execution (chapitres, tokens, erreurs) |
| `llm_idempotency_records` | Idempotence + reponse cachee |
| `llm_generation_payloads` | Optionnel (`STORE_SANITIZED_PAYLOADS=true`) : JSON rediges + `prompt_hash` + `astro_facts_hash` |

### Politique mode degrade (V1)

| Panne | Comportement |
|---|---|
| Echec persistence (prod publique) | Boot refuse si schema / config manquants |
| Echec persistence (local) | Log erreur ; generation peut continuer |
| Echec redaction | Fail closed (donnees non persistees en clair) |
| DB capabilities indisponible | Bootstrap registry si configure ; sinon erreur capability |
| Erreur safety / redaction prompt | Jamais de succes silencieux |

## Qualite redactionnelle

`ReadingQualityValidator` + `EditorialValidator` :

- longueur chapitre, cadrage interpretatif (vs liste de faits)
- repetition (score trigrammes), densite `astro_basis`
- jargon (beginner), disclaimer legal, determinisme
- fatalisme et conseils medical/juridique/financier (fixtures)

**Premium** : validation **bloquante** (`READING_QUALITY_FAILED`) si `chapter_orchestrated` **ou** `product_code=natal_premium`. **Basic** : warnings seulement.

## Observabilite et logs

### Traces structurees (stdout + fichier optionnel)

| Variable | Defaut | Role |
|---|---|---|
| `RUST_LOG` | — | Prioritaire sur `ASTRAL_LLM_LOG_LEVEL` (filtre `tracing`) |
| `ASTRAL_LLM_LOG_LEVEL` | `info` | Niveau global ; cibles `astral_llm.generation` / `astral_llm.provider` en `debug` |
| `ASTRAL_LLM_LOG_FORMAT` | `pretty` | `json` pour logs machine (CI, agregation) |
| `ASTRAL_LLM_LOG_FILE` | — | Fichier append (ex. `output/logs/astral_llm_api.log`) en plus de stdout |
| `ASTRAL_LLM_LOG_COMPILED_PROMPTS` | `true` | Journalise le prompt compile (`target=astral_llm.prompt`, champ `compiled_prompt`) |
| `ASTRAL_LLM_PROMPT_LOG_DIR` | `output/logs/prompts` | Fichier `.txt` par appel LLM : `{dir}/{run_id}/{chapter}_{attempt}.txt` |

Chaque generation emet des evenements correles par `run_id` (et `request_id` si present) :

- `generation started` — produit, provider, mode
- `provider call failed` — erreur OpenAI / fallback (avec chapitre si orchestration)
- `generation failed` — code erreur, details, steps d'audit
- `generation succeeded` — latence, nombre de chapitres

**Prompts compiles** (`ASTRAL_LLM_LOG_COMPILED_PROMPTS`, defaut `true`) :

- Tracing : cible `astral_llm.prompt`, champ `compiled_prompt` (actif si `ASTRAL_LLM_LOG_LEVEL=debug` ou filtre dedie).
- Fichiers : `ASTRAL_LLM_PROMPT_LOG_DIR` (defaut `output/logs/prompts/{run_id}/{chapter}_{attempt}.txt`, summary → `summary_summary.txt`).
- Module : `astral_llm_application::prompt_trace` (appele depuis `ChapterOrchestrator`, `GenerateReadingUseCase`, `SummarySynthesizer`).

### Audit PostgreSQL

Avec `ASTRAL_LLM_ENABLE_PERSISTENCE=true` :

- `llm_generation_runs` — statut terminal, `error_code`, latence, providers
- `llm_generation_steps` — detail par chapitre (tokens, latence, `error_code`)

Consultation API :

```powershell
.\scripts\show_generation_run.ps1 -RunId "<uuid>"
# GET /v1/runs/{run_id}
```

### Scripts E2E locaux

```powershell
.\scripts\generate_premium_reading_e2e.ps1 -IdempotencyKey "e2e-$(Get-Date -Format 'yyyyMMddHHmmss')"
# reponse : output/premium_reading_e2e.json
# journal client : output/logs/premium_reading_e2e_<timestamp>.json
# prompts compiles : output/logs/prompts/<run_id>/*.txt
.\scripts\show_generation_run.ps1 -RunId "<uuid>"
```

## Endpoints

| Methode | Route | Description |
|---|---|---|
| GET | `/health` | Sante (hors rate limit) |
| POST | `/v1/readings/generate` | Generation lecture |
| POST | `/v1/readings/validate` | Validation JSON vs schema |
| GET | `/v1/runs/{run_id}` | Audit run + steps (PostgreSQL requis) |
| GET | `/v1/providers` | Capacites modeles + `circuit_breakers` |
| GET | `/v1/schemas/{version}` | Schema JSON (ex. `natal_reading_v1`) |

## Composants application (reference)

| Composant | Role |
|---|---|
| `ConfigValidator` | Validation configuration au boot |
| `RequestValidator` | Entree HTTP / contrats |
| `ProductPolicyValidator` | Politique produit (provider, domaines, chapitres) |
| `ModelCapabilityRegistry` | Capacites modele (structured output, reasoning) |
| `FallbackPolicy` | Chaine fallback explicite |
| `AstroPayloadNormalizer` | Faits astro pour prompt |
| `PayloadSanitizer` | Injection / instructions custom |
| `SafetyResolver` / `SafetyGuard` | Politique safety pre/post |
| `PromptCompiler` | Assemblage prompts versionnes |
| `ProviderSchemaCompiler` | Schema provider-specific |
| `ProviderRouter` | Appel + fallback + circuit breaker |
| `GenerationTraceContext` | Logs correles par `run_id` (start/finish/provider) |
| `ResponseValidator` / `SchemaRegistry` | JSON structure |
| `ChapterEvidencePlanner` | Packs CORE/SUPPORTING/NUANCE par chapitre + `avoid_repeating` |
| `ChapterEvidenceCoherence` | Cohérence pack / `astro_basis` / corps (repair) |
| `ChapterEvidenceBasisEnricher` | CORE (+ SUPPORTING sauf chapitre `identity`) omis dans `astro_basis` |
| `ChapterWritingGuidance` | 4 §, anti-trigrammes, liste fact_id obligatoires, connecteurs generiques |
| `ReadingOpeningDiversityValidator` | Amorces cross-chapitre ; connecteurs generiques FR ignores |
| `PriorChapterUsage` | `avoid_repeating` semantique + exclusion aspects/dignites deja vus |
| `prompt_trace` | Journalisation prompt compile (fichier + tracing) |
| `ChapterOrchestrator` | Mode Premium (orchestration + repairs) |
| `ExecutionAudit` | Traces steps + agregat tokens run |
| `AstroBasisRoleNormalizer` | Roles canoniques alignes pack |
| `ReadingQualityValidator` | Qualite lecture (repetition amont ; `min_words` repair) |
| `EditorialValidator` | Regles redactionnelles (fixtures + fatalisme) |

## Tests

| Suite | Cible |
|---|---|
| `astral_llm_tests` | Flux Basic/Premium, fallback, policies |
| `astral_llm_injection_tests` | Injection astro, PII normalizer, custom instructions |
| `prompt_golden_tests` | Prompt compile sans PII / injection |
| `astral_llm_editorial_fixtures` | 3 fixtures redactionnelles + cas negatif |
| `astral_llm_load_tests` | Saturation semaphore / rate limit / circuit breaker |
| `astral_llm_load_tests` (`#[ignore]`) | Idempotence concurrente PostgreSQL |
| `provider_real_smoke` (`#[ignore]`) | OpenAI + Mistral + Anthropic (schema + auth) |
| `astral_llm_evidence_planner_tests` | Pool, packs, identity sans soleil, relationships descendant ruler |
| `astral_llm_evidence_coherence_tests` | Coherence pack / corps / astro_basis |
| `astral_llm_i18n_tests` | Locales + humanizer |
| Tests unitaires crates | Registry, circuit breaker, redaction, qualite Premium |

## Roadmap P2 (optimisation — Evidence Planner clos)

- Benchmark OpenAI : cout / latence / qualite par modele sur E2E Premium
- Certification Mistral / Anthropic (smoke + E2E rich)
- Enrichissement `llm_chapter_evidence_slots` / pool (noeuds, phases lunaires, dignites mineures, patterns d'aspects)
- Affinage style : reduire formulations « promptees » et densite consignes sans relacher `astro_basis` / safety
- Scoring qualite enrichi (metriques numeriques dans `quality` reponse)
- Fixtures redactionnelles supplementaires (langues / profils astrologues)

## Phases implementees

1. Contrats domain (request/response/errors)
2. Serveur Axum + FakeProvider
3. PromptCompiler (prompts versionnes)
4. SchemaRegistry (`natal_reading_v1`)
5. SafetyResolver + SafetyGuard
6. ProviderRouter + FallbackPolicy + circuit breaker
7. Adapters OpenAI / Mistral / Anthropic
8. AstroPayloadNormalizer + privacy (`store: false`)
9. ModelCapabilityRegistry + ProductGenerationPolicy
10. DomainResolver + ReadingPlan + ChapterOrchestrator
11. Persistence PostgreSQL + idempotence + audit steps
12. Production publique (ConfigValidator) + rate limit par cle + golden prompt
13. Validation produit : fixtures redactionnelles, load tests, qualite Premium bloquante, smoke providers
14. Premium : planner packs adaptatif, coherence evidence, i18n prompt/humanizer, logs prompts compiles, repairs min_words/repetition
15. Polish Premium : `descendant_ruler`, semantic keys, enrichisseur SUPPORTING, opening diversity + repair multi-chapitres, E2E sans repair career/relationships
16. Polish final : libelles `ruler:*` humanises ; cap supporting par `semantic_fact_key` (`max_supporting_semantic_chapters = 3`)
17. **Evidence Planner clos** — Premium interpretatif riche OpenAI **VALIDÉ PRODUIT** (E2E `0619a1e8`, 2026-06-04)
18. `EditorialValidator`, `READING_QUALITY_FAILED`, crate `astral_llm_api` lib pour tests
