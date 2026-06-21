Statut: closed

Objet:
- review adversariale de la Phase 1 du plan `plan-1782060238` qui converge les
  preloads ephemeris non natals derriere un seam applicatif partage.

Perimetre:
- `astral_calculator/src/application/mod.rs`
- `astral_calculator/src/application/chart_context.rs`
- `astral_calculator/src/features/simplified/service.rs`
- `astral_calculator/src/features/horoscope/application/horoscope_service.rs`
- `tests/calculation_reference_loader_tests.rs`
- `docs/BASIC_PAYLOAD_IMPLEMENTATION.md`

Cycle 1 - Finding:
- F1: le diff fermait bien l'invariant code "plus de preload manuel dans
  simplified/horoscope", mais il manquait encore les artefacts de gouvernance
  requis par `AGENTS.md` pour declarer la vague fermee. Risque: tranche
  techniquement valide mais non tracable dans la documentation refactor.

Correction:
- ajout de l'entree dediee dans `docs/BASIC_PAYLOAD_IMPLEMENTATION.md`;
- ajout du present artefact dans
  `docs/reviews/astral_calculator_refactor/`;
- ajout de l'artefact jumeau oriente frontieres dans
  `docs/reviews/astral_calculator_refactor_feature_boundaries/`;
- mise a jour des railguards pour rappeler que la sous-vague chart-context du
  2026-06-21 n'est fermee qu'avec cette double trace.

Verification:
- `cargo test -p astral_calculator --test calculation_reference_loader_tests`
- `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`
- `cargo test -p astral_calculator --test runtime_tests`
- `cargo test -p astral_calculator`

Findings restants: Aucun

Aucun finding ouvert.
