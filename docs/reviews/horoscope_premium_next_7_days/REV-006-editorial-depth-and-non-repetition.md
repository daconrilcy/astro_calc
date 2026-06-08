# REV-006 - Profondeur editoriale et non-repetition

## Contexte

Premium ne doit pas etre un Basic allonge.

## Checks

- Prompt Premium demande fenetres, strategie et pilotage temporel.
- Fake writer couvre toutes les sections Premium.
- Les guards existants de repetition, phrase tronquee, codes techniques et hints internes restent actifs.

## Findings

- P1 : absence initiale de critere explicite "Premium != Basic allonge".

## Corrections

- Guard et test `horoscope_premium_next_7_days_is_not_basic_shape_with_more_words`.

## Statut

Closed - aucun P0/P1 ouvert.
