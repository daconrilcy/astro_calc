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

## Cycle 4 - smoke runtime full natal

### Finding F5

Le runtime full natal Free a echoue sur
`missing projection label definition for family 'axis_balance' and code
'secondary_house_dominant'`. La table generique etait bien chargee, mais ses
codes ne correspondaient pas aux valeurs canoniques emises par
`house_axes.polarity_balance`.

### Correction

- remplacement des labels `axis_balance` seedes par les codes runtime
  `primary_house_dominant`, `secondary_house_dominant` et `balanced_axis`;
- maintien des textes publics existants;
- ajout d'une regression de seed pour verrouiller ces codes.

### Re-review

Aucun finding supplementaire.

Verification:
- `cargo test -p astral_calculator --test projection_label_catalog_tests`
- `cargo test -p astral_calculator --test engine_contract_tests llm_projection_humanizes_axis_supporting_factors`

## Conclusion runtime

Aucun finding ouvert.

## Cycle 5 - projection axis_balance secondaire

### Finding F6

Le nouveau run full natal Free ne fuitait plus le code
`secondary_house_dominant`, mais le label projete affichait `Mainly house 3`
alors que le resume humanise indiquait correctement une activation principale
par la maison 9. Le template DB-backed de `secondary_house_dominant` utilisait
`{secondary_house}` alors que `BasicHouseAxisEmphasis.primary_house` porte deja
la maison dominante calculee.

### Correction

- correction du seed `astral_projection_label_definitions.axis_balance` pour
  rendre `secondary_house_dominant` avec `{primary_house}`;
- synchronisation du `test_catalog()`;
- ajout d'une regression projection verifiant que le label `balance` et le
  resume restent alignes sur le cas secondaire dominant.

### Re-review

Aucun finding supplementaire.

Verification:
- `cargo test -p astral_calculator --test projection_label_catalog_tests`
- `cargo test -p astral_calculator --test engine_contract_tests llm_projection_secondary_axis_balance_matches_summary_house`

## Conclusion post-smoke

Aucun finding ouvert.
