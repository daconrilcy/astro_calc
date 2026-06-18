# REV-SHARED-ASTRO-MATH

Statut: closed

## Scope

Audit adversarial de la purification de `shared::astro_math`.

## Findings initiaux

- Medium: `shared::astro_math` importait `domain::HouseCuspFact`.

## Corrections

- Déplacement de `house_number_from_cusps` vers `astrology::house_geometry`.
- Mise à jour des imports éphémérides vers le nouveau chemin canonique.
- Ajout d'un test de gouvernance empêchant `crate::domain` et `HouseCuspFact` dans `shared::astro_math`.

## Re-review

Aucun finding ouvert.
