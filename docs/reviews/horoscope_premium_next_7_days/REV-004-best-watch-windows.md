# REV-004 - Best/watch windows

## Contexte

Les fenetres sont la valeur centrale du Premium.

## Checks

- `best_windows` est non vide.
- `watch_windows` est non vide si `watch_summary.status = active` ou `low`.
- `watch_summary.status = none` est reserve aux cas sans signal exploitable.
- Aucune fenetre n'est a la fois best et watch.
- Chaque `best_window` / `watch_window` reference au moins un `source_snapshot_key` existant dans `scan_plan.snapshots`.
- Chaque fenetre porte des `evidence_keys` valides.
- Les `best_windows` évitent un titre trop générique et différencient leurs `best_for`.

## Findings

- P1 : les schemas bloquaient les evidence vides avant le guard metier.
- P1 : risque de repair inventant une fenetre provider.
- P1 : le repair pouvait reconstituer des fenetres manquantes depuis la requete d'interpretation, ce qui masquait un writer Premium incomplet.
- P1 : Premium pouvait exposer une absence de vigilance (`none`) alors que des signaux exploitables permettaient une vigilance douce.
- P2 : les titres et `best_for` des meilleures fenetres pouvaient rester trop generiques.

## Corrections

- Guard dedie `HOROSCOPE_PERIOD_PREMIUM_WINDOW_EVIDENCE_MISSING`.
- Repair borne aux fenetres deja presentes dans la reponse provider et deja referencees dans la requete d'interpretation.
- Suppression du fallback qui remplissait `best_windows` / `watch_windows` quand le provider les omettait.
- Politique V1.1 ajoutee : construction de `watch_windows` low evidencées, non-overlap, a partir de signaux existants quand aucune tension forte ne ressort.
- Guard `HOROSCOPE_PERIOD_PREMIUM_WINDOWS_TOO_GENERIC` ajoute pour les titres et `best_for` trop indifferencies.
- Tests ajoutes pour snapshots, overlap, outside period, repair sans invention et fenetres manquantes non reparees.

## Statut

Closed - aucun P0/P1 ouvert.
