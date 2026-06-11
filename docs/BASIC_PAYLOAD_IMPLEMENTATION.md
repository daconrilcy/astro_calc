# Implementation du payload moteur route basic

## Workspace Cargo

Le depot est un workspace (`Cargo.toml` racine) avec les crates :

- `astral_calculator` : moteur de calcul, payloads, engine 4A, projection LLM
  (construction JSON) ;
- `astral_llm` : gateway LLM astrologique (service HTTP independant du moteur).
  Crates : `astral_llm_domain`, `astral_llm_application`, `astral_llm_providers`,
  `astral_llm_infra`, `astral_llm_api`. Voir `Astral_llm_implementation.md`.

Commandes depuis la racine : `cargo run -p astral_calculator`, `cargo test -p
astral_calculator`, `cargo run -p astral_llm_api`, `cargo test -p astral_llm_api
--test astral_llm_tests`. Tests d'integration dans `tests/` (racine depot).

Ce document decrit l'implementation actuelle du payload moteur route par
`product_code = "basic"` dans le binaire Rust `astral_calculator`.

## Note horoscope

- 2026-06-11 : le pipeline horoscope period `next_7_days_natal` introduit `semantic_brief_v2` comme chemin actif uniquement pour `horoscope_premium_next_7_days_natal`. Free et basic 7 days restent en `legacy_v1` selon le brief initial Premium-only. Rust calcule, score et structure les faits, le LLM redige la sortie publique `horoscope_period_response_v1`, et le postprocess V2 reste limite au nettoyage technique. La review adversariale a verrouille le contrat Premium-only, les cles exactes du brief, les materiaux de periode pour `week_overview`, les fenetres atomiques, `evidence` top-level uniquement, `astrologer_persona` toujours present et nullable, la compat langue, l'absence de fallback langue dans le prompt V2, le retry qualite cible et l'absence de prose publique ajoutee par Rust en V2. `semantic_brief_v2` est un input de redaction interne, jamais un contrat UI ; l'UI consomme uniquement `$.result.reading`. Suivi detaille : `docs/horoscope_period_v2_migration.md`.

Le cadrage du futur module horoscope est documente dans
[`docs/HOROSCOPE_IMPLEMENTATION.md`](HOROSCOPE_IMPLEMENTATION.md). Les
developpements horoscope doivent s'y referer pour l'architecture, les contrats,
le scoring, l'orchestration async et les railguards. Ce document Basic ne porte
que l'articulation historique avec le payload moteur natal/basic.

Le service period `horoscope_basic_next_7_days_natal` est egalement documente
dans `docs/HOROSCOPE_IMPLEMENTATION.md`; ce fichier ne duplique pas son contrat.

Le service `horoscope_free_next_7_days_natal` est une projection Free compacte
du meme moteur period. Il est `active`, utilise `next_7_days`,
`free_compact` et `daily_noon_7_days`, puis publie uniquement `summary`,
`dominant_theme`, `key_days` (libelle front "Jours a retenir"), `advice`,
`watch_summary`, `evidence_summary` et `quality`. Il n'expose jamais
`daily_timeline`, `best_days`, `watch_days`, windows, `domain_sections` ou
`strategy`, et son payload d'interpretation ne transmet pas de `daily_plans`
recopiables au writer. Les tests sont regroupes dans
`scripts/test_horoscope_free_next_7_days_fake.ps1`, inclus dans
`scripts/test_horoscope_period_all.ps1`, avec goldens sous `tests/golden/` et
reviews sous `docs/reviews/horoscope_free_next_7_days/`.
Les garde-fous Free imposent une sortie publique compacte entre 140 et 450 mots
sur plusieurs profils de calcul, enrichissent `watch_summary.status = "none"`
avec une marge d'observation concrete sans `evidence_keys`, et refusent les
`key_days` qui reprennent le vocabulaire Premium de meilleurs jours, fenetres ou
creneaux favorables. Les `key_days` restent donc des reperes utiles, avec une
raison suffisamment explicite, sans devenir des `best_days` deguises.

Le reprocessing centralise des reponses horoscope daily conserve les formes de
contrat par service. Le fallback `advice` a la racine est reserve aux payloads
daily compacts sans `slots`, `timeline`, `best_slots` ou `watch_slots`; il ne
doit pas etre ajoute au service Basic
`horoscope_basic_daily_natal_3_slots`, qui expose les conseils uniquement dans
`slots[]`. Les champs techniques de preuve (`*_key`, `*_keys`) sont exclus des
normalisations typographiques afin de conserver les valeurs exactes attendues par
les gardes d'evidence. Les tests
`text_reprocessing_horoscope_basic_daily_does_not_add_root_advice`
et `text_reprocessing_public_text_processors_preserve_technical_fields`
verrouillent cette non-regression.

La version Premium period `horoscope_premium_next_7_days_natal` est une extension
du flux horoscope period, pas du payload route basic natal historique. Elle
reutilise `horoscope_period_natal_request_v1`, impose ses profils depuis le
catalogue (`premium_rich`, `six_hour_7_days`) et ajoute windows/strategy dans la
couche application. Le calculateur continue de produire uniquement des faits.
Depuis le durcissement real E2E, ce service exige des champs UTC normalises,
refuse les sources/provider fake dans le script reel et expose des libelles
publics francais au lieu des `theme_code` internes. Les tonalites publiques
viennent exclusivement des labels actifs `horoscope_tone_labels` et sont
reinjectees depuis les `daily_plans`, sans conserver les tons inventes par le
provider. Les aspects period nommes sont bornes par le referentiel
`horoscope_orb_weight_bands`; un aspect trop large devient un fait de contexte
non aspecte. En E2E reel, la longueur publique respecte les bornes
`target_words_min`, `target_words_max` et `hard_limit_words` du profil
`basic_standard`, avec post-traitement de complementation ou condensation avant
rejet. Le payload period interne utilise des scores discriminants, des tonalites
diversifiees par evenement, des `key_days` limites aux pics nets, des
`best_days` qualitativement distincts, des `watch_days` uniquement lorsqu'une
vraie tension existe, et `watch_summary.status = "none"` quand aucun point de
vigilance ne ressort. Pour Premium period V1.1, une absence de tension forte
mais avec signaux exploitables produit une vigilance douce `status = "low"` et
des `watch_windows` evidencées ; `best_days` peut rester a deux dates si aucune
troisieme date suffisamment nette ne ressort. Les preuves period portent des hints de personnalisation
natale issus de `horoscope_natal_focus_labels`, les domaines publics couvrent 2
a 4 themes scores, les `daily_plans` portent une variation lexicale, les faits
de contexte ont des orbes nulles et `fallback_reason: null` hors fallback
explicite.
Les hints internes de personnalisation period (`summary_hint`, `advice_hint`,
`personalization_hint`, `natal_focus_hint`) ne sont pas du texte public : les
post-traitements les transforment en prose utilisateur et les guards refusent
toute fuite d'instruction interne ou phrase tronquee.
La sortie period publique refuse aussi le meta-discours de personnalisation
(`conseil generique`, `cette nuance reste liee`, `la lecture relie`,
`zones natales activees`, `le point d'appui concerne`, etc.) et les deux-points
sans espacement francais.

## Objectif

Etat courant au 2026-06-04 : le moteur Rust reste dans le perimetre du calcul
astrologique et des cles d'interpretation. Le payload route basic expose les
faits calcules, les contextes astrologiques, les dignites essentielles et
accidentelles (MVP), les orbes d'aspects majeurs canoniques (3F), les angles,
les dominantes, le contexte de rulership, les signaux actifs et `reading_plan`.

Les instructions destinees a un LLM sont hors perimetre et ne sont plus produites
dans le JSON de sortie. Cela inclut `llm_handoff_contract`, `drafting_plan`,
`writing_contract`, les champs publics `writing_guidance` et les
`aspect_context.writing_guidance`. Les schemas, goldens, scripts de verification
et validations runtime ont ete alignes sur ce contrat allege.

Les anciennes sections qui mentionnent `drafting_plan`, `llm_handoff_contract`
ou `writing_guidance` doivent etre lues comme historique de conception remplace
par cette mise a jour. La couche qui gere le LLM est desormais responsable de la
langue cible, du ton, du format de sortie, des consignes de redaction et des
regles d'evitement.

L'etape 1A a transforme le payload technique initial en payload moteur
exploitable par une couche applicative separee.

L'etape 1B enrichit le payload avec des signaux semantiques Basic : themes, tags,
poids de source et premiers signaux agreges. Le runtime ne produit pas une
interpretation finale.

L'etape 1C ajoute une deduplication de signaux et un plan de lecture Basic. Quand
un cluster actif represente deja plusieurs placements, ses sources secondaires
sont persistees en `merged` au lieu de remonter comme signaux actifs autonomes.
Le payload final expose aussi `reading_plan`, une sequence de slots qui regroupe
les signaux actifs sans imposer de consigne de redaction.

L'etape 2A enrichit les placements sans transformer le payload en dump de faits
runtime.
Chaque position conserve son role de preuve structuree, mais elle porte
maintenant le contexte utile du signe, de la maison, de l'objet et du mouvement.
Ces contextes alimentent aussi les signaux de placement, les tags semantiques et
une legere ponderation de priorite.

L'etape 2B ajoute un moteur MVP de dignites essentielles. Le payload expose une
liste top-level `dignities`, les positions concernees portent un
`dignity_context`, les placements actifs sont enrichis par leurs dignites et des
signaux `dignity:*` sont produits uniquement pour les dignites majeures
significatives. Les etats retrogrades gardent leur role de contexte de
placement, mais les consignes redactionnelles publiques ont ete retirees du
payload moteur courant.

L'etape 2C ajoute une couche interpretative controlee aux aspects. Les signaux
`aspect:*` ne portent plus seulement la geometrie, l'orbe, la phase et la force :
ils exposent aussi `aspect_context`, construit depuis `astral_aspect_profiles`,
`astral_aspect_interpretive_effects` et `astral_interpretive_valence`. Cette
couche separe la valence principale (`primary_valence`) des modificateurs
d'intensite (`intensity_modifier`) comme `amplifying`, ajoute une qualite
dynamique (`flow`, `tension`, `intensification`, etc.) et enrichit les tags et
les indices interpretatifs non prescriptifs.

L'etape 2D ajoute une synthese top-level `chart_emphasis` qui classe les
dominantes de signe, de maison et d'objet a partir des placements, clusters,
dignites et aspects forts deja calcules. Elle fournit une hierarchie quantifiee
et auditable avant toute synthese externe.

L'etape 2D.1 projetait historiquement cette synthese dans un plan de redaction.
Dans le payload moteur courant, les dominantes restent disponibles dans
`chart_emphasis` et peuvent etre referencees par une couche LLM externe, sans
champ de consigne redactionnelle produit par le runtime Rust.

L'etape 2C.1 enrichit ensuite `interpretive_hint` des aspects avec cette meme
couche interpretative. Le hint reste court et template, mais il ne dit plus
seulement que deux objets sont connectes par un aspect : il nomme l'effet
lisible de l'aspect, par exemple un flux de soutien, une tension active, une
polarite a equilibrer ou un contact amplifiant. Si une valence principale et un
modificateur d'intensite coexistent, le hint conserve la valence comme lecture
principale et ajoute l'intensification comme nuance.

L'etape 2E ajoute les quatre angles natals Basic : Ascendant, Descendant,
Midheaven / MC et IC. Le runtime les calcule depuis Swiss Ephemeris, les expose
comme faits structures top-level `angles`, genere des signaux `angle:*`, ajoute
l'Ascendant au slot `core_identity` et utilise le MC comme facteur de contexte
public plus secondaire. Les metadonnees d'angle viennent de `astral_angle_points`
et des objets `astral_chart_objects` associes, pas d'un mapping code.

L'etape 2E.1 retire le mapping de themes de maisons code en dur. Les
`theme_code` des maisons sont maintenant portes par `astral_houses.theme_code`,
relus dans `HouseReference`, ajoutes a `house_context` et utilises par les
signaux, les tags et `chart_emphasis.dominant_houses`.

L'etape 2E.2 traite les axes Ascendant-Descendant et MC-IC comme des axes
structurels, pas comme des aspects interpretatifs ordinaires. La detection
d'aspects ignore maintenant une opposition lorsque les deux positions portent
un `angle_context` reciproque sur le meme axe. L'agregation Basic ignore aussi
les aspects deja marques `is_structural_axis` pour eviter de regenerer un
signal `aspect:*` actif depuis une source d'audit. Enfin, le payload exclut ces
anciens signaux du slot `main_tension_or_support` et de la raison
`strong_aspect_participant`, afin que l'axe natal n'ecrase pas les vrais aspects
dynamiques du theme.

La revue adversariale de 2E.2 a corrige deux regressions observees sur un
payload produit par l'application : les codes courts `asc` / `dsc` sont resolus
vers les codes objets longs dans `angles[].opposite_angle_code`, et les anciens
signaux d'axe non marques sont rejetes a partir des paires d'angles qui
partagent le meme `axis`. Le builder supprime aussi les slots de lecture qui
n'ont plus aucune source primaire apres deduplication, afin de ne pas produire
de section vide.

L'etape 2E.3 preserve les vrais aspects dynamiques apres l'exclusion des axes
structurels. Les aspects angle-angle restent exclus du payload route basic comme
dynamiques representatives, qu'ils soient l'axe Ascendant-Descendant, l'axe
MC-IC, ou un autre aspect entre deux angles. Les aspects forts planete-planete
et planete-angle restent en revanche eligibles. Le filtrage preserve d'abord une
tension forte non structurelle si elle existe ; si aucune tension forte n'est
disponible mais qu'un aspect fort planete-planete ou planete-angle existe, il
preserve le meilleur aspect fort disponible afin que `main_tension_or_support`
ne disparaisse pas artificiellement.

L'etape 3A ajoute le premier enrichissement global avant synthese externe avec
`chart_context` et `positions[].visibility_context`. Le payload expose
desormais le cadre technique du theme natal, un contrat de projection
(evolue jusqu'au contrat courant `natal_structured_v13`), les contraintes de
fiabilite, la secte deduite du Soleil
et une synthese d'hemisphere. Chaque position porte aussi sa position
d'horizon (`above_horizon`, `below_horizon` ou `on_horizon`), l'ID canonique
issu de `astral_horizon_positions`, une source d'audit et le flag de visibilite
local. Depuis 3A.1, l'engine Swiss Ephemeris calcule l'altitude topocentrique
vraie des corps mobiles via un calcul equatorial topocentrique et `swe_azalt`.
Les angles recoivent aussi `horizon_position_id` depuis le meme referentiel,
mais gardent `altitude_deg = null` car leur contexte d'horizon vient de leur
nature geometrique. Pour les corps non-angle, l'altitude calculee est la source
autoritaire : si un ancien `facts_json.visibility_context` contient encore une
projection d'hemisphere contradictoire, le builder reconstruit
`horizon_position` et `source = "calculated_altitude"` depuis `altitude_deg`.
Les anciens faits persistants sans altitude finie, sans reference d'horizon
positive ou sans flag de visibilite ne sont plus reutilises : le runtime force
alors un nouveau calcul ephemeride complet au lieu de reconstruire un payload
enrichi depuis des faits obsoletes.

L'etape 3A.2 stabilisait alors le contrat en `natal_structured_v9`. Le schema
et le golden v9 sont dedies, et les artefacts v8 restent presents pour
historique. `chart_context.hemisphere_emphasis`
declare maintenant explicitement `count_scope = "mobile_chart_objects_only"`.
Le `visibility_context` des signaux de placement est recopie dans
`signals[].evidence.placement_context`, afin que la couche applicative n'ait
plus a recroiser les signaux avec `positions`. Les angles gardent un contexte
d'horizon, mais leur `is_visible` est `null` car un angle n'est pas visible
comme un corps astronomique. La secte et l'accent d'hemisphere servent
uniquement de contexte de ponderation, pas de section autonome.

L'etape 3D ajoute `lunar_phase_context` et fait passer le contrat a
`natal_structured_v12` quand les references lunaires sont injectees et le bloc
construit.

L'etape 3E ajoute `accidental_dignities`, enrichit
`positions[].accidental_dignity_context` et
`signals[].evidence.placement_context.accidental_dignity_context` pour les
signaux `object_position:*`, et fait passer le contrat courant a
`natal_structured_v13` lorsque les trois jeux de references (lunaire,
accidentel, secte) sont disponibles avec un `lunar_phase_context` construit.
Sans references accidentelles ou secte, le moteur reste en v12 sans bloc
accidentel.

L'etape 3G restructure **Limites connues** en decisions explicites
(`assumed_v13`, `llm_boundary`, `future_story`, `deprecated`) pour cadrer le
branchement LLM sans ambiguite sur les templates moteur.

L'etape 3F aligne les orbes de detection des aspects majeurs sur le referentiel
PostgreSQL : les cinq lignes `astral_aspects` de famille `major` portent
`default_orb_deg`, lues par `aspect_definitions()` et validees avant tout calcul
ephemeride (`validate_aspect_definitions` dans `runtime/references.rs`).
Contrairement a 3D (v12) ou 3E (v13), **3F ne cree pas de nouveau contrat JSON** :
`natal_structured_v13` reste le contrat courant. La detection geometrique
n'utilise plus `default_major_orb_deg` du profil produit ; ce champ reste charge
via `basic_payload_catalog` et n'est valide qu'en sanity check (positif, fini,
`<= astral_aspect_families.max_default_orb_deg` pour `major`), sans etre passe a
`EphemerisEngine::calculate_natal`. Seul
`aspect_min_strength` filtre ensuite les signaux `aspect:*` actifs.
Les **codes**, **angles** et **orbes** des aspects majeurs viennent de
`astral_aspects` (famille `major`). Le **nombre** attendu de majeurs est porte
par `astral_aspect_families.expected_aspect_count` (5 pour `major` dans le seed).
Le runtime ne duplique plus de liste de codes/angles en Rust : il valide
l'integrite des lignes chargees et compare leur effectif au referentiel famille.
Plafond d'orbe par famille `astral_aspect_families.max_default_orb_deg` (15° pour
`major` dans le seed) en validation et dans `canonical_aspect_orb_deg`.

`product_code = "basic"` reste volontairement conserve comme cle de routage
legacy pour les tables et les chemins runtime existants. Il ne decrit plus un
contrat produit minimal du moteur : le payload courant est un payload
moteur riche, route par `product_code = "basic"`, dont la profondeur
fonctionnelle est portee par `chart_context.payload_contract`, notamment
`calculation_scope`, `interpretation_scope` et `projection_depth`.

Le runtime conserve la chaine existante :

1. calcul des faits astrologiques ;
2. ecriture des positions, cuspides et aspects calcules ;
3. relecture des positions et aspects avec leur contexte de referentiel ;
4. aggregation des signaux ;
5. filtrage produit Basic ;
6. ecriture du payload canonique dans `astral_interpretation_generation_payloads`.

Cette etape prepare donc une entree propre, lisible, auditable et pre-orientee
editorialement.

## Etat de schema runtime

Le payload route basic depend directement du schema PostgreSQL materialise depuis
`json_db`. Apres les refactos de scoring, le runtime attend notamment :

- la table `astral_chart_object_signal_profiles`, source canonique des bases de
  priorite et des poids de source par objet calculable ;
- la colonne `astral_house_modalities.priority_delta`, source canonique du delta
  applique aux placements selon la modalite de maison ;
- la table `astral_interpretation_generated_outputs`, reservee aux sorties LLM
  localisees produites depuis un payload canonique ;
- la table `astral_accidental_dignity_condition_definitions`, source canonique
  des 15 codes de condition accidentelle MVP et de leurs `score_delta` ;
- la table `astral_object_sect_affinities`, source canonique des affinites de
  secte par objet pour le calcul `sect_affinity_*` ;
- la table `astral_lunar_phase_definitions`, source canonique des huit phases
  Soleil-Lune ;
- `astral_basic_product_scoring_profiles` : seuils d'emphase, limite de signaux
  actifs, `aspect_min_strength`, parametres d'axes de maisons (le champ
  `default_major_orb_deg` reste en base mais n'alimente plus la detection
  geometrique depuis 3F) ;
- `astral_aspects` : referentiel des aspects (`code`, `name`, `angle`, `family`,
  `default_orb_deg`) ; seuls les `family = 'major'` alimentent la detection ;
- `astral_aspect_families` : `expected_aspect_count` (5 pour `major`) et
  `max_default_orb_deg` (15° pour `major`) ;
- `astral_accidental_condition_triggers` : declencheurs MVP (modalite, mouvement,
  horizon, proximite angle, secte) ;
- `astral_accidental_scoring_params` et `astral_accidental_overall_polarity_bands` :
  baseline, orbe angle, paliers `overall_polarity` / `expression_quality` ;
- profil `astral_essential_dignity_score_weights` (deltas de priorite/poids
  signaux par type de dignite essentielle).

Le runtime charge ces references via `BasicPayloadCatalog` (`catalog.rs`) et
projette dans `chart_context` les snapshots `accidental_scoring` (baseline,
bornes min/max, orbe angle, bandes de polarite) et `product_scoring` pour la
validation de fraicheur (freshness) sans constantes en dur. Les payloads v13
sans ces snapshots ou avec des bandes non contigues sur `[0, 1]` sont rejetes.

Ces donnees ne doivent pas etre compensees par des valeurs applicatives en dur.
Si le binaire echoue avec une erreur SQL de relation ou de colonne manquante
(par exemple `column "default_orb_deg" does not exist` sur `astral_aspects`, ou
`column "expected_aspect_count" / "max_default_orb_deg" does not exist` sur
`astral_aspect_families`), la correction attendue est de resynchroniser PostgreSQL avec les fichiers
`json_db` via `scripts/import_json_db_to_postgres.py` ou le patch cible
`scripts/patch_astral_aspects_default_orb_deg.py` et
`scripts/patch_astral_aspect_families_expected_count.py`, pas de contourner la
lecture en Rust.

## Fichiers concernes

- `astral_calculator/src/catalog.rs` : `BasicPayloadCatalog` (charge depuis
  PostgreSQL en production via `repositories::basic_payload_catalog`) ;
  `test_catalog()` fournit un profil minimal pour les tests unitaires hors DB.
- `astral_calculator/src/domain.rs` : structures runtime et payload JSON.
- `astral_calculator/src/facts.rs` : helpers de libelles signe/maison.
- `astral_calculator/src/ephemeris.rs` : enrichissement des positions calculees.
- `astral_calculator/src/aspects.rs` : detection geometrique des aspects
  majeurs (`canonical_aspect_orb_deg`, `detect_aspects`) et calcul de l'orbe
  observe, de la phase et de la force brute.
