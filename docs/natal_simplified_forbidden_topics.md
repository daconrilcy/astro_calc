# Sujets interdits et contrôles LLM — natal simplifié

## Sémantique des champs `llm_payload`

| Champ | Signification |
|-------|---------------|
| `allowed_fact_codes` | Faits interprétables comme **affirmations** rédactionnelles (`sun.sign`, …) |
| `allowed_astro_basis_fact_ids` | Valeurs **exclusivement** autorisées pour `astro_basis.fact_id` (`placement:sun`, …) |
| `blocked_interpretation_fact_codes` | Interdits comme affirmation — **pas** interdits à mentionner en contexte de limitation |
| `excluded_feature_codes` | Features absentes du **calcul** (effet des limitations / scope) |
| `profile_excluded_feature_codes` | Features **calculées mais exclues du profil** (ASC, maisons, secte, placements en maison) |
| `allowed_limitation_mentions` | Sujets autorisés pour expliquer une limite (codes limitation + features + faits bloqués) |
| `forbidden_interpretation_topics` | Agrégat dérivé (prompt / doc) — **non consommé** directement par SafetyGuard |
| `forbidden_topics` | **Déprécié** — miroir de `forbidden_interpretation_topics` (compat lecture clients legacy) |

Invariant : `blocked_interpretation_fact_codes ∩ allowed_fact_codes = ∅` et `blocked ∩ allowed_astro_basis_fact_ids = ∅`.

## Règles de génération (calculateur)

```text
allowed_fact_codes =
  facts où reliability ∈ [calculated_from_declared_datetime, stable_across_uncertainty_window]
  → format {object}.sign

allowed_astro_basis_fact_ids =
  mêmes facts stables → format placement:{object}

blocked_interpretation_fact_codes =
  tous les ambiguous_facts → format {object}.sign

excluded_feature_codes = excluded_features (calcul)

profile_excluded_feature_codes =
  chargées depuis astral_simplified_profile_feature_exclusions (DB, seed json_db/)
  V1 natal_simplified : ascendant, houses, sect, house_placements (computed_scope_code null = global)

allowed_limitation_mentions =
  blocked + excluded + profile_excluded + codes/affects des limitations
  (ex. location_provided_without_usable_timezone, local_day_window)
```

## Exemples rédactionnels

| Situation | Interdit | Autorisé |
|-----------|----------|----------|
| Lune ambiguë | « Votre Lune en Cancer indique… » | « La Lune ne peut pas être interprétée de façon fiable sans heure de naissance. » |
| Soleil ambigu (équinoxe) | « Votre Soleil en Bélier… » | « Votre Soleil se situe dans une zone de changement possible entre deux signes. » |
| Pas d'ASC (scope) | « Votre Ascendant en Scorpion… » | « L'Ascendant nécessite une heure et un lieu de naissance. » |
| ASC calculé mais profil simplified | « Ascendant en Lion… » | « L'Ascendant a été calculé mais n'est pas utilisé dans cette lecture simplifiée. » |
| Pas de maisons | « En maison 7… » | « Les placements en maisons ne sont pas disponibles avec ces données. » |
| Lieu sans timezone | — | « Le lieu a été fourni, mais sans fuseau horaire exploitable ; il ne réduit pas l'incertitude de la journée. » |

## Règles profil `natal_simplified`

1. Ne jamais affirmer l'Ascendant par signe si `ascendant` ∈ `profile_excluded_feature_codes`.
2. Ne jamais affirmer un placement en maison numérotée si `houses` / `house_placements` exclus.
3. Ne jamais interpréter un fait `blocked_interpretation_fact_codes` comme certitude.
4. Utiliser `ambiguous_core_identity` si `sun.sign` est bloqué.
5. Citer `astro_basis.fact_id` uniquement depuis `allowed_astro_basis_fact_ids`.
6. Expliquer sobrement les limitations quand `allowed_limitation_mentions` l'autorise.
7. Ne pas compenser par des généralités mystiques.

## Garde-fous post-génération (Rust)

| Module | Rôle |
|--------|------|
| `simplified_reading_postprocess` | Typographie FR, summary compact, rôles interpretatifs, disclaimer, sanitisation script, **durcissement équinoxe** (`harden_ambiguous_core_identity_chapter`) |
| `AstroBasisValidator` | Existence des `fact_id` dans les faits normalisés |
| `simplified_reading_guard` | Whitelist `astro_basis.fact_id` ; affirmations FR signes bloqués ; ASC / maison numérotée si profil exclut ; **violations ambiguous_core** |
| `SafetyGuard` | Patterns médical / légal / financier ; `forbidden_wording` (codes techniques bloqués) ; appelle `reading_script_guard` en `fr` |
| `reading_script_guard` | Rejet caractères hors Latin étendu en `fr` — **invoqué par** `SafetyGuard` |

### Durcissement équinoxe (`sun.sign` bloqué) — 3 couches

Décision produit : `confidence` clampée à **`low`** (pas `medium`).

