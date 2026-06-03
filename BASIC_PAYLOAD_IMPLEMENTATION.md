# Implementation du payload moteur route basic

Ce document decrit l'implementation actuelle du payload moteur route par
`product_code = "basic"` dans le binaire Rust `rust_sqlx_connection_test`.

## Objectif

Etat courant au 2026-06-03 : le moteur Rust reste dans le perimetre du calcul
astrologique et des cles d'interpretation. Le payload route basic expose les
faits calcules, les contextes astrologiques, les dignites essentielles et
accidentelles (MVP), les angles, les dominantes, le contexte de rulership, les
signaux actifs et `reading_plan`.

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
  actifs, orbe majeur par defaut, parametres d'axes de maisons ;
- `astral_accidental_condition_triggers` : declencheurs MVP (modalite, mouvement,
  horizon, proximite angle, secte) ;
- `astral_accidental_scoring_params` et `astral_accidental_overall_polarity_bands` :
  baseline, orbe angle, paliers `overall_polarity` / `expression_quality` ;
- colonne `astral_aspects.default_orb_deg` et profil
  `astral_essential_dignity_score_weights` (deltas de priorite/poids signaux).

Le runtime charge ces references via `BasicPayloadCatalog` (`catalog.rs`) et
projette dans `chart_context` les snapshots `accidental_scoring` (baseline,
bornes min/max, orbe angle, bandes de polarite) et `product_scoring` pour la
validation de fraicheur (freshness) sans constantes en dur. Les payloads v13
sans ces snapshots ou avec des bandes non contigues sur `[0, 1]` sont rejetes.

Ces donnees ne doivent pas etre compensees par des valeurs applicatives en dur.
Si le binaire echoue avec une erreur SQL de relation ou de colonne manquante,
la correction attendue est de resynchroniser PostgreSQL avec les fichiers
`json_db`, pas de contourner la lecture en Rust.

## Fichiers concernes

- `rust_sqlx_connection_test/src/catalog.rs` : catalogue en memoire (profil produit,
  regles essentielles, triggers et scoring accidentel).
- `rust_sqlx_connection_test/src/domain.rs` : structures runtime et payload JSON.
- `rust_sqlx_connection_test/src/facts.rs` : helpers de libelles signe/maison.
- `rust_sqlx_connection_test/src/ephemeris.rs` : enrichissement des positions calculees.
- `rust_sqlx_connection_test/src/aspects.rs` : detection geometrique des aspects
  et calcul de l'orbe, de la phase et de la force brute.
- `rust_sqlx_connection_test/src/dignities.rs` : detection MVP des dignites essentielles majeures.
- `rust_sqlx_connection_test/src/payload/accidental_dignities.rs` : evaluation MVP
  des dignites accidentelles et projection vers positions, signaux et dominantes.
- `rust_sqlx_connection_test/src/payload/lunar_phase.rs` : construction de
  `lunar_phase_context` depuis les references lunaires.
- `rust_sqlx_connection_test/src/signals/` : construction, filtrage et
  priorisation des signaux du payload route basic.
- `rust_sqlx_connection_test/src/payload/` : assemblage du payload final et de
  ses blocs contractuels.
- `rust_sqlx_connection_test/src/repositories.rs` : persistance, relecture des
  positions et enrichissement SQL depuis les referentiels de signes, maisons,
  objets, angles et aspects.
- `rust_sqlx_connection_test/src/runtime/` : orchestration runtime,
  validation des references et regeneration des anciens payloads.
- `json_db/astral_accidental_dignity_condition_definitions.json` : definitions
  canoniques des 15 conditions accidentelles MVP.
- `json_db/astral_object_sect_affinities.json` : affinites de secte par objet.
- `json_db/astral_lunar_phase_definitions.json` : definitions des phases lunaires.
- `rust_sqlx_connection_test/schemas/basic_natal_structured_v8.schema.json` :
  schema JSON historique du contrat Basic v8.
- `rust_sqlx_connection_test/schemas/natal_structured_v9.schema.json` :
  schema JSON historique du contrat v9.
- `rust_sqlx_connection_test/schemas/natal_structured_v10.schema.json` :
  schema JSON historique du contrat `natal_structured_v10`.
- `rust_sqlx_connection_test/schemas/natal_structured_v11.schema.json` :
  schema JSON historique du contrat `natal_structured_v11`.
- `rust_sqlx_connection_test/schemas/natal_structured_v12.schema.json` :
  schema JSON historique du contrat `natal_structured_v12`.
- `rust_sqlx_connection_test/schemas/natal_structured_v13.schema.json` :
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
  stable v13 apres regeneration du payload par le moteur.

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
- `interpretive_hint` : phrase courte orientee utilisateur. Pour les aspects,
  elle inclut la qualite interpretative issue de `aspect_context`.
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
`astral_chart_object_signal_profiles`, `astral_interpretation_generated_outputs`
ou `astral_house_modalities.priority_delta`. Dans ce cas, le programme ne doit
pas demarrer avec des fallbacks. Il faut appliquer les ajouts issus de `json_db`
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
  `rust_sqlx_connection_test/schemas/natal_structured_v13.schema.json`
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
  `rust_sqlx_connection_test/schemas/natal_structured_v12.schema.json`
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
genere au golden v13. Le script force les variables d'environnement du scenario
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
dans `astral_calculated_aspects`. Avant de construire les signaux, le runtime
relit ces aspects via les joins de `repositories.rs` afin d'ajouter la famille
et les effets interpretatifs issus des referentiels. Ce meme chemin
est utilise pour un calcul frais et pour la regeneration d'un payload existant
juge obsolete.

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
comme cibles de tests d'integration dans `rust_sqlx_connection_test/Cargo.toml`.

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

