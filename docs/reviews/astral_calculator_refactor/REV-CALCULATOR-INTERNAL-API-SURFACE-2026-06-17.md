# Review adversariale — surface API calculateur interne

Date : 2026-06-17

## Perimetre

Vague progressive de clarification des surfaces HTTP calculateur :

- ajout des routes canoniques `/v1/internal/calculations/*` dans
  `astral_calculator_api` ;
- conservation des aliases legacy `/v1/calculations/*` ;
- bascule du client inter-services calculateur vers les routes internes ;
- documentation de `astral_gateway` comme facade publique recommandee.

## Cycle 1 — Findings

- P2 — `contracts/calculator/openapi.yaml` documentait les nouvelles routes
  internes horoscope mais pas leurs aliases legacy `/v1/calculations/horoscope/*`
  pourtant toujours exposes.
- P2 — La couverture d'equivalence interne/legacy ne verifiait qu'une route
  (`validate`) et ne protegeait pas les quatre autres aliases de calcul.

## Corrections Cycle 1

- Ajout des deux routes legacy horoscope dans l'OpenAPI calculateur avec les
  memes schemas que les routes internes.
- Ajout d'un test d'equivalence sur les cinq paires
  `/v1/internal/calculations/*` et `/v1/calculations/*`.

## Cycle 2 — Findings

Aucun finding ouvert.

## Points verifies

- Pas de migration de code HTTP dans `astral_calculator` : le moteur reste
  decouple d'Axum et des middlewares HTTP.
- Pas de suppression de routes legacy : les scripts existants et les tests
  d'integration directe calculateur restent compatibles.
- Pas de changement Docker : le mapping `8080:8080` reste disponible pour le
  developpement local.
- Les nouveaux chemins internes reutilisent les memes handlers, schemas,
  erreurs et controles readiness/auth que les chemins existants.

## Risques residuels acceptes

- Le nom `astral_calculator_api` reste visible dans Cargo, Docker et la
  documentation pour eviter un renommage a fort blast radius.
- Les routes legacy restent exposées ; leur extinction devra etre planifiee
  dans une vague separee avec migration des scripts.
