Status: `closed`

Scope: adversarial re-review of the port-neutralization wave after the domain/sqlx fix.

Initial findings:

- Medium: `infra/db/runtime_queries.rs` still imported `crate::features::natal::catalog::BasicPayloadCatalog`, i.e. the compatibility wrapper path, even though the canonical type now lives in `crate::domain`. This kept new infra code coupled to a transitional alias and weakened eventual wrapper removal.
- Low: the latest domain/sqlx correction and canonical-path cleanup were not yet captured in the adversarial review trail for this wave.

Corrections:

- Switched `infra/db/runtime_queries.rs` to `crate::domain::BasicPayloadCatalog`.
- Added a governance test preventing any file under `astral_calculator/src/infra/db` from importing `crate::features::natal::catalog::BasicPayloadCatalog`.
- Recorded this review loop as a closed follow-up artifact.

Verification:

- `cargo fmt`
- `cargo check -p astral_calculator`
- `cargo test -p astral_calculator --test refactor_governance_tests`

Findings restants: Aucun

Aucun finding ouvert.
