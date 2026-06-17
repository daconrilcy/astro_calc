# Review adversariale frontieres features — surface API calculateur interne

Date : 2026-06-17

## Perimetre

Verification des frontieres de features pour la vague
`/v1/internal/calculations/*`.

## Cycle 1 — Findings

- Aucun finding de frontiere metier ouvert.
- Finding transversal associe a la vague : la couverture d'equivalence de
  routes ne verifiait pas toutes les familles exposees (`natal`, `simplified`,
  `horoscope daily`, `horoscope period`).

## Corrections Cycle 1

- Ajout d'un test d'equivalence sur toutes les familles de routes HTTP
  calculateur, sans changer les frontieres metier.

## Cycle 2 — Findings

Aucun finding ouvert.

## Points verifies

- Les features produit `natal`, `simplified` et `horoscope` restent appelees
  via la facade runtime existante du calculateur.
- Les routes HTTP n'introduisent pas d'import feature-to-feature nouveau.
- Les calculs astrologiques ne sont pas deplaces dans `features/shared`.
- La nouvelle surface interne ne modifie pas les contrats JSON de sortie des
  features calculateur.
- `astral_gateway` garde l'orchestration publique metier et ne devient pas une
  dependance du calculateur.

## Risques residuels acceptes

- Les noms historiques `*_natal` des routes horoscope restent conserves pour
  compatibilite contractuelle ; un renommage metier plus large devra etre traite
  separement si necessaire.
