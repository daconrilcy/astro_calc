# REV-007 - Non-regression Basic period

## Contexte

Le Premium reutilise le moteur period existant.

## Checks

- Basic conserve `daily_noon_7_days`.
- Basic conserve 7 snapshots et 7 timeline entries.
- Basic n'exige pas `best_windows`, `watch_windows` ou `strategy`.
- Daily Free/Basic/Premium conserve 0/3/12 slots publics.

## Findings

- P1 : `max_key_days` Basic avait ete elargi par erreur.

## Corrections

- `basic_standard.max_key_days` conserve a 2.
- Suite `horoscope_tests` complete passee.

## Statut

Closed - aucun P0/P1 ouvert.
