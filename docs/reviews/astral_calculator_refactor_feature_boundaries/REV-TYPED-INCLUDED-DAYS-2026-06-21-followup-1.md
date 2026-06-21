Statut: closed

Objet:
- review frontieres du follow-up `included_days` consacre au placement des
  tests et au maintien du decode JSON au bord adaptateur.

Frontieres revues:
- `astral_calculator/src/infra/db/horoscope_repository.rs` garde le decode
  prive de `included_days` mais ne contient plus de tests inline;
- `tests/refactor_governance_tests.rs` devient le garde commun contre le retour
  de tests production sous `src`;
- `tests/refactor_governance_tests.rs` verifie aussi que le repository garde
  le decode JSON et l'erreur `InvalidRuntimeTable` contextualisee au bord
  adaptateur;
- `astral_calculator/src/features/horoscope/builders.rs` ne decode toujours pas
  `included_days` depuis JSON.

Verification:
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator --test horoscope_builders_tests`

Findings restants: Aucun

Aucun finding ouvert.
