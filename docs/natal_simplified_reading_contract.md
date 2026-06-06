# Contrat lecture natal simplifiée (produit)

Document de référence produit — règles de vérité avant implémentation code.

## Objectif

Produire une **interprétation simplifiée** (aussi : lecture partielle, lecture indicative, profil astrologique essentiel) à partir de données de naissance incomplètes, **sans simuler** Ascendant, maisons, secte ni placements en maisons lorsque l'entrée ne le permet pas.

## Quatre notions (+ deux niveaux d'exclusion)

| Notion | Rôle |
|--------|------|
| `input_precision` | Ce que l'utilisateur a fourni (audit + UX) |
| `computed_scope` | Ce que le moteur peut calculer honnêtement |
| `reliability` | Fiabilité de chaque fait astrologique |
| Contrôles LLM | `allowed_fact_codes`, `allowed_astro_basis_fact_ids`, `blocked_interpretation_fact_codes`, etc. |

Deux niveaux d'exclusion distincts dans `llm_payload` :

| Champ | Signification |
|-------|---------------|
| `excluded_feature_codes` | Features **non calculées** (effet de `limitations` / scope) |
| `profile_excluded_feature_codes` | Features **calculées mais non utilisées** par le profil `natal_simplified` — chargées depuis `astral_simplified_profile_feature_exclusions` (V1 : `ascendant`, `houses`, `sect`, `house_placements` pour tout scope) |

Cas typique `complete_birth_data` : `computed_scope = angular_chart`, `excluded_feature_codes = []`, mais `profile_excluded_feature_codes` contient toujours ASC/maisons. La lecture doit expliquer que c'est un **choix de niveau produit**, pas une donnée manquante.

## Matrice `input_precision` × `computed_scope`

| Entrée | `input_precision.level` | `computed_scope` |
|--------|-------------------------|------------------|
| Date seule | `date_only` | `stable_birth_date_profile` |
| Date + lieu sans timezone | `date_with_location_without_timezone` | `stable_birth_date_profile` |
| Date + timezone sans heure | `date_with_timezone_without_time` | `stable_birth_date_profile` |
| Date + lieu + timezone sans heure | `date_with_location_and_timezone_without_time` | `stable_birth_date_profile` |
| Date + heure + timezone sans lieu | `datetime_without_location` | `planetary_positions` |
| Date + heure + timezone + lieu | `complete_birth_data` | `angular_chart` |

## Règles fuseau V1

- **Pas** de résolution automatique fuseau ← coordonnées.
- Lieu sans `timezone` explicite **n'améliore pas** la fenêtre temporelle.
- `timezone` IANA explicite requis dès qu'une heure est fournie.
- Mention UX autorisée via `allowed_limitation_mentions` : `location_provided_without_usable_timezone`, `local_day_window`.

## Fenêtre d'incertitude

| Mode | Fenêtre UTC |
|------|-------------|
| Date seule sans timezone | ~50 h (UTC+14 → UTC-12 pour la date civile) |
| Date + timezone sans heure | 24 h journée locale |
| Date + heure + timezone | Instant déclaré |

Échantillonnage : 60 min (canonique DB), toujours `start_utc` et `end_utc` inclus.

## Limitations vs effets

- `limitations[]` = **causes** uniquement (`birth_time_missing`, `location_provided_without_usable_timezone`, …)
- `excluded_features[]` = **effets calculatoires** (`ascendant`, `houses`, `sect`, `house_placements`)

## Mapping `allowed_fact_codes` ↔ `astro_basis.fact_id`

Deux namespaces distincts :

| Usage | Format | Exemple |
|-------|--------|---------|
| Affirmations rédactionnelles (wording) | `{object}.sign` | `mercury.sign` |
| Citations structurées `astro_basis.fact_id` | `placement:{object}` | `placement:mercury` |

Le prompt impose : **utiliser exclusivement** `allowed_astro_basis_fact_ids` pour `astro_basis.fact_id` — jamais `allowed_fact_codes`.

Normalisation côté serveur (`evidence_fact_parse::normalize_chapter_astro_basis_fact_ids`) : `mercury.sign` → `placement:mercury` **après** parsing LLM, **avant** les validateurs, puis whitelist stricte.

