# Review adversariale - clean_text DB-backed labels

Statut final: Aucun finding ouvert.

Perimetre relu:
- `astral_projection_label_definitions` et son chargement runtime/catalogue;
- propagation des references runtime vers `build_engine_response` et
  `build_llm_projection_natal_v1`;
- remplacement des fallbacks metier de `clean_text.rs` par
  `ProjectionTextCatalog`;
- couverture de tests negatifs `invalid_projection_label_definition`.

## Cycle 1

### Finding F1

`astral_calculator_http` ne couvrait pas le nouveau variant
`RuntimeError::InvalidProjectionLabelDefinition`. La vague compilait et testait
le coeur `astral_calculator`, mais cassait la suite HTTP par match non
exhaustif dans `astral_calculator_http/src/error.rs`.

### Correction

- ajout du mapping HTTP de `InvalidProjectionLabelDefinition` vers
  `REFERENCE_DATA_MISSING`, aligne sur
  `InvalidProjectionReasonDefinition`.

### Verification

- `cargo test -p astral_calculator_http --test astral_calculator_http_tests`

## Cycle 2

### Re-review

Aucun finding supplementaire apres correction.

Constats verifies:
- pas de `v15` ni de changement de shape JSON public;
- les labels projection requis echouent explicitement quand une definition
  runtime manque;
- les themes/maisons/angles/mouvements/conditions consommes par la projection
  viennent bien des references runtime;
- les suites `cargo test -p astral_calculator`,
  `cargo test -p astral_calculator_http --test astral_calculator_http_tests`
  et `cargo test -p astral_llm_api --test contracts_publish_tests` restent
  vertes.

## Cycle 3

### Findings

- F2 - `tests/projection_label_catalog_tests.rs` existait mais n'etait pas
  enregistre dans `astral_calculator/Cargo.toml`; la parite stricte
  seed/catalogue n'etait donc pas executee par Cargo.
- F3 - les formes publiques d'angles `The Midheaven` et `The IC` restaient
  codees en dur dans `ProjectionTextCatalog::angle_display_label`.
- F4 - les labels de mouvement etaient encore assembles en code par suffixe
  `" motion"` dans `humanize_motion_label`.

### Corrections

- ajout du target Cargo `projection_label_catalog_tests`;
- ajout des familles `angle_display` et `motion_display` dans
  `astral_projection_label_definitions` et dans le seed strict de
  `test_catalog()`;
- `angle_display_label` et `humanize_motion_label` resolvent desormais leur
  forme publique finale via `ProjectionTextCatalog::projection_label`.

### Verification

- `cargo test -p astral_calculator --test projection_label_catalog_tests`
- `cargo test -p astral_calculator --test engine_contract_tests llm_projection_maps_jupiter_uranus_opposition`
- `cargo test -p astral_calculator --test engine_contract_tests llm_projection_conditions_exclude_redundant_direct_motion`
- `cargo test -p astral_calculator --test engine_contract_tests llm_projection_humanizes_axis_supporting_factors`
- `cargo test -p astral_calculator`
- `cargo test -p astral_calculator_http --test astral_calculator_http_tests`
- `cargo test -p astral_llm_api --test contracts_publish_tests`

## Conclusion finale

Aucun finding ouvert.
