# REV-IMPLEMENTATION-006-adversarial

- Statut: closed
- Portee auditee:
  - compatibilite publique des anciens chemins `astral_calculator::{natal,simplified,horoscope}`;
  - garanties de compilation apres remplacement des modules racine par des wrappers.

Findings corriges:
- F-001: les tests validaient les chemins canoniques `astral_calculator::features::*`, mais ne prouvaient pas explicitement que les anciens chemins publics continuaient a compiler apres le deplacement physique. Correction: ajout du test `legacy_public_feature_paths_still_compile`.

Findings restants: Aucun

Verification attendue:
- `cargo test -p astral_calculator --test refactor_governance_tests`
