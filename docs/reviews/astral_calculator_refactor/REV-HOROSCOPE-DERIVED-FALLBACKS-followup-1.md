# REV-HOROSCOPE-DERIVED-FALLBACKS follow-up 1

Statut: closed

## Scope

Re-review du finding restant: sources `derived_*` et génération synthétique silencieuse sans transits.

## Findings initiaux

- High: les helpers publics horoscope pouvaient produire des faits synthétiques avec sources `derived_daily_calculator_v1` et `derived_period_calculator_v1`.

## Corrections

- Suppression des sources `derived_*` dans `astral_calculator/src`.
- Un appel sans positions transitantes réelles retourne des slots/snapshots sans faits, avec `source: missing_transit_data` et warning explicite.
- Les tests ont été adaptés pour fournir des transits lorsqu'ils vérifient des faits réels, et pour vérifier l'absence de synthèse silencieuse sinon.
- Ajout d'un test de gouvernance interdisant les sources `derived_*`.

## Re-review

Aucun finding ouvert.
