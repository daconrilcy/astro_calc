# Sujets interdits et contrôles LLM — natal simplifié

## Sémantique des champs

| Champ | Signification |
|-------|---------------|
| `allowed_fact_codes` | Faits interprétables comme **affirmations** astrologiques |
| `blocked_interpretation_fact_codes` | Interdits comme affirmation interprétative — **pas** interdits à mentionner en contexte de limitation |
| `excluded_feature_codes` | Features absentes du calcul (ASC, maisons, secte…) |
| `allowed_limitation_mentions` | Sujets autorisés pour expliquer une limite |

`forbidden_topics` (prompt interne) = agrégat dérivé de ces champs.

## Règles de génération (automatiques)

```text
allowed_fact_codes =
  facts où reliability ∈ [calculated_from_declared_datetime, stable_across_uncertainty_window]

blocked_interpretation_fact_codes =
  tous les ambiguous_facts (ex. moon.sign si ambigu)

excluded_feature_codes = excluded_features

allowed_limitation_mentions =
  blocked_interpretation_fact_codes + excluded_feature_codes pertinents
```

## Exemples rédactionnels

| Situation | Interdit | Autorisé |
|-----------|----------|----------|
| Lune ambiguë | « Votre Lune en Cancer indique… » | « La Lune ne peut pas être interprétée de façon fiable sans heure de naissance. » |
| Pas d'ASC | « Votre Ascendant en Scorpion… » | « L'Ascendant nécessite une heure et un lieu de naissance. » |
| Pas de maisons | « En maison 7… » | « Les placements en maisons ne sont pas disponibles avec ces données. » |

## Règles profil `natal_simplified`

1. Ne jamais mentionner l'Ascendant s'il est dans `excluded_feature_codes`.
2. Ne jamais parler de maisons / secte si exclus.
3. Ne jamais interpréter un fait `blocked_interpretation_fact_codes` comme certitude.
4. Ne pas compenser par des généralités mystiques.
5. Expliquer sobrement les limitations quand pertinent.
6. Inviter à compléter heure/lieu pour enrichir le thème.

## SafetyGuard vs prompt

| Mécanisme | Source | Rôle |
|-----------|--------|------|
| Prompt (`task_instructions`, `llm_controls`) | `allowed_*`, `blocked_*`, `excluded_*` | Contraintes de génération LLM |
| `forbidden_wording` (post-validation) | **`blocked_interpretation_fact_codes` seulement** | Rejet substring sur le texte généré |

Les `excluded_feature_codes` (`sect`, `houses`, `ascendant`…) ne sont **pas** copiés dans `forbidden_wording` : des sous-chaînes comme `sect` provoqueraient des faux positifs en français (« section », « intersection »).

## `reference_based`

Hors `allowed_fact_codes` par défaut. Projection indicative uniquement ; pas d'affirmation interprétative.
