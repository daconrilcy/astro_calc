# Implementation du payload Basic runtime

Ce document decrit l'implementation actuelle du payload Basic dans le binaire Rust
`rust_sqlx_connection_test`.

## Objectif

L'etape 1A a transforme le payload technique initial en payload Basic exploitable
par une future couche de generation texte.

L'etape 1B enrichit maintenant ce payload avec des signaux semantiques Basic :
themes editoriaux, tags, indications de redaction, poids de source et premiers
signaux agreges. Le runtime ne produit toujours pas une interpretation finale,
mais il fournit une base plus directement exploitable pour une couche de
redaction.

L'etape 1C ajoute une deduplication editoriale et un plan de lecture Basic. Quand
un cluster actif represente deja plusieurs placements, ses sources secondaires
sont persistees en `merged` au lieu de remonter comme signaux actifs autonomes.
Le payload final expose aussi `reading_plan`, une sequence de slots qui indique
dans quel ordre exploiter les signaux actifs pour la future redaction.

L'etape 1D ajoute un contrat de redaction Basic par slot. Le runtime ne produit
pas encore un texte final, mais il transforme chaque item de `reading_plan` en
section attendue via `drafting_plan` : titre editorial, sources, objectif de
redaction, plafond de mots et consignes d'evitement.

L'etape 1E formalise le contrat canonique de handoff LLM. Le moteur Rust produit
un payload anglophone stable, deterministe et auditable ; il ne traduit pas, ne
choisit pas la formulation finale et ne depend pas d'une langue cible utilisateur.

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
placement, mais leur resume et leur `writing_guidance` sont maintenant plus
precis.

L'etape 2C ajoute une couche interpretative controlee aux aspects. Les signaux
`aspect:*` ne portent plus seulement la geometrie, l'orbe, la phase et la force :
ils exposent aussi `aspect_context`, construit depuis `astral_aspect_profiles`,
`astral_aspect_interpretive_effects` et `astral_interpretive_valence`. Cette
couche separe la valence principale (`primary_valence`) des modificateurs
d'intensite (`intensity_modifier`) comme `amplifying`, ajoute une qualite
dynamique (`flow`, `tension`, `intensification`, etc.) et enrichit les tags et
la guidance redactionnelle.

L'etape 2D ajoute une synthese top-level `chart_emphasis` qui classe les
dominantes de signe, de maison et d'objet a partir des placements, clusters,
dignites et aspects forts deja calcules. Elle fournit une hierarchie quantifiee
et auditable avant tout appel LLM.

L'etape 2D.1 projette cette synthese dans `drafting_plan` via des
`emphasis_refs` legeres. Ces references indiquent quel slot doit utiliser les
dominantes comme contexte de ponderation, sans creer de section supplementaire.

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
de section LLM vide.

L'etape 2E.3 preserve les vrais aspects dynamiques apres l'exclusion des axes
structurels. Les aspects angle-angle restent exclus du payload Basic comme
dynamiques representatives, qu'ils soient l'axe Ascendant-Descendant, l'axe
MC-IC, ou un autre aspect entre deux angles. Les aspects forts planete-planete
et planete-angle restent en revanche eligibles. Le filtrage preserve d'abord une
tension forte non structurelle si elle existe ; si aucune tension forte n'est
disponible mais qu'un aspect fort planete-planete ou planete-angle existe, il
preserve le meilleur aspect fort disponible afin que `main_tension_or_support`
ne disparaisse pas artificiellement. Le contrat canonique stabilise reste
`basic_natal_structured_v8`; les payloads existants qui recyclent un axe
structurel actif, perdent les vrais aspects dynamiques ou manquent les champs
d'angle obligatoires sont rejetes par la validation de reutilisation et
regeneres.

Le runtime conserve la chaine existante :

1. calcul des faits astrologiques ;
2. ecriture des positions, cuspides et aspects calcules ;
3. relecture des positions et aspects avec leur contexte de referentiel ;
4. aggregation des signaux ;
5. filtrage produit Basic ;
6. ecriture du payload canonique dans `astral_interpretation_generation_payloads`.

