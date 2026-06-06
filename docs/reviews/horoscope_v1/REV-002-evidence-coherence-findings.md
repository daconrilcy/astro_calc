# REV-002 — Evidence coherence

## Findings

Aucun finding ouvert apres refactor.

## Verifications adversariales

- Chaque `required_evidence_key` de slot existe dans `evidence[]`.
- La reponse finale ne peut pas citer une evidence inventee.
- Une evidence d'un autre slot est rejetee sauf cas `specificity = shared`.
- Les slots sans evidence doivent rester en fallback explicite.
