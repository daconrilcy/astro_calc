# REV-PORTS-BUILDERS-FAILFAST-2026-06-19 follow-up 2

Statut: closed

## Scope

Re-review du comportement de test DB côté `engine_contract_tests` après passage en fail-fast explicite.

## Findings revus

- Medium: le helper DB ne dépendait plus d'un fallback silencieux, mais son design relançait la connexion PostgreSQL à chaque test et dégradait le feedback de suite en cas d'indisponibilité DB.

## Corrections

- Les profils `llm_projection_natal_v1` sont maintenant chargés une seule fois par process de test via un cache `OnceLock` limité à la suite.
- Le message d'échec explicite reste centralisé et identique pour tous les tests dépendants.

## Verification

- `cargo test -p astral_calculator --test engine_contract_tests llm_projection_levels_share_identical_structure -- --test-threads=1`
- `cargo test -p astral_calculator --test engine_contract_tests -- --test-threads=1`

## Resultat

- La frontière applicative reste inchangée.
- Le comportement fail-fast DB reste explicite et devient également rapide à l'échelle de la suite.
- Aucun finding ouvert.