`llm_payload.forbidden_interpretation_topics` remplace le nom ambigu `forbidden_topics` (alias déprécié encore émis en miroir pour compat lecture). Détail sémantique : [`natal_simplified_forbidden_topics.md`](natal_simplified_forbidden_topics.md).

### Champs `llm_payload` (réponse calculateur)

| Champ | Rôle |
|-------|------|
| `allowed_fact_codes` | Affirmations rédactionnelles autorisées (`mercury.sign`) |
| `allowed_astro_basis_fact_ids` | IDs autorisés pour `astro_basis.fact_id` (`placement:mercury`) |
| `blocked_interpretation_fact_codes` | Faits ambigus — pas d'affirmation interprétative |
| `excluded_feature_codes` | Non calculé (scope / limitations) |
| `profile_excluded_feature_codes` | Calculé mais exclu du profil simplified |
| `allowed_limitation_mentions` | Limitations mentionnables en UX |
| `forbidden_interpretation_topics` | Agrégat dérivé (prompt / doc) — non consommé par SafetyGuard |
| `forbidden_topics` | **Déprécié** — miroir de `forbidden_interpretation_topics` |

## Routing chapitre — Soleil ambigu

Si `sun.sign` ∈ `blocked_interpretation_fact_codes` :

- Chapitre : **`ambiguous_core_identity`** (pas `identity`)
- Consigne : expliquer la zone de changement possible entre signes, puis placements stables secondaires avec prudence
- Interdit : affirmation d'un signe solaire déterministe

Profil : `chapter_types` inclut `identity` et `ambiguous_core_identity`.

## Réponse API orchestrée (`POST /v1/readings/natal/simplified`)

Corps requête : champs `astro_simplified_natal_request_v1` **à la racine** (`request_contract_version`, `birth`, …) + `user_language` (défaut `fr`) + `audience_level` (défaut `beginner`).

### Enveloppe orchestrée (calcul réussi)

Retournée uniquement lorsque le calculateur a répondu **200** et que la génération a été tentée :

```json
{
  "reading_completeness": "partial",
  "calculation": { "response_contract_version": "astro_simplified_natal_response_v1" },
  "reading": {
    "status": "success",
    "run_id": "...",
    "reading": {
      "schema_version": "natal_reading_v1",
      "chapters": []
    }
  },
  "run_id": "..."
}
```

`reading` est un `GenerateReadingResponse` tagué (`status`). En succès, `reading.reading` est le `NatalReadingResponse` (chapitres, summary, legal, quality).

### Codes HTTP — matrice entrée invalide

| Endpoint | HTTP | Code métier | Enveloppe `{ calculation, reading }` |
|----------|------|-------------|----------------------------------------|
| `POST /v1/calculations/natal/simplified` | **422** | `VALIDATION_FAILED` | Non (error_response_v1) |
| `POST /v1/readings/natal/simplified` | **400** | `INVALID_INPUT` | Non (error_response_v1) |

Recette E2E : **12/12** calculateur (7 positifs + 5 négatifs **422**), **7/7** lectures positives, **5/5** négatifs orchestration **400** (`test_natal_simplified_e2e.ps1` phases 1, 2 et 2b).

### Codes HTTP — orchestration (succès et post-génération)

| Situation | HTTP | Corps |
|-----------|------|-------|
| Succès | 200 | Enveloppe orchestrée, `reading.status: success` |
| Garde simplified ou SafetyGuard | 422 | Enveloppe orchestrée, `reading.status: safety_rejected`, `violations[]` |
| Échec génération (qualité, provider, …) | 4xx/5xx | Enveloppe orchestrée, `reading.status: failed` |

Messages typiques en `safety_rejected` :

- `generated content failed simplified reading guard` — whitelist astro_basis, signes FR bloqués, ASC/maison si profil exclut
- `generated content failed safety validation` — SafetyGuard (patterns sensibles, `forbidden_wording`, script inattendu via `reading_script_guard`)

Le corps contient toujours `calculation` + `reading` en 422 (audit client).

### Erreurs entrée — avant enveloppe orchestrée

Validation gateway (`validate_simplified_calculation_request`) ou rejet calculateur **avant** génération → réponse **400** `INVALID_INPUT`, **sans** enveloppe `{ calculation, reading }` :

