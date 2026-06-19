Statut: closed

Perimetre: re-review adversariale des frontieres de projection et de maintenabilite.

Findings initiaux:

- Medium: la projection LLM restait materialisee quasi integralement dans `engine/projection/builder.rs`, ce qui maintenait une responsabilite unique trop large et laissait la vague 3 du plan non realisee.

Corrections:

- Decoupage physique du builder en sous-modules metier sous `engine/projection/builder/`.
- Conservation d'un point d'entree unique `build_llm_projection_natal_v1`.
- Ajout d'un garde-fou de gouvernance sur la taille/responsabilite de `builder.rs` et l'existence des sous-modules attendus.

Verification:

- `cargo check -p astral_calculator`
- `cargo test -p astral_calculator --test engine_contract_tests -- --test-threads=1`
- `cargo test -p astral_calculator --test refactor_governance_tests`

Findings restants: Aucun

Aucun finding ouvert.
