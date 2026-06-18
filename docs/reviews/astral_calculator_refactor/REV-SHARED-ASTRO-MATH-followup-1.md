# REV-SHARED-ASTRO-MATH follow-up 1

Statut: closed

## Scope

Re-review du finding restant: IDs canoniques de mouvement codés dans `shared::astro_math`.

## Findings initiaux

- Medium: `shared::astro_math::motion_state_id` retournait les IDs `1`, `2`, `3` pour les états de mouvement.

## Corrections

- Suppression de `motion_state_id` depuis `shared::astro_math`.
- Ajout de `astrology::motion::motion_state_for_speed`, qui résout l'état par code depuis les références runtime chargées en DB.
- Ajout d'un test de gouvernance interdisant les IDs de mouvement codés dans `shared::astro_math`.

## Re-review

Aucun finding ouvert.
