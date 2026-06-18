# REV-ASTROLOGY-TRANSITS follow-up 1

Statut: closed

## Findings

- High: `nearest_major_transit_match` bornait les transits seulement avec l'orbe global horoscope et ne respectait pas l'orbe canonique fourni par `AspectDefinition`.

## Corrections

- `astrology::transits` utilise maintenant `canonical_aspect_orb_deg` pour borner chaque aspect par son référentiel quand il est fourni.
- L'orbe effectif est `min(orbe référentiel, orbe global)` lorsque les deux existent.
- Ajout du test `horoscope_period_calculator_respects_reference_aspect_orbs_when_supplied`.

## Re-review

Aucun finding ouvert.
