# Contrat API d'intégration — v1

Spécification normative de l'API d'intégration métier (`astral_llm_api`). Les schémas JSON associés sont publiés sous `contracts/llm/integration_*_v1.schema.json`.

## Périmètre V1

| Opération | Endpoint |
|-----------|----------|
| Découvrir les services | `GET /v1/services` |
| Détail contrat d'un service | `GET /v1/services/{code}/contract` |
| Soumettre un job async | `POST /v1/jobs` |
| Suivre un job | `GET /v1/jobs/{run_id}` |
| Notification push (optionnel) | Mercure — Phase 4 |

**Hors V1** : routes sync génériques (`POST /v1/readings/natal/{profile}`, `POST /v1/services/{code}/generate-sync`).

Les endpoints historiques `POST /v1/readings/generate` et `POST /v1/readings/natal/simplified` ont ete retires du runtime courant.

Breaking changes facade publique du 2026-06-14:

- `GET /v1/services` ne publie plus `supports_sync_legacy`
- `GET /v1/services` ne publie plus `endpoints.submit_sync_legacy`
- la source de verite de migration publique est `api_surface`
- les handlers sync legacy ont ete supprimes de `astral_llm_api`

## Enveloppe job vs payload métier

L'enveloppe `integration_job_request_v1` **ne connaît pas** la forme des payloads métier. Le champ `payload` est un objet JSON **opaque** dans le schéma d'enveloppe (`type: object`, sans `$ref` vers contrats calculateur/LLM).

```json
{
  "service_code": "natal_simplified",
  "payload": {},
  "user_language": "fr",
  "audience_level": "beginner",
  "astrologer_profile": {}
}
```

Validation en **deux temps** :

1. Enveloppe → `integration_job_request_v1` → **400** `INVALID_INPUT`
2. `payload` → `payload_contract` du service (catalogue `llm_integration_services`) → **422** `PAYLOAD_VALIDATION_FAILED`

Pour les services `*_from_payload`, une **gate applicative** supplémentaire exige que `payload.product_context.interpretation_profile_code` soit égal au `profile_code` catalogue du service → **422** `PAYLOAD_VALIDATION_FAILED` si mismatch.

Le contrat métier exact est documenté via `GET /v1/services/{code}/contract`.

## Machine d'état — `queued` partout

**Règle stricte V1** : le statut initial et la sémantique publique utilisent **`queued`** uniquement. Pas de `pending` en DB, worker, API ni tests E2E.

| Statut | Terminal ? | Description |
|--------|------------|-------------|
| `queued` | non | Job accepté, en attente de traitement worker |
| `running` | non | Worker en cours d'exécution |
| `completed` | oui | Succès — `result` disponible au poll |
| `failed` | oui | Échec technique ou métier retryable/terminal |
| `safety_rejected` | oui | Garde sécurité — `result` partiel possible |
| `cancelled` | oui | Réservé V1 (non émis) |
| `expired` | oui | Réservé schéma (non émis en V1) |

## Identifiants

```text
llm_jobs.run_id              = identifiant public d'intégration (POST/GET /v1/jobs)
llm_jobs.generation_run_id   = FK optionnelle → llm_generation_runs(id)
```

- `GET /v1/runs/{id}` = audit LLM interne (`generation_run_id`)
- `GET /v1/jobs/{run_id}` = job d'intégration
- L'égalité `run_id == generation_run_id` peut survenir en pratique mais **n'est pas contractuelle**

## Idempotence

Header **`Idempotency-Key` obligatoire** lorsque la persistance stricte est active (`ASTRAL_LLM_ENABLE_PERSISTENCE=true` en production/intégration).

Contrainte DB : `UNIQUE (tenant_id, idempotency_key)` — une clé est **unique par tenant, tous services confondus** (sémantique Stripe-like).

| Cas | HTTP | Corps |
|-----|------|-------|
| Nouvelle clé | **202** | `run_id`, `status: queued`, `poll_url` |
| Même clé + même hash + `queued`/`running` | **202** | même `run_id`, statut courant |
| Même clé + même hash + terminal `completed` | **200** | statut + **`result` inclus** |
| Même clé + hash différent | **409** | `IDEMPOTENCY_CONFLICT` |
| Même clé + **autre `service_code`** | **409** | `IDEMPOTENCY_CONFLICT` |
| Même clé + même hash + **autre `api_key_id`** | **409** | `IDEMPOTENCY_CONFLICT` |

