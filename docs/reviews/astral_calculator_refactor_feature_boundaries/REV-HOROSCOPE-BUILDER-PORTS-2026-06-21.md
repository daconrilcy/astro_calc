Statut: closed

Objet:
- review frontieres de la nouvelle sous-responsabilite
  `application/ports/horoscope_builder.rs`.

Frontieres revues:
- le module extrait depend seulement de `async_trait` et de
  `crate::shared::error::RuntimeError`;
- aucune dependance `application -> features` ou `application -> infra/db`
  n'est introduite;
- les feature builders et le repository DB continuent d'implementer ou de
  consommer les ports applicatifs via la facade `application::ports`.

Verification:
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator --test horoscope_builders_tests`

Findings restants: Aucun

Aucun finding ouvert.
