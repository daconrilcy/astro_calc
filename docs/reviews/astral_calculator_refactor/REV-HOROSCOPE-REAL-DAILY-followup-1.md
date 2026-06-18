# REV-HOROSCOPE-REAL-DAILY follow-up 1

Statut: closed

## Findings

- Medium: quand un slot daily recevait des positions Swiss Ephemeris mais pas l'objet préféré par l'index du slot, le calcul gardait la source `swisseph_daily_calculator_v1` tout en retombant sur une longitude dérivée.

## Corrections

- Ajout de `preferred_transit_position` dans `astrology::transits`.
- `daily.rs` utilise maintenant une position transitante standard réellement disponible avant de recourir au fallback dérivé.
- Ajout du test `horoscope_daily_with_transits_uses_available_real_position_when_preferred_object_is_missing`.

## Re-review

Aucun finding ouvert.
