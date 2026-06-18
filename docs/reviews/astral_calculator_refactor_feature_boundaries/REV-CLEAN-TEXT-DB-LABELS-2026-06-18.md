# Review adversariale - frontieres clean_text DB-backed labels

Statut final: Aucun finding ouvert.

Frontieres verifiees:
- aucun import `domain -> infra`;
- les references SQL restent chargees via `infra/db/runtime_queries/*`;
- la projection LLM consomme des slices runtime injectes dans le contexte, sans
  acces DB direct;
- la nouvelle table generique reste limitee aux familles projection non
  couvertes par les referentiels existants.

## Cycle 1

### Finding F1

Le changement de contrat interne `RuntimeError` n'etait pas propage au bord
HTTP. La frontiere moteur -> HTTP restait incoherente malgre un coeur
metier/tests projection valides.

### Correction

- alignement de `astral_calculator_http/src/error.rs` sur le nouveau variant
  `InvalidProjectionLabelDefinition`.

## Cycle 2

### Re-review

Aucun finding supplementaire.

Boucle de validation:
- tests de contrat projection enrichis avec cas de succes et d'echec sur
  references manquantes;
- parite stricte seed `json_db/astral_projection_label_definitions.json` /
  `test_catalog()` fermee par `tests/projection_label_catalog_tests.rs`.

## Cycle 3

### Findings

- F2 - le test de parite `projection_label_catalog_tests` n'etait pas rattache
  a `astral_calculator/Cargo.toml`, donc la verification annoncee ne tournait
  pas dans les suites Cargo.
- F3 - les labels publics d'angles `The Midheaven` et `The IC` restaient dans
  le code du resolver au lieu du referentiel de projection.
- F4 - les labels publics de mouvement restaient construits par suffixe
  `" motion"` dans le code.

### Corrections

- ajout du target Cargo dedie a `tests/projection_label_catalog_tests.rs`;
- ajout de `angle_display` et `motion_display` dans
  `astral_projection_label_definitions`;
- `ProjectionTextCatalog` garde les references existantes comme source
  d'identification, puis resout la forme publique finale via les labels
  projection DB-backed.

### Re-review

Aucun finding supplementaire.

Boucle de validation:
- `cargo test -p astral_calculator --test projection_label_catalog_tests`
- `cargo test -p astral_calculator`
- `cargo test -p astral_calculator_http --test astral_calculator_http_tests`
- `cargo test -p astral_llm_api --test contracts_publish_tests`

## Cycle 4

### Finding F5

La documentation de synthese de vague gardait encore une annonce de
`prochaine vague clean_text.rs` apres la fermeture effective de cette meme
vague. La frontiere code <-> documentation etait donc propre dans le runtime,
mais pas dans la trace de livraison.

### Correction

- retrait de la section prospective obsolete dans
  `docs/BASIC_PAYLOAD_IMPLEMENTATION.md`;
- alignement de la synthese de vague sur l'etat reel du code et des reviews.

### Re-review

Aucun finding supplementaire.

## Conclusion finale

Aucun finding ouvert.
