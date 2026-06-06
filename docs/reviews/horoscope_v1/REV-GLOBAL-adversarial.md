# REV-GLOBAL — Horoscope V1 slot-based

## Checklist bloquante

- [x] Tests fake horoscope passes.
- [x] Goldens slot-based mis a jour.
- [x] Guards qualite ajoutes.
- [x] Reviews adversariales creees.
- [x] Service conserve en `beta`.
- [x] Aucun Premium 2h slots, maison locale, Ascendant/MC du moment, aspect
  mineur, nouveau worker ou nouvelle table de jobs ajoute.

## Decision

Le refactor V1 est clos cote fake lorsque le smoke Docker fake passe dans
l'environnement cible.
