Status: `closed`

Scope: re-review after implementation of the maintainability wave.

Initial findings:

- Medium: [astral_calculator/src/engine/projection/builder.rs](/c:/dev/astral_calculation/astral_calculator/src/engine/projection/builder.rs) still contained nearly all LLM projection assembly logic, so wave 3 of the approved refactor plan was not actually completed despite the documentation claiming closure.

Corrections:

- Split the projection builder into named submodules under `astral_calculator/src/engine/projection/builder/`:
  `chart.rs`, `reading_order.rs`, `identity.rs`, `themes.rs`, `placements.rs`, `strengths.rs`, `relationships.rs`, `house_axes.rs`, `keywords.rs`.
- Kept `builder.rs` as the orchestration entry point plus shared helpers only.
- Added a governance test preventing `builder.rs` from growing back into a monolith and requiring the split submodules to exist.

Verification:

- `cargo fmt`
- `cargo check -p astral_calculator`
- `cargo test -p astral_calculator --test engine_contract_tests -- --test-threads=1`
- `cargo test -p astral_calculator --test refactor_governance_tests`

Findings restants: Aucun

Aucun finding ouvert.
