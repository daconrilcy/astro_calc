# REV-004 — Contract and public API

## Findings

Aucun finding ouvert apres refactor.

## Verifications adversariales

- L'infrastructure async existante reste inchangée : `POST /v1/jobs`, polling,
  idempotence et worker existants.
- Le service reste en `beta`, pas en `active`.
- `horoscope_response_v1` est enrichi par ajout de champs, sans suppression des
  champs publics existants.
- Les codes techniques publics comme `[morning]` sont rejetes.