Cette etape prepare donc une entree propre, lisible, auditable et pre-orientee
editorialement.

## Fichiers concernes

- `rust_sqlx_connection_test/src/domain.rs` : structures runtime et payload JSON.
- `rust_sqlx_connection_test/src/facts.rs` : helpers de libelles signe/maison.
- `rust_sqlx_connection_test/src/ephemeris.rs` : enrichissement des positions calculees.
- `rust_sqlx_connection_test/src/aspects.rs` : detection geometrique des aspects
  et calcul de l'orbe, de la phase et de la force brute.
- `rust_sqlx_connection_test/src/dignities.rs` : detection MVP des dignites essentielles majeures.
- `rust_sqlx_connection_test/src/signals.rs` : construction et filtrage des signaux Basic.
- `rust_sqlx_connection_test/src/payload.rs` : assemblage du payload final.
- `rust_sqlx_connection_test/src/repositories.rs` : persistance, relecture des
  positions et enrichissement SQL depuis les referentiels de signes, maisons,
  objets, angles et aspects.
- `rust_sqlx_connection_test/src/runtime.rs` : orchestration et regeneration des anciens payloads.
- `rust_sqlx_connection_test/schemas/basic_natal_structured_v8.schema.json` :
  schema JSON du contrat Basic v8.
- `tests/golden/basic_payload_v8_paris_1990.json` : fixture golden du contrat
  Basic v8.
- `tests/contract_basic_v8_tests.rs` : validation schema, golden et invariants
  metier non negociables.
- `scripts/verify_basic_v8_golden.ps1` : verification CI/local de projection
  stable apres regeneration du payload par le moteur.

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
  ]
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
ambigues. Depuis la stabilisation v8, le contrat refuse aussi de reutiliser un
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
publique. Depuis la stabilisation v8, `signals[].evidence.opposite_angle_code`
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

