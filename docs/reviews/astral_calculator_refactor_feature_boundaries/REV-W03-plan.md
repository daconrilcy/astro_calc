# REV-W03 plan

- Statut: closed
- Perimetre: clarification des noms internes horoscope/simplified sans changer les contrats publics.
- Invariants:
  - les champs contractuels `transits_to_natal`, `natal_house` et codes de service restent inchanges;
  - les fonctions horoscope disposent de noms canoniques neutres;
  - les anciens noms publics `*_natal` restent wrappers compatibles;
  - aucun deplacement de `horoscope/period.rs` vers `astrology/transits` dans cette vague.
- Verification:
  - `cargo test -p astral_calculator --test refactor_governance_tests`
  - `cargo test -p astral_calculator`
- Findings restants: Aucun.
