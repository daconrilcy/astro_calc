Statut: closed

Objet:
- review adversariale de la correction finale sur `features/horoscope/period.rs` et `features/simplified/service.rs`.

Perimetre:
- `astral_calculator/src/features/horoscope/period.rs`
- `astral_calculator/src/features/horoscope/mod.rs`
- `astral_calculator/src/features/simplified/service.rs`
- `tests/astral_calculator_http_tests.rs`
- `tests/refactor_governance_tests.rs`

Cycle 1 - Finding:
- F1: bien que le `panic!` runtime ait disparu, rien ne verrouillait encore le fait que les wrappers publics `calculate_horoscope_period*` restent non-panicking et retournent des erreurs controlees sur entree invalide.

Corrections:
- les wrappers publics `calculate_horoscope_period`, `calculate_horoscope_period_from_positions`, `calculate_horoscope_period_from_transits` et leurs alias legacy retournent maintenant `Result<..., RuntimeError>`;
- ajout de `horoscope_period_public_wrappers_return_error_for_invalid_request` dans `tests/astral_calculator_http_tests.rs`;
- ajout de `horoscope_public_period_api_has_no_expect_wrappers` dans `tests/refactor_governance_tests.rs`;
- suppression du lookup DB redondant `geocentric` dans `features/simplified/service.rs` au profit de `references.geocentric_coordinate_reference_system_id`.

Verification:
- `cargo test -p astral_calculator_http --test astral_calculator_http_tests`
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`
- `cargo test -p astral_calculator`

Findings restants: Aucun

Aucun finding ouvert.
