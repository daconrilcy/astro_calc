Status: `closed`

Follow-up:
- adversarial re-review after moving the `NatalReusePolicy` coverage out of `src/` and into root `tests/`.

Finding:
- the previous iteration added targeted tests inside `astral_calculator/src/features/natal/application/natal_calculation_service.rs`, which violated the workspace rule requiring tests under the root `tests/` directory.

Correction:
- removed the in-source test module;
- added [tests/natal_reuse_policy_tests.rs](../tests/natal_reuse_policy_tests.rs) covering the same four reuse-policy scenarios through `NatalCalculationService::calculate_basic_with_catalog()`.

Validation:
- `rg -n "#\\[cfg\\(test\\)\\]|#\\[test\\]|tokio::test" astral_calculator/src -S`
- `cargo test -p astral_calculator --test natal_reuse_policy_tests`
- `cargo test -p astral_calculator`
- `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`

Findings:
- Aucun finding ouvert.