- `astral_calculator/src/dignities.rs` : detection MVP des dignites essentielles majeures.
- `astral_calculator/src/payload/accidental_dignities.rs` : evaluation MVP
  des dignites accidentelles et projection vers positions, signaux et dominantes.
- `astral_calculator/src/payload/lunar_phase.rs` : construction de
  `lunar_phase_context` depuis les references lunaires.
- `astral_calculator/src/signals/` : construction, filtrage et
  priorisation des signaux du payload route basic.
- `astral_calculator/src/payload/` : assemblage du payload final et de
  ses blocs contractuels.
- `astral_calculator/src/repositories.rs` : persistance, relecture des
  positions et enrichissement SQL depuis les referentiels de signes, maisons,
  objets, angles et aspects.
- `astral_calculator/src/runtime/` : orchestration runtime,
  validation des references (`validate_aspect_definitions`,
  `major_aspect_family_reference` dans `repositories.rs`) et regeneration des
  anciens payloads.
- `astral_calculator/src/runtime/service.rs` : `calculate_natal_basic`
  (chargement `aspect_definitions`, validation, puis calcul ephemeride).
- `json_db/astral_accidental_dignity_condition_definitions.json` : definitions
  canoniques des 15 conditions accidentelles MVP.
- `json_db/astral_object_sect_affinities.json` : affinites de secte par objet.
- `json_db/astral_lunar_phase_definitions.json` : definitions des phases lunaires.
- `json_db/astral_aspects.json` : referentiel des aspects (codes, angles, orbes).
- `json_db/astral_aspect_families.json` : familles d'aspects,
  `expected_aspect_count` et `max_default_orb_deg`.
- `astral_calculator/src/models.rs` : `AspectDefinition` (avec
  `max_default_orb_deg` issu du JOIN famille) et `MajorAspectFamilyReference`.
- `scripts/patch_astral_aspects_default_orb_deg.py` : colonne et orbes
  `astral_aspects.default_orb_deg`, coherence effectif / famille.
- `scripts/psql_docker.py` : execution SQL via `docker compose exec` (fallback `psql` local).
- `scripts/patch_astral_aspect_families_expected_count.py` : colonnes
  `expected_aspect_count` et `max_default_orb_deg` sur `astral_aspect_families`.
- `tests/aspects_tests.rs` : non-regression geometrie, axes structurels et orbes
  par aspect.
- `tests/common/json_db.rs` : fixtures de test chargees depuis
  `json_db/astral_aspects.json` (`include_str!`) pour eviter la derive entre seed
  et tests.
- `tests/runtime_tests.rs` : validation referentiel aspects (`validate_aspect_definitions_*`).
- `astral_calculator/schemas/basic_natal_structured_v8.schema.json` :
  schema JSON historique du contrat Basic v8.
- `astral_calculator/schemas/natal_structured_v9.schema.json` :
  schema JSON historique du contrat v9.
- `astral_calculator/schemas/natal_structured_v10.schema.json` :
  schema JSON historique du contrat `natal_structured_v10`.
- `astral_calculator/schemas/natal_structured_v11.schema.json` :
  schema JSON historique du contrat `natal_structured_v11`.
- `astral_calculator/schemas/natal_structured_v12.schema.json` :
  schema JSON historique du contrat `natal_structured_v12`.
- `astral_calculator/schemas/natal_structured_v13.schema.json` :
  schema JSON du contrat courant `natal_structured_v13`.
- `tests/golden/basic_payload_v8_paris_1990.json` : fixture golden historique
  du contrat Basic v8.
- `tests/golden/natal_payload_v9_paris_1990.json` : fixture golden du contrat
  historique v9.
- `tests/golden/natal_payload_v10_paris_1990.json` : fixture golden historique
  du contrat v10.
- `tests/golden/natal_payload_v11_paris_1990.json` : fixture golden historique
  du contrat v11.
- `tests/golden/natal_payload_v12_paris_1990.json` : fixture golden historique
  du contrat v12.
- `tests/golden/natal_payload_v13_paris_1990.json` : fixture golden du contrat
  courant v13 (`chart_calculation_id: 27`).
- `tests/contract_basic_v8_tests.rs` : validation schema, golden et invariants
  metier non negociables pour le contrat courant v13.
- `scripts/verify_basic_v8_golden.ps1` : verification CI/local de projection
  stable historique v8. Ce script ne regenere plus le payload courant ; il
  valide le golden v8 conserve, ou un fichier v8 fourni explicitement.
- `scripts/verify_natal_v9_golden.ps1` : verification CI/local de projection
  stable historique v9 apres regeneration du payload par le moteur.
- `scripts/verify_natal_v10_golden.ps1` : verification CI/local historique de
  projection stable v10 apres regeneration du payload par le moteur.
- `scripts/verify_natal_v11_golden.ps1` : verification CI/local historique de
  projection stable v11 apres regeneration du payload par le moteur.
- `scripts/verify_natal_v12_golden.ps1` : verification CI/local historique de
  projection stable v12.
- `scripts/verify_natal_v13_golden.ps1` : verification CI/local de projection
  stable v13 apres regeneration du payload par le moteur (`signal_keys` triees
  avant diff).

## Contrat des positions

Chaque position expose maintenant les champs lisibles en plus des IDs :

```json
{
  "object_code": "sun",
  "object_name": "Sun",
  "longitude_deg": 84.8759,
  "sign_id": 3,
  "sign_code": "gemini",
  "sign_name": "Gemini",
  "house_id": 9,
  "house_number": 9,
  "house_name": "Beliefs",
  "motion_state_id": 1,
  "sign_context": {
    "element": "air",
    "element_label": "Air",
    "modality": "mutable",
    "modality_label": "Mutable",
    "polarity": "yang",
    "polarity_label": "Yang",
    "keywords": ["communication", "curiosity"]
  },
  "house_context": {
    "theme_code": "beliefs"
  },
  "house_modality": {
    "code": "cadent",
    "label": "Cadent",
    "accidental_strength": "weak_or_background",
    "interpretation_weight": "lower_for_external_manifestation"
  },
  "object_context": {
    "role": "luminary",
    "role_label": "Luminary",
    "nature": ["luminary"],
    "is_luminary": true,
    "is_planet_symbolic": false,
    "is_visible_to_naked_eye": true
  },
  "motion_context": {
    "motion_state": "direct",
    "label": "Direct",
    "motion_family": "forward"
  },
  "dignity_context": [
    {
      "fact_type": "essential_dignity",
      "dignity_type": "domicile",
      "dignity_label": "Domicile",
      "polarity": "dignity",
      "strength_score": 1.0
    }
  ],
  "visibility_context": {
    "horizon_position_id": 1,
    "horizon_position": "above_horizon",
    "altitude_deg": 12.5,
    "is_visible": true,
    "source": "calculated_altitude"
  }
}
```

Les IDs restent presents pour l'audit et les relations DB. Les libelles viennent :

- des referentiels `astral_signs`, `astral_sign_profiles`,
  `astral_sign_keywords`, `astral_houses`, `astral_house_modalities`,
  `astral_chart_object_definitions`, `astral_object_nature_assignments`,
  `astral_object_motion_states` et `astral_angle_points` charges avant le
  calcul pour les nouveaux faits ;
- des joins equivalents quand un payload est reconstruit depuis la DB.

`house_context.theme_code` vient de `astral_houses.theme_code`. Le runtime ne
derive pas ce code depuis le nom de maison, car certains cas ne sont pas
mecaniques (`Self` -> `identity`, `Health` -> `work_health`,
`Transformation` -> `shared_resources`, `Subconscious` -> `inner_world`).

Le calcul geometrique conserve seulement les operations derivees de la
longitude : slot zodiacal et numero de maison. Les IDs, codes et noms de signes
ou de maisons sont resolus depuis les tables. Le runtime refuse de calculer si
les 12 signes ou les 12 maisons ne sont pas presents ou si les references sont
ambigues. Depuis la stabilisation v9, le contrat refuse aussi de reutiliser un
payload existant si les contextes de placement utiles sont absents ou
incomplets.

Depuis 2B.1, `dignity_context` est toujours expose comme tableau. Il vaut `[]`
quand l'objet ne recoit aucune dignite essentielle majeure reconnue par le MVP.
Cette convention evite les `null` dans le contrat JSON et couvre les placements
qui peuvent cumuler deux dignites classiques, par exemple Mercure en Vierge
(`domicile` et `exaltation`) ou Mercure en Poissons (`detriment` et `fall`).

## Contrat des angles 2E

Le payload expose les quatre angles principaux dans un tableau top-level
`angles`. Ces faits sont aussi presents dans `positions`, mais `angles` donne un
acces direct au triptyque Basic et aux axes :

```json
{
  "angle_code": "ascendant",
  "angle_name": "Ascendant",
  "axis": "horizontal",
  "opposite_angle_code": "descendant",
  "longitude_deg": 155.4787,
  "sign_id": 6,
  "sign_code": "virgo",
  "sign_name": "Virgo",
  "house_id": 1,
  "house_number": 1,
  "house_name": "Self"
}
```

Les longitudes de l'Ascendant et du MC viennent de `swiss_eph::safe::houses`.
Le Descendant et l'IC sont derives par opposition exacte a 180 degres. Les
metadonnees non geometriques (`axis`, code oppose, maison associee, libelles,
description, ordre de tri) viennent de `astral_angle_points` et des objets
calculables actifs de `astral_chart_objects`. La table de reference utilise des
codes courts (`asc`, `dsc`, `mc`, `ic`) dans `angle_context`, mais le tableau
top-level `angles` expose `opposite_angle_code` sous forme de code objet long
quand le correspondant est present dans les positions (`ascendant`,
`descendant`, `mc`, `ic`). Cette resolution est faite depuis les positions
relues, pas depuis un mapping code en dur.

Les signaux d'angle utilisent la forme stable `angle:<angle_code>:sign:<sign>`,
par exemple `angle:ascendant:sign:virgo`. L'Ascendant est place dans
`core_identity` avec le Soleil et la Lune. Le MC peut alimenter
`background_factors` comme contexte de vocation, visibilite ou direction
publique. Depuis la stabilisation v9, `signals[].evidence.opposite_angle_code`
conserve le code court issu du referentiel (`dsc`, `asc`, `ic`, `mc`) et
`signals[].evidence.opposite_angle_object_code` expose le code objet long
homogene avec `angles[].opposite_angle_code`.

Les oppositions Ascendant-Descendant et MC-IC restent des axes structurels. Elles
ne doivent pas produire de signal actif `aspect:*`, ne doivent pas alimenter
`main_tension_or_support`, et ne doivent pas ajouter la raison
`strong_aspect_participant` a `chart_emphasis.dominant_objects`. Le runtime les
reconnait soit au moment de la detection geometrique via
`angle_context.angle_point_code`, `opposite_angle_code` et `axis`, soit au moment
de l'assemblage/validation du payload via les paires d'angles qui partagent le
meme `axis`.

Pour eviter qu'un ancien calcul `completed` soit recycle sans angles, le runtime
verifie que les positions persistantes contiennent tous les objets d'angle
attendus par `astral_angle_points`. Si ce n'est pas le cas, il cree un nouvel
`execution_attempt` au lieu de reconstruire un payload incomplet.

## Contrat des signaux

Les signaux actifs du payload route basic sont limites a 12.

Un signal de position contient un titre lisible, des champs semantiques et les
preuves techniques dans `evidence` :

```json
{
  "signal_key": "object_position:sun",
  "theme_code": "beliefs",
  "title": "Sun in Gemini, house 9",
  "summary": "Sun is placed in Gemini and the Beliefs house, emphasizing this chart factor through a concrete, readable placement.",
  "priority_score": 100.0,
  "confidence_score": 0.95,
  "interpretive_hint": "Sun expresses through Gemini qualities in the field of Beliefs.",
  "semantic_tags": [
    "placement",
    "sun",
    "gemini",
    "learning",
    "adaptability",
    "house_9",
    "beliefs",
    "philosophy",
    "travel"
  ],
  "source_weight": 1.0,
  "aggregation_group": "gemini:house_9",
  "aspect_context": null,
  "evidence": {
    "fact_type": "object_position",
    "chart_object_id": 1,
    "object_code": "sun",
    "object_name": "Sun",
    "sign_id": 3,
    "sign_code": "gemini",
    "sign_name": "Gemini",
    "house_id": 9,
    "house_number": 9,
    "house_name": "Beliefs",
    "longitude_deg": 84.8759,
    "placement_context": {
      "sign_context": {
        "element": "air",
        "modality": "mutable",
        "polarity": "yang",
        "keywords": ["communication", "curiosity"]
      },
      "house_context": {
        "theme_code": "beliefs"
      },
      "house_modality": {
        "code": "cadent",
        "accidental_strength": "weak_or_background",
        "interpretation_weight": "lower_for_external_manifestation"
      },
      "object_context": {
        "role": "luminary",
        "nature": ["luminary"],
        "is_luminary": true
      },
      "motion_context": {
        "motion_state": "direct",
        "label": "Direct",
        "motion_family": "forward"
      },
      "dignity_context": [],
      "visibility_context": {
        "horizon_position_id": 1,
        "horizon_position": "above_horizon",
        "altitude_deg": 12.5,
        "is_visible": true,
        "source": "calculated_altitude"
      }
    },
    "essential_dignities": []
  }
}
```

Un signal d'aspect utilise les codes stables dans `signal_key`, mais pas dans le
texte utilisateur :

```json
{
  "signal_key": "aspect:sun:mercury:conjunction",
  "theme_code": "aspect",
  "title": "Sun conjunction Mercury",
  "summary": "Sun and Mercury form a conjunction with 1.01 degrees of orb; the phase is separating.",
  "priority_score": 69.92,
  "confidence_score": 0.85,
  "interpretive_hint": "Read this conjunction as an amplifying contact between Sun and Mercury, with attention to the separating phase.",
  "semantic_tags": [
    "aspect",
    "conjunction",
    "major",
    "intensification",
    "amplifying",
    "high_strength"
  ],
  "source_weight": 1.75,
  "aggregation_group": "aspect:conjunction",
  "aspect_context": {
    "aspect_family": "major",
    "primary_valence": null,
    "intensity_modifier": "amplifying",
    "secondary_effect": null,
    "dynamic_quality": "intensification",
    "phase_state": "separating",
    "valence_family": "intensity",
    "is_tonal_valence": false,
    "is_intensity_modifier": true
  },
  "evidence": {
    "fact_type": "aspect",
    "source_chart_object_id": 1,
    "source_object_code": "sun",
    "source_object_name": "Sun",
    "target_chart_object_id": 3,
    "target_object_code": "mercury",
    "target_object_name": "Mercury",
    "aspect_id": 1,
    "aspect_code": "conjunction",
    "aspect_name": "Conjunction",
    "aspect_family": "major",
    "orb_deg": 1.0084,
    "phase_state": "separating",
    "is_applying": false,
    "is_exact": false,
    "strength_score": 0.874,
    "calculation_notes": {
      "aspect_code": "conjunction",
      "aspect_name": "Conjunction",
      "exact_angle_deg": 0.0,
      "orb_limit_deg": 8.0,
      "separation_deg": 1.0084
    }
  }
}
```

### Champs semantiques 1B

Les champs ajoutes par l'etape 1B sont :

- `theme_code` : theme editorial principal du signal. Pour les placements et
  angles, il vient de `astral_houses.theme_code` via `house_context`; pour les
  aspects, dignites ou autres familles, il vient de la famille de signal.
- `summary` : phrase templatee de synthese moteur sur le signal ; aide
  structuree, pas texte final utilisateur (voir `llm_boundary`, section Limites connues).
- `interpretive_hint` : indice court derive du contexte (placement, aspect, dignite).
  Aide moteur structuree, pas prose finale ; pour les aspects, integre la valence 2C
  via `aspect_context`.
- `semantic_tags` : tags stables utiles pour grouper, filtrer ou guider la
  redaction.
- `source_weight` : poids relatif de la source astrologique. Il est fourni par
  `astral_chart_object_signal_profiles.source_weight` via
  `object_context.signal_scoring`.
- `aggregation_group` : cle de regroupement editoriale.

Ces champs sont stockes dans `astral_interpretation_signals.payload_json`, puis
remontes dans le payload final par `payload.rs`.

`aspect_context` est egalement expose au niveau de chaque `BasicSignal`. Il
contient un objet structure uniquement pour les signaux `aspect:*` ; pour les
placements, dignites et clusters, il est serialise a `null`.

### Champs contextuels 2A

Les champs ajoutes par l'etape 2A sont volontairement limites aux preuves utiles
pour le calcul et la redaction. Ils ne recopient pas les faits runtime bruts,
mais ils peuvent embarquer un referentiel semantique complet quand ce
referentiel est directement exploitable par une couche applicative externe.

- `sign_context` : element, modalite zodiacale, polarite et liste complete des
  mots-cles principaux du signe depuis `astral_sign_keywords.keywords_json`.
- `house_context` : contexte editorial canonique de maison, dont
  `theme_code`, depuis `astral_houses.theme_code`.
- `house_modality` : modalite de maison, force accidentelle, delta de priorite
  et poids d'interpretation.
- `object_context` : role astrologique, nature principale et indicateurs de
  visibilite/symbolique. Il inclut aussi `signal_scoring`, issu de
  `astral_chart_object_signal_profiles`, avec `position_priority_base`,
  `angle_priority_base` et `source_weight`.
- `motion_context` : etat de mouvement lisible, libelle et famille de mouvement.

Dans `positions`, ces contextes sont exposes directement comme preuves
structurees. Dans les signaux de placement, ils sont imbriques dans
`evidence.placement_context`, afin de rester associes au fait astrologique et de
ne pas creer un bloc redactionnel autonome.

La liste `sign_context.keywords` reste volontairement non tronquee. Elle
represente le vocabulaire semantique disponible pour le signe, pas une liste de
points a rediger un par un. Ces mots-cles servent a guider la synthese externe,
a eviter l'invention et a permettre une lecture plus riche sans exposer les
preuves brutes comme une section autonome.

Les tags semantiques des placements integrent aussi les codes utiles comme
`air`, `mutable`, `yang`, `cadent`, `luminary` ou `direct`. La priorite d'un
placement est calculee a partir de donnees canoniques : base de priorite issue
de `astral_chart_object_signal_profiles.position_priority_base`, delta de
modalite issu de `astral_house_modalities.priority_delta`, puis delta de
dignite borne. Les angles utilisent `angle_priority_base` quand il est defini.

### Validation des profils de scoring

Les valeurs de scoring ne sont pas des fallbacks applicatifs. Elles font partie
du referentiel canonique charge depuis la base :

- chaque objet actif et calculable doit avoir une ligne
  `astral_chart_object_signal_profiles` pour la `reference_version_id` du
  calcul ;
- `position_priority_base` doit etre present et compris entre `0` et `100` ;
- `source_weight` doit etre present et positif ou nul ;
- les objets dont le role astrologique est `angle` doivent aussi avoir
  `angle_priority_base` entre `0` et `100` ;
- chaque maison doit exposer une modalite avec
  `astral_house_modalities.priority_delta`.

Le runtime valide ces preconditions avant le calcul. Si une valeur manque, il
renvoie une erreur de reference au lieu de produire un `priority_score` ou un
`source_weight` sous-pondere par defaut. Les fonctions de scoring Rust restent
pures et lisent uniquement `object_context.signal_scoring` et
`house_modality.priority_delta`; elles ne redefinissent pas de mapping par
`object_code`.

Point de vigilance operationnel : une base deja creee avant l'ajout de ces
referentiels peut contenir les anciennes tables mais manquer
`astral_chart_object_signal_profiles`, `astral_interpretation_generated_outputs`,
`astral_house_modalities.priority_delta`, `astral_aspects.default_orb_deg`, ou
les metadonnees `astral_aspect_families.expected_aspect_count` /
`astral_aspect_families.max_default_orb_deg`.
Dans ce cas, le programme ne doit pas demarrer avec des fallbacks. Il faut
appliquer les ajouts issus de `json_db`, executer
`scripts/patch_astral_aspect_families_expected_count.py` puis
`scripts/patch_astral_aspects_default_orb_deg.py` (metadonnees famille + orbes
par aspect + verification que le nombre de majeurs correspond a
`expected_aspect_count` et que chaque orbe respecte `max_default_orb_deg`),
ou relancer l'import PostgreSQL complet si la conservation des donnees runtime
n'est pas requise.

### Champs de dignite 2B

Les champs ajoutes par l'etape 2B sont :

- `dignities` : liste top-level des dignites essentielles detectees dans le
  theme, avec objet, signe, type de dignite, polarite, score et eventuel
  `signal_key` actif associe.
- `positions[].dignity_context` : preuve courte de dignite pour la position,
  exposee comme tableau quand plusieurs dignites s'appliquent.
- `signals[].evidence.essential_dignities` : tableau de preuves de dignite
  rattache a un signal de placement.
- `dignity:*` : signaux autonomes produits seulement pour les dignites majeures
  significatives.

Exemple top-level :

```json
{
  "dignities": [
    {
      "object_code": "saturn",
      "object_name": "Saturn",
      "sign_id": 10,
      "sign_code": "capricorn",
      "sign_name": "Capricorn",
      "dignity_type": "domicile",
      "dignity_label": "Domicile",
      "polarity": "dignity",
      "strength_score": 1.0,
      "signal_key": "dignity:saturn:domicile:capricorn"
    }
  ]
}
```

Exemple de signal de dignite :

```json
{
  "signal_key": "dignity:saturn:domicile:capricorn",
  "theme_code": "functional_strength",
  "title": "Saturn strongly placed in Capricorn",
  "summary": "Saturn is in Capricorn, a sign where its function is reinforced by domicile.",
  "priority_score": 88.0,
  "confidence_score": 0.95,
  "interpretive_hint": "Treat Saturn in Capricorn as a domicile modifier for the existing placement signal.",
  "semantic_tags": [
    "dignity",
    "saturn",
    "capricorn",
    "domicile",
    "functional_strength",
    "structure",
    "responsibility"
  ],
  "source_weight": 0.75,
  "aggregation_group": "dignity:saturn",
  "aspect_context": null,
  "evidence": {
    "fact_type": "essential_dignity",
    "chart_object": "saturn",
    "sign_code": "capricorn",
    "dignity_type": "domicile",
    "polarity": "dignity",
    "strength_score": 1.0,
    "is_major": true
  }
}
```

Les dignites modifient les placements de facon moderee :

- le `priority_score` d'un placement recoit un delta borne a `+9.0` meme si
  plusieurs dignites s'appliquent ;
- le `source_weight` d'un placement recoit un delta borne a `+0.2` ;
- les signaux `dignity:*` ont leur propre `priority_score`, mais restent soumis
  au filtrage du payload route basic de 12 signaux actifs ;
- les dignites actives liees a un objet selectionne dans `reading_plan` sont
  ajoutees aux sources du slot, y compris quand le placement de l'objet a ete
  fusionne dans un cluster ;
