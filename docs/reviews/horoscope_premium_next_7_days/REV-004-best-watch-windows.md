# REV-004 - Best/watch windows

## Contexte

Les fenetres sont la valeur centrale du Premium.

## Checks

- `best_windows` est non vide.
- `watch_windows` est non vide ou `watch_summary.status = none`.
- Aucune fenetre n'est a la fois best et watch.
- Chaque `best_window` / `watch_window` reference au moins un `source_snapshot_key` existant dans `scan_plan.snapshots`.
- Chaque fenetre porte des `evidence_keys` valides.

## Findings

- P1 : les schemas bloquaient les evidence vides avant le guard metier.
- P1 : risque de repair inventant une fenetre provider.
- P1 : le repair pouvait reconstituer des fenetres manquantes depuis la requete d'interpretation, ce qui masquait un writer Premium incomplet.

## Corrections

- Guard dedie `HOROSCOPE_PERIOD_PREMIUM_WINDOW_EVIDENCE_MISSING`.
- Repair borne aux fenetres deja presentes dans la reponse provider et deja referencees dans la requete d'interpretation.
- Suppression du fallback qui remplissait `best_windows` / `watch_windows` quand le provider les omettait.
- Tests ajoutes pour snapshots, overlap, outside period, repair sans invention et fenetres manquantes non reparees.

## Statut

Closed - aucun P0/P1 ouvert.
