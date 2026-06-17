# REV-IMPLEMENTATION-005-adversarial

- Statut: closed
- Portee auditee:
  - wrappers legacy `src/natal`, `src/simplified`, `src/horoscope`;
  - gouvernance post-deplacement physique sous `src/features`;
  - risque de derive future des anciens namespaces racine.

Findings corriges:
- F-001: les wrappers legacy racine etaient bien minimalistes dans l'implementation, mais aucun test ne garantissait qu'ils restent de purs re-exports de compatibilite. Correction: ajout du test `legacy_root_feature_modules_are_compatibility_wrappers_only`, qui interdit tout fichier Rust autre que `mod.rs` dans ces dossiers et verifie le contenu exact du re-export.

Findings restants: Aucun

Verification attendue:
- `cargo test -p astral_calculator --test refactor_governance_tests`
