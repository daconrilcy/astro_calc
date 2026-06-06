# REV-006 - Editorial depth and non-repetition

Status: closed.

Checks:

- Fake Premium covers summary, best slots, watch slots, timeline, domains and
  advice.
- Technical slot codes such as `slot_00_02` cannot appear in public text.
- If `location.label` is absent, the response does not invent a city.

Findings:

- P1: Existing code leak guard only covered Free/Basic slot codes.
  Correction: guard now rejects `slot_` in public horoscope text.

- P1: Optional location label could be overfilled.
  Correction: Premium period includes `location_label` only when supplied.

Result: P0/P1 fixed.