**Replay terminal (figé V1)** : pas de variante 202 sans résultat pour un job `completed`. Paramètre `include_result=false` reporté.

### Exemple cross-service → 409

```http
POST /v1/jobs
Idempotency-Key: order-12345
Content-Type: application/json
```

```json
{
  "service_code": "natal_simplified",
  "payload": { "request_contract_version": "astro_simplified_natal_request_v1", "birth": { "date": "1990-06-15" } },
  "user_language": "fr",
  "audience_level": "beginner"
}
```

→ **202** `{ "run_id": "aaa-111-...", "status": "queued", ... }`

```http
POST /v1/jobs
Idempotency-Key: order-12345
```

```json
{
  "service_code": "natal_basic_from_payload",
  "payload": { "...": "..." },
  "user_language": "fr",
  "audience_level": "beginner"
}
```

→ **409**

```json
{
  "error": {
    "code": "IDEMPOTENCY_CONFLICT",
    "message": "Idempotency-Key already used for a different service or payload"
  }
}
```

**Conséquence** : une clé = une soumission logique unique par tenant. Deux services différents → deux clés distinctes.

Ownership replay : la contrainte reste tenant-wide, mais seul le même
fingerprint `api_key_id` peut rejouer un job existant. Une autre clé API du
même tenant qui réutilise la même `Idempotency-Key` reçoit **409** au lieu du
résultat.

## Hashing

### Objet logique job (`idempotency_payload_hash`)

Champs inclus (minimum) :

```text
service_code
payload
user_language
audience_level
astrologer_profile
```

Deux soumissions avec le même `payload` métier mais des options de génération différentes → hash différent → pas de replay involontaire.

### Algorithme (canonique)

```text
1. Construire l'objet logique (idem ci-dessus pour idempotence)
2. Canonicaliser JSON : clés triées récursivement, encodage UTF-8 stable
3. Exclure champs volatils côté client (timestamps, request_id optionnel)
4. Exclure Idempotency-Key du corps hashé (header HTTP uniquement)
5. SHA-256(hex) du JSON canonique
```

- `idempotency_payload_hash` : objet logique job
- `request_payload_hash` : corps complet soumis (audit/debug)

Implémentation : `astral_llm_infra::canonical_json_hash`.

## Matrice HTTP — POST /v1/jobs

| Code | Code erreur / sémantique | Exemple |
|------|--------------------------|---------|
| **202** | `JOB_ACCEPTED` — nouveau job ou replay en cours | `{ "run_id": "...", "status": "queued", "poll_url": "/v1/jobs/..." }` |
| **200** | Replay idempotent `completed` | `{ "run_id": "...", "status": "completed", "result": { ... } }` |
| **400** | `INVALID_INPUT` — enveloppe | `{ "error": { "code": "INVALID_INPUT", "message": "service_code is required" } }` |
| **400** | `IDEMPOTENCY_KEY_REQUIRED` | `{ "error": { "code": "IDEMPOTENCY_KEY_REQUIRED", "message": "..." } }` |
| **401** | `UNAUTHORIZED` | `{ "error": { "code": "UNAUTHORIZED", "message": "..." } }` |
| **403** | `FORBIDDEN` | `{ "error": { "code": "FORBIDDEN", "message": "..." } }` |
| **404** | `SERVICE_NOT_FOUND` — inconnu ou `availability` ∉ {active, beta} | `{ "error": { "code": "SERVICE_NOT_FOUND", "message": "..." } }` |
| **409** | `IDEMPOTENCY_CONFLICT` | voir exemple cross-service |
| **422** | `PAYLOAD_VALIDATION_FAILED` | `{ "error": { "code": "PAYLOAD_VALIDATION_FAILED", "message": "...", "details": { "errors": [...] } } }` |
| **429** | `RATE_LIMITED` | `{ "error": { "code": "RATE_LIMITED", "message": "..." } }` |
| **501** | `SERVICE_NOT_IMPLEMENTED` — active/beta mais orchestrateur absent (temporaire) | `{ "error": { "code": "SERVICE_NOT_IMPLEMENTED", "message": "..." } }` |
| **503** | `SERVICE_UNAVAILABLE` — persistance indisponible | `{ "error": { "code": "SERVICE_UNAVAILABLE", "message": "..." } }` |

### GET /v1/jobs/{run_id}