| Couche | Mécanisme | Fichier |
|--------|-----------|---------|
| 1 — Post-traitement | Corrige code chapitre → `ambiguous_core_identity` ; force `confidence=low` ; retire `placement:sun` / `placement:moon` du basis ; préfixe incertitude si lexique absent (aligné PS1) | `simplified_reading_postprocess.rs` |
| 2 — Garde + fallback | `ambiguous_core_identity_violations` ; si violations ambiguous-only → `apply_simplified_body_fallback` + re-postprocess → `fallback_used=true` | `simplified_reading_guard.rs`, `single_pass_hardening.rs` |
| 3 — Prompt | `task_fragment` profil : confidence low obligatoire, pas sun/moon en basis, phrase d'ouverture exemple | `natal_simplified.json` |

Ordre dans `single_pass_hardening.rs` :

1. Génération LLM (+ retry script si `max_script_repair_attempts` > 1)
2. Post-traitement serveur : disclaimer, typographie FR, rôles interpretatifs, **durcissement équinoxe**, summary compact, sanitisation script
3. Fallback body déterministe si script persiste (`script_body_fallback`)
4. Fallback body déterministe équinoxe si violations ambiguous-only (`ambiguous_core_body_fallback`)
5. `AstroBasisValidator`
6. `simplified_reading_guard` (+ `ambiguous_core_identity_violations`)
7. `SafetyGuard` (+ `reading_script_guard`)
8. `ReadingQualityValidator` (non bloquant)

Recette courante : `test_natal_simplified_calculator.ps1` pour le calculateur (**12/12**) et `test_integration_jobs_e2e.ps1` pour l'orchestration async V1 `natal_simplified`. OpenAI optionnel : scripts premium courants.

### Gate qualité OpenAI (`-UseReal`)

Activée historiquement par `Assert-SimplifiedStrictOpenAiQuality` dans `scripts/lib/simplified_natal_assertions.ps1` pendant la phase de certification sync supprimée.

| Sévérité | Contrôle |
|----------|----------|
| P0 | `astro_basis.fact_id` ∈ `allowed_astro_basis_fact_ids` ; pas de `ascendant` / `house` / `sect` dans basis |
| P0 | Pas d'affirmation ASC ou maison numérotée (regex FR) |
| P0 | Si `sun.sign` bloqué → chapitre `ambiguous_core_identity` avec vocabulaire d'incertitude et **`confidence=low`** |
| P1 | Body chapitre 120–650 mots ; summary ≤75 mots, title ≤14 ; pas de `…` tronqué |
| P1 | Apostrophes FR (`l impression`, `d un`, …) ; `interpretive_role` ∈ {core, supporting, nuance} |

Recette certifiée REV-014 (2026-06-06, pré-durcissement) : **7/7** cas positifs, P0=0, P1=0. Artefacts : `output/natal_simplified_openai/2026-06-06T100348Z/`.

Recertification post-durcissement équinoxe (REV-020, 2026-06-06) : **7/7**, `gate_passed: true`, modèle `gpt-5-mini`. Artefacts : `output/natal_simplified_openai/2026-06-06T125816Z/`.

Les artefacts `quality_summary.json` mentionnés ci-dessous appartiennent à l'ancienne certification sync et sont conservés comme référence historique uniquement.

## SafetyGuard vs prompt

| Mécanisme | Source | Rôle |
|-----------|--------|------|
| Prompt (`task_instructions`, `llm_controls`) | `allowed_*`, `blocked_*`, `profile_excluded_*` | Contraintes de génération LLM |
| `forbidden_wording` (SafetyGuard) | **`blocked_interpretation_fact_codes` seulement** | Rejet substring codes techniques (`moon.sign`) |
| `simplified_reading_guard` | `blocked_*` + catalogue signes FR | Rejet « Soleil en Bélier », etc. |
| `reading_script_guard` | `language = fr` | Rejet contamination de script |

Les `excluded_feature_codes` et `profile_excluded_feature_codes` ne sont **pas** copiés dans `forbidden_wording` : des sous-chaînes comme `sect` provoqueraient des faux positifs (« section », « intersection »).

## `reference_based`

Hors `allowed_fact_codes` par défaut. Projection indicative uniquement ; pas d'affirmation interprétative.

## Tests

| Commande | Couverture |
|----------|------------|
| `cargo test -p astral_llm_application simplified_reading_guard` | Whitelist, signes bloqués FR |
| `cargo test -p astral_llm_application french_typography` | Élisions FR manquantes |
| `cargo test -p astral_llm_application simplified_reading_postprocess` | Summary compact, rôles, fallback |
| `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests` | `forbidden_interpretation_topics` + alias legacy |
| `cargo test -p astral_llm_api --test astral_llm_simplified_reading_tests` | Prompt, routing chapitre, golden |
| `.\scripts\test_natal_simplified_calculator.ps1` | 12 calculateur (7 positifs + 5 négatifs **422**) |
| `.\scripts\test_integration_jobs_e2e.ps1` | Orchestration async V1 `natal_simplified` |

Fixtures golden :

- `tests/golden/simplified_natal_calculation_stable_1990-06-15.json` — Soleil stable
- `tests/golden/simplified_natal_calculation_equinox_1990-03-21.json` — Soleil + Lune ambigus, chapitre `ambiguous_core_identity`
