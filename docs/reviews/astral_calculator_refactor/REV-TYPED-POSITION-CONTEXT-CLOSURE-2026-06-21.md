Statut: closed

Objet:
- review adversariale de la fermeture Phase 1 du contexte de position typé.

Finding:
- Aucun finding bloquant apres correction. Le risque principal etait de
  conserver `astrology/ephemeris.rs` comme assembleur inline de `facts_json`
  et de laisser des lecteurs payload contourner la source typée.

Preuves:
- `astral_calculator/src/domain/chart_facts.rs` porte maintenant les
  constructeurs `PositionFactContext::from_calculated_position` et
  `PositionFactContext::from_angle_position`;
- `astral_calculator/src/astrology/ephemeris.rs` consomme ces constructeurs au
  lieu de construire inline les blocs `facts_json`;
- `astral_calculator/src/features/natal/payload/build/house_axes.rs` lit
  `object_context` et `angle_context` via les helpers typés de
  `ObjectPositionFact`.
- Le typage plus large de `BasicSignal.evidence` et la structure detaillee des
  sources sous `signals[].evidence.placement_context.accidental_dignity_context`
  restent explicitement différes et ne sont pas revendiques par cette fermeture
  Phase 1.

Verification:
- `cargo test -p astral_calculator --test position_fact_context_tests`
- `cargo test -p astral_calculator --test payload_shared_characterization_tests`
- `cargo test -p astral_calculator --test payload_tests`
- `cargo test -p astral_calculator --test runtime_tests`

Findings restants: Aucun

Aucun finding ouvert.
