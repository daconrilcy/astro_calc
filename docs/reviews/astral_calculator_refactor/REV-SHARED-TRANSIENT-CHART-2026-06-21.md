Statut: closed

Objet:
- review adversariale de la tranche 2026-06-21 qui converge l'execution
  ephemeris transitoire non natale derriere `application/transient_chart.rs`.

Perimetre:
- `astral_calculator/src/application/mod.rs`
- `astral_calculator/src/application/transient_chart.rs`
- `astral_calculator/src/features/simplified/service.rs`
- `astral_calculator/src/features/horoscope/application/horoscope_service.rs`
- `tests/transient_chart_tests.rs`
- `tests/refactor_governance_tests.rs`
- `docs/BASIC_PAYLOAD_IMPLEMENTATION.md`

Cycle 1 - Finding:
- F1: le diff refermait la duplication de code, mais il manquait la preuve
  exécutable du seam partage et un garde-fou pour empecher le retour des appels
  directs a `EphemerisEngine::calculate_chart` dans les services non natals.
  Risque: regression silencieuse vers des boucles divergentes `simplified` /
  `horoscope`.

Correction:
- ajout d'un test dedie `tests/transient_chart_tests.rs` pour verifier que le
  seam remplace bien la date UTC cible et le `product_code` sans muter l'entree
  de base;
- ajout d'un garde-fou dans `tests/refactor_governance_tests.rs` qui impose
  l'usage de `calculate_transient_chart_facts` dans `simplified` et
  `horoscope`, et interdit les appels directs `.calculate_chart(` dans ces
  fichiers;
- mise a jour de `docs/BASIC_PAYLOAD_IMPLEMENTATION.md` pour lier la double
  trace review de cette sous-vague.

Verification:
- `cargo test -p astral_calculator --test transient_chart_tests`
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator --test runtime_tests`
- `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`

Findings restants: Aucun

Aucun finding ouvert.