| Code | Description |
|------|-------------|
| **200** | `integration_job_status_v1` — inclut `result` si `completed` |
| **401** | Non authentifié |
| **404** | Job inexistant, non autorisé (ownership tenant + api_key_id), ou expiré/purgé |

Ownership : même `tenant_id` (header `X-Tenant-Id`, défaut `default`) et même fingerprint `api_key_id` qu'à la soumission.

## 404 vs 501

```text
availability ∈ {planned, disabled, deprecated}
  et GET /v1/services sans ?include=planned
  → service non listé ; POST /v1/jobs → 404 SERVICE_NOT_FOUND

availability ∈ {active, beta}
  mais orchestrateur/worker pas encore implémenté
  → POST /v1/jobs → 501 SERVICE_NOT_IMPLEMENTED avant mise en file
```

Phase historique pilote : seul `natal_simplified` etait initialement executable ; les services `*_from_payload` sont maintenant classes `deprecated` et restent hors catalogue public courant.

## Rétention

```text
Jobs terminaux (completed / failed / safety_rejected) :
  result_json conservé jusqu'à expires_at (TTL configurable)
  purge complète après TTL
  GET /v1/jobs/{run_id} après purge → 404 JOB_NOT_FOUND

Statut expired : réservé schéma, non émis en V1
(pas de 200 status=expired)
```

## Mercure (Phase 4)

Topic : `tenants/{tenant_id}/jobs/{run_id}`

Événement minimal :

```json
{
  "run_id": "uuid",
  "status": "completed",
  "poll_url": "/v1/jobs/uuid"
}
```

Le corps complet du résultat reste accessible via poll HTTP. `supports_mercure: true` dans le catalogue pour les services éligibles.

## Schémas publiés

| Schéma | Rôle |
|--------|------|
| `integration_job_request_v1` | Enveloppe soumission |
| `integration_job_response_v1` | Réponse submit (202) |
| `integration_job_status_v1` | Poll + replay 200 |
| `integration_service_v1` | Item catalogue |
| `integration_service_contract_v1` | Détail contrat service |

Découverte : `GET /v1/contracts` sur `astral_llm_api`.

Le catalogue V1 publie `api_surface` pour distinguer explicitement :

- `async_job_v1_status`: surface async V1 encore courante pour l'intégration jobs
- `sync_legacy_status`: route sync legacy interne ou historique
- `public_gateway_v2_status`: surface publique recommandee cote `astral_gateway`
- `recommended_entrypoint`: endpoint public v2 recommande quand il existe

## Auth et tenancy

- Auth : `Authorization: Bearer <key>` ou `X-API-Key` si `ASTRAL_LLM_API_KEY` configuré
- `X-Tenant-Id` : identifiant tenant (défaut `default`)
- Dev sans auth : `api_key_id` fixe `key:dev-local`

## Positionnement des surfaces

- `astral_gateway` porte la surface publique recommandee pour l'orchestration metier (`/v2/natal/*`, `/v2/horoscope/*`)
- `astral_llm_api` conserve le contrat async V1 pour l'integration par jobs (`/v1/services`, `/v1/jobs`)
- `astral_calculator_api` est une surface HTTP technique interne au calculateur ; les appels inter-services utilisent `/v1/internal/calculations/*`
- les routes calculateur historiques `/v1/calculations/*` restent des aliases legacy compatibles, pas l'entree recommandee pour les nouveaux appels internes
- les routes sync heritagees restent hors contrat public V2 et sont exposees comme `legacy` dans `api_surface`

## Legacy runtime

- les handlers sync legacy ont ete supprimes du runtime courant
- `ASTRAL_LLM_ENABLE_LEGACY_PRODUCT_CODE_SHIM=false` coupe la migration implicite des anciens `product_code` (`natal_basic`, `natal_premium`) vers `natal_prompter`
- `ASTRAL_LLM_LEGACY_PRODUCT_CODE_SHIM_CUTOFF_DATE=YYYY-MM-DD` permet une extinction datee de ce shim

## Routes sync — hors contrat V1 intégration

Les routes legacy sync ne font pas partie du contrat d'intégration async V1.
## Service horoscope basic daily natal 3 slots

- `service_code` : `horoscope_basic_daily_natal_3_slots`
- `availability` : `beta`
- `payload_contract` : `horoscope_basic_daily_natal_request`
- `calculation_output_contract` : `horoscope_calculation_response`
- `reading_output_contract` : `horoscope_response`

Exemple `POST /v1/jobs` :

