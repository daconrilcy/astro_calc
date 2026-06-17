# REV-b994848 - Adversarial loop

- Commit revu: `b994848 refactor(calculator): tighten boundaries and projections`
- Date: 2026-06-17
- Statut: closed

## Axes de review

- Non-regression des contrats publics apres deplacement des calculs `aspects` et `ephemeris`.
- Maintien du perimetre DB sous `infra/db`, avec orchestration runtime a la bordure.
- Absence de dependance interdite `domain -> infra`.
- Absence d'appel direct aux details internes d'une autre feature produit depuis `simplified` et `horoscope`.
- Couverture des tests de gouvernance, payload, projection, API et publication de schemas.

## Commandes executees

- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator`
- `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`
- `cargo test -p astral_calculator_api --test astral_calculator_api_tests`
- `cargo test -p astral_llm_api --test contracts_publish_tests`
- `git show --check HEAD`

## Resultat

Aucun finding ouvert.

## Decision

Pas de correction code requise dans cette boucle. La refacto du commit `b994848` est closee pour les invariants controles ici.
