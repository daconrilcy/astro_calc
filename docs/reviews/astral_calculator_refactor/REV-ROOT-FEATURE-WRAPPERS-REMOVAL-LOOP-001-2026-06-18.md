# Review adversariale loop 001 - suppression wrappers racine features

Statut: closed

Perimetre audite:
- Diff de suppression des modules racine `natal`, `simplified`, `horoscope`.
- Garde-fous de `tests/refactor_governance_tests.rs`.
- Documentation `docs/BASIC_PAYLOAD_IMPLEMENTATION.md`.

Findings:
- F-001: le test `removed_root_feature_modules_do_not_reappear` interdisait les
  dossiers `src/{natal,simplified,horoscope}`, mais pas les fichiers modules
  equivalents `src/{natal,simplified,horoscope}.rs`.
  Correction: verification ajoutee pour les fichiers modules racine.
- F-002: le test `removed_natal_astrology_wrappers_do_not_reappear` interdisait
  seulement `features/natal/{aspects,ephemeris}.rs`, mais pas une
  reintroduction sous forme de dossier `features/natal/{aspects,ephemeris}/`.
  Correction: verification ajoutee pour les dossiers modules.
- F-003: la review principale de suppression des wrappers n'etait pas verrouillee
  par un test de gouvernance.
  Correction: ajout du test `root_feature_wrapper_removal_review_is_closed`.
- F-004: la documentation conservait une mention litterale des anciens imports
  racine, ce qui brouillait les recherches d'audit textuel.
  Correction: reformulation sans ancien chemin exact.

Findings restants: Aucun.

Verification attendue:
- `cargo fmt`
- `cargo test -p astral_calculator --test refactor_governance_tests`
- recherches `rg` sur les anciens chemins racine et wrappers natal.