- dans les slots qui limitent un nombre d'objets, comme `expression_style` ou
  `background_factors`, les dignites associees ne consomment pas le quota
  d'objets. Elles accompagnent l'objet selectionne au lieu de remplacer un autre
  placement attendu ;
- un meme signal n'est assigne qu'une fois par defaut. S'il est candidat a un
  second slot sans role distinct, il reste dans son slot primaire et remonte
  seulement dans `secondary_slot_candidates` pour audit.

Le MVP couvre les dignites essentielles majeures par signe :

- `domicile` ;
- `exaltation` ;
- `detriment` ;
- `fall`.

Il ne couvre pas encore les dignites mineures par triplicite, terme ou face.

### Champs d'aspect 2C

Les champs ajoutes par l'etape 2C sont :

- `signals[].aspect_context` : contexte interpretatif structure pour chaque
  signal `aspect:*` ; il vaut `null` pour les autres familles de signaux.
- `aspect_context.aspect_family` : famille issue de `astral_aspects.family`.
- `aspect_context.primary_valence` : valence principale issue des effets
  `primary_valence` de `astral_aspect_interpretive_effects`.
- `aspect_context.intensity_modifier` : modificateur d'intensite issu des
  effets `intensity_modifier`, par exemple `amplifying`.
- `aspect_context.secondary_effect` : effet secondaire eventuel issu des effets
  `secondary_effect`.
- `aspect_context.dynamic_quality` : qualite redactionnelle derivee de la
  valence ou du modificateur (`flow`, `tension`, `adjustment`,
  `intensification`, `symbolic`, `integration`, `contextual`).
- `aspect_context.valence_family`, `is_tonal_valence` et
  `is_intensity_modifier` : famille et flags issus de
  `astral_interpretive_valence` pour l'effet effectivement expose. Quand un
  aspect n'a pas de valence principale mais a un modificateur, comme la
  conjonction avec `amplifying`, ces champs decrivent le modificateur.
- `signals[].interpretive_hint` pour les aspects : phrase courte derivee de
  `primary_valence`, `intensity_modifier` ou `dynamic_quality`, sans exposer les
  cles techniques au lecteur final. Quand une valence principale et un
  modificateur sont presents ensemble, le hint exprime les deux sans traiter le
  modificateur comme une valence autonome.

Le runtime ne lit pas une colonne texte libre sur `astral_aspect_profiles` pour
decider la valence. Il passe par :

1. `astral_aspect_profiles.aspect_id` ;
2. `astral_aspect_interpretive_effects.aspect_profile_id` ;
3. `astral_interpretive_valence.id`.

Les roles d'effet sont contractuels. `primary_valence` peut guider le ton
principal d'un aspect ; `intensity_modifier` augmente ou focalise l'intensite,
mais ne decide pas seul si l'aspect est facilitant ou tendu. Ainsi une
conjonction expose `intensity_modifier = "amplifying"` et
`primary_valence = null`.

Les tags semantiques des aspects reprennent cette couche interpretative. Une
conjonction forte peut par exemple produire :

```json
[
  "aspect",
  "conjunction",
  "major",
  "intensification",
  "amplifying",
  "high_strength"
]
```

Un sextile ou trigone ajoute typiquement `flow` et une valence comme
`supportive` ou `harmonious`. Un carre ou une opposition ajoute typiquement
`tension` et une valence comme `dynamic_challenging` ou `polarizing`.

### Synthese de dominance 2D

Le payload expose maintenant `chart_emphasis`, calcule cote code avant toute
synthese externe. Cette couche resume la hierarchie globale du theme pour eviter
de deduire les dominantes depuis les signaux bruts :

```json
{
  "chart_emphasis": {
    "dominant_signs": [
      {
        "sign_code": "capricorn",
        "score": 0.87,
        "reasons": [
          "sun_in_sign",
          "saturn_domicile",
          "sign_house_cluster",
          "multiple_objects"
        ]
      }
    ],
    "dominant_houses": [
      {
        "house_number": 2,
        "theme_code": "resources",
        "score": 0.87,
        "reasons": [
          "sun_in_house",
          "cluster",
          "saturn_domicile"
        ]
      }
    ],
    "dominant_objects": [
      {
        "object_code": "saturn",
        "score": 0.78,
        "reasons": [
          "domicile",
          "cluster_participant",
          "capricorn_emphasis",
          "strong_aspect_participant"
        ]
      }
    ]
  }
}
```

Les scores sont normalises dans chaque famille entre `0` et `1` contre une
echelle fixe, pas contre le meilleur element du theme. Un score de `1.0`
signifie donc une saturation forte de la famille concernee, pas simplement le
rang 1 local. Les raisons sont
des cles auditables issues des placements (`sun_in_sign`, `sun_in_house`), des
concentrations (`multiple_objects`, `sign_house_cluster`, `cluster`), des
dignites (`saturn_domicile`, `domicile`), de la dominante de signe reportee sur
les objets (`capricorn_emphasis`) et des aspects actifs forts
(`strong_aspect_participant`).

Le filtrage garde le resume au niveau "dominance" plutot qu'au niveau simple
classement :

- `dominant_signs` et `dominant_houses` ne gardent que les scores `>= 0.35`,
  sauf fallback vers le meilleur item quand aucun score ne franchit ce seuil ;
- `dominant_objects` ne garde que les scores `>= 0.50` qui ont au moins une
  raison autre que `placement`, sauf fallback vers le meilleur objet quand aucun
  objet ne franchit ce seuil ;
- les listes sont triees par score decroissant et tronquees a 3 signes,
  3 maisons et 5 objets ;
- un objet present uniquement parce qu'il est un luminaire ou une planete
  importante de base ne doit pas etre presente comme dominant si aucun autre
  indice ne le soutient ;
- les raisons comme `gemini_emphasis` ou `capricorn_emphasis` ne sont ajoutees
  aux objets que si le signe concerne franchit lui-meme le seuil de dominance.

Exemple reel genere avec les valeurs de verification Paris / 2024-06-15 :

```json
{
  "chart_emphasis": {
    "dominant_signs": [
      {
        "sign_code": "gemini",
        "score": 1.0,
        "reasons": [
          "sun_in_sign",
          "mercury_in_sign",
          "venus_in_sign",
          "jupiter_in_sign",
          "multiple_objects",
          "mercury_domicile",
          "jupiter_detriment",
          "sign_house_cluster"
        ]
      }
    ],
    "dominant_houses": [
      {
        "house_number": 9,
        "theme_code": "beliefs",
        "score": 1.0,
        "reasons": [
          "sun_in_house",
          "mercury_in_house",
          "jupiter_in_house",
          "uranus_in_house",
          "multiple_objects",
          "mercury_domicile",
          "jupiter_detriment",
          "cluster"
        ]
      }
    ],
    "dominant_objects": [
      {
        "object_code": "mercury",
        "score": 0.8566,
        "reasons": [
          "placement",
          "domicile",
          "cluster_participant",
          "strong_aspect_participant",
          "gemini_emphasis"
        ]
      }
    ]
  }
}
```

L'etape 2D.1 est historique dans le payload moteur courant : les dominantes ne
sont plus projetees dans un plan de redaction. `chart_emphasis` reste un bloc
top-level de ponderation et d'audit. Une couche LLM externe peut l'utiliser
comme contexte, mais le runtime Rust ne produit pas de references de section ni
de consignes de redaction associees.

## Signaux agreges du payload route basic

L'etape 1B ajoute un premier type de signal agrege :

```json
{
  "signal_key": "cluster:capricorn:house_2",
  "theme_code": "resources",
  "title": "Strong concentration in Capricorn, house 2",
  "summary": "4 chart factors are concentrated in Capricorn and the Resources house, giving extra interpretive weight to this area of the chart.",
  "priority_score": 99.0,
  "confidence_score": 0.9,
  "interpretive_hint": "Read this as a repeated emphasis: Capricorn qualities are focused through the themes of the Resources house.",
  "semantic_tags": [
    "cluster",
    "capricorn",
    "house_2",
    "resources",
    "structure",
    "responsibility",
    "security",
    "value"
  ],
  "source_weight": 2.3,
  "aggregation_group": "capricorn_house_2_cluster",
  "aspect_context": null,
  "evidence": {
    "fact_type": "position_cluster",
    "cluster_type": "sign_house",
    "sign_code": "capricorn",
    "sign_name": "Capricorn",
    "house_number": 2,
    "house_name": "Resources",
    "source_signals": [
      "object_position:sun",
      "object_position:saturn",
      "object_position:neptune",
      "object_position:uranus"
    ],
    "source_objects": [
      "sun",
      "saturn",
      "neptune",
      "uranus"
    ]
  }
}
```

Un cluster `sign_house` est produit quand au moins trois objets sont places dans
le meme couple `(sign_code, house_number)`. Il entre dans le meme filtrage du
payload route basic que les autres signaux et compte donc dans la limite des 12
signaux actifs.

## Filtrage du payload route basic

Le filtrage est applique dans `signals.rs` :

- les signaux sont tries par `priority_score` decroissant ;
- les aspects dont `strength_score < 0.4` passent en `suppressed` ;
- les aspects angle-angle passent aussi en `suppressed` des l'agregation, sauf
  les axes structurels Ascendant-Descendant et MC-IC qui ne creent pas de signal
  actif du tout ;
- les signaux `dignity:*` sont ajoutes avant le tri final quand la dignite est
  majeure et suffisamment significative ;
- les clusters semantiques sont ajoutes avant le tri final ;
- les sources secondaires d'un cluster retenu actif passent en `merged`, sauf
  Soleil, Lune, Ascendant et MC qui restent actifs comme marqueurs centraux ;
- quand des fusions liberent des places dans les 12 signaux du payload route
  basic, le runtime
  remonte les prochains signaux eligibles sans reactiver les aspects faibles ;
- si aucun aspect de tension fort n'est actif apres le filtrage initial alors
  qu'un carre ou une opposition atteint `strength_score >= 0.75`, le runtime
  remplace le signal actif non essentiel le moins prioritaire par la meilleure
  tension forte disponible. Cette protection de filtrage reste volontairement
  geometrique afin de ne pas perdre les tensions majeures classiques, mais les
  axes structurels d'angles et les aspects angle-angle ne sont pas eligibles a
  cette preservation ;
- si aucun aspect fort planete-planete ou planete-angle n'est actif apres cette
  premiere preservation, mais qu'un tel aspect atteint `strength_score >= 0.75`,
  le runtime preserve le meilleur aspect fort disponible afin de garder une
  dynamique de lecture exploitable ;
- seuls les signaux actifs relus depuis la DB restent eligibles au payload ;
- `payload.rs` filtre encore les anciens aspects d'axe structurel non marques et
  les anciens aspects angle-angle actifs quand les positions d'angle definissent
  ces objets, puis tronque le resultat final a 12 signaux comme garde de lecture.

Les signaux supprimes restent persistables dans `astral_interpretation_signals`
avec `suppression_state = 'suppressed'`, mais ne remontent pas dans le payload
route basic final.

Les signaux `merged` sont egalement persistables dans
`astral_interpretation_signals`, mais ils ne remontent pas dans le payload final
car les requetes de lecture ne selectionnent que `suppression_state = 'active'`.
Ils conservent une trace `editorial_state` dans `payload_json` avec la cle du
cluster qui les represente.

Apres une evolution du format des cles ou du filtrage, la table peut conserver
d'anciens signaux en `suppressed`, par exemple d'anciens aspects avec des cles
techniques historiques. Ils restent utiles pour l'audit, mais ne sont pas
consideres par le payload final tant que leur `suppression_state` n'est pas
`active`.

Si un ancien signal d'axe structurel est encore actif, par exemple
`aspect:ascendant:descendant:opposition`, le payload final le retire quand les
positions contiennent deux angles du meme `axis`. Le validateur runtime refuse
aussi de reutiliser un payload persiste qui contient encore un tel signal.

## Plan de lecture moteur route basic

`reading_plan` ordonne les signaux actifs par slots editoriaux (`core_identity`,
`dominant_cluster`, `main_tension_or_support`, etc.). C'est un **plan de lecture
moteur**, pas un plan redactionnel obligatoire pour le LLM (etiquette `llm_boundary`,
section Limites connues).

Le payload final contient maintenant `reading_plan` :

```json
{
  "reading_plan": [
    {
      "slot": "core_identity",
      "title": "Core identity markers",
      "source_signal_keys": [
        "object_position:sun",
        "object_position:moon"
      ],
      "primary_signal_keys": [
        "object_position:sun",
        "object_position:moon"
      ],
      "secondary_slot_candidates": []
    },
    {
      "slot": "dominant_cluster",
      "title": "Dominant repeated theme",
      "source_signal_keys": [
        "cluster:capricorn:house_2",
        "dignity:saturn:domicile:capricorn"
      ],
      "primary_signal_keys": [
        "cluster:capricorn:house_2",
        "dignity:saturn:domicile:capricorn"
      ],
      "secondary_slot_candidates": [
        {
          "signal_key": "object_position:sun",
          "primary_slot": "core_identity",
          "candidate_slot": "dominant_cluster"
        }
      ]
    },
    {
      "slot": "main_tension_or_support",
      "title": "Main dynamic aspect",
      "source_signal_keys": [
        "aspect:moon:neptune:sextile",
        "aspect:sun:moon:sextile",
        "aspect:jupiter:uranus:opposition"
      ],
      "primary_signal_keys": [
        "aspect:moon:neptune:sextile",
        "aspect:sun:moon:sextile",
        "aspect:jupiter:uranus:opposition"
      ],
      "secondary_slot_candidates": []
    }
  ]
}
```

Le plan est construit dans `payload.rs` a partir des signaux actifs. Il reste
dans le moteur, mais il n'est pas une consigne LLM : c'est seulement une
structure de regroupement et de priorisation des signaux. Il indique quels
signaux sont centraux, quels signaux sont secondaires et quels regroupements
servent l'audit du payload. La couche LLM externe peut s'en servir comme entree,
mais elle porte seule les objectifs de redaction, la langue cible, le ton et les
regles d'evitement.

Les slots suivants sont produits quand les sources correspondantes existent :

- `core_identity` : Soleil, Lune, Ascendant ;
- `dominant_cluster` : premier cluster actif, sources candidates associees et
  dignites actives des objets sources, puis resolution editoriale des doublons ;
- `main_tension_or_support` : jusqu'a trois aspects actifs prioritaires,
  reequilibres avec `aspect_context` pour conserver au moins un appui ou une
  tension quand ces dynamiques existent dans les aspects actifs ; les axes
  structurels d'angles et les autres aspects angle-angle en sont exclus ;
- `expression_style` : Mercure, Venus, Mars, avec leurs dignites actives si
  elles sont presentes ;
- `background_factors` : MC, Jupiter, Saturne, Uranus, Neptune, Pluton si
  encore actifs, avec leurs dignites actives si elles sont presentes.

Chaque item expose `source_signal_keys` et `primary_signal_keys`. Aujourd'hui,
ces deux listes sont identiques apres resolution editoriale ; `primary_signal_keys`
rend explicite que ces signaux sont les sources principales du slot. Quand un
signal etait candidat a un slot ulterieur mais a deja ete assigne, le slot
ulterieur ne le repete pas dans `source_signal_keys` et expose plutot :

```json
{
  "secondary_slot_candidates": [
    {
      "signal_key": "dignity:saturn:domicile:capricorn",
      "primary_slot": "dominant_cluster",
      "candidate_slot": "background_factors"
    }
  ]
}
```

Apres cette deduplication editoriale, un slot qui n'a plus aucune source
primaire est supprime du `reading_plan`.
Un signal qui n'etait plus qu'un candidat secondaire reste auditable uniquement
si le slot candidat est conserve par au moins une autre source primaire ; un
candidat secondaire seul ne suffit pas a conserver une section vide.

Pour eviter une lecture trop lisse, `main_tension_or_support` utilise maintenant
`aspect_context.dynamic_quality` et `aspect_context.primary_valence` pour
equilibrer les dynamiques. Si les premiers aspects selectionnes ne contiennent
aucune tension alors qu'un aspect actif porte `dynamic_quality = "tension"` ou
une valence comme `dynamic_challenging` ou `polarizing`, un des aspects est
remplace par cette tension. Symetriquement, si aucun appui n'est present alors
qu'un aspect actif porte `dynamic_quality = "flow"` ou une valence comme
`supportive` ou `harmonious`, un appui est reintegre. Le slot reste limite a
trois aspects.

Cette logique s'applique aussi quand le filtrage du payload route basic a du
liberer une place dans les 12 signaux actifs : un signal actif non essentiel
peut etre remplace par la meilleure tension forte disponible selon le garde-fou geometrique
carre/opposition, hors axes structurels d'angles et hors aspects angle-angle.
Les clusters et les marqueurs centraux ou expressifs restent proteges ; un signal
de dignite autonome peut en revanche ceder sa place si le budget est sature et
qu'aucune tension forte n'est encore active.

Depuis 2E.3, si aucune tension forte n'est disponible mais qu'un aspect fort
planete-planete ou planete-angle existe, le meme mecanisme preserve le meilleur
aspect fort disponible. `main_tension_or_support` n'est donc absent que
lorsqu'aucun aspect actif ou preservable ne reste apres l'exclusion des axes
structurels et des autres aspects angle-angle.

Depuis l'etape 3E, `natal_structured_v13` est le contrat
courant verrouille par trois niveaux complementaires :

- le JSON Schema
  `astral_calculator/schemas/natal_structured_v13.schema.json`
  valide la forme du contrat moteur, les blocs obligatoires, les quatre angles,
  les bornes de score, `chart_context`, `house_axis_emphasis`,
  `lunar_phase_context`, `accidental_dignities`,
  `positions[].accidental_dignity_context` et
  `signals[].evidence.placement_context.accidental_dignity_context` pour les
  signaux `object_position:*` ;
- la fixture `tests/golden/natal_payload_v13_paris_1990.json` conserve un
  payload complet de reference pour le scenario Paris 1990 ;
- `tests/contract_basic_v8_tests.rs` valide les invariants metier du contrat
  courant v13 et conserve aussi une validation schema des goldens historiques
  v8, v10, v11 et v12.

`natal_structured_v12` reste historique avec `lunar_phase_context` uniquement.

Avant 3E, `natal_structured_v12` etait le contrat courant verrouille par trois
niveaux complementaires :

- le JSON Schema
  `astral_calculator/schemas/natal_structured_v12.schema.json`
  valide la forme du contrat moteur, les blocs obligatoires, les quatre angles,
  les bornes de score, `chart_context`, `house_axis_emphasis`,
  `lunar_phase_context` et les contraintes schema exprimables.
  Il refuse aussi les champs semantiques de signal obligatoires a `null`, les
  contextes de position obligatoires a `null`, les proprietes parasites dans
  `aspect_context`, et les `visibility_context` mobiles sans altitude calculee
  ou sans flag `is_visible` booleen ;
- la fixture `tests/golden/natal_payload_v12_paris_1990.json` conserve un
  payload complet de reference pour le scenario Paris 1990 ;
- `tests/contract_basic_v8_tests.rs` valide les invariants metier du contrat
  courant v12 et conserve aussi une validation schema des goldens historiques v8, v10 et v11 :
  sources de plan existantes, absence d'aspect angle-angle actif, conservation
  de `aspect:jupiter:uranus:opposition`, unicite des signaux primaires et
  garde-fous contre des sections autonomes `chart_emphasis` / `chart_context`,
  ainsi que la coherence des axes de maisons.

La review adversariale de cette stabilisation a ajoute des tests negatifs et a
resserre l'alignement entre schema et validation runtime. Un payload qui valide
le schema ne doit plus pouvoir contourner les champs obligatoires attendus par
`is_current_basic_payload`, et un payload runtime ne doit plus etre considere
courant si ses angles top-level ne sont pas exactement le quatuor canonique ou
si le `visibility_context` d'un signal de placement mobile contredit son
altitude calculee.

La regeneration complete du golden courant depend de Postgres et de Swiss
Ephemeris. Le test unitaire ne reconstruit donc pas le theme depuis le moteur.
Pour couvrir ce risque en CI ou en verification locale,
`scripts/verify_natal_v13_golden.ps1` lance le moteur avec le scenario golden
Paris / `1990-01-02T03:04:05Z`, puis compare une projection stable du payload
genere au golden v13 (les `signal_keys` actives sont triees avant comparaison :
l'ordre des 12 signaux n'est pas contractuel). Le script force les variables
d'environnement du scenario
golden et les restaure ensuite, afin d'eviter qu'un `ASTRAL_OUTPUT_MODE`,
`ASTRAL_PRODUCT_CODE` ou identifiant de referentiel deja present ne modifie la
verification. Il peut aussi comparer un fichier deja genere via :

```powershell
.\scripts\verify_natal_v13_golden.ps1 -GeneratedPayloadPath .\output\basic_payload_current.json
```

`scripts/verify_natal_v12_golden.ps1` reste disponible pour le golden historique
v12.

Le script `scripts/verify_basic_v8_golden.ps1` reste disponible uniquement pour
valider le golden historique v8 ou un fichier v8 fourni explicitement. Il ne
regenere plus de payload depuis le moteur courant, car celui-ci produit
desormais `natal_structured_v13` lorsque les references lunaires, accidentelles
et secte sont chargees.
Le script `scripts/verify_natal_v11_golden.ps1` reste disponible pour le golden
historique v11.
Le script `scripts/verify_natal_v9_golden.ps1` reste disponible pour le golden
historique v9.
Le script `scripts/verify_natal_v10_golden.ps1` reste disponible pour le golden
historique v10.

## Annexe historique - handoff LLM retire

Les anciennes versions du document decrivaient une section nommee "Contrat
canonique de handoff LLM" et des blocs redactionnels destines a piloter une
generation de texte. Ces blocs ne font plus partie du payload moteur courant.
Ils sont conserves uniquement comme historique de conception et ne doivent pas
etre utilises pour valider, regenerer ou consommer un payload courant.

Le moteur Rust produit un payload astrologique canonique en anglais technique :
faits calcules, contextes, dominantes, rulership, signaux et `reading_plan`.
La couche LLM externe construit son propre contrat de prompt a partir de ces
donnees. Elle decide de la langue cible, du format de sortie, des objectifs de
redaction, du ton et des garde-fous textuels. Aucune section LLM, aucun plan de
redaction et aucune consigne de style ne sont attendus dans le JSON moteur.

La validation de reutilisation des payloads existants force maintenant aussi :

- pour chaque signal `aspect:*`, un `aspect_context` avec famille, valence
  primaire eventuelle, modificateur d'intensite eventuel, qualite dynamique,
  phase, `valence_family` et flags tonal/intensite ;