```json
{
  "service_code": "horoscope_basic_daily_natal_3_slots",
  "payload": {
    "date": "2026-06-06",
    "timezone": "Europe/Paris",
    "target_language": "fr",
    "chart_calculation_id": "123",
    "audience_level": "general"
  },
  "user_language": "fr",
  "audience_level": "beginner"
}
```

Exemple `GET /v1/jobs/{run_id}` complete :

```json
{
  "run_id": "00000000-0000-0000-0000-000000000000",
  "service_code": "horoscope_basic_daily_natal_3_slots",
  "status": "completed",
  "result": {
    "calculation": {},
    "interpretation_request": {},
    "reading": {
      "contract_version": "horoscope_response",
      "service_code": "horoscope_basic_daily_natal_3_slots",
      "summary": {
        "title": "Une journée à ajuster avec précision",
        "text": "Résumé court de la tonalité générale."
      },
      "slots": [
        {
          "slot_code": "morning",
          "title": "Matin",
          "theme": "Organisation",
          "tone": "focused",
          "text": "Texte public du matin rédigé depuis les preuves du slot.",
          "advice": "Conseil distinct du matin.",
          "best_for": ["organization", "routine"],
          "watch_point": "avoid_opening_too_many_topics",
          "evidence_keys": ["slot:morning:moon:natal_house:6"]
        }
      ],
      "quality": {
        "evidence_coverage": 1.0,
        "slot_diversity_passed": true,
        "french_typography_passed": true,
        "generic_language_passed": true
      }
    }
  }
}
```

Erreurs possibles : `HOROSCOPE_PAYLOAD_INVALID`,
`HOROSCOPE_NATAL_CHART_REQUIRED`, `HOROSCOPE_CALCULATOR_UNAVAILABLE`,
`HOROSCOPE_CALCULATION_FAILED`, `HOROSCOPE_SCORING_FAILED`,
`HOROSCOPE_NO_SIGNIFICANT_SIGNAL`, `HOROSCOPE_EVIDENCE_MISMATCH`,
`HOROSCOPE_RESPONSE_INVALID`, `HOROSCOPE_SLOT_REPETITION_FAILED`,
`HOROSCOPE_SLOT_TOO_GENERIC`, `HOROSCOPE_SLOT_ASTRO_REFERENCE_MISSING`,
`HOROSCOPE_PUBLIC_SLOT_CODE_LEAK`, `HOROSCOPE_FRENCH_TYPOGRAPHY_FAILED`,
`SERVICE_NOT_IMPLEMENTED`,
`IDEMPOTENCY_CONFLICT`.

## Service horoscope free daily

- `service_code` : `horoscope_free_daily`
- `availability` : `beta`
- `payload_contract` : `horoscope_daily_natal_request`
- `calculation_output_contract` : `horoscope_calculation_response`
- `reading_output_contract` : `horoscope_response`

`horoscope_free_daily` est personnalise natal en V1 : `chart_calculation_id` est
obligatoire et `birth_data` inline est refuse. Ce service n'est pas un horoscope
general par signe.

Pour `horoscope_free_daily`, `day` est uniquement un slot technique de
projection dans les payloads internes. Il ne constitue pas une section publique
et ne doit jamais apparaitre dans la reponse utilisateur.

Statut de validation V1 :

- reponse publique sans `slots` : PASS ;
- aucune fuite publique de `day` / `slot:day` : PASS ;
- `advice` present : PASS ;
- `evidence_keys` present et non vide : PASS ;
- `quality` present : PASS ;
- non-regression Basic : PASS ;
- tests horoscope : PASS -- 45/45 ;
- typographie francaise : PASS selon les regles actuelles.

