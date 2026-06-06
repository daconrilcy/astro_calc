# REV-004 - Best and watch slots

Status: closed.

Checks:

- `best_slots` and `watch_slots` are non-empty in Premium responses.
- A slot cannot appear in both sets in V1.

Findings:

- P1: Overlap between `best_slots` and `watch_slots` would create a public
  contradiction.
  Correction: Premium guard returns
  `HOROSCOPE_PREMIUM_CONTRADICTORY_SLOT_CLASSIFICATION` on overlap.

Result: P0/P1 fixed.
