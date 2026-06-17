# REV-W00 adversarial

- Statut: closed
- Findings initiaux:
  - P1: imports interdits `crate::natal::aspects` et `crate::natal::ephemeris` encore presents dans `simplified` et `horoscope`.
  - P2: absence de test statique verrouillant les nouveaux invariants de frontiere.
  - P2: absence de documentation explicite de la cible `astrology/` dans les regles de workspace.
- Resolution:
  - les imports interdits ont ete migres vers `crate::astrology::*`;
  - `tests/refactor_governance_tests.rs` verrouille les invariants de frontiere;
  - les regles de workspace et la documentation de refacto ont ete mises a jour.
- Findings restants: Aucun apres `REV-W00-followup-1.md`.
