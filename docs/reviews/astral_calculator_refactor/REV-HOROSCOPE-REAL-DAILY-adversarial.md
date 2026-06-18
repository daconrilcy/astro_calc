# REV-HOROSCOPE-REAL-DAILY

Statut: closed

## Scope

Audit adversarial de la vague retirant les sources fake du calcul horoscope daily runtime.

## Findings initiaux

- High: `calculate_horoscope_daily` exposait `fake_calculator_v1` et `fake_calculator_premium_v1` comme provenance runtime.
- Medium: le service applicatif daily ne chargeait pas les positions natales persistées et ne calculait pas les transits de slots.

## Corrections

- `daily.rs` produit désormais `derived_daily_calculator_v1` pour la fonction pure et `swisseph_daily_calculator_v1` quand des transits calculés sont fournis.
- `HoroscopeService::calculate_daily` charge les positions natales, les référentiels et calcule les transits par slot via l'éphéméride.
- Un test de gouvernance interdit `fake_calculator_` dans `astral_calculator/src`.

## Re-review

Aucun finding ouvert.