La normalisation de l'apostrophe typographique (`'` -> `’`) n'est pas une regle
bloquante V1. Les controles actuels ciblent les elisions cassees (`l impression`)
et les ponctuations invalides telles que `Conseil:`.

Exemple `POST /v1/jobs` :

```json
{
  "service_code": "horoscope_free_daily",
  "payload": {
    "date": "2026-06-06",
    "timezone": "Europe/Paris",
    "target_language": "fr",
    "chart_calculation_id": "123",
    "audience_level": "general"
  },
  "user_language": "fr",
  "audience_level": "beginner"
}
```

Exemple `GET /v1/jobs/{run_id}` complete :

```json
{
  "run_id": "00000000-0000-0000-0000-000000000000",
  "service_code": "horoscope_free_daily",
  "status": "completed",
  "result": {
    "calculation": {},
    "interpretation_request": {},
    "reading": {
      "contract_version": "horoscope_response",
      "service_code": "horoscope_free_daily",
      "summary": {
        "title": "Votre tendance du jour",
        "text": "Texte court de tendance generale, relie a une preuve astrologique fournie."
      },
      "advice": "Conseil court et concret.",
      "watch_point": "Point de vigilance court.",
      "evidence_keys": ["slot:day:moon:natal_house:6"],
      "quality": {
        "evidence_coverage": 1.0,
        "slot_diversity_passed": "not_applicable",
        "french_typography_passed": true,
        "generic_language_passed": true
      }
    }
  }
}
```

Erreurs possibles : `HOROSCOPE_PAYLOAD_INVALID`,
`HOROSCOPE_NATAL_CHART_REQUIRED`, `HOROSCOPE_NATAL_CHART_NOT_FOUND`,
`HOROSCOPE_CALCULATOR_UNAVAILABLE`, `HOROSCOPE_CALCULATION_FAILED`,
`HOROSCOPE_SCORING_FAILED`, `HOROSCOPE_NO_SIGNIFICANT_SIGNAL`,
`HOROSCOPE_EVIDENCE_MISMATCH`, `HOROSCOPE_RESPONSE_INVALID`,
`HOROSCOPE_PUBLIC_SLOT_CODE_LEAK`, `HOROSCOPE_FRENCH_TYPOGRAPHY_FAILED`,
`SERVICE_NOT_IMPLEMENTED`, `IDEMPOTENCY_CONFLICT`.

### Service `horoscope_premium_daily_local_2h_slots`

- `availability` : `beta`
- `payload_contract` : `horoscope_premium_daily_local_request`
- `calculation_output_contract` : `horoscope_calculation_response`
- `reading_output_contract` : `horoscope_response`
- Endpoint : `POST /v1/jobs`

Premium V1 est un horoscope quotidien local personnalise en 12 creneaux publics
de 2 heures. Il requiert un theme natal existant, une timezone IANA et une
localisation de reference. La localisation sert au calcul du ciel local,
Ascendant, MC et maisons locales.

Payload minimal :

```json
{
  "service_code": "horoscope_premium_daily_local_2h_slots",
  "payload": {
    "date": "2026-06-06",
    "timezone": "Europe/Paris",
    "target_language": "fr",
    "chart_calculation_id": "123",
    "location": {
      "latitude": 48.8566,
      "longitude": 2.3522,
      "label": "Paris"
    },
    "audience_level": "general",
    "detail_level": "premium_rich"
  },
  "user_language": "fr",
  "audience_level": "beginner"
}
```

Regles publiques :

- `chart_calculation_id`, `timezone`, `location.latitude` et
  `location.longitude` sont obligatoires.
- `birth_data` inline est refuse.
- `location.label` est optionnel ; s'il est absent, la reponse ne doit pas
  inventer de ville.
- La reponse Premium contient exactement `timeline[12]`, ordonnee selon le
  profil horaire, avec labels publics horaires.
- `best_slots` et `watch_slots` sont non vides, evidences et sans
  chevauchement.
- Les textes publics ne doivent jamais exposer de `slot_code` technique.

Erreurs possibles :

`HOROSCOPE_NATAL_CHART_REQUIRED`, `HOROSCOPE_LOCATION_REQUIRED`,
`HOROSCOPE_TIMEZONE_REQUIRED`, `HOROSCOPE_PAYLOAD_INVALID`,
`HOROSCOPE_PREMIUM_LOCAL_CHART_MISSING`,
`HOROSCOPE_PREMIUM_TIMELINE_MISSING`,
`HOROSCOPE_PREMIUM_SLOT_EVIDENCE_MISSING`,
`HOROSCOPE_PREMIUM_CONTRADICTORY_SLOT_CLASSIFICATION`,
`HOROSCOPE_PUBLIC_SLOT_CODE_LEAK`, `HOROSCOPE_EVIDENCE_MISMATCH`.

### Service `horoscope_free_next_7_days_natal`

Statut catalogue : `active`.

Positionnement produit : Free = comprendre la tendance. Ce service reste natal
personnalise et synthetique ; il ne publie pas la timeline Basic ni les
fenetres/strategie Premium.

Payload : `horoscope_period_natal_request`.

Contraintes :

- `chart_calculation_id`, `anchor_date`, `timezone`, `target_language` requis
- `birth_data` inline refuse
- `period_profile_code = next_7_days`
- `detail_profile_code = free_compact`
- `scan_profile_code = daily_noon_7_days`
- scan de 7 snapshots exactement

Exemple payload :

```json
{
  "service_code": "horoscope_free_next_7_days_natal",
  "payload": {
    "anchor_date": "2026-06-07",
    "timezone": "Europe/Paris",
    "target_language": "fr",
    "chart_calculation_id": "123",
    "audience_level": "general"
  }
}
```

Shape reponse publique :

```json
{
  "contract_version": "horoscope_period_response",
  "service_code": "horoscope_free_next_7_days_natal",
  "period_resolution": {},
  "summary": { "title": "Vos 7 prochains jours", "text": "..." },
  "dominant_theme": { "theme": "organisation", "text": "..." },
  "key_days": [{ "date": "2026-06-10", "title": "Repère du jour", "reason": "...", "evidence_keys": [] }],
  "advice": "...",
  "watch_summary": { "status": "none", "text": "...", "evidence_keys": [] },
  "evidence_summary": [{ "date": "2026-06-10", "evidence_key": "...", "label": "..." }],
  "quality": {}
}
```

Le front affiche `key_days` sous le libelle public "Jours a retenir".
Champs interdits en Free : `daily_timeline`, `best_days`, `watch_days`,
`week_overview`, `best_windows`, `watch_windows`, `domain_sections`,
`strategy`. Le payload d'interpretation Free garde les snapshots et preuves,
mais n'envoie pas de `daily_plans` au writer. `watch_summary.status` accepte
`none`, `low` ou `present`; `present` reste interdit aux shapes Basic/Premium.

Validation fake :

```powershell
.\scripts\test_horoscope_free_next_7_days_fake.ps1
```

### Service `horoscope_basic_next_7_days_natal`

- `availability` : `beta`
- `payload_contract` : `horoscope_period_natal_request`
- `calculation_output_contract` : `horoscope_period_calculation_response`
- `reading_output_contract` : `horoscope_period_response`
- Endpoint : `POST /v1/jobs`

Ce service est un horoscope de periode Basic personnalise sur theme natal. Il
utilise `period_profile_code = next_7_days`, `detail_profile_code =
basic_standard` et `scan_profile_code = daily_noon_7_days` depuis le catalogue.
Le payload public ne peut pas surcharger ces profils.

Payload minimal :

```json
{
  "service_code": "horoscope_basic_next_7_days_natal",
  "payload": {
    "anchor_date": "2026-06-07",
    "timezone": "Europe/Paris",
    "target_language": "fr",
    "chart_calculation_id": "123",
    "audience_level": "general"
  }
}
```

Regles publiques :

- `anchor_date` est une date civile locale interpretee dans `timezone`.
- `chart_calculation_id`, `anchor_date`, `timezone` et `target_language` sont
  obligatoires.
- `birth_data` inline est refuse.
- La fenetre est resolue par `astral_time_window`.
- Les champs `start_datetime_utc`, `end_datetime_utc` et
  `reference_datetime_utc` sont normalises en UTC reel (`+00:00` ou `Z`).
- La reponse contient exactement `daily_timeline[7]`, alignee sur
  `period_resolution.included_dates`.
- `best_days` et `watch_days` ne peuvent pas se chevaucher.
- `best_days` est borne par le profil de detail ; le maximum catalogue n'est pas
  un minimum, et l'API peut retourner moins d'entrees si les signaux valides ne
  justifient pas de date supplementaire.
- En E2E reel, le calculateur ne doit pas retourner de source `fake_*`, le
  writer ne doit pas utiliser le provider `fake`, et le texte public ne doit pas
  exposer les codes internes (`theme_code`, `period:`, `natal_`, `transit_*`).

Erreurs possibles :

`HOROSCOPE_PERIOD_PROFILE_UNSUPPORTED`,
`HOROSCOPE_PERIOD_ANCHOR_DATE_REQUIRED`,
`HOROSCOPE_PERIOD_TIMEZONE_REQUIRED`,
`HOROSCOPE_PERIOD_NATAL_CHART_REQUIRED`,
`HOROSCOPE_PERIOD_SCAN_PLAN_INVALID`,
`HOROSCOPE_PERIOD_CALCULATION_FAILED`,
`HOROSCOPE_PERIOD_EVENT_OUTSIDE_WINDOW`,
`HOROSCOPE_PERIOD_TECHNICAL_CODE_LEAK`,
`HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH`,
`HOROSCOPE_PERIOD_EVIDENCE_MISSING`.

### Service `horoscope_premium_next_7_days_natal`

- `availability` : `beta`
- `payload_contract` : `horoscope_period_natal_request`
- `calculation_output_contract` : `horoscope_period_calculation_response`
- `reading_output_contract` : `horoscope_period_response`
- `detail_profile_code` : `premium_rich`
- `scan_profile_code` : `six_hour_7_days`
- Endpoint : `POST /v1/jobs`

Ce service est la version Premium V1 de l'horoscope des 7 prochains jours. Il
reste natal, sans localisation obligatoire, et reutilise l'infrastructure async
existante.

Payload minimal :

```json
{
  "service_code": "horoscope_premium_next_7_days_natal",
  "payload": {
    "anchor_date": "2026-06-07",
    "timezone": "Europe/Paris",
    "target_language": "fr",
    "chart_calculation_id": "123",
    "audience_level": "general"
  }
}
```

Regles publiques :

- `chart_calculation_id`, `anchor_date`, `timezone` et `target_language` sont
  obligatoires.
- `birth_data` inline est refuse.
- `period_profile_code`, `detail_profile_code` et `scan_profile_code` viennent
  du catalogue service.
- Le scan `six_hour_7_days` contient 28 snapshots : 00:00, 06:00, 12:00 et
  18:00 pour chacune des 7 dates incluses.
- `best_days` et `watch_days` designent des dates ; `watch_days` represente des
  journees de vigilance forte.
- Pour Premium, `best_days` contient jusqu'a 3 dates distinctes. Il peut rester
  a 2 si seules deux dates sont suffisamment nettes apres scoring,
  deduplication et exclusions `key_days`/`watch_days`.
- `best_windows` et `watch_windows` designent des plages horaires et doivent
  referencer des `source_snapshot_keys` existants.
- La reponse Premium contient `strategy`, 3 a 5 `domain_sections`, 7 entrees
  `daily_timeline`, `best_windows` et des `watch_windows` evidences. Si aucune
  tension forte ne ressort mais que des signaux exploitables existent,
  `watch_summary.status = low` decrit une vigilance douce. `status = none`
  n'est utilise que sans signal exploitable. `watch_windows` peut donc etre non
  vide avec `watch_days = []` uniquement en statut `low`.
- Le profil `premium_rich` vise 1600 a 2600 mots et impose une limite dure de 3200 mots.
- Les `best_windows` Premium doivent avoir des titres et `best_for`
  différenciés ; un titre trop générique est refusé.

Reponse abregee :

```json
{
  "contract_version": "horoscope_period_response",
  "service_code": "horoscope_premium_next_7_days_natal",
  "period_resolution": {},
  "week_overview": {},
  "key_days": [],
  "best_days": [],
  "watch_days": [],
  "watch_summary": { "status": "active|low|none", "text": "...", "evidence_keys": [] },
  "best_windows": [],
  "watch_windows": [],
  "daily_timeline": [],
  "domain_sections": [],
  "strategy": {},
  "advice": {},
  "evidence_summary": [],
  "quality": {}
}
```

Erreurs Premium possibles :

`HOROSCOPE_PERIOD_PREMIUM_WINDOWS_MISSING`,
`HOROSCOPE_PERIOD_PREMIUM_STRATEGY_MISSING`,
`HOROSCOPE_PERIOD_PREMIUM_DOMAIN_DEPTH_MISSING`,
`HOROSCOPE_PERIOD_PREMIUM_WINDOW_EVIDENCE_MISSING`,
`HOROSCOPE_PERIOD_PREMIUM_WINDOW_OVERLAP`,
`HOROSCOPE_PERIOD_PREMIUM_WINDOWS_TOO_GENERIC`,
`HOROSCOPE_PERIOD_BROKEN_FRENCH_FRAGMENT`,
`HOROSCOPE_PERIOD_PREMIUM_INSUFFICIENT_DETAIL`,
`HOROSCOPE_PERIOD_TECHNICAL_CODE_LEAK`,
`HOROSCOPE_PERIOD_INTERNAL_GUIDANCE_LEAK`,
`HOROSCOPE_PERIOD_BROKEN_SENTENCE`.