Depuis `rust_sqlx_connection_test` :

```powershell
cargo test
cargo test --features swisseph-engine
cargo clippy --features swisseph-engine -- -D warnings
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

- Les angles du payload route basic sont exposes, mais leurs interpretations restent limitees aux
  faits structures et aux signaux `angle:*`.
- Le contexte de maitrise 3B expose les liens structurels au LLM, mais ne cree
  pas encore de signaux actifs `rulership:*`.
- Les resumes restent des phrases templatees, pas une interpretation finale.
- Les `interpretive_hint` restent aussi des templates, meme si les hints
  d'aspect integrent maintenant la valence 2C.
- Les clusters du payload route basic ne couvrent pour l'instant que les concentrations
  `sign_house`.
- Le moteur de dignites essentielles 2B est un MVP code-side. Il couvre les
  dignites majeures par signe, pas encore les dignites mineures (terme,
  triplicite, face).
- Le moteur de dignites accidentelles 3E est un MVP base-references. Il couvre
  15 conditions (maison, proximite angle, mouvement, horizon, secte) mais pas
  combustion, cazimi, hayz complet ni paliers d'orb 3 degre / 6 degre ;
  l'orb de proximite angle est fixe a 10 degre cote moteur.
- Le programme consomme les libelles des referentiels tels quels. Il ne gere pas la traduction.
- La redaction LLM doit rester une etape ulterieure.

## Organisation du module payload

`rust_sqlx_connection_test/src/payload.rs` a ete remplace par le dossier
`rust_sqlx_connection_test/src/payload/` afin de separer les responsabilites
sans modifier le contrat public `rust_sqlx_connection_test::payload`.

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

- `ascendant_ruler` et `mc_ruler`;
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
servir le cluster, et `mc_ruler` peut servir le fond lorsque le MC y est traite.
Ces decisions appartiennent a la couche LLM, pas au payload moteur.

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

- `rust_sqlx_connection_test/schemas/natal_structured_v10.schema.json`;
- `tests/golden/natal_payload_v10_paris_1990.json`;
- `scripts/verify_natal_v10_golden.ps1`.

Ce decoupage reste volontairement simple: les donnees doctrinales restent dans
les fichiers `json_db` et sont lues par la base; aucune correspondance
signe -> maitre n'est codee en Rust. Les fonctions gardent une portee limitee
au module quand elles ne font pas partie de l'API publique.

## Organisation du module signals

`rust_sqlx_connection_test/src/signals.rs` a ete remplace par le dossier
`rust_sqlx_connection_test/src/signals/` afin de separer l'agregation des
signaux du payload route basic par responsabilite, sans modifier l'API publique
`rust_sqlx_connection_test::signals`.

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

Le fichier racine `rust_sqlx_connection_test/src/aspects.rs` reste separe du
module `signals/aspect_signals.rs`: le premier detecte les faits d'aspects
depuis les positions calculees, alors que le second transforme un `AspectFact`
en contexte de signal du payload route basic. Les fusionner melangerait le calcul des faits et
la preparation editoriale du payload.

Ce refactor reste strictement structurel: aucune nouvelle donnee canonique n'a
ete ajoutee en dur et le comportement conserve est valide par:

```powershell
cargo fmt --manifest-path rust_sqlx_connection_test/Cargo.toml
cargo clippy --manifest-path rust_sqlx_connection_test/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path rust_sqlx_connection_test/Cargo.toml
```

## Organisation du module runtime

`rust_sqlx_connection_test/src/runtime.rs` a ete remplace par le dossier
`rust_sqlx_connection_test/src/runtime/` afin de separer les responsabilites
runtime sans modifier l'API publique `rust_sqlx_connection_test::runtime`.
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

- `rust_sqlx_connection_test/schemas/natal_structured_v11.schema.json`;
- `tests/golden/natal_payload_v11_paris_1990.json`;
- `scripts/verify_natal_v11_golden.ps1`;
- tests de non-regression dans `tests/payload_tests.rs`,
  `tests/runtime_tests.rs` et `tests/contract_basic_v8_tests.rs`.

Le contrat courant et ses artefacts de verification sont documentes dans la
section 3E et dans `Fichiers concernes`.

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

- `rust_sqlx_connection_test/src/payload/accidental_dignities.rs` ;
- `rust_sqlx_connection_test/src/runtime/payload_freshness/accidental_dignities.rs` ;
- `rust_sqlx_connection_test/schemas/natal_structured_v13.schema.json` ;
- `tests/golden/natal_payload_v13_paris_1990.json` ;
- `scripts/verify_natal_v13_golden.ps1` ;
- tests dans `tests/payload_tests.rs`, `tests/runtime_tests.rs`,
  `tests/contract_basic_v8_tests.rs`.

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

- `rust_sqlx_connection_test/src/payload/lunar_phase.rs`;
- `rust_sqlx_connection_test/src/runtime/payload_freshness/lunar_phase.rs`;
- `rust_sqlx_connection_test/schemas/natal_structured_v12.schema.json`;
- `tests/golden/natal_payload_v12_paris_1990.json`;
- `scripts/verify_natal_v12_golden.ps1`;
- tests de non-regression dans `tests/payload_tests.rs`,
  `tests/runtime_tests.rs` et `tests/contract_basic_v8_tests.rs`.
