# REV-W02 plan

- Statut: closed
- Perimetre: extraction du moteur d'ephemerides vers `astral_calculator/src/astrology/ephemeris.rs`.
- Invariants:
  - `EphemerisEngine` et `SwissEphemerisEngine` vivent sous `astrology/ephemeris.rs`;
  - `calculate_chart` est la methode canonique;
  - `calculate_natal` reste seulement un wrapper compatible du trait;
  - `natal/ephemeris.rs` reste seulement un wrapper de compatibilite;
  - aucune ouverture DB ni logique repository dans `astrology`.
- Verification:
  - `cargo test -p astral_calculator --test refactor_governance_tests`
  - `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`
- Findings restants: Aucun.
