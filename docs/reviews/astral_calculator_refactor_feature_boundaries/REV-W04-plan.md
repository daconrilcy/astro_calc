# REV-W04 plan

- Statut: closed
- Perimetre: nettoyage final de compatibilite et exposition publique.
- Decision:
  - les wrappers `natal::aspects`, `natal::ephemeris` et les fonctions horoscope `*_natal` sont conserves pour compatibilite publique;
  - la suppression est differee tant que des tests publics et consommateurs historiques les referencent;
  - `lib.rs` expose explicitement `astrology` et conserve les anciens modules de compatibilite.
- Verification:
  - `cargo test -p astral_calculator --test refactor_governance_tests`
  - `cargo test -p astral_calculator`
  - `cargo test -p astral_calculator_api --test astral_calculator_api_tests`
- Findings restants: Aucun.
