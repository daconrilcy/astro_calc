# REV-W02 followup 1

- Statut: closed
- Corrections confirmees:
  - `astrology/ephemeris.rs` porte le trait et l'implementation Swiss Ephemeris;
  - `natal/ephemeris.rs` est un re-export compatible;
  - les appels internes observables utilisent `EphemerisEngine::calculate_chart`.
- Commandes de verification:
  - `cargo test -p astral_calculator --test refactor_governance_tests`
  - `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`
- Findings restants: Aucun.
