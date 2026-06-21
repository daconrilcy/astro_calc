Statut: closed

Objet:
- review adversariale de la vague Phase 1 qui retire l'ownership production du
  fixture builder natal `test_catalog()` et supprime le fallback payload vers ce
  catalogue de test.

Perimetre:
- `astral_calculator/src/features/natal/catalog.rs`
- `astral_calculator/src/features/natal/payload/build/mod.rs`
- `tests/common/mod.rs`
- `tests/common/natal_catalog.rs`
- `tests/contract_basic_v8_tests.rs`
- `tests/dignities_tests.rs`
- `tests/engine_contract_tests.rs`
- `tests/natal_reuse_policy_tests.rs`
- `tests/payload_shared_characterization_tests.rs`
- `tests/payload_tests.rs`
- `tests/projection_label_catalog_tests.rs`
- `tests/runtime_tests.rs`
- `tests/signals_tests.rs`
- `docs/BASIC_PAYLOAD_IMPLEMENTATION.md`

Cycle 1 - Finding:
- F1: la vague etait techniquement fermee cote code et verification, mais elle
  ne produisait pas encore les artefacts de review adversariale fermes requis
  par le processus de refacto. Risque: slice non tracable dans la gouvernance,
  malgre la fermeture de l'invariant `src/**` ne depend plus de `test_catalog()`.

Correction:
- ajout de cet artefact de review dedie a la vague dans
  `docs/reviews/astral_calculator_refactor/`;
- ajout de l'artefact jumeau oriente frontieres dans
  `docs/reviews/astral_calculator_refactor_feature_boundaries/`;
- mise a jour des railguards `RAILGUARD.md` et `astral_calculator/RAILGUARD.md`
  pour expliciter que la vague du 2026-06-21 n'est fermee qu'avec ces deux
  traces de review.

Verification:
- `cargo test -p astral_calculator --test payload_tests`
- `cargo test -p astral_calculator --test runtime_tests`
- `cargo test -p astral_calculator --test signals_tests`
- `cargo test -p astral_calculator --test engine_contract_tests -- --test-threads=1`
- `cargo test -p astral_calculator --test deprecated_root_alias_compat_tests`
- `cargo test -p astral_calculator`

Findings restants: Aucun

Aucun finding ouvert.
