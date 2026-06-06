# REV-001 — Slot differentiation

## Findings

Aucun finding ouvert apres refactor.

## Verifications adversariales

- La shortlist n'est plus une liste globale recopiee : `slots[]` porte les
  `required_evidence_keys` par moment.
- Les trois slots du golden fake ont des themes, conseils et `best_for`
  differents.
- Le guard `HOROSCOPE_SLOT_REPETITION_FAILED` rejette les textes repetes et la
  copie de `day_overview`.
- `specificity = fallback` exige une absence d'evidence et un `fallback_reason`.
