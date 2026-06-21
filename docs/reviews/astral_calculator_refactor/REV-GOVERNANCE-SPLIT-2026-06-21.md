Statut: closed

Objet:
- review adversariale de la scission des tests de gouvernance du refactor
  `astral_calculator`.

Finding:
- Aucun finding bloquant apres correction. Le risque principal etait de
  declarer la vague comme une simple maintenance sans review, alors qu'elle
  modifie la gouvernance executable et l'enregistrement Cargo des suites.

Preuves:
- `tests/refactor_governance_tests.rs` reste le garde principal des frontieres
  metier et ne redeclenche pas un monolithe unique;
- `tests/refactor_governance_runtime_tests.rs` regroupe les invariants runtime
  et facade;
- `tests/refactor_governance_review_tests.rs` regroupe les invariants de
  reviews et de consommateurs inter-crates;
- `tests/refactor_governance_support.rs` centralise uniquement les helpers de
  lecture et d'allowlist necessaires aux tests de gouvernance;
- `astral_calculator/Cargo.toml` enregistre les suites thematiques dediees;
- la vague ne change pas le runtime metier ni les contrats JSON publics.

Verification:
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator --test refactor_governance_runtime_tests`
- `cargo test -p astral_calculator --test refactor_governance_review_tests`

Findings restants: Aucun

Aucun finding ouvert.