- pour chaque signal `aspect:*`, au moins un effet interpretatif non vide parmi
  `primary_valence`, `intensity_modifier` ou `secondary_effect` ;
- des slots connus uniquement ;
- l'ordre canonique des slots du payload route basic ;
- un `reading_plan` non vide, sans slot vide, dont les sources primaires
  referencent des signaux existants et uniques ;
- `lunar_phase_context` present avec `phase_code`, `phase_family`,
  `sun_moon_angle_deg`, `distance_to_exact_phase_deg` et
  `phase_progress_ratio` ;
- l'angle Soleil-Lune recalcule depuis les longitudes du Soleil et de la Lune
  avec une tolerance de 0.01 degre ;
- l'angle tombe dans l'intervalle canonique du `phase_code` declare ;
- `related_signal_keys` limite aux signaux actifs `object_position:sun` et
  `object_position:moon` quand ces signaux existent ;
- les tags `lunar_phase` et `sun_moon_cycle` presents dans
  `lunar_phase_context.semantic_tags`.

## Persistance

Le payload canonique est serialize et upserte dans :

`astral_interpretation_generation_payloads`

La contrainte fonctionnelle est :

```text
(chart_calculation_id, product_code, language_id)
```

Le runtime ecrit aussi les signaux dans `astral_interpretation_signals`.
Avant chaque reecriture des signaux d'un calcul, les signaux existants du meme
`chart_calculation_id` sont passes en `suppressed`. Les signaux recalcules sont
ensuite re-upsertes avec leur etat courant. Cela evite qu'un ancien signal actif
reste visible apres un changement de format de cle ou de filtrage.

Pour les aspects, le calcul ephemeride persiste d'abord les faits geometriques
dans `astral_calculated_aspects` en appliquant les orbes canoniques
`astral_aspects.default_orb_deg`, bornes par `astral_aspect_families.max_default_orb_deg`
(validation `validate_aspect_definitions` + `major_aspect_family_reference` avant
le calcul). Avant de construire les signaux, le runtime relit ces aspects via
les joins de `repositories.rs` afin d'ajouter la famille et les effets
interpretatifs issus des referentiels. Ce meme chemin est utilise pour un calcul
frais et pour la regeneration d'un payload existant juge obsolete.

Dans cette table, `language_id` designe la langue canonique du payload, pas la
langue cible utilisateur. Pour le moteur Rust, le runtime ecrit toujours la
langue canonique `en`, afin de ne pas dupliquer le meme payload pour `fr`, `it`,
`es`, etc.

Les sorties localisees produites par le service LLM doivent etre stockees dans
une table separee :

`astral_interpretation_generated_outputs`

Cette table fait partie du schema canonique meme lorsqu'elle ne contient encore
aucune ligne. Son absence indique une base PostgreSQL non alignee avec `json_db`
et doit etre corrigee cote schema.

La contrainte fonctionnelle proposee est :

```text
(generation_payload_id, target_language_id, prompt_contract_version, provider_code, model_code)
```

Cette table porte la langue cible finale via `target_language_id`, ainsi que le
fournisseur, le modele, la version de prompt et la sortie structuree localisee.

Si un calcul idempotent est deja `completed`, le runtime tente de reutiliser le
payload existant. Il ne le reutilise que si le contrat enrichi est present :

- 12 signaux maximum ;
- au moins un signal ;
- `dignities` structurees presentes et coherentes avec les signaux
  `dignity:*` actifs ;
- `angles` top-level present avec exactement les quatre faits canoniques
  `ascendant`, `descendant`, `mc` et `ic`, leurs axes attendus
  (`horizontal` pour Ascendant/Descendant, `vertical` pour MC/IC), leurs
  oppositions attendues (`ascendant <-> descendant`, `mc <-> ic`), une longitude
  normalisee et une maison associee valide ; un signal
  `angle:ascendant:sign:*` actif doit aussi exister ;
- `chart_emphasis` present, avec au moins une dominante de signe, de maison et
  d'objet, chacune scoree, justifiee par des raisons non vides et triee par
  score decroissant dans sa famille ;
- positions avec `sign_code`, `sign_name`, `sign_context`, `house_context`,
  `house_modality`, `object_context` et `dignity_context` sous forme de tableau ;
  `motion_context` est requis pour les objets mobiles, mais les angles peuvent
  le laisser a `null` car ils n'ont pas d'etat de mouvement planetaire ;
  `visibility_context.horizon_position_id` est requis pour toutes les positions
  et `visibility_context.altitude_deg` est requis pour les corps non-angle ;
  pour ces corps, `visibility_context.source` doit etre
  `calculated_altitude` et `visibility_context.horizon_position` doit etre
  coherent avec le signe de l'altitude (`> 0` au-dessus, `< 0` en-dessous,
  `0` sur l'horizon) ;
  pour les angles, `visibility_context.source` doit rester `angle_context` et
  `visibility_context.altitude_deg` / `visibility_context.is_visible` doivent
  rester `null` ;
- signaux avec `evidence` objet non nul ;
- signaux avec `theme_code`, `summary`, `confidence_score`,
  `interpretive_hint`, `semantic_tags`, `source_weight` et
  `aggregation_group` non nuls et non vides quand le type est textuel ;
- aucun signal actif `aspect:*` entre deux angles ; les anciens payloads qui
  contiennent un aspect angle-angle actif sont regeneres ;
- signaux `angle:*` avec `evidence.fact_type = "chart_angle"` et
  `evidence.opposite_angle_code` court non vide ainsi que
  `evidence.opposite_angle_object_code` coherent avec
  `angles[].opposite_angle_code` ;
- signaux `aspect:*` avec `aspect_context` complet, au moins un effet
  interpretatif non vide, `dynamic_quality`, `phase_state`, `valence_family`,
  `is_tonal_valence` et `is_intensity_modifier` ;
- absence de signal `aspect:*` qui represente une opposition structurelle entre
  deux angles du meme `axis`, meme si l'ancien signal n'est pas marque
  `is_structural_axis` ;
- signaux de placement avec `evidence.placement_context` complet, incluant un
  `visibility_context` mobile dont `source = "calculated_altitude"`,
  `altitude_deg` est fini, `is_visible` est booleen, et `horizon_position` est
  coherent avec le signe de l'altitude ;
- signaux de placement avec `evidence.essential_dignities` sous forme de
  tableau ;
- signaux `dignity:*` rattaches a une entree correspondante dans
  `payload.dignities` ;
- pour chaque signal `dignity:*`, coherence stricte entre son `signal_key`, son
  evidence (`chart_object`, `sign_code`, `dignity_type`) et l'entree
  correspondante dans `payload.dignities` ;
- `reading_plan` present, non vide, compose de slots uniques, ordonnes, sans
  section vide, et de sources primaires qui existent dans les signaux du
  payload ;
