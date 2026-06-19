Statut: closed

Perimetre: re-review adversariale des chemins canoniques apres neutralisation des ports et retrait de `sqlx` du domaine.

Findings initiaux:

- Medium: `infra/db/runtime_queries.rs` dependait encore d'un chemin de compatibilite `features::natal::catalog::BasicPayloadCatalog` au lieu du chemin canonique `domain::BasicPayloadCatalog`.
- Low: aucun garde-fou n'interdisait explicitement cette rechute dans `infra/db`.

Corrections:

- Migration de l'import vers `crate::domain::BasicPayloadCatalog`.
- Ajout d'un test de gouvernance interdisant ce chemin legacy dans `astral_calculator/src/infra/db`.
- Cloture documentee de cette boucle de review.

Verification:

- `cargo check -p astral_calculator`
- `cargo test -p astral_calculator --test refactor_governance_tests`

Findings restants: Aucun

Aucun finding ouvert.
