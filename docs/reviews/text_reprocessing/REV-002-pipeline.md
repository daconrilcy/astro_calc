# REV-002 - Pipeline

## Perimetre

`TextRetreatmentPipeline`, registries et trait `TextRetreatmentProcessor`.

## Findings adversariales

- Aucun P0/P1/P2/P3 ouvert.
- Note: l'ordre par defaut des processors est explicite et teste par idempotence. Un registre externe pourra le remplacer lors du branchement futur.

## Corrections appliquees

- Les processors generiques supportent les langues/services extensibles via listes de support ouvertes.
- Les processors non applicables produisent un audit `skipped`.
- Test d'idempotence ajoute.
