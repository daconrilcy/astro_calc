Statut: closed

Objet:
- review adversariale de la tranche 2026-06-21 qui deplace le decode
  `included_days` hors du builder horoscope et le limite au repository DB.

Perimetre:
- `astral_calculator/src/application/ports.rs`
- `astral_calculator/src/infra/db/horoscope_repository.rs`
- `astral_calculator/src/features/horoscope/builders.rs`
- `tests/refactor_governance_tests.rs`
- `docs/BASIC_PAYLOAD_IMPLEMENTATION.md`

Cycle 1 - Finding:
- F1: le typage applicatif de `included_days` supprimait bien le
  `serde_json::Value` de `HoroscopePeriodProfile`, mais sans garde-fou il
  restait possible de reintroduire un `serde_json::from_value::<Vec<String>>`
  dans le builder horoscope lors d'une evolution locale.

Correction:
- ajout d'un test de gouvernance qui interdit le decode JSON
  `serde_json::from_value::<Vec<String>>` dans
  `astral_calculator/src/features/horoscope/builders.rs`;
- conservation du decode unique au repository DB avec erreur
  `InvalidRuntimeTable` contextualisee.

Verification:
- `cargo test -p astral_calculator --test horoscope_builders_tests`
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator`

Findings restants: Aucun

Aucun finding ouvert.
