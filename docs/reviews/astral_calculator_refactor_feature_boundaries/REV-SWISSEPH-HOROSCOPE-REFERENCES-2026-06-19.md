Statut: closed

Objet:
- review adversariale orientee frontieres de couche pour la vague Swiss Ephemeris + horoscope period + references de calcul.

Frontieres verifiees:
- `application/` reste neutre et n'importe pas `crate::features::*`;
- `astrology/` porte la coordination Swiss Ephemeris partagee;
- `features/natal`, `features/simplified`, `features/horoscope` consomment le chargeur applicatif canonique au lieu de reconstruire localement `CalculationReferenceData`;
- aucun retour de `panic!` dans les chemins runtime horoscope.

Cycle 1 - Findings:
- F1: absence de test explicite sur le nouveau point d'entree applicatif `load_calculation_reference_data(...)`, ce qui laissait un angle mort sur la resolution DB par codes canoniques.
- F2: absence de garde de gouvernance sur les deux invariants de frontiere introduits par cette vague:
  - un seul lock Swiss Ephemeris defini sous `astrology/`;
  - aucun `panic!` dans `features/horoscope`.

Corrections:
- ajout de `tests/calculation_reference_loader_tests.rs`;
- ajout des checks de gouvernance `swiss_ephemeris_lock_is_centralized` et `horoscope_runtime_has_no_panic_paths`.

Verification:
- `cargo test -p astral_calculator --test calculation_reference_loader_tests`
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator`

Findings restants: Aucun

Aucun finding ouvert.
