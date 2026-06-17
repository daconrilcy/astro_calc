# Review adversariale - consommateurs internes calculateur

Date: 2026-06-17
Statut: closed

## Perimetre

Vague de migration des consommateurs internes courants depuis les routes legacy
`/v1/calculations/*` vers les routes canoniques
`/v1/internal/calculations/*`.

Hors perimetre : suppression des aliases legacy, renommage de crate/service,
changement Docker, modification des contrats JSON.

## Cycle 1

### Findings

- P2 - La migration mecanique a remplace des mentions de compatibilite dans
  `docs/GUIDE_DEBUTANT_DOCKER.md`, faisant presenter les aliases legacy comme
  `/v1/internal/calculations/*` au lieu de `/v1/calculations/*`. Cela rendait
  la documentation de transition ambigue.

### Corrections

- Les mentions de compatibilite du guide Docker ont ete restaurees vers
  `/v1/calculations/*`.
- Les consommateurs operationnels ont conserve les routes canoniques internes.
- Un test de gouvernance a ete ajoute pour interdire le retour de
  `/v1/calculations/*` dans les consommateurs internes courants, avec allowlist
  limitee aux contrats, alias, tests legacy, docs de compatibilite et reviews.

## Cycle 2

### Findings

Aucun finding ouvert.

## Conclusion

Aucun finding ouvert.
