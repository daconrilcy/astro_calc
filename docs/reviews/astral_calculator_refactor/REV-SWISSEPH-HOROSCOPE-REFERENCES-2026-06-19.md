Statut: closed

Objet:
- review adversariale de la vague Swiss Ephemeris + horoscope period + chargeur canonique des references de calcul dans `astral_calculator`.

Perimetre revu:
- `astral_calculator/src/astrology/swisseph_runtime.rs`
- `astral_calculator/src/astrology/ephemeris.rs`
- `astral_calculator/src/application/calculation_references.rs`
- `astral_calculator/src/features/horoscope/period.rs`
- `astral_calculator/src/features/horoscope/application/horoscope_service.rs`
- `astral_calculator/src/features/simplified/service.rs`
- `astral_calculator/src/features/natal/application/natal_calculation_service.rs`

Cycle 1 - Findings:
- F1: la vague ajoutait un nouveau chargeur canonique `load_calculation_reference_data(...)` sans test cible dedie. Risque: une regression sur les ids canoniques `tropical` / `geocentric` ou sur l'assemblage des listes de references pourrait revenir sans etre detectee.
- F2: la vague centralisait bien le lock Swiss Ephemeris et supprimait le `panic!` runtime horoscope, mais aucune garde de gouvernance n'empechait la reintroduction d'un second lock local ou d'un nouveau `panic!` dans `features/horoscope`.

Corrections appliquees:
- ajout de `tests/calculation_reference_loader_tests.rs` avec doubles de ports minimaux et verification explicite des ids canoniques et des lignes chargees;
- ajout dans `tests/refactor_governance_tests.rs` de deux gardes:
  - `swiss_ephemeris_lock_is_centralized`
  - `horoscope_runtime_has_no_panic_paths`
- declaration du nouveau test dans `astral_calculator/Cargo.toml`.

Validation:
- `cargo test -p astral_calculator --test calculation_reference_loader_tests`
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator`

Findings restants: Aucun

Aucun finding ouvert.
