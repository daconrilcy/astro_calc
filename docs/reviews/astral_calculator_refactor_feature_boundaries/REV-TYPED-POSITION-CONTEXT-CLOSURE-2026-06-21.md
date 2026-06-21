Statut: closed

Objet:
- review frontieres de la fermeture Phase 1 du contexte de position typé.

Frontieres revues:
- la conversion `facts_json` reste sous `astral_calculator/src/domain`;
- `astrology::ephemeris` ne depend pas de `infra/db` et ne redevient pas un
  assembleur payload;
- `features/natal/payload/build/house_axes.rs` consomme des acces typed-first
  sans elargir la surface publique;
- aucun test de comportement n'est deplace hors de `tests/`.

Verification:
- `cargo test -p astral_calculator --test position_fact_context_tests`
- `cargo test -p astral_calculator --test payload_shared_characterization_tests`
- `cargo test -p astral_calculator --test payload_tests`
- `cargo test -p astral_calculator --test runtime_tests`

Findings restants: Aucun

Aucun finding ouvert.
