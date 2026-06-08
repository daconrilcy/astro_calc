# REV-003 - Events et scoring

## Contexte

Les events Premium sont construits cote application a partir des snapshots du calculateur.

## Checks

- Chaque event expose `snapshot_key`.
- Les scores restent discriminants.
- Les limites `premium_rich` pilotent main events et evidence.
- Le calculateur ne produit aucun scoring editorial.

## Findings

- P1 : les events Basic ne portaient pas la cle de snapshot.

## Corrections

- `snapshot_key` ajoute aux evidences et events period.
- Scores Premium agreges dans `premium_scores`.

## Statut

Closed - aucun P0/P1 ouvert.
