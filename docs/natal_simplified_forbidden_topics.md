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
| `forbidden_topics` | Agrégat dérivé (prompt / doc) — **non consommé** directement par SafetyGuard |

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
  ascendant, houses, sect, house_placements (choix produit natal_simplified)

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
| `simplified_reading_guard` | Whitelist `astro_basis.fact_id` ; affirmations FR signes bloqués ; ASC / maison numérotée si profil exclut |
| `SafetyGuard` | Patterns médical / légal / financier ; `forbidden_wording` (codes techniques bloqués) ; appelle `reading_script_guard` en `fr` |
| `reading_script_guard` | Rejet caractères hors Latin étendu en `fr` (ex. devanagari, bengali) — **invoqué par** `SafetyGuard`, pas en chaîne séparée |
| `AstroBasisValidator` | Existence des `fact_id` dans les faits normalisés |

Ordre dans `generate_reading_use_case` (profil `natal_simplified`, mode `single_pass`) :

1. Parse JSON LLM + normalisation `astro_basis.fact_id` (`normalize_chapter_astro_basis_fact_ids`)
2. `AstroBasisValidator` (existence des `fact_id` dans les faits normalisés)
3. `simplified_reading_guard` (whitelist + affirmations FR + profil ASC/maisons)
4. `SafetyGuard` — inclut `reading_script_guard` (script inattendu en `fr`) + patterns sensibles + `forbidden_wording`
5. `ReadingQualityValidator` — gate **non bloquante** pour ce profil (`quality.blocking_gate: false`)

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
| `cargo test -p astral_llm_api --test astral_llm_simplified_reading_tests` | Prompt, routing chapitre, golden |
| `.\scripts\test_natal_simplified_e2e.ps1` | 12 calculateur + 7 lectures |

Fixtures golden :

- `tests/golden/simplified_natal_calculation_stable_1990-06-15.json` — Soleil stable
- `tests/golden/simplified_natal_calculation_equinox_1990-03-21.json` — Soleil + Lune ambigus, chapitre `ambiguous_core_identity`
