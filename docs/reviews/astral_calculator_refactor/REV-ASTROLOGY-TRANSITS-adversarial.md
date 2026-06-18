# REV-ASTROLOGY-TRANSITS

Statut: closed

## Scope

Audit adversarial de l'extraction des calculs transit/aspect hors des features horoscope.

## Findings initiaux

- Medium: `features/horoscope/period.rs` recalculait localement les aspects majeurs.
- Medium: les produits daily et period risquaient de diverger sur l'orbe et la sélection du meilleur aspect.

## Corrections

- Ajout de `astrology::transits` avec sélection de l'objet transitant standard, meilleur aspect transit-vers-natal et fallback d'aspect majeur le plus proche.
- `period.rs` et `daily.rs` consomment ce module au lieu de porter leur propre nearest-major-aspect.

## Re-review

Aucun finding ouvert.