```json
{
  "status": "failed",
  "error": { "code": "INVALID_INPUT", "message": "..." },
  "request_id": "..."
}
```

Cas concernés : contrat obsolète, format date (`not-a-date`), lieu sans lat/lon, heure sans timezone.  
Une date calendaire impossible (`2024-02-30`) passe le format gateway mais est rejetée par le **calculateur** (même chemin 400 avec détail calculateur dans `error.details`).

### Calculateur seul (`POST /v1/calculations/natal/simplified`)

Erreurs métier entrée → **422** `VALIDATION_FAILED` (format, date impossible, coordonnées, contrat, heure sans timezone). Voir scripts `test_natal_simplified_calculator.ps1` (cas négatifs).

## Wording UX public

Utiliser : interprétation simplifiée, lecture partielle, lecture indicative, profil astrologique essentiel.

Ne **pas** exposer : dégradée, `degraded`, `minimum_reading_level: degraded`.

Validation gateway (`validate_simplified_calculation_request`) ou rejet calculateur **avant** génération → réponse **400** `INVALID_INPUT`, **sans** enveloppe `{ calculation, reading }` :

Contrat technique API : `reading_completeness: partial` (V1 — seule valeur émise par `reading_hint` calculateur). La valeur schema `simplified` est **réservée** (forward-compat, `x-deprecated-enum-values` dans le schéma JSON) et **n'est pas émise** en V1 ; les assertions E2E exigent strictement `partial`.

## Entrée API — lieu calculatoire

- `birth.location` : lat/lon obligatoires si présent.
  - Calculateur seul → **422**
  - Orchestration lecture → **400** `INVALID_INPUT` (validation gateway)
- Date calendaire impossible (ex. `2024-02-30`) :
  - Calculateur seul → **422**
  - Orchestration lecture → **400** (rejet calculateur remonté par le client HTTP LLM)
- Libellé UX non calculatoire : `input_metadata.location_label` uniquement.

## Miroir legacy `planets{}`

Présent dans `simplified_payload.payload` (natal simplifié) et dans les projections smoke `natal_light`. Rempli seulement si `reliability ∈ [stable_across_uncertainty_window, calculated_from_declared_datetime]`. Les objets ambigus (`ambiguous_facts`) restent absents ou `null` dans `planets{}`.

## Payload prompt LLM (scrubbing)

Avant envoi au LLM, le gateway retire du `data_payload` :

- Faits / `planets` correspondant aux `blocked_interpretation_fact_codes`
- Compteurs `position_count`, `house_cusp_count`, `aspect_count` (évite inférence maisons/aspects sans fait autorisé)

## Post-traitement serveur (profil `natal_simplified`, `single_pass`)

Après génération LLM, avant validation finale (`simplified_reading_postprocess.rs`) :

| Étape | Effet |
|-------|--------|
| Disclaimer | Injection canonique `legal.disclaimer` |
| Typographie FR | Restauration élisions (`l impression` → `l'impression`) via `french_typography.rs` |
| Rôles `astro_basis` | Normalisation vers `core` / `supporting` / `nuance` (`domain_score` → `supporting`) |
| Summary | `summary.short_text` = 1–2 phrases complètes extraites du body (max 75 mots), **sans** troncature `…` |
| Sanitisation script | Retrait caractères hors alphabet latin FR (`reading_script_guard`) |
| Fallback body | Texte déterministe si contamination script persiste après retries |

OpenAPI : `contracts/calculator/openapi.yaml` (422 calculateur), `contracts/llm/openapi.yaml` (400 entrée / 422 `safety_rejected` orchestration).

## Hiérarchie scopes

```text
stable_birth_date_profile < planetary_positions < angular_chart < full_natal
```

`natal_structured_v13` reste réservé au thème complet (`full_natal`).

## Références

- Contrôles anti-hallucination : [`natal_simplified_forbidden_topics.md`](natal_simplified_forbidden_topics.md)
- Revue adversariale : [`reviews/natal_simplified/REV-011-adversarial-findings.md`](reviews/natal_simplified/REV-011-adversarial-findings.md)
- Golden fixtures : `tests/golden/simplified_natal_calculation_stable_1990-06-15.json`, `tests/golden/simplified_natal_calculation_equinox_1990-03-21.json`