- `chart_context` coherent avec la secte solaire, la source de visibilite du
  Soleil, les comptes d'hemisphere, `hemisphere_emphasis.count_scope =
  "mobile_chart_objects_only"` et les positions enrichies par altitude /
  horizon ;
- `chart_context.payload_contract.contract_version = "natal_structured_v13"` ;
- `lunar_phase_context` present, angle Soleil-Lune recalcule avec tolerance
  0.01 degre, `phase_code` coherent avec l'intervalle canonique,
  `related_signal_keys` limites aux signaux actifs Soleil/Lune, et tags
  `lunar_phase` et `sun_moon_cycle` dans `semantic_tags` ;
- `accidental_dignities` present, non vide, sans entree pour les angles, avec
  au moins une condition par evaluation, codes uniques par objet, scores bornes,
  `overall_score` recalcule depuis les `score_delta`, `overall_polarity` et
  `expression_quality` coherents, `related_signal_key` egal a
  `object_position:<object_code>` quand le signal de placement est actif ;
- `positions[].accidental_dignity_context` tableau (vide pour les angles) aligne
  avec `accidental_dignities` ;
- signaux `object_position:*` avec
  `evidence.placement_context.accidental_dignity_context` recopie depuis la
  position correspondante ;
- coherence recalculee entre conditions accidentelles et faits de position
  (`house_modality`, `motion_context`, `visibility_context`, proximite angle
  <= 10 degre, secte chart) ;
- `primary_signal_keys` aligne avec `source_signal_keys`, et
  `secondary_slot_candidates` coherents avec les slots conserves du
  `reading_plan` ;
- chaque signal primaire apparait dans un seul slot de `reading_plan`; les
  candidats editoriaux supplementaires passent par `secondary_slot_candidates` ;
- slots du `reading_plan` connus et dans l'ordre canonique du payload route
  basic ;
- absence d'anciens templates connus comme `by a opposition`.

Sinon, les signaux sont reconstruits depuis les positions persistantes et les
aspects persistants relus avec leur contexte interpretatif, puis le payload est
reecrit.

## Verification

### Migration des tests a la racine

Les tests Rust sont stockes dans le repertoire racine `tests/` et declares
comme cibles de tests d'integration dans `astral_calculator/Cargo.toml`.

Les modules `src/*.rs` ne contiennent plus de bloc `#[cfg(test)]` ni
`include!`. Les helpers controles par ces tests sont exposes explicitement
quand ils font partie du contrat verifie par la suite de tests.

Fichiers de tests migres :

- `tests/aspects_tests.rs`
- `tests/dignities_tests.rs`
- `tests/facts_tests.rs`
- `tests/idempotency_tests.rs`
- `tests/main_tests.rs`
- `tests/payload_tests.rs`
- `tests/runtime_tests.rs`
- `tests/signals_tests.rs`

Depuis `astral_calculator` :

```powershell
cargo test
cargo test --features swisseph-engine
cargo clippy --features swisseph-engine -- -D warnings
.\scripts\verify_natal_v13_golden.ps1
```

Le golden v13 compare une projection stable (dont `signal_keys` triees) ; il ne
substitue pas aux tests d'orbe dans `tests/aspects_tests.rs` (voir section 3F).

Avant un run contre PostgreSQL, verifier que le schema est aligne avec `json_db`
(au minimum `astral_aspect_families.expected_aspect_count`,
`astral_aspects.default_orb_deg` pour 3F, tables de scoring et catalogue pour
v13). En cas d'erreur SQL sur une colonne manquante :

```powershell
docker compose up -d
python scripts/patch_astral_aspect_families_expected_count.py
python scripts/patch_astral_aspects_default_orb_deg.py
# ou import complet :
python scripts/import_json_db_to_postgres.py
```

Run complet avec les valeurs d'exemple :

```powershell
$env:ASTRAL_BIRTH_DATETIME_UTC = "2024-06-15T12:00:00Z"
$env:ASTRAL_LATITUDE_DEG = "48.8566"
$env:ASTRAL_LONGITUDE_DEG = "2.3522"
$env:ASTRAL_EPHEMERIS_PATH = "..\ephe\se-2026a"
cargo run --features swisseph-engine
```

Pour ecrire un exemple inspectable dans `../output` :

```powershell
cargo run --features swisseph-engine -- --file
```

Le run attendu doit afficher le payload moteur courant route par
`product_code = "basic"`. Il doit contenir :

- `product_code = "basic"` ;
- un `chart_context` top-level avec le type de theme, les IDs de referentiels,
  le contrat de projection `natal_structured_v13`, la secte et la synthese
  d'hemisphere, dont `hemisphere_emphasis.count_scope =
  "mobile_chart_objects_only"` ;
- des positions avec `sign_code`, `sign_name`, `house_number`, `house_name`,
  `sign_context`, `house_context`, `house_modality`, `object_context` et
  `dignity_context` sous forme de tableau, vide quand aucune dignite n'est
  detectee ; `motion_context` est present pour les objets mobiles et peut etre
  `null` pour les angles ; `visibility_context` expose le contexte d'horizon
  exploitable avant synthese externe, avec `altitude_deg` calcule pour les corps mobiles et
  `horizon_position_id` renseigne pour toutes les positions ; pour les corps
  mobiles, `source` doit etre `calculated_altitude`, tandis que les angles
  restent en `angle_context` avec `altitude_deg = null` et `is_visible = null` ;
- une liste `angles` top-level avec exactement Ascendant, Descendant, MC et IC,
  reliee aux signes et maisons calcules, avec les axes `horizontal` /
  `vertical` attendus et `opposite_angle_code` resolu vers le code objet long
  correspondant quand il existe dans le payload ;
- des signaux `angle:*` dont `evidence.opposite_angle_object_code` expose le meme
  code objet long, tout en conservant le code court source dans
  `evidence.opposite_angle_code` ;
- une liste `dignities` top-level, vide ou non selon le theme, mais coherente
  avec les signaux `dignity:*` actifs ;
- un `chart_emphasis` top-level avec `dominant_signs`, `dominant_houses` et
  `dominant_objects` scorees et auditees par `reasons`, sans objet
  `placement`-only quand une vraie emphase existe par dignite, cluster,
  dominante de signe ou aspect fort, et sans amplification artificielle des
  angles par leurs axes structurels ;
- un `house_axis_emphasis` top-level avec au plus trois axes de maisons
  significatifs, audites par `house_scores`, `source_signal_keys`,
  `source_context_keys` et `reasons` ;
- un `lunar_phase_context` top-level avec phase, angle Soleil-Lune, progression
  et tags `lunar_phase` / `sun_moon_cycle` ;
- un bloc `accidental_dignities` top-level pour les objets mobiles avec au moins
  une condition detectee, et `positions[].accidental_dignity_context` (tableau
  vide pour les angles) ;
- des signaux `object_position:*` dont `evidence.placement_context` inclut
  `accidental_dignity_context` aligne avec la position ;
- au plus 12 signaux ;
- un `reading_plan` non vide ;
- un `reading_plan` sans slot vide et sans opposition structurelle d'angle dans
  `main_tension_or_support` ;
- des titres sans IDs techniques ;
- des champs semantiques 1B sur chaque signal ;
- un `aspect_context` sur chaque signal `aspect:*`, avec les modificateurs
  d'intensite separes de la valence primaire, et les flags
  `is_tonal_valence` / `is_intensity_modifier` renseignes ;
- aucun signal actif `aspect:ascendant:descendant:opposition` ou
  `aspect:mc:ic:opposition` produit par les axes structurels ;
- un `evidence.placement_context` complet sur chaque signal de placement,
  incluant `visibility_context` ;
- un `evidence.essential_dignities` tableau sur chaque signal de placement ;
- des signaux `dignity:*` seulement pour les dignites majeures significatives ;
- un cluster `cluster:<sign_code>:house_<number>` quand au moins trois objets
  partagent le meme signe et la meme maison ;
- des IDs conserves dans `evidence` ;
- une ecriture/upsert dans `astral_interpretation_generation_payloads`.

Le service LLM doit ensuite composer sa requete dans un module separe, par
exemple :

```json
{
  "target_language_code": "fr",
  "payload": {
    "...": "canonical English payload produced by the Rust engine"
  }
}
```

## Limites connues

Cette section transforme les reserves historiques en **decisions produit et techniques
explicites** avant branchement d'un service LLM. Le moteur v13 ne produit pas de
redaction finale : il calcule, structure et oriente la lecture.

Chaque entree porte une etiquette :

| Etiquette | Sens |
|-----------|------|
| `assumed_v13` | Limite volontaire du contrat moteur v13 ; non bloquante avant LLM |
| `llm_boundary` | Point que la couche LLM doit comprendre ; risque de mauvaise interpretation si ignore |
| `future_story` | Extension prevue ; nom court pour le backlog |
| `deprecated` | Ancienne formulation de limite devenue fausse, obsolete ou deplacee ailleurs |

Voir aussi l'etape **3G** (fin de document) pour les criteres d'acceptation de cette passe.

### `assumed_v13` — Perimetre moteur accepte

- **Angles factuels** — Les quatre angles sont exposes en `angles[]` et via des
  signaux `angle:*` (faits structurels, tags, poids). Il n'y a pas d'interpretation
  narrative autonome des angles au-dela de ces signaux et de leur slot
  `reading_plan` (`core_identity` pour l'Ascendant, MC en contexte secondaire).
- **Pas de signaux `rulership:*`** — Le bloc `rulership_context` (3B) expose
  maitres, chaines et receptions pour ponderation externe ; le moteur n'emet pas de
  signaux actifs `rulership:*`.
- **Pas de signaux `house_axis:*`** — `house_axis_emphasis` (3C) synthetise les
  axes de maisons significatifs en top-level ; aucun signal actif
  `house_axis:*` ni slot dedie dans `reading_plan`.
- **Clusters `sign_house` uniquement** — Seuls les regroupements
  `cluster:<sign_code>:house_<n>` (au moins trois objets, meme signe et maison)
  sont agreges comme signaux actifs.
- **Dignites essentielles majeures** — Signaux `dignity:*` limites a domicile,
  exaltation, detriment et chute ; regles et poids depuis PostgreSQL via
  `BasicPayloadCatalog`, sans terme, triplicite ni face (voir `future_story`).
- **Dignites accidentelles MVP** — Couverture des 15 conditions canoniques
  (maison, proximite angle, mouvement, horizon, secte) depuis
  `astral_accidental_dignity_condition_definitions`. Combustion, cazimi, hayz
  complet et paliers fins d'orbe de proximite angle (3° / 6°) sont exclus du
  contrat v13. L'orb de proximite angle vient de
  `chart_context.accidental_scoring` (snapshot catalogue accidentel).
- **Pas de traduction** — Libelles des referentiels consommes tels quels (langue
  des seeds / base). La langue cible et la localisation appartiennent au service
  LLM (`target_language_code` en entree de couche externe).
- **Pas de redaction LLM dans le moteur** — Aucun texte final utilisateur, aucun
  `drafting_plan`, `writing_contract` ni `writing_guidance` dans le JSON de sortie
  (voir objectif en tete de document).

### `llm_boundary` — A respecter avant branchement LLM

- **`summary` et `interpretive_hint`** — Phrases templatees et indices semantiques
  produits par le moteur (`signals[]`, parfois reprises en evidence). Ce ne sont
  **pas** des textes finaux affichables : matiere structuree pour la couche LLM,
  qui ne doit pas les recopier tels quels.
- **`reading_plan`** — Sequence de slots moteur (`slot`, titres, cles de signaux
  primaires/secondaires). Plan de **lecture** des faits calcules, pas un plan
  redactionnel obligatoire ni un sommaire d'article.
- **`product_code = "basic"`** — Cle legacy de routage (tables produit, chemins
  runtime, profils de scoring). Elle ne designe plus un payload « minimal » : la
  profondeur reelle est portee par `chart_context.payload_contract`
  (`calculation_scope`, `interpretation_scope`, `projection_depth`).
- **`default_major_orb_deg`** — Toujours present sur le profil produit / catalogue
  pour coherence, mais **n'alimente plus la geometrie** depuis 3F : orbes de
  detection depuis `astral_aspects.default_orb_deg` (famille `major`). Ne pas
  reutiliser ce champ pour recalculer ou valider des aspects cote LLM.
- **Ordre de `signals[]`** — Non contractuel. L'ordre de lecture recommande vient
  de `reading_plan` (slots et `primary_signal_keys`), pas de l'index du tableau
  `signals`.

### `future_story` — Backlog nomme

| Story | Description |
|-------|-------------|
| `essential_minor_dignities` | Triplicite, terme, face ; signaux et evidence dedies |
| `solar_conditions` | Combustion, cazimi, under beams |
| `hayz_complete` | Hayz au-dela du MVP secte / horizon actuel |
| `angle_proximity_orb_tiers` | Paliers fins d'orbe de proximite angle (ex. 3° / 6°) |
| `aspect_orb_rules_runtime` | Resolution de `astral_aspect_orb_rules` et orbes par systeme dans `astral_aspect_definitions` au runtime |
| `aspect_patterns` | Hubs, patterns, T-square, grand trine, structures aspectuelles avancees |
| `cluster_enrichment` | Clusters au-dela de `sign_house` (themes, dignites, autres regroupements) |

### `deprecated` — Reserves retirees

- **« Les aspects majeurs viennent de PostgreSQL » comme limite** — Etat nominal
  post-3F, documente en section 3F ; ce n'est plus une reserve mais le referentiel
  de detection.
- **« Les resumes / hints templatees » sans nuance** — Remplace par les entrees
  `llm_boundary` ci-dessus : le moteur assume des templates, la couche LLM assume
  la transformation en prose finale.

## Organisation du module payload

`astral_calculator/src/payload.rs` a ete remplace par le dossier
`astral_calculator/src/payload/` afin de separer les responsabilites
sans modifier le contrat public `astral_calculator::payload`.

- `mod.rs` orchestre la construction du payload moteur route basic.
- `angles.rs`, `chart_context.rs`, `dignities.rs`, `emphasis.rs`,
  `house_axes.rs`, `lunar_phase.rs`, `accidental_dignities.rs`, `rulership.rs`,
  `reading_plan.rs` isolent les blocs metier du payload.
- `signal_filters.rs` centralise les predicats partages sur les signaux et
  aspects.
- `json.rs` centralise les extractions defensives depuis les payloads JSON.
- `chart_context.rs` porte le contrat moteur courant (`natal_structured_v13`
  quand les references lunaires, accidentelles et secte sont injectees).

## Etape 3B - Rulership / dispositors context

Le payload natal route basic est passe a `natal_structured_v10` pour ajouter le
bloc top-level `rulership_context`. Ce bloc est contextuel : il sert de signal
structurel pour la couche applicative ou LLM externe sans devenir une section
autonome du payload moteur.

Le moteur lit les maitres de signes depuis la base via
`astral_object_sign_dignities`, filtre les dignites de type `domicile`, puis
resout les objets, signes, versions de reference et systemes doctrinaux par
jointure. Aucune correspondance signe -> maitre n'est codee dans le moteur.

`rulership_context` expose:

- `ascendant_ruler`, `mc_ruler` et `descendant_ruler` (optionnel, schema v13 — maître du signe sur le Descendant, ex. Taureau → Vénus) ;
- les maitres des signes et maisons dominants;
- les `dispositor_links` des objets mobiles;
- les `rulership_chains` limitees a une profondeur de 6 avec detection de cycle;
- les vrais `final_dispositors`;
- les `mutual_receptions` separees des final dispositors.

Pour les signes ayant plusieurs maitres selon les systemes doctrinaux, le bloc
expose a la fois `ruler_object_codes` et `ruler_sources[].object_code`. Ainsi,
chaque source doctrinale reste explicitement rattachee au maitre qu'elle
fournit, au lieu d'exposer une liste de sources ambiguë.

`rulership_context` est consomme comme contexte de ponderation externe : le
maitre de l'Ascendant peut servir l'identite, les maitres de dominantes peuvent
servir le cluster, `mc_ruler` la direction publique / carriere, et
`descendant_ruler` la sphère relationnelle (maison 7). Ces decisions
appartiennent a la couche LLM (`astral_llm`), pas au payload moteur.

Implementation calculateur : `astral_calculator/src/payload/rulership.rs`
(`angle_ruler("descendant", "relationship_angle_ruler", …)`). Test :
`basic_payload_exposes_rulership_context_from_reference_rules` dans
`tests/payload_tests.rs`.

### Etape 3B.1 - Coherence du referentiel et endpoints

La stabilisation 3B.1 corrige le referentiel moderne dans
`json_db/astral_object_sign_dignities.json`:

- Scorpion moderne -> Pluton;
- Verseau moderne -> Uranus;
- Poissons moderne -> Neptune;
- les detriments modernes opposes sont alignes avec ces domiciles.

Le golden v10 verrouille explicitement le cas Scorpion:

```json
{
  "ruler_object_codes": ["mars", "pluto"],
  "ruler_sources": [
    { "astral_system_code": "traditional", "object_code": "mars" },
    { "astral_system_code": "modern", "object_code": "pluto" }
  ]
}
```

Les terminaisons de chaines sont separees pour eviter toute ambiguite:

- `final_dispositors` contient uniquement les objets qui disposent d'eux-memes
  en fin de chaine;
- `mutual_receptions` contient les boucles a deux objets;
- les cycles plus longs restent visibles dans `rulership_chains` avec
  `termination = "cycle"`.

La validation runtime verifie que `final_dispositors` et `mutual_receptions`
derivent exactement de `rulership_chains`. Un payload dont les endpoints sont
coherents en forme JSON mais incoherents avec les chaines est rejete par
`is_current_basic_payload`.

La relecture des payloads persistants est compatible avec cette evolution de
contrat: si un ancien payload stocke expose encore l'ancienne forme d'endpoint
qui melangeait final dispositor et reception mutuelle, il est considere comme
obsolete et le runtime reconstruit un payload courant depuis les faits
persistes au lieu d'echouer au demarrage.

La reutilisation d'un payload persiste compare aussi les `ruler_sources`
completes avec les domiciles relus depuis PostgreSQL pour la version de
reference courante: systeme, type de dignite, objet, poids et flag primaire.
Si la table `astral_object_sign_dignities` a ete resynchronisee apres une
correction de referentiel, un payload structurellement valide mais construit
avec d'anciennes sources de maitrise est rejete et reconstruit.

Artefacts ajoutes:

- `astral_calculator/schemas/natal_structured_v10.schema.json`;
- `tests/golden/natal_payload_v10_paris_1990.json`;
- `scripts/verify_natal_v10_golden.ps1`.

Ce decoupage reste volontairement simple: les donnees doctrinales restent dans
les fichiers `json_db` et sont lues par la base; aucune correspondance
signe -> maitre n'est codee en Rust. Les fonctions gardent une portee limitee
au module quand elles ne font pas partie de l'API publique.

## Organisation du module signals

`astral_calculator/src/signals.rs` a ete remplace par le dossier
`astral_calculator/src/signals/` afin de separer l'agregation des
signaux du payload route basic par responsabilite, sans modifier l'API publique
`astral_calculator::signals`.

- `mod.rs` conserve l'orchestration de `aggregate_basic_signals`.
- `constants.rs` centralise les constantes partagees du module.
- `angles.rs`, `positions.rs`, `dignity.rs`, `dignity_helpers.rs`,
  `aspect_signals.rs` et `clusters.rs` isolent la construction des familles de
  signaux.
- `limits.rs` regroupe les regles de suppression, preservation et remplissage
  de la limite du payload route basic.
- `relations.rs`, `context.rs`, `tags.rs` et `utils.rs` gardent les helpers
  transverses limites au module.

Le contrat public reste limite a:

- `aggregate_basic_signals`;
- `BASIC_MAX_ACTIVE_SIGNALS`;
- `indefinite_article`.

Les sous-modules n'utilisent pas d'import global `use super::*`: chaque fichier
declare ses dependances explicitement. Les helpers purement locaux restent
prives au fichier, et les helpers partages entre sous-modules utilisent
`pub(super)` uniquement quand c'est necessaire.

Le fichier racine `astral_calculator/src/aspects.rs` reste separe du
module `signals/aspect_signals.rs`: le premier detecte les faits d'aspects
depuis les positions calculees, alors que le second transforme un `AspectFact`
en contexte de signal du payload route basic. Les fusionner melangerait le calcul des faits et
la preparation editoriale du payload.

Ce refactor reste strictement structurel: aucune nouvelle donnee canonique n'a
ete ajoutee en dur et le comportement conserve est valide par:

```powershell
cargo fmt --manifest-path astral_calculator/Cargo.toml
cargo clippy --manifest-path astral_calculator/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path astral_calculator/Cargo.toml
```

## Organisation du module runtime

`astral_calculator/src/runtime.rs` a ete remplace par le dossier
`astral_calculator/src/runtime/` afin de separer les responsabilites
runtime sans modifier l'API publique `astral_calculator::runtime`.
Le module reste une couche d'orchestration: il ne porte pas de donnees
canoniques applicatives et ne contourne pas les referentiels lus depuis la base.

- `mod.rs` conserve les exports publics historiques :
  `ChartCalculationRuntimeService`, `RuntimeError`,
  `is_current_basic_payload`, `validate_calculation_references`,
  `validate_chart_object_signal_profiles` et `validate_house_axis_references`.
- `error.rs` isole `RuntimeError` et ses codes d'erreur stables.
- `service.rs` orchestre le calcul natal route basic, l'idempotence, la persistance
  et la regeneration des payloads obsoletes.
- `references.rs` regroupe les validations des references chargees depuis la
  base de donnees et des profils de scoring des objets.
- `payload_freshness.rs` expose la facade `is_current_basic_payload` et compose
  les validations de reutilisation.
- `payload_freshness/chart_context.rs` verifie le contrat moteur courant
  `natal_structured_v13`.
- `payload_freshness/lunar_phase.rs` verifie `lunar_phase_context` et l'angle
  Soleil-Lune recalcule.
- `payload_freshness/accidental_dignities.rs` verifie `accidental_dignities`,
  les resumes de position, la recopie dans les signaux de placement et la
  coherence conditions / faits.
- `payload_freshness/angles.rs` verifie les quatre angles canoniques et leurs
  preuves.
- `payload_freshness/aspects.rs` verifie le contexte interpretatif des aspects
  et rejette les aspects d'axes structurels ou angle-angle.
- `payload_freshness/dignities.rs` verifie la coherence entre dignites
  structurees et signaux `dignity:*`.
- `payload_freshness/emphasis.rs` verifie les dominantes de signe, maison et
  objet.
- `payload_freshness/rulership.rs` verifie la structure du contexte de maitrise
  et des chaines de dispositors.
- `payload_freshness/house_axes.rs` verifie la structure, la coherence calculee
  et les sources de `house_axis_emphasis`.
- `payload_freshness/placements.rs` verifie les contextes de positions et de
  signaux de placement, dont le `visibility_context` mobile recopie dans
  `evidence.placement_context`.
- `payload_freshness/plan.rs` verifie `reading_plan` et la coherence de ses
  sources.
- `runtime/references.rs` valide les definitions d'aspects majeurs avant calcul
  (`validate_aspect_definitions`, plafond `max_default_orb_deg` depuis la base).
  La fraicheur des payloads reutilises ne revalide pas les orbes des faits deja
  persistes dans `astral_calculated_aspects` (hors perimetre 3F).
- `payload_freshness/json.rs` et `payload_freshness/text.rs` regroupent les
  helpers transverses limites a la validation de reutilisation.

Ce decoupage reste strictement structurel: aucune nouvelle donnee canonique n'a
ete ajoutee en dur, les chemins publics existants restent disponibles, et les
tests runtime valident le comportement conserve. Les helpers internes restent
prives au fichier ou `pub(super)` quand ils sont partages entre sous-modules.

## 3C - House axis emphasis

L'etape 3C a porte le contrat a `natal_structured_v11` avec un nouveau bloc
top-level `house_axis_emphasis` (supersede par `natal_structured_v12` depuis
l'etape 3D). Ce bloc reste dans le perimetre moteur: il synthetise les
axes de maisons significatifs sans creer de slot `reading_plan`, sans signal
`house_axis:*`, et sans instruction de redaction LLM.

Les axes utilises viennent des tables canoniques existantes:

- `astral_house_axis_definitions`;
- `astral_house_axis_members`;
- `astral_houses` pour les numeros de maisons et `theme_code`.

Le repository expose `house_axis_references`, puis le runtime les injecte dans
`build_basic_payload_with_references`. Le builder `payload/house_axes.rs` croise
les references d'axes avec les positions, angles, dignites, `chart_emphasis`,
signaux actifs et `rulership_context`.

Chaque item expose:

- `axis_code`, `houses`, `theme_codes`;
- `house_scores` pour l'audit par maison;
- `primary_house`, `secondary_house`, `axis_score`, `polarity_balance`;
- `source_signal_keys`, filtres sur les signaux actifs existants;
- `source_context_keys`, `reasons`, `interpretive_hint`.

`interpretive_hint` est aligne sur `polarity_balance`: un axe dominant d'un
cote parle d'activation principale par la maison dominante et de contrepoint
secondaire par la maison opposee; un axe equilibre parle explicitement des deux
poles fortement actifs. Le hint reste factuel et moteur, sans instruction LLM.

Quand un signal d'aspect actif relie un objet place dans chaque maison de
l'axe, le builder ajoute la raison `cross_axis_aspect` au niveau de l'axe et
dans les deux entrees `house_scores[].reasons`. Cette raison n'ajoute pas de
bonus de score supplementaire: elle explique pourquoi l'aspect deja source est
structurant pour la polarite.

Les scores sont bornes entre 0 et 1. Le score d'axe combine la maison la plus
forte et une part secondaire de la maison opposee. Le payload conserve au plus
trois axes, tries par `axis_score` descendant, et ne remonte pas d'axe
`weak_axis`.

La validation runtime refuse les payloads v10 comme obsoletes et verifie:

- `chart_context.payload_contract.contract_version = "natal_structured_v11"`;
- presence et taille de `house_axis_emphasis`;
- correspondance stricte entre `axis_code`, paires de maisons et `theme_codes`
  canoniques;
- scores bornes;
- coherence calculee de `axis_score`, `primary_house`, `secondary_house` et
  `polarity_balance` avec `house_scores`;
- coherence exacte de `interpretive_hint` avec `polarity_balance`;
- coherence de `cross_axis_aspect` avec un signal d'aspect actif reliant un
  objet dans chaque maison de l'axe;
- `source_signal_keys` existants dans `signals`;
- absence de doublons dans les sources d'axe;
- tri descendant et absence d'axe faible.

Les references d'axes chargees depuis la base sont validees avant construction
du payload: le runtime attend exactement les six axes canoniques du contrat v11,
avec leurs maisons opposees et leurs themes de maisons correspondants. Une base
incomplete ou incoherente echoue donc avant persistance d'un payload v11.

Artefacts historiques de l'etape 3C :

- `astral_calculator/schemas/natal_structured_v11.schema.json`;
- `tests/golden/natal_payload_v11_paris_1990.json`;
- `scripts/verify_natal_v11_golden.ps1`;
- tests de non-regression dans `tests/payload_tests.rs`,
  `tests/runtime_tests.rs` et `tests/contract_basic_v8_tests.rs`.

Le contrat courant et ses artefacts de verification sont documentes dans les
sections 3E et 3F et dans `Fichiers concernes`.

## 3E - Accidental dignity MVP

Le contrat courant passe a `natal_structured_v13` avec un bloc top-level
`accidental_dignities` organise par objet mobile. Ce bloc expose les
conditions accidentelles deja calculables avec les donnees presentes : modalite
de maison, proximite aux angles, mouvement (retrograde et stationnaire
uniquement — pas de condition `direct_motion`), horizon local et secte MVP.

### Gate de version et references

Le builder choisit `natal_structured_v13` uniquement si les trois conditions
suivantes sont reunies :

- references lunaires injectees et `lunar_phase_context` construit ;
- references accidentelles non vides (`astral_accidental_dignity_condition_definitions`) ;
- references secte non vides (`astral_object_sect_affinities`).

Sinon le moteur reste en `natal_structured_v12` (phase lunaire seule) ou
`natal_structured_v11` (sans phase lunaire). Le service runtime charge toujours
les trois jeux de references et appelle `build_basic_payload_with_accidental_references`.

Validations au demarrage (`references.rs`) :

- exactement 15 codes de condition (`ACCIDENTAL_CONDITION_CODES`) ;
- exactement 7 objets de secte MVP (`sun`, `jupiter`, `saturn`, `moon`, `venus`,
  `mars`, `mercury`).

### Perimetre moteur strict

- pas de famille de signaux `accidental_dignity:*` ;
- pas de reranking massif de `chart_emphasis` : seule la raison
  `accidental_context` peut etre ajoutee aux objets deja presents dans
  `dominant_objects` ;
- pas de `llm_handoff_contract`, `drafting_plan` ni `writing_guidance` ;
- pas de persistance dediee dans `astral_calculated_condition_matches` ni
  `astral_calculated_dignity_evaluations` (hors scope 3E).

### Enrichissements de projection

- `accidental_dignities[]` : evaluations par objet mobile avec au moins une
  condition, triees par `object_code` ;
- `positions[].accidental_dignity_context` : resume (`condition_code`,
  `condition_family`, `polarity`, `strength_score`) ; tableau vide pour les
  angles ;
- `signals[].evidence.placement_context.accidental_dignity_context` pour les
  signaux actifs `object_position:*` uniquement (les signaux `angle:*` ne
  portent pas ce champ dans le schema v13).

Structure type d'une evaluation :

```json
{
  "object_code": "mars",
  "object_name": "Mars",
  "overall_score": 0.83,
  "overall_polarity": "fortified",
  "expression_quality": "strong_external_manifestation",
  "related_signal_key": "object_position:mars",
  "conditions": [
    {
      "condition_code": "angular_house",
      "condition_family": "house_modality",
      "polarity": "dignity",
      "strength_score": 0.75,
      "score_delta": 0.25,
      "source": { "house_modality_code": "angular" },
      "interpretive_hint": "Object placed in an angular house."
    }
  ]
}
```

### Conditions MVP (canon DB)

| Code | Famille | Declencheur moteur |
|------|---------|-------------------|
| `angular_house` | `house_modality` | `house_modality_code = angular` |
| `succedent_house` | `house_modality` | `succedent` |
| `cadent_house` | `house_modality` | `cadent` |
| `near_ascendant` | `angle_proximity` | distance <= 10 degre a l'Ascendant |
| `near_descendant` | `angle_proximity` | idem Descendant |
| `near_mc` | `angle_proximity` | idem MC |
| `near_ic` | `angle_proximity` | idem IC |
| `retrograde_motion` | `motion` | `motion_context.is_retrograde = true` |
| `stationary_motion` | `motion` | `motion_context.is_stationary = true` |
| `above_horizon` | `horizon` | `visibility_context.horizon_position_id` |
| `below_horizon` | `horizon` | idem |
| `on_horizon` | `horizon` | idem |
| `sect_affinity_match` | `sect` | affinite objet = `chart_context.sect.chart_sect` |
| `sect_affinity_mismatch` | `sect` | affinite opposee |
| `sect_affinity_variable_unresolved` | `sect` | affinite variable non resolue (ex. Mercure) |

Les `score_delta` et `strength_score` viennent exclusivement de
`json_db/astral_accidental_dignity_condition_definitions.json`. L'orbe de
proximite aux angles et la baseline `0.5` viennent de
`astral_accidental_scoring_params` ; les paliers de polarite globale viennent de
`astral_accidental_overall_polarity_bands`. Les paliers d'orb 3 degre / 6 degre
avec scores differencies ne sont pas implementes (MVP : un seul orbe max).

Detection des angles pour la proximite : longitude depuis `positions` dont
`role` ou `role_label` indique un angle (`is_angle` dans le builder et la
freshness).

Score global par objet :

- `raw_score = somme(score_delta)` ;
- `overall_score = round4(clamp(0.5 + raw_score, 0.0, 1.0))` ;
- `overall_polarity` (seuils inclusifs sur `overall_score`) :
  - `fortified` : score >= 0.70 ;
  - `mixed_or_contextual` : 0.45 <= score < 0.70 ;
  - `weakened` : 0.30 <= score < 0.45 ;
  - `strongly_weakened` : score < 0.30.
  Exemple golden Paris : Mercure a `overall_score = 0.28` avec
  `cadent_house`, `retrograde_motion` et `below_horizon` => `strongly_weakened`.
  Pluton, lui, est `fortified` (`angular_house`, `near_ascendant`).
- `expression_quality` derive de la polarite (`strong_external_manifestation`,
  etc.) ;
- `related_signal_key = object_position:<code>` seulement si le signal de
  placement est actif dans les 12 signaux retenus.

### Reutilisation runtime (`is_current_basic_payload`)

Les payloads v12 sans `accidental_dignities` sont obsoletes et regeneres.
La freshness `accidental_dignities.rs` verifie notamment :

- bloc non vide, sans evaluation pour un angle ;
- codes de condition connus, uniques par objet ;
- `overall_score` recalcule depuis les deltas ;
- alignement `accidental_dignities` <-> `positions[].accidental_dignity_context`
  <-> `signals[].evidence.placement_context.accidental_dignity_context` ;
- recalcul des conditions depuis les faits de position (maison, mouvement,
  horizon, orb angle, secte).

### Golden Paris 1990 (v13)

Fixture : `tests/golden/natal_payload_v13_paris_1990.json`.

Cas verifies dans `tests/contract_basic_v8_tests.rs` :

- Mars : `angular_house`, `sect_affinity_match` ;
- Jupiter : `retrograde_motion` ;
- Pluton : `near_ascendant`, `overall_polarity = fortified` ;
- Mercure : `overall_score = 0.28`, `overall_polarity = strongly_weakened` (seuil
  `< 0.30`, pas `weakened`) ;
- aucune entree accidentelle pour les angles ;
- rejet runtime des payloads v12, scores incoherents, codes inconnus, doublons
  de conditions, contextes signaux/positions desynchronises.

### Review adversariale (corrections integrees)

- gate v13 exige les trois jeux de references, pas seulement les conditions
  accidentelles ;
- validation stricte des 15 codes, familles, polarites, bornes de score ;
- sync obligatoire signaux `object_position:*` <-> positions ;
- detection angle alignee builder + freshness (`role` + `role_label`) ;
- schema v13 : `accidental_dignity_context` requis sur preuve de placement
  objet, pas sur signaux angle ;
- tests negatifs supplementaires dans `contract_basic_v8_tests.rs` et
  `payload_tests.rs`.

### Hors perimetre 3E

- combustion, cazimi, under beams, hayz complet, rejoicing ;
- conditions heliacales fines, vitesse relative avancee ;
- dignites mineures essentielles (terme, triplicite, face) ;
- aspects aux maitres ou dispositors comme dignite accidentelle ;
- tables de persistance `astral_calculated_condition_matches` /
  `astral_calculated_dignity_evaluations`.

### Artefacts

- `astral_calculator/src/payload/accidental_dignities.rs` ;
- `astral_calculator/src/runtime/payload_freshness/accidental_dignities.rs` ;
- `astral_calculator/schemas/natal_structured_v13.schema.json` ;
- `tests/golden/natal_payload_v13_paris_1990.json` ;
- `scripts/verify_natal_v13_golden.ps1` ;
- tests dans `tests/payload_tests.rs`, `tests/runtime_tests.rs`,
  `tests/contract_basic_v8_tests.rs`.

## 3F - Orbes d'aspects canoniques

L'etape 3F ne change pas le contrat JSON (`natal_structured_v13` reste courant).
Elle verrouille la source des orbes de **detection** des aspects majeurs sur la
base de donnees, conformement a la regle « canonique = PostgreSQL ». Elle ne
modifie ni `aspect_context` (2C), ni le filtrage des signaux actifs au-dela de
ce qui existait deja via `aspect_min_strength` du profil produit.

### Perimetre moteur

- catalogue des aspects majeurs entierement en base (`astral_aspects` +
  `astral_aspect_families`) : codes, angles, orbes, effectif attendu, plafond
  d'orbe par famille ;
- orbes de detection par aspect depuis `astral_aspects.default_orb_deg` ;
- validation stricte du referentiel majeur avant calcul ephemeride ;
- aucune constante Rust de liste de codes/angles ni de plafond d'orbe (`15` vit
  dans `json_db/astral_aspect_families.json`) ;
- pas de repli `default_major_orb_deg` du profil dans `detect_aspects` ;
- pas de nouveau bloc top-level, pas de signal `aspect:*` supplementaire, pas de
  changement de `reading_plan`.

### Referentiel

Sources canoniques :

| Table / seed | Role |
|--------------|------|
| `astral_aspect_families` / `json_db/astral_aspect_families.json` | Metadonnees par famille (`expected_aspect_count`, `max_default_orb_deg`) |
| `astral_aspects` / `json_db/astral_aspects.json` | Definition de chaque aspect (`code`, `angle`, `family`, `default_orb_deg`) |

Metadonnees famille `major` (seed actuel) :

| Champ | Valeur seed |
|-------|-------------|
| `expected_aspect_count` | 5 |
| `max_default_orb_deg` | 15.0 |

Lignes `astral_aspects` avec `family = 'major'` (seed actuel) :

| `code` | `angle` | `default_orb_deg` (seed) |
|--------|---------|--------------------------|
| `conjunction` | 0 | 8.0 |
| `sextile` | 60 | 6.0 |
| `square` | 90 | 6.0 |
| `trine` | 120 | 6.0 |
| `opposition` | 180 | 8.0 |

Les aspects `family != 'major'` ne sont pas lus par `aspect_definitions()` et
restent hors detection MVP.

Distinction importante :

- **orbe de detection** : `astral_aspects.default_orb_deg` (3F) ;
- **seuil signal actif** : `astral_basic_product_scoring_profiles.aspect_min_strength`
  (deja migre dans le catalogue produit) ;
- **`default_major_orb_deg`** : toujours present sur le profil `basic` / v13 pour
  coherence du catalogue, mais n'est plus passe a `EphemerisEngine::calculate_natal`
  ni utilise par `detect_aspects`.

Les orbes observes (`AspectFact.orb_deg`, `calculation_notes_json.orb_limit_deg`)
restent des faits runtime calcules a partir des longitudes ; ils ne sont pas
persistes comme referentiel dans `astral_aspect_orb_rules`.

### Runtime

Point d'entree : `ChartCalculationRuntimeService::calculate_natal_basic` dans
`runtime/service.rs`.

Chaine liee aux orbes (apres validation des profils de signaux des objets actifs) :

1. `major_aspect_family_reference()` — lit `expected_aspect_count` et
   `max_default_orb_deg` pour `name = 'major'` ;
2. `aspect_definitions()` — `SELECT` sur `astral_aspects` avec `INNER JOIN astral_aspect_families`
   (`family = 'major'`), exposant aussi `max_default_orb_deg` sur chaque ligne ;
3. `basic_payload_catalog(product_code, "natal_structured_v13", reference_version_id)` ;
4. `validate_aspect_definitions(&aspects, product_orb, expected_count, max_orb)` :
   - effectif des lignes chargees = `expected_aspect_count` ;
   - chaque ligne a `family = 'major'`, `max_default_orb_deg` coherent avec la
     famille, `id`/`code` uniques, `name` non vide ;
   - `angle` fini dans `[0, 180]` (depuis `astral_aspects`) ;
   - `default_orb_deg` present, fini, dans `(0, max_default_orb_deg]` ;
   - `default_major_orb_deg` du profil : sanity check uniquement (`<= max_orb`).
5. `EphemerisEngine::calculate_natal(..., &aspect_definitions, ...)` — dans
   `ephemeris.rs`, `detect_aspects(&positions, aspects)` lit chaque orbe via
   `canonical_aspect_orb_deg` et propage `aspect.family` dans `AspectFact.aspect_family` ;
6. persistance des faits dans `astral_calculated_aspects` (`orb_deg` observe,
   `calculation_notes_json.orb_limit_deg` = orbe canonique applique) ;
7. relecture SQL + enrichissement interpretatif (2C) avant signaux `aspect:*`.

Reutilisation d'un calcul `completed` : les aspects geometriques deja persistes
ne sont pas recalcules ; seuls les payloads juges obsoletes repassent par le moteur
complet.

Fonctions et modules :

| Role | Fichier |
|------|---------|
| Famille majeurs | `repositories.rs` — `major_aspect_family_reference()` |
| Lecture referentiel | `repositories.rs` — `aspect_definitions()` (JOIN famille) |
| Validation | `runtime/references.rs` — `validate_aspect_definitions` |
| Detection | `aspects.rs` — `canonical_aspect_orb_deg`, `detect_aspects` |
| Calcul natal | `ephemeris.rs` — `SwissEphemerisEngine::calculate_natal` (sans `default_major_orb_deg`) |
| Orchestration | `runtime/service.rs` — `calculate_natal_basic` |

Erreurs typiques :

- SQL `column "default_orb_deg" does not exist` → schema PostgreSQL non aligne ;
- `expected N major aspect definitions from astral_aspect_families, found M` →
  effectif `astral_aspects` (famille major) different de
  `astral_aspect_families.expected_aspect_count` ;
- `missing major aspect family reference` / `missing expected_aspect_count` /
  `invalid max_default_orb_deg for major aspect family` → metadonnees famille
  absentes ou invalides ;
- `inconsistent max_default_orb_deg for major aspect ...` → JOIN incoherent entre
  aspect et famille ;
- `missing default_orb_deg for major aspect ...` / `invalid default_orb_deg ...` ;
- `invalid angle for major aspect ...` → angle hors `[0, 180]` ou non fini ;
- `invalid product default_major_orb_deg (sanity check only; ...)` → profil
  `astral_basic_product_scoring_profiles` incoherent, independant des orbes par aspect.

Synchronisation PostgreSQL :

```powershell
docker compose up -d
python scripts/patch_astral_aspect_families_expected_count.py
python scripts/patch_astral_aspects_default_orb_deg.py
# ou, si plusieurs tables/colonnes manquent :
python scripts/import_json_db_to_postgres.py
```

Les scripts de patch utilisent `docker compose exec postgres psql` lorsque `psql`
n'est pas installe sur l'hote (cas habituel avec Postgres dans Docker). Un `psql`
local reste utilise s'il est dans le `PATH` et que `DATABASE_URL` est defini.

Le script `patch_astral_aspect_families_expected_count.py` assure
`expected_aspect_count` et `max_default_orb_deg` (major : 5 et 15°).
`patch_astral_aspects_default_orb_deg.py` assure `default_orb_deg` sur chaque
aspect, verifie l'effectif des majeurs contre `expected_aspect_count`, et refuse
tout `default_orb_deg` strictement superieur au `max_default_orb_deg` de la
famille `major`. L'import complet recree les tables `json_db` (DROP + INSERT).

### Hors perimetre 3F

- resolution prioritaire de `astral_aspect_orb_rules` (paires, luminaires, angles,
  contexte `natal`) ;
- aspects mineurs et avances ;
- modification du contrat v13, des familles de signaux ou de `aspect_context` ;
- revalidation des orbes sur payloads reutilises deja `completed` (les faits
  `astral_calculated_aspects` restent ceux du calcul initial).

### Tests

Le golden `tests/golden/natal_payload_v13_paris_1990.json` et
`scripts/verify_natal_v13_golden.ps1` ne suffisent pas a verrouiller les cinq
orbes : le budget `max_active_signals` (12) ne laisse en pratique qu'un petit
nombre de signaux `aspect:*` actifs sur Paris (souvent un seul, par ex. opposition
avec `orb_limit_deg = 8.0` dans les notes de calcul). La projection golden trie
les `signal_keys` avant diff (ordre non contractuel).

Couverture dediee 3F :

**`tests/common/json_db.rs`** — charge `json_db/astral_aspects.json` et
`json_db/astral_aspect_families.json` via `include_str!` (pas de derive entre
fixtures et seed).

**`tests/aspects_tests.rs`** :

- `json_db_seed_major_aspects_match_runtime_validation` — le seed JSON passe
  `validate_aspect_definitions` (effectif + `max_default_orb_deg` famille) ;
- `canonical_major_aspect_orbs_match_json_db_seed` — seuil inclus / exclu et
  `orb_limit_deg` pour chaque majeur du seed ;
- `detect_aspects_applies_each_canonical_orb_with_full_major_set` ;
- `detect_aspects_uses_per_aspect_orb_not_product_fallback` ;
- `canonical_aspect_orb_deg_rejects_orb_above_family_max` ;
- `aspect_phase_uses_relative_speed` ;
- `structural_angle_axes_are_not_detected_as_aspects`.

**`tests/runtime_tests.rs`** :

- `validate_aspect_definitions_accepts_canonical_major_orbs` ;
- `validate_aspect_definitions_rejects_missing_orb` ;
- `validate_aspect_definitions_rejects_zero_orb` ;
- `validate_aspect_definitions_rejects_excessive_orb` ;
- `validate_aspect_definitions_rejects_non_finite_orb` ;
- `validate_aspect_definitions_rejects_duplicate_aspect_id` ;
- `validate_aspect_definitions_rejects_incomplete_major_set` ;
- `validate_aspect_definitions_rejects_extra_major_aspect_count` ;
- `validate_aspect_definitions_rejects_invalid_major_aspect_angle` ;
- `validate_aspect_definitions_rejects_inconsistent_family_max_orb` ;
- `validate_aspect_definitions_rejects_invalid_product_fallback`.

**Catalogue de tests** : `catalog::test_catalog()` expose encore
`default_major_orb_deg: 8.0` pour les builders sans DB ; ce n'est pas le chemin
production et n'alimente pas `detect_aspects`.

### Artefacts

- `json_db/astral_aspects.json` ;
- `json_db/astral_aspect_families.json` ;
- `scripts/patch_astral_aspect_families_expected_count.py` ;
- `scripts/patch_astral_aspects_default_orb_deg.py` ;
- `astral_calculator/src/aspects.rs` ;
- `astral_calculator/src/runtime/references.rs` ;
- `astral_calculator/src/runtime/mod.rs` ;
- `astral_calculator/src/runtime/service.rs` ;
- `astral_calculator/src/ephemeris.rs` ;
- `astral_calculator/src/repositories.rs` ;
- `tests/common/json_db.rs` ;
- `tests/aspects_tests.rs` ;
- `tests/runtime_tests.rs` ;
- `scripts/verify_natal_v13_golden.ps1` (projection stable, complementaire).

## 4A — Engine input contract and dual output envelope

Objectif : separer explicitement la vue **audit** (payload moteur exhaustif) de la
vue **LLM** (projection stable, epuree, parametrable par niveau), sans que le
niveau de projection influence le calcul brut.

### Contrats JSON

- `astral_calculator/schemas/astro_engine_request_v1.schema.json` :
  entree moteur (`calculation.type`, `birth.date/time/timezone`, localisation,
  `projection.level`) ;
- `astral_calculator/schemas/astro_engine_response_v1.schema.json` :
  enveloppe de sortie (`request_echo`, `calculation_result`, `audit_payload`,
  `llm_payload`) ;
- `astral_calculator/schemas/llm_projection_natal_v1.schema.json` :
  structure fixe de la projection LLM (cles stables ; la richesse varie par
  tableaux et limites, pas par schema).

### Donnees canoniques

- `json_db/astral_llm_projection_profiles.json` : profils nommes
  `compact`, `standard`, `rich`, `expert` pour `llm_projection_natal_v1`
  (limites documentees en table dans la section **4B** ;
  `max_background_placements`, `max_accidental_conditions_per_object` ajoutes en 4B).

### Code

- `astral_calculator/src/engine/` : types requete/reponse, resolution
  timezone (`chrono-tz`), mapping vers `NatalChartInput`, assemblage de
  `AstroEngineResponse` ;
- `astral_calculator/src/llm_projection/` : mapper
  `natal_structured_v13` → `llm_projection_natal_v1` (voir section **4B** pour
  l'architecture `builder` / `dynamics` / `clean_text` et les regles de
  humanisation) ;
- `ChartCalculationRuntimeService::calculate_natal_engine` : calcule toujours le
  payload audit complet via `calculate_natal_basic`, puis projette selon
  `projection.level` uniquement pour `llm_payload`.

### Regles

- `projection.level` ne modifie pas `audit_payload` ni le calcul ephemerides ;
- `audit_payload.payload` reste le contrat `natal_structured_v13` courant ;
- `llm_payload` est en anglais, sans langue cible, ton ni consigne redactionnelle ;
- entree natal stricte : `date` + `time` + `timezone` + latitude/longitude
  (pas de deduction implicite de fuseau).

### Tests

- `tests/engine_contract_tests.rs` : contrats 4A/4B (schema, enveloppe, invariance
  `audit_payload`, goldens LLM et enveloppe rich). Liste complete des cas 4B dans
  la section **4B — Tests**.

Regeneration des goldens LLM :

```bash
cd astral_calculator
UPDATE_LLM_GOLDENS=1 cargo test --test engine_contract_tests write_llm_projection_goldens_when_env_set
```

### Sortie CLI (validation artefact 4A)

Par defaut, `cargo run --features swisseph-engine` appelle `calculate_natal_engine`
et ecrit une enveloppe `astro_engine_response_v1` (`output/astro_engine_response_*.json`
avec `--file`). Ce JSON contient `request_echo`, `calculation_result`, `audit_payload`
et `llm_payload`.

Variables utiles :

- `ASTRAL_PROJECTION_LEVEL` : `compact` | `standard` | `rich` | `expert`
- `ASTRAL_BIRTH_DATE` + `ASTRAL_BIRTH_TIME` + `ASTRAL_BIRTH_TIMEZONE` (entree stricte 4A)
- ou `ASTRAL_BIRTH_DATETIME_UTC` + `ASTRAL_BIRTH_TIMEZONE` (repli, timezone defaut `UTC`)
- `ASTRAL_OUTPUT_CONTRACT=engine` (defaut) ou `audit` pour forcer le payload v13 brut

Chemin legacy audit seul (scripts `verify_natal_v13_golden.ps1`) :

```powershell
cargo run --features swisseph-engine -- --audit-only
```

Verification enveloppe 4A :

```powershell
.\scripts\verify_engine_response_4a.ps1 -ProjectionLevel rich
.\scripts\verify_engine_response_4a.ps1 -ProjectionLevel compact
.\scripts\verify_engine_response_4a.ps1 -ProjectionLevel standard
```

Pour comparer les niveaux : `audit_payload` doit rester identique entre deux runs
qui ne changent que `ASTRAL_PROJECTION_LEVEL` ; seul `llm_payload` varie en richesse.

### Criteres d'acceptation 4A

1. Schemas `astro_engine_request_v1` et `astro_engine_response_v1` presents.
2. Le moteur accepte une requete JSON avec `calculation.type`, naissance,
   timezone, localisation et `projection.level` (natal seul pour l'instant).
3. La reponse contient toujours `audit_payload` (v13 brut) et `llm_payload`
   (projection epuree).
4. `projection.level` ne change pas le calcul brut.
5. `projection.level` ne change que la richesse de `llm_payload`.
6. `llm_payload` sans IDs techniques ni redondances id/code/name.
7. Pas de langue, ton ni consigne redactionnelle dans `llm_payload`.
8. `llm_payload.contract_version = "llm_projection_natal_v1"`.
9. Tests golden compact / standard / rich avec meme structure, contenus plus ou
   moins riches.

### Revue adversariale (correctifs 4A)

Findings traites dans le code :

- **Profil compact** : `ruler` ascendant et `relationship_network` desormais
  coupes quand `include_rulership_details = false` (avant : ruler encore present).
- **Scores** : `overall_score` / `strength_score` omis sauf profil `expert`
  (`include_scores`).
- **Limites effectives** : troncature `reasons`, `reading_order.focus`,
  `essential_dignities` (top N par force), `house_axes` avec libelles canoniques
  (`axis_labels` depuis `json_db`, pas `axis_code` brut).
- **Conditions** : deduplication + secte correcte (`Day/Night sect match`, pas
  toujours « Night »).
- **Profils DB** : `resolve_projection_profile` — repli seed uniquement si table
  absente ou profil inconnu en base, plus de `unwrap` masquant les erreurs DB.
- **Reference version** : `default_reference_version_id()` au lieu de `1` en dur.
- **Idempotence client** : `client_idempotency_key` dans `NatalChartInput` et
  document d'idempotence stable.
- **Validation** : `validate_request_early` avant les requetes SQL.
- **House system** : libelle `name` depuis la base, pas capitalisation du code.
- **Tests** : timezone Paris/UTC, regles compact, idempotence client, goldens
  regeneres.

## 4B — LLM projection quality hardening

Objectif : rendre `llm_payload` **exploitable par un futur service LLM** — propre,
stable, sans IDs techniques, sans redondances — sans modifier le calcul
astrologique, sans toucher a `audit_payload`, et sans appel LLM.

Contrat conserve : `llm_projection_natal_v1` (pas de bump v2). Les sections
top-level restent identiques entre `compact`, `standard`, `rich` et `expert` ;
seule la **richesse du contenu** change.

```text
audit_payload.payload  → natal_structured_v13 brut, technique, auditable
llm_payload            → llm_projection_natal_v1 epure, lisible, stable
```

### Perimetre strict

- pas de modification du pipeline `payload/` (calcul v13) ;
- pas de reranking des signaux ;
- pas de langue cible, ton, prompt ni `writing_guidance` dans `llm_payload` ;
- la projection ne recopie jamais le brut : elle **traduit** les faits techniques.

### Architecture code

| Fichier | Role |
|---------|------|
| `src/llm_projection/mod.rs` | Point d'entree, exports publics |
| `src/llm_projection/builder.rs` | Orchestration des sections |
| `src/llm_projection/dynamics.rs` | Aspects majeurs + phase lunaire |
| `src/llm_projection/clean_text.rs` | Humanisation, limites keywords, labels |
| `src/llm_projection/humanize.rs` | Re-export `clean_text` (compat) |
| `src/llm_projection/profiles.rs` | Profils seed + `merge_seed_limits` |
| `src/llm_projection/types.rs` | Structures serde de la projection |
| `src/llm_projection/axis_labels.rs` | Libelles d'axes depuis `json_db` |

Point d'entree : `build_llm_projection_natal_v1(payload, profile, ctx)`.

`build_dynamics` est appele **une seule fois** par build ; le resultat est
reutilise pour `dynamics`, `reading_order` (slot Main dynamic) et
`keywords.by_area.dynamics` (coherence garantie).

### Profils de projection (donnees canoniques)

Source : `json_db/astral_llm_projection_profiles.json` (table
`astral_llm_projection_profiles` en base, repli seed si table absente ou profil
inconnu).

`resolve_projection_profile` charge la base puis applique `merge_seed_limits` :
les champs `max_background_placements` et
`max_accidental_conditions_per_object` sont toujours alignes sur le seed (les
lignes DB anciennes peuvent ne pas porter ces colonnes).

| Niveau | keywords | supporting | background | aspects | axes | accidental | rulership | degres | scores |
|--------|----------|------------|------------|---------|------|------------|-----------|--------|--------|
| compact | 3 | 3 | 0 | 1 | 1 | non (0 lignes) | non | non | non |
| standard | 6 | 5 | 3 | 2 | 2 | oui (max 3 cond./objet) | oui | non | non |
| rich | 12 | 8 | 5 | 3 | 3 | oui (max 4) | oui | oui | non |
| expert | 20 | 10 | 8 | 5 | 3 | oui (max 6) | oui | oui | oui |

`projection_limits.effective_limits` expose aussi `max_background_placements` et
`max_accidental_conditions_per_object` pour transparence runtime.

**Reserve** : `max_core_placements` est dans le schema/profils mais non branche
sur `placements.primary` (vide par design : Soleil/Lune sont dans
`core_identity` uniquement).

### Mapping par section

#### `dynamics.major_aspects`

Selection (tous requis) :

```text
signal_key starts with "aspect:"
evidence.fact_type = "aspect"
aspect_context != null
aspect_family = "major"   (evidence ou aspect_context)
```

Tri par `priority_score` decroissant, troncature `max_aspects`.

Objet projete :

```json
{
  "aspect": "Jupiter opposition Uranus",
  "objects": ["Jupiter", "Uranus"],
  "quality": "Tension",
  "valence": "Polarizing",
  "orb_degrees": 0.76,
  "phase": "Separating",
  "keywords": ["growth", "freedom", "..."]
}
```

- `aspect` ← `signal.title`
- `objects` ← `evidence.source_object_name` + `target_object_name`
- `quality` ← `aspect_context.dynamic_quality` humanise
- `valence` ← `primary_valence` ou `intensity_modifier` humanise
- `orb_degrees` ← `evidence.orb_deg` arrondi 2 decimales (**pas** dans `aspect_context`)
- `phase` ← `evidence.phase_state` humanise
- `keywords` ← tags semantiques nettoyes + max **2** keywords signe par planete

Cas golden Paris 1990 (`rich`) : opposition Jupiter–Uranus presente avec
`quality = Tension`, `valence = Polarizing`, `orb_degrees ≈ 0.76`, `phase = Separating`.

#### `dominant_themes`

| Ancien (4A) | 4B |
|-------------|-----|
| `sign` / `strength` / `reasons` | `name` / `importance` / `supporting_factors` |
| `house` imbrique | `number` + `theme` |
| `object` | `name` |

`importance` : `Very high` / `High` / `Moderate` / `Low` (seuils 0.85 / 0.65 / 0.45).
`supporting_factors` : codes `reasons` du brut passes par `humanize_reason` (voir
table ci-dessous), dedupliques, limites par `max_keywords_per_item`.
`score` : uniquement si `include_scores` (profil `expert`).

#### `strengths`

- `essential_dignities[]` : toutes les dignites du payload (tri force), champs
  `object`, `dignity`, `sign`, `meaning` (plus `effect`).
- `accidental_conditions[]` : si `include_accidental_conditions` ; labels
  humanises, dedupliques (casse ignoree), plafond
  `max_accidental_conditions_per_object` ; `overall_score` seulement en `expert`.

#### `relationship_network`

Present si `include_rulership_details` (standard+). Forme epuree :

- `ascendant_ruler` : `ascendant_sign`, `traditional_ruler`, `modern_ruler`,
  `main_ruler_placement` (noms planetes depuis positions, pas codes bruts).
- `midheaven_ruler` : `midheaven_sign`, `ruler`, `ruler_placement`.
- `final_dispositors[]` : `{ object, source_objects[] }`.
- `mutual_receptions[]` : `{ objects[], source_objects[] }`.

Exclus de la projection : `context_key`, `ruler_sources`, `astral_system_id`,
`dispositor_signal_key`, etc.

#### `house_axes`

Libelle d'axe via `axis_labels` (seed `astral_house_axis_definitions.json` ou
refs runtime). Champs : `axis`, `houses[]`, `balance`, `importance`, `summary`,
`supporting_factors` (raisons humanisees, limite `max_keywords_per_item`).

#### `reading_order`

Sections lisibles (plus de `slot` / `primary_signal_keys` dans la sortie) :

| Slot moteur | Section LLM |
|-------------|-------------|
| `core_identity` | Core identity |
| `dominant_cluster` | Dominant theme |
| `main_tension_or_support` | Main dynamic |
| `expression_style` | Expression style |
| `background_factors` | Background factors |

`focus` : titres de signaux ou synthese (ex. « Capricorn emphasis », « Jupiter
opposition Uranus » depuis le premier `major_aspect`), jamais de `signal_key`.

#### `placements`

- `primary` : vide (Soleil/Lune exclus — deja dans `core_identity`).
- `supporting` puis `background` : autres planetes mobiles, triees par priorite
  metier, limites `max_supporting_placements` / `max_background_placements`.
- `conditions` : humanisees, plafonnees comme les accidentelles ; mouvement
  via `humanize_motion_label` (`Direct` → `Direct motion`, etc.).
- `longitude_deg` : si `include_degrees` (`rich`, `expert`).

#### `keywords`

- `main` : deduplication, limite `max_keywords_per_item * 2`.
- `by_area` : cles lisibles (`identity`, `resources`, `roots`, …) ;
  sous-bloc `dynamics` alimente depuis les aspects deja calcules.
- Termes techniques (`cadent`, `sect`, …) filtres sauf niveau `expert`
  (**pas** lie a `include_scores`).

### Humanisation (`clean_text.rs`)

**Raisons dominantes** (extrait) :

| Code brut | Libelle LLM |
|-----------|-------------|
| `sign_house_cluster` | Several chart factors are concentrated in the same sign and house |
| `saturn_domicile` | Saturn in domicile |
| `strong_aspect_participant` | Involved in a strong major aspect |
| `accidental_context` | Reinforced or modified by accidental conditions |
| `cross_axis_aspect` | A major aspect connects both sides of this house axis |
| `sun_luminary_in_house` | The Sun highlights this house |
| `rulership_context` | Supported by rulership links |

**Conditions accidentelles** (extrait) :

| Code | Libelle |
|------|---------|
| `cadent_house` | Cadent house |
| `retrograde_motion` | Retrograde motion |
| `near_ascendant` | Close to the Ascendant |
| `sect_affinity_mismatch` | Sect mismatch: does not match the chart's day/night sect |
| `sect_affinity_variable_unresolved` | Variable sect affinity |

### Cles interdites dans `llm_payload`

La projection ne doit pas contenir (test `assert_no_technical_ids`) :

`signal_key`, `*_id`, `*_code`, `source_weight`, `priority_score`,
`confidence_score`, `aggregation_group`, `evidence`, `chart_object_id`,
`reference_version_id`, `product_code`, `context_key`, `ruler_sources`,
`axis_code`, `primary_signal_keys`, `slot`, etc.

Pas de `target_language`, `"tone"`, `prompt`, `writing_guidance` dans le JSON
serialise.

### Tests (`tests/engine_contract_tests.rs`)

Structure et schema :

- `llm_projection_levels_share_identical_structure`
- `llm_projection_golden_compact_standard_rich`
- `engine_response_envelope_shape_from_v13_golden`
- `audit_payload_identical_across_projection_levels_in_envelope`

Aspects :

- `llm_projection_includes_active_major_aspect`
- `llm_projection_maps_jupiter_uranus_opposition`

Humanisation et dedup :

- `llm_projection_humanizes_dominant_theme_reasons`
- `llm_projection_humanizes_accidental_conditions`
- `llm_projection_reading_order_has_no_signal_keys`
- `llm_projection_placements_exclude_core_luminaries`
- `llm_projection_accidental_conditions_are_deduplicated`

Niveaux :

- `non_expert_does_not_include_scores` / `expert_may_include_scores`
- `compact_has_fewer_keywords_than_rich` / `compact_has_fewer_placements_than_rich`
- `compact_has_zero_background_placements`

### Goldens

- `tests/golden/llm_projection_natal_v1_paris_1990_{compact,standard,rich}.json`
- `tests/golden/astro_engine_response_v1_paris_1990_rich.json` (enveloppe complete ;
  construit depuis le golden v13 en test, pas un run moteur DB complet)

Regeneration :

```powershell
cd astral_calculator
$env:UPDATE_LLM_GOLDENS = "1"
$env:UPDATE_ENGINE_RESPONSE_GOLDEN = "1"
cargo test --test engine_contract_tests write_
```

Validation locale :

```powershell
cd astral_calculator
cargo test --test engine_contract_tests
cargo clippy --features swisseph-engine -- -D warnings
```

### Criteres d'acceptation 4B

1. `llm_payload.dynamics.major_aspects` contient l'opposition Jupiter–Uranus en
   golden Paris 1990 (`rich`).
2. Aucun champ technique interdit dans `llm_payload`.
3. Raisons et conditions humanisees (pas de codes `snake_case` residuels dans les
   blocs publics).
4. `reading_order` sans cles de signaux.
5. `relationship_network` sans champs rulership techniques.
6. Meme structure JSON pour tous les niveaux ; richesse variable seulement.
7. `audit_payload` identique entre niveaux pour une meme entree.
8. `cargo test`, `cargo test --features swisseph-engine`, clippy `-D warnings`,
   goldens LLM et enveloppe rich passent.

### Revue adversariale — synthese des correctifs (4A + 4B)

**4A (enveloppe)** : profil compact sans rulership ; scores reserves a `expert` ;
limites effectives ; secte jour/nuit ; validation early ; alignement CLI engine/audit.

**4B (qualite projection)** :

| Finding | Correctif |
|---------|-----------|
| `major_aspects` vide (`orb` cherche dans `aspect_context`) | `orb_degrees` depuis `evidence.orb_deg` |
| Triple calcul `build_dynamics` | Un seul appel, partage reading_order/keywords |
| Soleil/Lune dupliques dans `placements` | Exclus de `placements` (present dans `core_identity`) |
| Keywords d'aspect = dump signe (12+ tags) | Max 2 keywords objet + tags semantiques filtres |
| Limite axes en dur (`8`) | `profile.max_keywords_per_item` |
| `essential_dignities` plafonne par `max_dominant_objects` | Toutes les dignites du payload |
| Conditions non dedupliquees | `push_unique` insensible a la casse |
| `"Direct"` brut dans conditions | `humanize_motion_label` |
| Rulers en `title_case` de code | Noms depuis `object_names` / positions |
| Profil DB sans colonnes background | `merge_seed_limits` depuis seed |
| Keywords techniques lies a `include_scores` | Filtre reserve au niveau `expert` |
| `effective_limits` incomplets | `max_background_placements`, `max_accidental_conditions_per_object` |

### 4B.1 — Final public wording cleanup (implemente)

Micro-passe de polish sur les textes publics de `llm_payload` :

| Correction | Implementation |
|------------|----------------|
| `shared_resources` dans `house_axes.summary` | `humanize_axis_summary` : parentheses remplacees par le libelle maison (`Transformation`, `Resources`, …) depuis `house_ref_from_payload` ; repli `humanize_residual_snake_case` |
| `supporting_factors` mecaniques (`ascendant angle in house`, `identity theme`, …) | Extensions `humanize_reason` : `ascendant_angle_in_house`, `ic_angle_in_house`, `*_theme`, etc. |
| `Direct motion` redondant dans `conditions` | `is_unremarkable_motion_condition` : pas de doublon avec `motion` ; seuls etats remarquables (retrograde, secte, maison, horizon, angles) |

Tests ajoutes :

- `llm_projection_axis_summary_has_no_snake_case_themes`
- `llm_projection_humanizes_axis_supporting_factors`
- `llm_projection_conditions_exclude_redundant_direct_motion`

**4B est close** apres cette passe. Suite : brancher le service LLM aval sur `llm_payload`
(sans `audit_payload` sauf mode debug).

Vigilance non bloquante (hors scope 4B.1) : `keywords.by_area` utilise encore des
cles internes en snake_case (ex. `shared_resources`). Ce ne sont pas des phrases
publiques ; acceptable pour le contrat actuel. Si la regle devient « zero
snake_case dans tout `llm_payload`, y compris les cles », migrer `by_area` vers
un tableau `{ "area": "Shared resources", "keywords": [...] }` (changement de
schema, a planifier en 4C ou avant branchement LLM).

Reserve : `max_core_placements` non branche ; goldens enveloppe `compact`/`standard`
optionnels (rich + 3 LLM de projection).

### Revue adversariale CLI / env (correctifs)

Findings traites apres branchement `calculate_natal_engine` par defaut :

- **Divergence engine vs audit** : les cles `ASTRAL_*_REFERENCE_SYSTEM` /
  `ASTRAL_HOUSE_SYSTEM` (prioritaires) et les `*_ID` sont resolus depuis le
  seed `json_db` pour les deux chemins ; le mode `--audit-only` n'ignore plus
  les cles string.
- **Naissance partielle** : `ASTRAL_BIRTH_DATE` + `ASTRAL_BIRTH_TIME` sans
  `ASTRAL_BIRTH_TIMEZONE` → erreur explicite (plus de repli silencieux sur UTC).
- **Flags CLI** : `--engine` et `--audit-only` ensemble → erreur.
- **Idempotence** : `ASTRAL_IDEMPOTENCY_KEY` vide filtree (engine + audit).
- **Projection** : `ASTRAL_PROJECTION_LEVEL` validee a la construction de la
  requete engine.
- **`.env.example`** : variables 4A, cles metier et triplet date/heure/fuseau.
- **Test structure** : `engine_envelope_is_not_flat_v13_payload` (pas de
  `product_code` a la racine).

Reste connu :

- Golden enveloppe `astro_engine_response_v1_paris_1990_rich.json` construit
  hors DB a partir du golden v13 (test de structure, pas run moteur complet).
- `verify_engine_response_4a.ps1` verifie les cles, pas le JSON Schema complet.
- `ASTRAL_BIRTH_DATETIME_UTC` seul reste un repli legacy ; preferer le triplet
  local pour aligner engine et audit.

### Criteres d'acceptation 3F

1. PostgreSQL : pour `major`, `expected_aspect_count = 5` et
   `max_default_orb_deg = 15` ; autant de lignes `astral_aspects` avec
   `family = 'major'`, chacune avec `default_orb_deg` dans `(0, 15]` et `angle`
   dans `[0, 180]`.
2. `validate_aspect_definitions` refuse tout ecart (effectif vs famille, coherence
   du `max_default_orb_deg` sur chaque ligne, integrite des lignes, orbe,
   doublon d'`id`, profil produit invalide en sanity check).
3. `detect_aspects` / `canonical_aspect_orb_deg` utilisent `default_orb_deg` et
   le plafond `max_default_orb_deg` porte par chaque definition (JOIN famille),
   sans repli `default_major_orb_deg`.
4. `EphemerisEngine::calculate_natal` n'a pas de parametre `default_major_orb_deg`.
5. `cargo test`, `cargo test --features swisseph-engine`,
   `cargo clippy --features swisseph-engine -- -D warnings`, et
   `scripts/verify_natal_v13_golden.ps1` passent.
6. `cargo run --features swisseph-engine` ne echoue plus sur colonne ou orbe
   majeur manquant apres patch ou import.

## 3G - Known limitations cleanup and LLM readiness boundary

Passe documentaire et de cadrage (pas d'implementation moteur majeure). Objectif :
rendre la section **Limites connues** impossible a mal interpreter avant de brancher
un service LLM sur le payload `natal_structured_v13`.

### Perimetre

- reclassification de chaque reserve historique en `assumed_v13`, `llm_boundary`,
  `future_story` ou `deprecated` ;
- clarification explicite des champs `summary`, `interpretive_hint` et
  `reading_plan` comme aides moteur, non comme sortie redactionnelle ;
- rappel que `product_code = "basic"` est une cle legacy de routage ;
- liste des futures stories avec identifiants courts (table `future_story`).

### Hors perimetre 3G

- implementation des stories du backlog (dignites mineures, combustion, patterns, etc.) ;
- nouveau contrat JSON ou version `natal_structured_v14` ;
- module LLM, traduction ou persistance de sorties redigees (deja hors moteur).

### Criteres d'acceptation 3G

1. Chaque limite connue est etiquetee `assumed_v13`, `llm_boundary`, `future_story`
   ou `deprecated` ; aucune puce vague ne subsiste dans **Limites connues**.
2. `summary`, `interpretive_hint` et `reading_plan` sont decrits comme aides
   moteur dans **Limites connues**, **Champs semantiques 1B** et **Plan de lecture**.
3. `product_code = "basic"` est explicitement legacy ; la profondeur est renvoyee
   vers `chart_context.payload_contract` (deja en tete de document et en
   `llm_boundary`).
4. Les futures stories portent un nom court stable (table `future_story`).
5. Rien dans **Limites connues** ne laisse croire que le moteur produit une
   redaction finale ou des textes prets a l'affichage.

## 3D - Lunar phase context

L'etape 3D a porte le contrat a `natal_structured_v12` avec un nouveau bloc top-level
`lunar_phase_context`. Ce bloc qualifie la phase lunaire natale comme relation
cyclique Soleil-Lune. Il reste dans le perimetre moteur: pas de signal actif
`lunar_phase:*`, pas de nouveau slot `reading_plan`, et pas de consigne LLM.

Les phases viennent de la table canonique
`astral_lunar_phase_definitions`, ajoutee dans
`json_db/astral_lunar_phase_definitions.json`. Le runtime valide que la table
active contient huit phases structurellement coherentes: codes uniques, familles
de cycle valides, degres dans `[0, 360)`, intervalles de 45 degres, ancres
exactes incluses dans leurs intervalles et couverture circulaire continue de
360 degres avant de construire le payload.

Le calcul expose:

- `sun_moon_angle_deg = normalize_360(moon_longitude_deg - sun_longitude_deg)`;
- selection de la phase dont l'intervalle contient cet angle, y compris
  l'intervalle circulaire `new_moon` autour de 0 degre;
- `distance_to_exact_phase_deg` comme distance circulaire a l'ancre exacte;
- `phase_progress_ratio` comme progression dans l'intervalle de phase;
- `related_signal_keys` limite aux signaux actifs `object_position:sun` et
  `object_position:moon`;
- `related_reading_slots = ["core_identity"]` quand ce slot existe.

Le builder marque le payload en `natal_structured_v12` uniquement quand le bloc
`lunar_phase_context` a ete construit depuis les references lunaires injectees
et que les references accidentelles ou secte ne sont pas injectees (sinon v13).
Les chemins de construction historiques sans ces references restent en
`natal_structured_v11` et n'emettent pas de champ `lunar_phase_context` nul.

La validation runtime refuse desormais les payloads v11 et v12 sans accidentel
comme obsoletes lorsque le service charge les references 3E, et
verifie que le bloc existe, que l'angle Soleil-Lune est recalcule avec une
tolerance de 0.01 degre, que l'angle tombe dans l'intervalle du `phase_code`,
que la progression est bornee entre 0 et 1, que les sources referencent des
signaux actifs, et que les tags `lunar_phase` et `sun_moon_cycle` sont presents.

Les artefacts ajoutes sont:

- `astral_calculator/src/payload/lunar_phase.rs`;
- `astral_calculator/src/runtime/payload_freshness/lunar_phase.rs`;
- `astral_calculator/schemas/natal_structured_v12.schema.json`;
- `tests/golden/natal_payload_v12_paris_1990.json`;
- `scripts/verify_natal_v12_golden.ps1`;
- tests de non-regression dans `tests/payload_tests.rs`,
  `tests/runtime_tests.rs` et `tests/contract_basic_v8_tests.rs`.

## astral_llm — Gateway LLM (2026-06-03)

Service hexagonal Rust expose via Axum, independant du moteur `astral_calculator`.

### Entites

| Entite | Role |
|---|---|
| `GenerateReadingRequest` | Contrat d'entree versionne (astro_result, profil, engine, contract) |
| `NatalReadingResponse` | Contrat de sortie `natal_reading_v1` (summary, chapters, legal, quality) |
| `SafetyPolicy` | Regles obligatoires + merge product/override |
| `ProviderCapabilities` | Abstraction par capacites (JSON strict, reasoning, streaming…) |
| `llm_generation_runs` | Audit PostgreSQL (hashes, latence, provider, status) |

### Tables PostgreSQL

- `llm_generation_runs` — metadonnees d'execution
- `llm_generation_payloads` — payloads sanitises (FK run_id)

Script : `astral_llm/crates/astral_llm_infra/sql/llm_generation_runs.sql`

### Prompts versionnes

- `astral_llm/prompts/natal_prompter/v1/` — prompts communs ; profils `natal_light` / `natal_basic` / `natal_premium` en JSON (`config/natal_interpretation_profiles/`, table `llm_interpretation_profiles`)
- Ops profils : `scripts/manage_natal_interpretation_profiles.ps1`

### Tests

- `tests/astral_llm_tests.rs` — integration FakeProvider (single_pass, chapter_orchestrated, safety)
- `tests/astral_llm_evidence_planner_tests.rs` — pool, packs, exclusions core inter-chapitres
- `tests/astral_llm_evidence_coherence_tests.rs` — coherence pack / corps / astro_basis
- `tests/astral_llm_astro_basis_tests.rs` — Premium minimal vs golden riche
- Tests unitaires dans les crates application et providers

### Premium — Interpretive Evidence Planner — **CHANTIER CLOS** (2026-06-04)

**Statut produit** : **Premium interpretatif riche OpenAI — VALIDÉ PRODUIT**. E2E certification V1 (2026-06-05) : run `744fccda` (~33 s, 6 steps `generated`, `gpt-5-mini` + `gpt-5-nano`).

**Statut fige V1** : OpenAI V1 prod **clos** ; Evidence Planner **clos** ; Benchmark OpenAI **clos** (`gpt-5-mini` + `gpt-5-nano`). Certification multi-provider (Mistral / Anthropic) **reportee**. Prochain travail produit : enrichissement evidence + affinage style — voir `Astral_llm_implementation.md`.

**Modeles Premium** : `chapter_models` du profil `natal_premium` (+ `config/llm_product_models.conf` pour defaut produit `natal_prompter`). Valeurs courantes : chapitres `gpt-5-mini`, summary `gpt-5-nano`.

Couche entre `AstroPayloadNormalizer` et `PromptCompiler` pour le profil `natal_premium` (`product_code=natal_prompter`, code :
`astral_llm/crates/astral_llm_application`, tests racine `tests/astral_llm_*`).

#### Donnees entree (payload / calculateur)

- Tables canoniques : `astral_llm/crates/astral_llm_infra/sql/llm_evidence_canonical.sql` (miroir bootstrap `evidence_canonical.rs` si DB vide).
- **`rulership_context` → faits `house_ruler`** (`astro_fact_extractor.rs`) :
  `ascendant_ruler`, `mc_ruler`, `descendant_ruler` (si present), `dominant_house_rulers`.
  Chaque fait angle porte `source_house_number` (1 / 7 / 10 / 4) pour matcher les
  `llm_evidence_requirements` par maison (`evidence_fact_parse::house_number_from_fact`).
- Payload E2E : `request-premium-rich.json` et golden `tests/golden/natal_payload_v13_paris_1990.json`
  incluent `descendant_ruler` (Descendant Taureau → Vénus maison 3).

#### Planner (`ChapterEvidencePlanner`)

- `InterpretiveEvidence` : `semantic_fact_key`, `sign_code` ; signaux `object_position:*`
  alignes sur `placement:*` via `compute_semantic_fact_key`.
- Slots : `llm_chapter_evidence_slots` — ex. `relationships` / `house_ruler` / objet
  **`descendant`** (plus maison 7 seule) ; `career` / `house_ruler` / objet `mc`.
- **`PriorChapterUsage`** : `avoid_repeating` = cles semantiques des cores precedents +
  aspects/dignites deja vus (tous tiers) ; overlap inter-chapitres sur `semantic_fact_key`.
- **`chapter_excludes_candidate`** : pas de Soleil dans le pack `identity` ; pas de
  `ruler:angle:mc:*` dans le pack `relationships` (`chapter_evidence_planner.rs`).
- **`inject_blocking_requirements`** : `career_ruler_10`, `relationships_ruler_7`, etc.
- Identity : cores ascendant + mars (pas de soleil) ; test `identity_pack_excludes_sun`.

#### Prompt et post-traitement chapitre

- Bloc `chapter_evidence_pack` (CORE / SUPPORTING / NUANCE), libelles i18n
  (`to_chapter_evidence_pack_block`, `AstroLabelHumanizer`) — faits `ruler:angle:mc:*`,
  `ruler:angle:descendant:*`, `ruler:dominant_house:house_N:*` → ex. FR
  « Maître du Milieu du Ciel : Soleil » (plus de `Maitre (mc) : sun` dans le pack prompt).
- **Soft cap supporting** : meme `semantic_fact_key` en supporting limitee a
  `max_supporting_semantic_chapters` (defaut **3**) sur les chapitres precedents ;
  exemption si le fait est requis par un `llm_evidence_requirements` bloquant
  `house_ruler` du chapitre (`supporting_cap_exempt_for_chapter`).
- **`ChapterWritingGuidance`** : 4 paragraphes, anti-trigrammes, liste obligatoire des
  `fact_id` (core + supporting) pour `astro_basis`, connecteurs generiques deconseilles
  en paragraphes 2–4.
- Ordre post-LLM chapitre : `AstroBasisRoleNormalizer` → **`ChapterEvidenceBasisEnricher`**
  → `AstroBasisRoleNormalizer` → `AstroLabelHumanizer` → `AstroBasisValidator` →
  **`ChapterEvidenceCoherence`**.
- **Enrichisseur** (`chapter_evidence_basis_enricher.rs`) : complete les **CORE** manquants
  pour tous les chapitres ; complete aussi les **SUPPORTING** manquants sauf `identity`
  (evite `repair_evidence` LLM quand seuls des `fact_id` manquent dans `astro_basis`).
- **`ChapterEvidenceCoherence`** : repair LLM `repair_evidence` si orphelins dans le
  `body` ou incoherence non couverte par l'enrichisseur.

#### Qualite lecture (apres tous les chapitres)

- **`ReadingOpeningDiversityValidator`** (`text_trigrams.rs`) : doublons d'amorces chapitre
  (5 mots) et paragraphe (4 mots) ; connecteurs generiques FR (`par ailleurs`, `en synthèse`, …)
  **ignores** en cross-chapitre (`is_generic_paragraph_opening`).
- **`repair_opening_duplicates`** : jusqu'a 6 tours, tous les chapitres en violation
  (`chapter_orchestrator.rs`, attempt `repair_opening`).
- Erreurs : `PREMIUM_EVIDENCE_DIVERSITY_FAILED`, `ASTRO_BASIS_INVALID`, `READING_QUALITY_FAILED`.

#### Tests et E2E

- `tests/astral_llm_evidence_planner_tests.rs` : pool, packs,
  `identity_pack_excludes_sun`, `relationships_pack_prefers_descendant_ruler_not_mc`,
  `prompt_pack_humanizes_ruler_labels_in_french`, `sun_supporting_semantic_key_capped_at_three_chapters`, …
- `tests/astral_llm_evidence_coherence_tests.rs`, `tests/astral_llm_astro_basis_tests.rs`
- `cargo test -p astral_llm_application` (enrichisseur, trigrams, coherence unitaires)
- E2E : `scripts/generate_premium_reading_e2e.ps1` — profil cible ~40–50 s, **6 steps
  `generated`** (aucun `repair_evidence` / `repair_opening` si enrichisseur + prompts OK)
- Prompts traces : `output/logs/prompts/{run_id}/*.txt` (`ASTRAL_LLM_PROMPT_LOG_DIR`)

**Limites editoriales acceptees en prod** : amorces parfois artificielles (« En développant… », « En prenant en compte… ») ; prose parfois scolaire du fait de la densite des consignes chapitre — a traiter en optimisation style, pas en reouverture planner.

Detail pipeline, roadmap optimisation : `Astral_llm_implementation.md`.

### i18n reponse LLM (2026-06-04)

- Tables : `llm_writing_locales`, `llm_astro_basis_roles`, `llm_aspect_type_labels` (`llm_i18n_canonical.sql`)
- Prompt : `WritingLanguageDirective` (fr/en/es/de) ; bloc ASTRO DATA : libelles humanises (signes, aspects, dignites) selon `user_language`
- Post-LLM : `AstroLabelHumanizer`, `AstroBasisRoleNormalizer` (2 passages autour de l'enrichisseur)
- Extracteur : `extract_signal` conserve le bloc `evidence` dans `NormalizedAstroFact.value`
- Tests : `tests/astral_llm_i18n_tests.rs`, `tests/astral_llm_evidence_coherence_tests.rs`

Documentation detaillee : `Astral_llm_implementation.md`.

## API HTTP calculateur + Docker Compose (2026-06-05)

### Crate `astral_calculator_api`

- Binaire : `cargo run -p astral_calculator_api` (port **8080** par defaut).
- Endpoints : `/health/live`, `/health/ready` (503 + `error_response_v1` si non pret),
  `/v1/contracts`, `/v1/schemas/{version}`, `/v1/reference/status`,
  `/v1/calculations/validate`, `/v1/calculations/natal`, `/openapi.yaml`.
- Contrats publics : repertoire [`contracts/`](contracts/) (schemas + OpenAPI + exemples).
- Test coherence schemas : `cargo test -p astral_llm_api --test contracts_publish_tests`.
- Test API : `cargo test -p astral_calculator_api --test astral_calculator_api_tests`.

### Docker Compose local

```powershell
docker compose up -d --build
.\scripts\docker_bootstrap.ps1
.\scripts\docker_compose_smoke.ps1
```

- Reseau : `astral_net` — `http://astral_calculator_api:8080`, `http://astral_llm_api:8081`.
- PostgreSQL interne (`expose: 5432`) ; port hote optionnel via `docker-compose.dev-db-port.yml`.
- Ephemerides : volume `./ephe:/app/ephe:ro` (non bakees dans l'image).
- Profils Compose : `calculator`, `llm`, `full` ; `postgres` sans profil.

---

## Natal simplifié (v2.4)

Moteur : `astral_calculator/src/simplified/` — résolution `input_precision`, fenêtre d'incertitude, fiabilité faits, payload `natal_simplified_structured_v1`.

### Tables canoniques (`json_db/` → Postgres)

| Table | Rôle |
|-------|------|
| `astral_calculation_scopes` | `stable_birth_date_profile`, `planetary_positions`, `angular_chart`, `full_natal` |
| `astral_birth_input_precision_levels` | 6 niveaux V1 dont `date_with_location_and_timezone_without_time` |
| `astral_simplified_calculation_policies` | `uncertainty_sampling_minutes = 60`, orb cusp UX |
| `astral_simplified_limitation_codes` | causes (`birth_time_missing`, …) |
| `astral_fact_reliability_levels` | stable / ambiguous / reference_based / … |

Repository : `load_simplified_catalog()` dans `simplified/repository.rs`.

### Algorithme fenêtre (CS-004 / CS-005)

1. Date seule ou lieu sans timezone → fenêtre UTC mondiale ~50h (UTC-12 … UTC+14 autour de la date).
2. Date + timezone sans heure → journée civile locale 24h.
3. Date + heure + timezone → instant déclaré.
4. Échantillons : `start_utc`, `end_utc`, puis pas `uncertainty_sampling_minutes` (60 en seed).
5. Signes collectés dans l'ordre d'observation, dédupliqués ; 1 signe → stable, 2+ → ambiguous.
6. `cusp_warning_orb_deg` : alerte UX seulement, n'affecte pas stable/ambiguous.

### Endpoints

- `POST /v1/calculations/natal/simplified` — contrats `astro_simplified_natal_*`
- `POST /v1/readings/natal/simplified` (LLM API) — orchestration birth → calcul → profil `natal_simplified` (HTTP **422** si `safety_rejected` post-génération ; **400** si entrée invalide avant calcul)

### Champs `llm_payload` (calculateur → LLM)

| Champ | Rôle |
|-------|------|
| `allowed_fact_codes` | Affirmations rédactionnelles autorisées (`mercury.sign`) |
| `allowed_astro_basis_fact_ids` | IDs autorisés pour `astro_basis.fact_id` (`placement:mercury`) |
| `blocked_interpretation_fact_codes` | Faits ambigus — pas d'affirmation interprétative |
| `excluded_feature_codes` | Non calculé (scope / limitations) |
| `profile_excluded_feature_codes` | Calculé mais exclu du profil `natal_simplified` — source canonique : table `astral_simplified_profile_feature_exclusions` (seed `json_db/`, loader `load_profile_feature_exclusions`) |
| `allowed_limitation_mentions` | Limitations mentionnables en UX |
| `forbidden_interpretation_topics` | Agrégat documentaire (prompt interne) ; `forbidden_topics` reste un alias déprécié en sortie |

Implémentation : `astral_calculator/src/simplified/payload.rs` (`build_llm_controls`), exclusions profil via `SimplifiedCatalog::profile_feature_exclusions_for` (DB, pas de constante Rust).

### Données canoniques exclusions profil (F-07)

Table **`astral_simplified_profile_feature_exclusions`** : `profile_code`, `computed_scope_code` (nullable = exclusion globale profil), `feature_code`, `exclusion_kind`, `sort_order`. Seed V1 : 4 lignes `natal_simplified` / `profile_interpretation_excluded`. Import : `python scripts/import_json_db_to_postgres.py`.

Distinct de **`astral_simplified_calculation_policies`** (fenêtre d'incertitude, échantillonnage) : les exclusions sont une règle **profil d'interprétation**, pas une policy de calcul.

Pipeline `single_pass` durci (`single_pass_hardening.rs`) :

1. Génération LLM (+ retry si violation script-only, max = `quality.max_script_repair_attempts` du profil DB)
2. Post-traitement serveur (`simplified_reading_postprocess.rs`) : disclaimer canonique, typographie FR (`french_typography.rs`), rôles interpretatifs normalisés, summary compact (1–2 phrases, sans `…`), sanitisation script
3. Fallback body déterministe si script persiste (`ambiguous_core_identity` / `identity`)
4. Parse + `normalize_chapter_astro_basis_fact_ids` + `AstroBasisValidator`
5. `simplified_reading_guard` — whitelist astro_basis, affirmations FR, profil ASC/maisons
6. `SafetyGuard` — inclut `reading_script_guard`
7. `ReadingQualityValidator` — non bloquant (`blocking_gate: false`)

Profil `natal_simplified` : `quality.max_script_repair_attempts: 2` (1 retry) dans `config/natal_interpretation_profiles/natal_simplified.json` → table `llm_interpretation_profiles`.

E2E recette : `test_natal_simplified_e2e.ps1` active `-ForceFake` par défaut (provider fake, sans OpenAI). Recette OpenAI optionnelle : `-UseReal`.

Autres modules :

- `simplified_reading.rs` — validation entrée orchestration, scrub payload prompt (faits bloqués, compteurs angular)
- `french_typography.rs` — restauration élisions FR (`l impression` → `l'impression`)
- `reading_script_guard.rs` — détection + `sanitize_text_for_french_script`

Note : ~~constante Rust `PROFILE_INTERPRETATION_EXCLUDED`~~ **F-07 closed** — table `astral_simplified_profile_feature_exclusions` (REV-013).

### Tests et E2E

| Commande | Périmètre |
|----------|-----------|
| `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests` | Moteur calculateur |
| `cargo test -p astral_calculator_api --test astral_calculator_api_tests` | Route HTTP calculateur |
| `cargo test -p astral_llm_api --test astral_llm_simplified_reading_tests` | Prompt, routing, golden |
| `cargo test -p astral_llm_application reading_script_guard` | Sanitisation + détection script |
| `cargo test -p astral_llm_application french_typography` | Élisions FR |
| `cargo test -p astral_llm_application simplified_reading_postprocess` | Summary compact + fallback body |
| `.\scripts\test_natal_simplified_e2e.ps1` | **12** calculateur + **7** lectures + **5** négatifs **400** |
| `.\scripts\test_natal_simplified_reading.ps1 -NegativeOnly` | Négatifs orchestration seuls |
| `.\scripts\docker_simplified_natal_smoke.ps1` | Smoke rapide `date_only` |

Golden : `tests/golden/simplified_natal_calculation_stable_1990-06-15.json`, `tests/golden/simplified_natal_calculation_equinox_1990-03-21.json`.

Revue adversariale : [`docs/reviews/natal_simplified/REV-011-adversarial-findings.md`](reviews/natal_simplified/REV-011-adversarial-findings.md).

Guide débutant pas à pas : [`docs/GUIDE_DEBUTANT_DOCKER.md`](GUIDE_DEBUTANT_DOCKER.md) §9 (tutoriel natal simplifié).

---

## API d'intégration externe (V1)

Orchestration async pour applications tierces via `astral_llm_api` + worker dédié.

### Tables

| Table | Rôle |
|-------|------|
| `llm_integration_services` | Catalogue services (`availability`, `payload_contract`, `supports_*`) |
| `llm_jobs` | Jobs async (`run_id` public, FK optionnelle `generation_run_id` → `llm_generation_runs`) |

Seed : `json_db/llm_integration_services.json`. Import : `python scripts/import_json_db_to_postgres.py` puis `.\scripts\manage_integration_services.ps1 -Submit`.

Durcissement Docker/import :

- `scripts/import_json_db_to_postgres.py` accepte les FK vers
  `llm_interpretation_profiles`, table runtime migrée par `astral_llm_api` /
  `astral_llm_worker` et donc absente de `json_db`.
- `scripts/manage_integration_services.ps1 -Submit` rend le schéma catalogue
  idempotent quand la table a été créée par l'import JSON : ajout de
  `updated_at`, index `availability`, et contrainte primaire/unique
  `service_code` si absente.
- Les images `astral_llm_api` et `astral_llm_worker` copient tout `contracts/`
  et définissent `WORKDIR /app`, car les validateurs chargent aussi les
  contrats calculateur via chemins relatifs (`contracts/calculator/...`).
- Le smoke `scripts/test_integration_jobs_e2e.ps1` charge `.env`, envoie les
  headers `Authorization` / `X-API-Key`, vérifie `contracts.payload`, attend
  jusqu'à 300 s, et couvre le replay idempotent cross-service en `409`.
- `scripts/docker_update_integration_stack.ps1` automatise le cycle complet :
  `docker compose up -d --build`, import `json_db`, soumission catalogue,
  **sync LLM** (`config/natal_interpretation_profiles/*.json` +
  `config/llm_product_models.conf` via `scripts/lib/sync_llm_catalog.ps1`),
  restart LLM API/worker, readiness HTTP, catalogue public et smoke jobs E2E.
  Options utiles : `-SkipBuild`, `-SkipImport`, `-SkipLlmSync`,
  `-SkipCatalogueSubmit`, `-SkipSmoke`, `-RunRustChecks`. Avec `-SkipBuild`, le
  demarrage passe par `docker compose up -d --no-build` pour eviter un warning
  Compose parasite.

### Endpoints

- `GET /v1/services`, `GET /v1/services/{code}/contract`
- `POST /v1/jobs` (header `Idempotency-Key`, statut initial **`queued`**)
- `GET /v1/jobs/{run_id}`

Contrat : [`docs/integration_api_contract.md`](integration_api_contract.md).

`POST /v1/jobs` refuse un service `active` / `beta` sans orchestrateur V1 avant
persistance (`501 SERVICE_NOT_IMPLEMENTED`). Le replay idempotent est
tenant-wide mais limité au même fingerprint `api_key_id` ; une autre clé API du
même tenant reçoit `409 IDEMPOTENCY_CONFLICT`.
Le précontrôle d'idempotence s'exécute avant la validation détaillée du payload
si la clé existe déjà, afin de ne pas masquer un conflit par un `422` de schéma
d'un autre service.

### Code Rust

| Module | Crate | Rôle |
|--------|-------|------|
| `integration_routes.rs` | `astral_llm_api` | Routes HTTP catalogue + jobs |
| `integration_job_validator.rs` | `astral_llm_application` | Enveloppe → payload → gate from_payload |
| `unified_reading_orchestrator.rs` | `astral_llm_application` | simplified / full natal / from_payload |
| `engine_reading.rs` | `astral_llm_application` | Mapping moteur → `generate_reading_request_v1` |
| `job_persistence.rs` | `astral_llm_infra` | Persistance + idempotence tenant-wide |
| `canonical_json_hash.rs` | `astral_llm_infra` | Hash SHA-256 JSON canonique |
| `mercure_publisher.rs` | `astral_llm_infra` | Notifications push optionnelles |
| `main.rs` | `astral_llm_worker` | Poll `llm_jobs` SKIP LOCKED |

### Rétention jobs

- `llm_jobs.expires_at` est calculé depuis `ASTRAL_LLM_IDEMPOTENCY_TTL_HOURS`
  en heures réelles (`min = 1h`), sans arrondi implicite en semaines.
- Les jobs terminaux expirés (`completed`, `failed`, `safety_rejected`,
  `cancelled`, `expired`) sont purgés physiquement par le worker et avant les
  accès HTTP jobs (`POST /v1/jobs`, `GET /v1/jobs/{run_id}`).
- Les jobs non terminaux expirés ne sont pas supprimés par cette purge ; ils
  restent gérés par la récupération `running` stale / file worker.
- `llm_jobs.generation_run_id` est un lien audit optionnel : le worker ne le
  renseigne que si la ligne existe dans `llm_generation_runs`. En mode Docker
  fake, un job peut donc terminer correctement sans ligne d'audit génération.
  Les erreurs SQL de persistance terminale sont loggées et ne produisent plus
  un faux log `job finished`.

### Tests

| Commande | Périmètre |
|----------|-----------|
| `cargo test -p astral_llm_api --test integration_services_tests` | Schémas + seed catalogue |
| `cargo test -p astral_llm_api --test integration_jobs_tests` | Validator, hashing, golden engine |
| `.\scripts\test_integration_jobs_e2e.ps1` | E2E natal_simplified async |
| `.\scripts\test_natal_from_birth_e2e.ps1` | E2E full natal (`natal_basic`) |

Reviews : [`docs/reviews/integration_api/INDEX.md`](reviews/integration_api/INDEX.md).

# Articulation horoscope

Les services horoscope V1 (`horoscope_basic_daily_natal_3_slots` et
`horoscope_free_daily`) sont cadres dans
[`HOROSCOPE_IMPLEMENTATION.md`](HOROSCOPE_IMPLEMENTATION.md). Ils ne modifient
pas le payload natal/basic existant et consomment un `chart_calculation_id` deja
calcule.

Le service Premium V1 `horoscope_premium_daily_local_2h_slots` est cadre dans le
meme document. Il conserve la meme infrastructure async, ajoute une localisation
obligatoire, 12 creneaux publics de 2 heures, `local_chart` par slot et des
guards Premium dedies.

Correctif review adversariale Premium : le schema calculateur publie accepte
desormais les 12 slots Premium au niveau commun `slots.maxItems`; le guard
Premium verifie que `local_chart` contient Ascendant, MC et 12 maisons ; les
`best_slots` / `watch_slots` doivent referencer un label de timeline connu et
ne peuvent citer que les preuves planifiees pour ce creneau. Le paquet
`evidence` conserve toutes les preuves requises par les slots meme lorsque
`main_signals` reste borne a 24.

Suite de validation dediee : `scripts/test_horoscope_premium_daily_all.ps1`
regroupe les tests Rust du service, les contrats, les checks integration et le
smoke HTTP fake Premium. Elle est appelee par
`scripts/docker_update_integration_stack.ps1` pendant les smokes Docker.
Le smoke fake Premium imprime un resume de succes plutot que le payload complet :
le marqueur calculateur `FAKE_PREMIUM_LOCAL_DATA_STABLE_FOR_TESTS` reste une
donnee de test voulue dans `calculation_warnings`, sans etre remonte comme une
alerte de deroule.
Le test E2E reel Docker dedie est
`tests/e2e_real/04_horoscope_premium_daily.e2e.ps1` ; il couvre catalogue,
contrat, schema, job async Premium, calcul local `timeline[12]`, `local_chart`,
`best_slots` / `watch_slots` et absence de fuite `slot_`.

Pour `horoscope_free_daily`, `day` est uniquement un slot technique de
projection. Il ne constitue pas une section publique et ne doit jamais apparaitre
dans la reponse utilisateur.

Le plan de refactor slot-based et les reviews adversariales sont suivis dans
[`docs/reviews/horoscope_v1/`](reviews/horoscope_v1/INDEX.md).

## E2E reels Docker dedies (2026-06-06)

Les tests HTTP reels pour une application Docker deja demarree sont regroupes
dans `tests/e2e_real/`.

- `01_calculator_services.e2e.ps1` couvre les services calculateur publics :
  contrats, schemas, validation, natal complet, natal simplifie et horoscope
  daily-natal.
- `02_llm_sync_services.e2e.ps1` couvre les endpoints LLM synchrones :
  contrats, providers, schemas, generation, validation et lecture natal
  simplifiee orchestree.
- `03_integration_catalog_services.e2e.ps1` lit `GET /v1/services`, selectionne
  tous les services `active` / `beta`, puis soumet un job reel Docker pour
  chacun. La suite echoue si un nouveau service propose n'a pas de constructeur
  E2E explicite.
- `run_real_e2e.ps1` lance toute la suite :
  `.\tests\e2e_real\run_real_e2e.ps1`. Chaque execution produit un rapport
  Markdown dans `output/e2e_real_reports/` par defaut, ou au chemin fourni via
  `-ReportPath`. Le runner cree aussi un dossier de diagnostics avec transcript
  PowerShell et, en cas d'echec, les derniers logs Docker du worker, des API et
  de PostgreSQL.

Script de consultation manuelle :
`.\scripts\show_real_horoscope_text.ps1` soumet un vrai job
`horoscope_basic_daily_natal_3_slots` sur Docker, attend le resultat, affiche les
textes des slots et ecrit un JSON + un Markdown dans `output/horoscope_real/`.

## Utilitaire fenetre temporelle (2026-06-07)

La crate workspace `astral_time_window` fournit le module partageable
`time_window` pour transformer une intention produit en fenetre locale concrete
a fin exclusive.

- Types publics : `PeriodWindowRequest`, `ResolvedPeriodWindow`,
  `PeriodProfileDefinition`, `PeriodWindowResolver`, `PeriodWindowError`.
- Source canonique des profils : `json_db/astral_time_period_profiles.json`
  (`day`, `next_7_days`, `next_14_days`, semaines ISO et `custom_date_range`).
- La crate ne lit pas PostgreSQL directement : les definitions sont injectees
  dans le resolver par la couche appelante apres chargement DB ou fixture.
- Contrats publics : `contracts/common/period_window_request_v1.schema.json` et
  `contracts/common/period_window_response_v1.schema.json`, references dans
  `contracts/versions.json`.
- Sortie normalisee : `start_datetime_local`, `end_datetime_local`, `timezone`,
  `duration_days`, `end_exclusive = true`, avec `included_days` pour les profils
  semaine/workweek.

Regles principales : `next_N_days` inclut la date d'ancrage et termine a J+N a
00:00 locale ; les semaines utilisent le lundi ISO ; `custom_date_range` recoit
des dates inclusives et retourne une fin exclusive au lendemain de
`custom_end_date`.

Tests : `cargo test -p astral_time_window` ou
`.\scripts\test_time_window_service.ps1`. Le script est aussi appele par
`.\scripts\docker_update_integration_stack.ps1` dans la phase smoke, sauf avec
`-SkipSmoke`.

## E2E reel Premium period (2026-06-08)

Le service `horoscope_premium_next_7_days_natal` dispose d'un script E2E reel
dedie : `.\scripts\test_horoscope_premium_next_7_days_real_e2e.ps1`.

Il soumet le service via `POST /v1/jobs`, verifie le provider LLM reel,
l'absence de fallback, le scan `six_hour_7_days` a 28 snapshots, la timeline 7
jours, `strategy`, 3 a 5 `domain_sections`, `best_windows` non vide,
`watch_windows` coherentes avec `watch_summary`, les references
`source_snapshot_keys`, les evidence publiques et la limite dure `premium_rich`.
Depuis V1.1, Premium produit `watch_summary.status = "low"` avec 1 a 3
`watch_windows` douces quand aucune tension forte ne ressort mais que des
signaux exploitables existent. `status = "none"` reste reserve aux cas sans
signal exploitable. Les `best_windows` doivent avoir des titres et `best_for`
differencies, et `premium_scores.domain_score` mesure une couverture variable
des themes/evidence plutot qu'un placeholder constant.
Le script est appele par `test_horoscope_premium_next_7_days_all.ps1`,
`test_horoscope_period_all.ps1` et `docker_update_integration_stack.ps1` quand
les options de reel period sont activees.

## Module text_reprocessing v1 isole (2026-06-08)

Un module Rust dedie au retraitement des textes LLM a ete ajoute sans branchement
aux flux applicatifs existants.

- Contrats extensibles : `astral_llm_domain::text_reprocessing`.
- Pipeline et processors : `astral_llm_application::text_reprocessing`.
- Services couverts par fixtures isolees : `horoscope_daily`,
  `horoscope_period`, `natal_theme`, `natal_simplified`,
  `calculator_projection`, `prompt_trace`, `shared`.
- Le module reprend les fonctionnalites de sanitation, typographie, longueur,
  anti-repetition, humanisation de libelles, normalisation `astro_basis`,
  validation qualite, fallback, guidance prompt et formatting trace.
- Les anciennes fonctions restent la source de verite runtime : aucun appel
  existant n'a ete remplace.

Documentation dediee : `docs/TEXT_REPROCESSING_MODULE.md`.
Reviews adversariales : `docs/reviews/text_reprocessing/`.

Tests cibles :

```powershell
cargo test -p astral_llm_domain text_reprocessing
cargo test -p astral_llm_application text_reprocessing
```

Les tests dedies sont stockes dans `tests/text_reprocessing_domain_tests.rs`
et `tests/text_reprocessing_application_tests.rs`, puis rattaches aux crates par
targets `[[test]]`.

## Branchement text_reprocessing service par service (2026-06-08)

Le module `text_reprocessing` est branche progressivement via
`astral_llm_application::text_reprocessing_service_adapter`.

- `prompt_trace`: `format_compiled_messages` delegue a `reprocess_prompt_trace`.
- `natal_simplified`: sanitation et typographie postprocess deleguent a
  `reprocess_natal_simplified`.
- `horoscope_daily`: les rendus fake daily passent par
  `reprocess_horoscope_daily` apres construction structurelle.
- `horoscope_period`: les reponses provider passent par
  `reprocess_horoscope_period` apres repair/tone et avant validation.
  La sanitation de chaine publique periode est centralisee dans
  `ScriptSanitizerProcessor`; `sanitize_period_public_string` reste un wrapper.
- `natal_theme`: la lecture finale orchestree passe par
  `reprocess_natal_theme` apres assemblage.
- Fixtures editoriales premium: `EditorialValidator` valide une copie de lecture
  via `reprocess_natal_theme_with_context`; `AstroBasisDensityProcessor`
  complete les chapitres selon `min_astro_basis_per_chapter` uniquement depuis
  `allowed_evidence_by_chapter` pour les lectures multi-chapitres, ou
  `allowed_evidence_keys` pour un payload mono-chapitre.
- `calculator_projection`: helper `reprocess_calculator_projection` disponible;
  aucun point runtime direct n'a ete identifie dans `astral_llm_application`.

Reviews de connexion: `docs/reviews/text_reprocessing_connection/`.
Fixtures de migration: `tests/fixtures/text_reprocessing_migration/`.

Verification residuelle corrigee: `cargo test -p astral_llm_api --test
astral_llm_editorial_fixtures` est passant. Les fixtures sources ne sont pas
modifiees; le retraitement est applique sur une copie de validation.
