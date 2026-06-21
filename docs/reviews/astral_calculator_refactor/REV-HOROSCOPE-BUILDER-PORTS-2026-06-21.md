Statut: closed

Objet:
- review adversariale de l'extraction des contrats horoscope-builder hors du
  hub `application/ports.rs`.

Finding:
- Aucun finding bloquant apres correction. Le risque principal verifie etait
  une extraction qui casserait les imports existants ou deplacerait trop de
  familles de ports en une seule passe.

Preuves:
- `astral_calculator/src/application/ports/horoscope_builder.rs` contient
  seulement les DTOs horoscope-builder et le trait `HoroscopeBuilderCatalog`;
- `astral_calculator/src/application/ports.rs` re-exporte ces contrats et reste
  le chemin compatible pour les consommateurs;
- `application/ports.rs` passe de 428 a 380 lignes.

Verification:
- `cargo test -p astral_calculator --test horoscope_builders_tests`
- `cargo test -p astral_calculator --test refactor_governance_tests`

Findings restants: Aucun

Aucun finding ouvert.
