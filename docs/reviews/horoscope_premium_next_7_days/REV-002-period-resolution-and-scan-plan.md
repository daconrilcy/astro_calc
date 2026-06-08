# REV-002 - Period resolution et scan plan

## Contexte

Le Premium doit reutiliser `astral_time_window` et ne jamais resoudre la periode dans le calculateur.

## Checks

- `period_resolution` vient de `astral_time_window`.
- `scan_plan.snapshot_count = duration_days * snapshots_per_day`.
- Premium produit 28 snapshots pour 7 jours.
- Les snapshots `00:00` locaux restent dans la periode locale meme si leur UTC tombe la veille.

## Findings

- P1 : la validation du scan supposait un snapshot par date.

## Corrections

- Validation generalisee via `expected_snapshots_per_day`.
- Tests ajoutes pour 28 snapshots et shift UTC de minuit.

## Statut

Closed - aucun P0/P1 ouvert.
