Statut: closed

Objet:
- review adversariale de la finalisation de la vague ports applicatifs de references, apres suppression des derniers usages du trait composite `ReferenceCatalog` dans les services `engine`, `horoscope` et `simplified`.

Perimetre:
- `astral_calculator/src/engine/application/runtime_facade_service.rs`
- `astral_calculator/src/features/horoscope/application/horoscope_service.rs`
- `astral_calculator/src/features/simplified/application/simplified_natal_service.rs`
- `astral_calculator/src/features/simplified/service.rs`
- `tests/refactor_governance_tests.rs`
- `BASIC_PAYLOAD_IMPLEMENTATION.md`

Cycle 1 - Finding:
- F1: la finalisation de cette sous-vague modifiait des frontieres applicatives sans produire les artefacts de review adversariale fermes requis par le processus. Risque: la vague serait techniquement en place mais non tracable ni verifiee dans la gouvernance de refacto.

Correction:
- ajout des reviews dediees dans `docs/reviews/astral_calculator_refactor/` et `docs/reviews/astral_calculator_refactor_feature_boundaries/`;
- ajout d'une garde dans `tests/refactor_governance_tests.rs` pour exiger la presence et le statut ferme de ces artefacts.

Verification:
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`

Findings restants: Aucun

Aucun finding ouvert.
