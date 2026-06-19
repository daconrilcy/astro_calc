Statut: closed

Objet:
- review adversariale orientee frontieres pour la correction finale des wrappers publics horoscope period et de la duplication `geocentric` cote simplified.

Constats revus:
- l'API publique pure `horoscope period` ne doit plus exposer de chemin panic/expect;
- `simplified` ne doit pas refaire un lookup de reference quand l'id canonique est deja charge par le chargeur applicatif.

Cycle 1 - Finding:
- F1: absence de garde de gouvernance explicite contre le retour d'un `expect(...)` dans le chemin public `features/horoscope/period.rs`.

Corrections:
- wrappers publics `period` passes en `Result`;
- garde `horoscope_public_period_api_has_no_expect_wrappers`;
- suppression de la duplication `coordinate_reference_system_id_by_key(\"geocentric\")` dans `simplified/service.rs`.

Verification:
- `cargo test -p astral_calculator_http --test astral_calculator_http_tests`
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`

Findings restants: Aucun

Aucun finding ouvert.
