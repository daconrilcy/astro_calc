Statut: closed

Objet:
- review frontieres de la scission des tests de gouvernance du refactor
  `astral_calculator`.

Frontieres revues:
- les tests de gouvernance restent sous le repertoire racine `tests/`;
- aucun test comportemental n'est deplace dans `astral_calculator/src`;
- les nouvelles suites sont enregistrees comme targets Cargo dediees;
- l'allowlist de `tests/refactor_governance_support.rs` reste bornee aux
  references legacy et aux fichiers de gouvernance qui doivent pouvoir citer
  ces chemins;
- la vague ne modifie pas le runtime metier, les schemas DB, ni les contrats
  JSON publics.

Verification:
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator --test refactor_governance_runtime_tests`
- `cargo test -p astral_calculator --test refactor_governance_review_tests`

Findings restants: Aucun

Aucun finding ouvert.
