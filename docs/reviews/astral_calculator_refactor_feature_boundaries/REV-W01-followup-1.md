# REV-W01 followup 1

- Statut: closed
- Corrections confirmees:
  - module canonique `astrology/aspects.rs` conserve;
  - wrapper compatible `natal/aspects.rs` limite a un re-export;
  - gouvernance statique active contre les imports interdits depuis `simplified` et `horoscope`.
- Commandes de verification:
  - `cargo test -p astral_calculator --test refactor_governance_tests`
  - `cargo test -p astral_calculator`
- Findings restants: Aucun.
