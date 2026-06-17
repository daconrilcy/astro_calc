# REV-W00 followup 1

- Statut: closed
- Corrections appliquees:
  - creation de `astral_calculator/src/astrology/{mod,aspects,ephemeris}.rs`;
  - conservation des chemins publics historiques via wrappers `natal::aspects` et `natal::ephemeris`;
  - migration des nouveaux appels internes vers `crate::astrology::*` et `calculate_chart`;
  - ajout de tests de gouvernance dans `tests/refactor_governance_tests.rs`;
  - mise a jour des regles dans `AGENTS.md`.
- Findings restants: Aucun.
