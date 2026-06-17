# REV-W03 followup 1

- Statut: closed
- Corrections appliquees:
  - ajout des noms canoniques neutres pour les fonctions horoscope;
  - conservation des wrappers publics historiques;
  - mise a jour des appels internes dans `horoscope/application/horoscope_service.rs`.
- Commandes de verification:
  - `cargo test -p astral_calculator --test refactor_governance_tests`
  - `cargo test -p astral_calculator`
- Findings restants: Aucun.
