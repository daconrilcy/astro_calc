Statut: closed

Objet:
- review adversariale orientee frontieres pour la Phase 1 du plan
  `plan-1782060238`, qui deplace l'ownership du preload chart-context non natal
  vers `application/chart_context.rs`.

Frontieres revues:
- `src/application/chart_context.rs` est le seul seam approuve pour charger
  `reference_version_id`, `chart_objects`, `aspect_definitions`,
  `house_system` et `calculation references` des flux non natals;
- `src/features/simplified/service.rs` et
  `src/features/horoscope/application/horoscope_service.rs` restent des
  orchestrateurs produit et ne reconstituent plus localement ce preload;
- aucun import `crate::features::*` n'entre dans le seam applicatif partage;
- les validations metier specifiques restent locales aux features et ne sont pas
  absorbees dans un helper transversal trop large.

Cycle 1 - Finding:
- F1: absence d'artefacts de review fermes specifiques a cette borne
  `application` -> `features`, alors meme que la vague change une responsabilite
  de chargement partagee sensible pour les futures phases.

Correction:
- ajout des deux artefacts de review dedies a la sous-vague;
- mise a jour des railguards pour figer l'invariant "pas de preload manuel
  non natal hors `application/chart_context.rs`";
- ajout de l'entree documentaire associee dans
  `docs/BASIC_PAYLOAD_IMPLEMENTATION.md`.

Verification:
- `rg -n "active_chart_objects|aspect_definitions\\(|house_system\\(|load_(default_)?calculation_reference_data" astral_calculator/src/features/simplified/service.rs astral_calculator/src/features/horoscope/application/horoscope_service.rs`
- `cargo test -p astral_calculator --test calculation_reference_loader_tests`
- `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`
- `cargo test -p astral_calculator --test runtime_tests`
- `cargo test -p astral_calculator`

Findings restants: Aucun

Aucun finding ouvert.
