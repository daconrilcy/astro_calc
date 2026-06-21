Statut: closed

Objet:
- review frontieres du split `runtime_queries/reference/systems.rs`.

Frontieres revues:
- tout le SQL reste sous `astral_calculator/src/infra/db/runtime_queries`;
- aucune couche applicative ou feature produit n'importe le nouveau module;
- la responsabilite extraite est une capacite infra existante, pas une
  abstraction speculative;
- les repositories gardent leurs methodes actuelles et les callers restent
  source-compatibles.

Verification:
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator`

Findings restants: Aucun

Aucun finding ouvert.
