# REV-A - Frontieres De Couches

- Status: `closed`
- Decision: `fix now`

## Findings

- Aucun finding bloquant sur la frontiere `domain -> infra` apres remplacement des alias SQLx par de vrais types domaine et ajout du test de gouvernance.

## Notes

- Les contrats JSON publics restent inchanges.
- Les repositories infra conservent le SQL existant, mais le typage de sortie passe par le domaine.
