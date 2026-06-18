# Review adversariale - suppression wrappers racine features

Statut: closed

Contexte:
- Suppression volontaire des anciens modules racine
  `astral_calculator::{natal,simplified,horoscope}`.
- Suppression des wrappers internes `features/natal::{aspects,ephemeris}`.
- Les chemins canoniques sont desormais `features::*` pour les produits et
  `astrology::*` pour les calculs reutilisables.

Findings:
- F-001: risque de reintroduction des dossiers racine legacy.
  Correction: test de gouvernance `removed_root_feature_modules_do_not_reappear`.
- F-002: risque de reintroduction de wrappers astrologiques sous
  `features/natal`.
  Correction: test de gouvernance `removed_natal_astrology_wrappers_do_not_reappear`.
- F-003: risque de casser silencieusement les chemins canoniques.
  Correction: test de compilation `canonical_public_feature_paths_compile`.

Aucun finding ouvert.

Verification attendue:
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator`
- `cargo test -p astral_calculator_http --test astral_calculator_http_tests`
- `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`