Les signaux actifs du payload Basic sont limites a 12.

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
  "writing_guidance": "Use this as a concise placement cue; combine it with nearby cluster or aspect signals before drafting final text.",
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
      "dignity_context": []
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
  "writing_guidance": "Indicate that the factor intensifies what is already present, and combine it with another tonal valence when possible. Treat amplifying as an intensity modifier, not as a supportive or challenging valence by itself.",
  "aspect_context": {
    "aspect_family": "major",
    "primary_valence": null,
    "intensity_modifier": "amplifying",
    "secondary_effect": null,
    "dynamic_quality": "intensification",
    "phase_state": "separating",
    "valence_family": "intensity",
    "is_tonal_valence": false,
    "is_intensity_modifier": true,
    "writing_guidance": "Indicate that the factor intensifies what is already present, and combine it with another tonal valence when possible."
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
- `source_weight` : poids relatif de la source astrologique. Soleil et Lune
  valent plus que les planetes lentes.
- `aggregation_group` : cle de regroupement editoriale.
- `writing_guidance` : consigne courte pour la future couche de redaction.

Ces champs sont stockes dans `astral_interpretation_signals.payload_json`, puis
remontes dans le payload final par `payload.rs`.

`aspect_context` est egalement expose au niveau de chaque `BasicSignal`. Il
contient un objet structure uniquement pour les signaux `aspect:*` ; pour les
placements, dignites et clusters, il est serialise a `null`.

### Champs contextuels 2A

Les champs ajoutes par l'etape 2A sont volontairement limites aux preuves utiles
pour le calcul et la redaction. Ils ne recopient pas les faits runtime bruts,
mais ils peuvent embarquer un referentiel semantique complet quand ce
referentiel est directement exploitable par le LLM.

- `sign_context` : element, modalite zodiacale, polarite et liste complete des
  mots-cles principaux du signe depuis `astral_sign_keywords.keywords_json`.
- `house_context` : contexte editorial canonique de maison, dont
  `theme_code`, depuis `astral_houses.theme_code`.
- `house_modality` : modalite de maison, force accidentelle et poids
  d'interpretation.
- `object_context` : role astrologique, nature principale et indicateurs de
  visibilite/symbolique.
- `motion_context` : etat de mouvement lisible, libelle et famille de mouvement.

Dans `positions`, ces contextes sont exposes directement comme preuves
structurees. Dans les signaux de placement, ils sont imbriques dans
`evidence.placement_context`, afin de rester associes au fait astrologique et de
ne pas creer un bloc redactionnel autonome.

La liste `sign_context.keywords` reste volontairement non tronquee. Elle
represente le vocabulaire semantique disponible pour le signe, pas une liste de
points a rediger un par un. Le contrat LLM continue donc d'interdire de lister
les placements ou d'exposer les preuves brutes ; ces mots-cles servent a guider
le choix lexical, a eviter l'invention et a permettre une synthese plus riche.

Les tags semantiques des placements integrent aussi les codes utiles comme
`air`, `mutable`, `yang`, `cadent`, `luminary` ou `direct`. La priorite d'un
placement est legerement ajustee par la modalite de maison : angular augmente le
poids, succedent l'augmente faiblement, cadent le baisse faiblement.

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
  "writing_guidance": "Use this to strengthen the object's placement signal without overstating ease or outcome.",
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
  au filtrage Basic de 12 signaux actifs ;
- les dignites actives liees a un objet selectionne dans `reading_plan` sont
  ajoutees aux sources du slot, y compris quand le placement de l'objet a ete
  fusionne dans un cluster ;
- dans les slots qui limitent un nombre d'objets, comme `expression_style` ou
  `background_factors`, les dignites associees ne consomment pas le quota
  d'objets. Elles accompagnent l'objet selectionne au lieu de remplacer un autre
  placement attendu ;
- un meme signal n'est redige qu'une fois par defaut. S'il est candidat a un
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
- `aspect_context.writing_guidance` : guidance de la valence ou du modificateur
  depuis `astral_interpretive_valence`, completee par le runtime quand il faut
  rappeler qu'un modificateur d'intensite ne doit pas etre lu comme une valence
  favorable ou difficile.
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

Le payload expose maintenant `chart_emphasis`, calcule cote code avant tout appel
LLM. Cette couche resume la hierarchie globale du theme sans demander au LLM de
deduire seul les dominantes depuis les signaux bruts :

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

L'etape 2D.1 ajoute ensuite des references legeres dans le plan de redaction :

```json
{
  "slot": "dominant_cluster",
  "emphasis_refs": {
    "dominant_signs": ["gemini"],
    "dominant_houses": [9],
    "dominant_objects": ["mercury", "sun", "jupiter"]
  }
}
```

Ces references sont attachees au slot `dominant_cluster` quand il existe. Sinon,
elles sont attachees au slot `core_identity` en fallback. Les autres slots
gardent des `emphasis_refs` vides. Le LLM doit les lire comme un contexte de
poids relatif pour les sections existantes, jamais comme une invitation a creer
une section autonome `chart_emphasis`.

## Signaux agreges Basic

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
  "writing_guidance": "Use this cluster before individual placements and merge repeated wording from its source signals.",
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
le meme couple `(sign_code, house_number)`. Il entre dans le meme filtrage Basic
que les autres signaux et compte donc dans la limite des 12 signaux actifs.

## Filtrage Basic

Le filtrage est applique dans `signals.rs` :

- les signaux sont tries par `priority_score` decroissant ;
- les aspects dont `strength_score < 0.4` passent en `suppressed` ;
- les aspects angle-angle passent aussi en `suppressed` des l'agregation, sauf
  les axes structurels Ascendant-Descendant et MC-IC qui ne creent pas de signal
  Basic du tout ;
- les signaux `dignity:*` sont ajoutes avant le tri final quand la dignite est
  majeure et suffisamment significative ;
- les clusters semantiques sont ajoutes avant le tri final ;
- les sources secondaires d'un cluster retenu actif passent en `merged`, sauf
  Soleil, Lune, Ascendant et MC qui restent actifs comme marqueurs centraux ;
- quand des fusions liberent des places dans les 12 signaux Basic, le runtime
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
  dynamique redactionnelle exploitable ;
- seuls les signaux actifs relus depuis la DB restent eligibles au payload ;
- `payload.rs` filtre encore les anciens aspects d'axe structurel non marques et
  les anciens aspects angle-angle actifs quand les positions d'angle definissent
  ces objets, puis tronque le resultat final a 12 signaux comme garde de lecture.

Les signaux supprimes restent persistables dans `astral_interpretation_signals`
avec `suppression_state = 'suppressed'`, mais ne remontent pas dans le payload
Basic final.

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

## Plan de lecture Basic

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

Le plan est construit dans `payload.rs` a partir des signaux actifs, avec les
slots suivants quand les sources correspondantes existent :

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
rend explicite que ces signaux doivent etre rediges dans ce slot. Quand un
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

`drafting_plan` reprend exactement `source_signal_keys`, `primary_signal_keys`
et `secondary_slot_candidates` du `reading_plan`.

Apres cette deduplication editoriale, un slot qui n'a plus aucune source
primaire est supprime du `reading_plan`. Le `drafting_plan` etant derive du
`reading_plan`, il ne contient donc pas de section sans `source_signal_keys`.
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

Cette logique s'applique aussi quand le filtrage Basic a du liberer une place
dans les 12 signaux actifs : un signal actif non essentiel peut etre remplace par
la meilleure tension forte disponible selon le garde-fou geometrique
carre/opposition, hors axes structurels d'angles et hors aspects angle-angle.
Les clusters et les marqueurs centraux ou expressifs restent proteges ; un signal
de dignite autonome peut en revanche ceder sa place si le budget est sature et
qu'aucune tension forte n'est encore active.

Depuis 2E.3, si aucune tension forte n'est disponible mais qu'un aspect fort
planete-planete ou planete-angle existe, le meme mecanisme preserve le meilleur
aspect fort disponible. `main_tension_or_support` n'est donc absent que
lorsqu'aucun aspect actif ou preservable ne reste apres l'exclusion des axes
structurels et des autres aspects angle-angle.

Depuis la passe de stabilisation du contrat, `basic_natal_structured_v8` est
verrouille par trois niveaux complementaires :

- le JSON Schema
  `rust_sqlx_connection_test/schemas/basic_natal_structured_v8.schema.json`
  valide la forme du contrat, les constantes LLM, les blocs obligatoires, les
  quatre angles, les bornes de score et les contraintes schema exprimables. Il
  refuse aussi les extensions silencieuses de `must_use`, les champs
  semantiques de signal obligatoires a `null`, les contextes de position
  obligatoires a `null` et les proprietes parasites dans `aspect_context` ;
- la fixture `tests/golden/basic_payload_v8_paris_1990.json` conserve un payload
  complet de reference pour le scenario Paris 1990 ;
- `tests/contract_basic_v8_tests.rs` valide les invariants metier qui ne
  doivent pas regresser : sources de plan existantes, alignement
  `reading_plan` / `drafting_plan`, absence d'aspect angle-angle actif,
  conservation de `aspect:jupiter:uranus:opposition`, unicite des signaux
  primaires et garde-fou contre une section autonome `chart_emphasis`.

La review adversariale de cette stabilisation a ajoute des tests negatifs et a
resserre l'alignement entre schema et validation runtime. Un payload qui valide
le schema ne doit plus pouvoir contourner les champs obligatoires attendus par
`is_current_basic_payload`, et un payload runtime ne doit plus etre considere
courant si ses angles top-level ne sont pas exactement le quatuor canonique.

La regeneration complete du golden depend de Postgres et de Swiss Ephemeris. Le
test unitaire ne reconstruit donc pas le theme depuis le moteur. Pour couvrir ce
risque en CI ou en verification locale, le script
`scripts/verify_basic_v8_golden.ps1` lance le moteur avec le scenario golden
Paris / `1990-01-02T03:04:05Z`, puis compare une projection stable du payload
genere au golden. Le script force les variables d'environnement du scenario
golden et les restaure ensuite, afin d'eviter qu'un `ASTRAL_OUTPUT_MODE`,
`ASTRAL_PRODUCT_CODE` ou identifiant de referentiel deja present ne modifie la
verification. Il peut aussi comparer un fichier deja genere via :

```powershell
.\scripts\verify_basic_v8_golden.ps1 -GeneratedPayloadPath .\output\basic_payload_current.json
```

## Contrat canonique de handoff LLM

Le payload final contient aussi `llm_handoff_contract` et `drafting_plan`.
`llm_handoff_contract` pose les contraintes globales que le futur service LLM
doit respecter :

```json
{
  "llm_handoff_contract": {
    "contract_version": "basic_natal_structured_v8",
    "payload_language_code": "en",
    "target_language_policy": "provided_by_llm_service",
    "audience_level": "beginner",
    "tone": "clear, warm, non fatalistic",
    "must_use": [
      "chart_emphasis",
      "dignities",
      "angles",
      "signals",
      "reading_plan",
      "drafting_plan"
    ],
    "must_not": [
      "invent facts not present in source signals",
      "mention technical IDs",
      "list placements mechanically",
      "translate technical keys such as signal_key, theme_code, semantic_tags, slot, or aggregation_group",
      "expose raw evidence unless explicitly requested",
      "treat chart_emphasis as a standalone section instead of weighting context",
      "make deterministic or fatalistic predictions"
    ],
    "output_format": "structured_sections"
  }
}
```

`drafting_plan` est derive du
`reading_plan` et conserve les memes slots et les memes sources, mais les rend
directement exploitables par une future couche de generation controlee.

Les champs redactionnels generes par `drafting_plan` sont en anglais. Cette
contrainte couvre `section_title`, `writing_objective` et `avoid`, afin que le
payload Basic reste coherent avec les libelles de referentiel exposes par le
runtime. Le moteur peut exprimer des contraintes comme `plain language`,
`beginner` ou `non fatalistic`, mais il ne doit jamais demander de rediger dans
une langue cible finale.

```json
{
  "drafting_plan": [
    {
      "slot": "dominant_cluster",
      "section_title": "A Capricorn dominant theme around Resources",
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
      ],
      "writing_objective": "Explain in plain language that the chart emphasizes Capricorn, Resources, and structure, responsibility, security, grouping the cluster evidence and Saturn dignity context instead of enumerating placements.",
      "max_words": 120,
      "avoid": [
        "repeat each placement one by one",
        "use technical IDs",
        "make fatalistic predictions",
        "add information that is absent from the source signals"
      ]
    }
  ]
}
```

Les slots ont des objectifs specialises :

- `core_identity` : presenter les marqueurs centraux ;
- `dominant_cluster` : expliquer la dominante sans enumerer chaque placement ;
- `main_tension_or_support` : decrire les dynamiques principales en distinguant
  appuis et tensions a partir de la valence interpretative des aspects ;
- `expression_style` : synthetiser pensee, communication, desir et action ;
- `background_factors` : garder les facteurs de fond proportionnes.

Cette couche est volontairement contractuelle : elle donne au futur LLM une
liste de sections a rediger, mais elle ne lui demande pas encore de produire un
theme complet libre. Le service LLM recevra ce payload canonique avec un champ
separe comme `target_language_code = "fr"` et produira la sortie localisee dans
un module distinct.

La validation de reutilisation des payloads existants force maintenant aussi :

- un `llm_handoff_contract` canonique exact pour Basic ;
- pour chaque signal `aspect:*`, un `aspect_context` avec famille, valence
  primaire eventuelle, modificateur d'intensite eventuel, qualite dynamique,
  phase, `valence_family`, flags tonal/intensite et guidance redactionnelle ;
- pour chaque signal `aspect:*`, au moins un effet interpretatif non vide parmi
  `primary_valence`, `intensity_modifier` ou `secondary_effect` ;
- des slots connus uniquement ;
- l'ordre canonique des slots Basic ;
- un `drafting_plan` strictement aligne sur les sources du `reading_plan` ;
- l'absence de lettres non ASCII dans les consignes redactionnelles 1D.

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
relit ces aspects via les joins de `repositories.rs` afin d'ajouter la famille,
les effets interpretatifs et la guidance issus des referentiels. Ce meme chemin
est utilise pour un calcul frais et pour la regeneration d'un payload existant
juge obsolete.

Dans cette table, `language_id` designe la langue canonique du payload, pas la
langue cible utilisateur. Pour le moteur Rust, le runtime ecrit toujours la
langue canonique `en`, afin de ne pas dupliquer le meme payload pour `fr`, `it`,
`es`, etc.

Les sorties localisees produites par le service LLM doivent etre stockees dans
une table separee :

`astral_interpretation_generated_outputs`

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
- `llm_handoff_contract` present et conforme au contrat canonique
  `basic_natal_structured_v8` ;
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
- signaux avec `evidence` objet non nul ;
- signaux avec `theme_code`, `summary`, `confidence_score`,
  `interpretive_hint`, `semantic_tags`, `source_weight`, `aggregation_group` et
  `writing_guidance` non nuls et non vides quand le type est textuel ;
- aucun signal actif `aspect:*` entre deux angles ; les anciens payloads qui
  contiennent un aspect angle-angle actif sont regeneres ;
- signaux `angle:*` avec `evidence.fact_type = "chart_angle"` et
  `evidence.opposite_angle_code` court non vide ainsi que
  `evidence.opposite_angle_object_code` coherent avec
  `angles[].opposite_angle_code` ;
- signaux `aspect:*` avec `aspect_context` complet, au moins un effet
  interpretatif non vide, `dynamic_quality`, `phase_state`, `valence_family`,
  `is_tonal_valence`, `is_intensity_modifier` et `writing_guidance` ;
- absence de signal `aspect:*` qui represente une opposition structurelle entre
  deux angles du meme `axis`, meme si l'ancien signal n'est pas marque
  `is_structural_axis` ;
- signaux de placement avec `evidence.placement_context` complet ;
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
- `drafting_plan` present, non vide, aligne sur les slots et sources du
  `reading_plan`, sans section vide, avec `section_title`,
  `writing_objective`, `max_words` et `avoid` renseignes ;
- `drafting_plan[].emphasis_refs` aligne sur `chart_emphasis`, renseigne sur
  `dominant_cluster` quand ce slot existe, sinon sur `core_identity`, et vide
  sur les autres slots ;
- chaque item de `drafting_plan` contient la regle d'evitement
  `turn chart_emphasis into a standalone section` ;
- `primary_signal_keys` aligne avec `source_signal_keys`, et
  `secondary_slot_candidates` coherents entre `reading_plan` et `drafting_plan` ;
- chaque signal primaire apparait dans un seul slot de `reading_plan`; les
  candidats editoriaux supplementaires passent par `secondary_slot_candidates` ;
- slots du `reading_plan` connus et dans l'ordre Basic canonique ;
- champs redactionnels du `drafting_plan` en anglais canonique, sans lettres
  non ASCII ;
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

Le run attendu doit afficher le payload canonique Basic. Il doit contenir :

- `product_code = "basic"` ;
- `llm_handoff_contract.payload_language_code = "en"` ;
- `llm_handoff_contract.target_language_policy = "provided_by_llm_service"` ;
- `llm_handoff_contract.contract_version = "basic_natal_structured_v8"` ;
- des positions avec `sign_code`, `sign_name`, `house_number`, `house_name`,
  `sign_context`, `house_context`, `house_modality`, `object_context` et
  `dignity_context` sous forme de tableau, vide quand aucune dignite n'est
  detectee ; `motion_context` est present pour les objets mobiles et peut etre
  `null` pour les angles ;
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
- au plus 12 signaux ;
- un `reading_plan` non vide ;
- un `reading_plan` sans slot vide et sans opposition structurelle d'angle dans
  `main_tension_or_support` ;
- un `drafting_plan` non vide, sans slot vide, et aligne sur le `reading_plan` ;
- des `emphasis_refs` dans `drafting_plan`, rattachees au slot
  `dominant_cluster` si present, sinon a `core_identity`, et utilisees comme
  contexte de ponderation ;
- des titres sans IDs techniques ;
- des champs semantiques 1B sur chaque signal ;
- un `aspect_context` sur chaque signal `aspect:*`, avec les modificateurs
  d'intensite separes de la valence primaire, et les flags
  `is_tonal_valence` / `is_intensity_modifier` renseignes ;
- aucun signal actif `aspect:ascendant:descendant:opposition` ou
  `aspect:mc:ic:opposition` produit par les axes structurels ;
- un `evidence.placement_context` complet sur chaque signal de placement ;
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

- Les angles Basic sont exposes, mais leurs interpretations restent limitees aux
  faits structures et aux signaux `angle:*`.
- Les resumes restent des phrases templatees, pas une interpretation finale.
- Les `interpretive_hint` et `writing_guidance` restent aussi des templates,
  meme si les hints d'aspect integrent maintenant la valence 2C.
- Les clusters Basic ne couvrent pour l'instant que les concentrations
  `sign_house`.
- Le moteur de dignites 2B est un MVP code-side. Il couvre les dignites
  essentielles majeures par signe, pas encore les dignites mineures ni les
  dignites accidentelles.
- Le programme consomme les libelles des referentiels tels quels. Il ne gere pas la traduction.
- La redaction LLM doit rester une etape ulterieure.

## Organisation du module payload

`rust_sqlx_connection_test/src/payload.rs` a ete remplace par le dossier
`rust_sqlx_connection_test/src/payload/` afin de separer les responsabilites
sans modifier le contrat public `rust_sqlx_connection_test::payload`.

- `mod.rs` orchestre la construction du `BasicPayload`.
- `angles.rs`, `dignities.rs`, `emphasis.rs`, `reading_plan.rs` et
  `drafting_plan.rs` isolent les blocs metier du payload Basic.
- `signal_filters.rs` centralise les predicats partages sur les signaux et
  aspects.
- `json.rs` centralise les extractions defensives depuis les payloads JSON.
- `contract.rs` conserve le contrat LLM Basic v8.

Ce decoupage reste volontairement simple: aucune nouvelle donnee canonique n'a
ete ajoutee en dur, et les fonctions gardent une portee limitee au module quand
elles ne font pas partie de l'API publique.

## Organisation du module signals

`rust_sqlx_connection_test/src/signals.rs` a ete remplace par le dossier
`rust_sqlx_connection_test/src/signals/` afin de separer l'agregation des
signaux Basic par responsabilite, sans modifier l'API publique
`rust_sqlx_connection_test::signals`.

- `mod.rs` conserve l'orchestration de `aggregate_basic_signals`.
- `constants.rs` centralise les constantes partagees du module.
- `angles.rs`, `positions.rs`, `dignity.rs`, `dignity_helpers.rs`,
  `aspect_signals.rs` et `clusters.rs` isolent la construction des familles de
  signaux.
- `limits.rs` regroupe les regles de suppression, preservation et remplissage
  de la limite Basic.
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
en contexte de signal Basic. Les fusionner melangerait le calcul des faits et
la preparation editoriale du payload.

Ce refactor reste strictement structurel: aucune nouvelle donnee canonique n'a
ete ajoutee en dur et le comportement conserve est valide par:

```powershell
cargo fmt --manifest-path rust_sqlx_connection_test/Cargo.toml
cargo clippy --manifest-path rust_sqlx_connection_test/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path rust_sqlx_connection_test/Cargo.toml
```
