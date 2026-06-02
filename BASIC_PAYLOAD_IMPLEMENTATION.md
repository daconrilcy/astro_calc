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

Le runtime conserve la chaine existante :

1. calcul des faits astrologiques ;
2. ecriture des positions, cuspides et aspects calcules ;
3. aggregation des signaux ;
4. filtrage produit Basic ;
5. ecriture du payload canonique dans `astral_interpretation_generation_payloads`.

Cette etape prepare donc une entree propre, lisible, auditable et pre-orientee
editorialement.

## Fichiers concernes

- `rust_sqlx_connection_test/src/domain.rs` : structures runtime et payload JSON.
- `rust_sqlx_connection_test/src/facts.rs` : helpers de libelles signe/maison.
- `rust_sqlx_connection_test/src/ephemeris.rs` : enrichissement des positions calculees.
- `rust_sqlx_connection_test/src/aspects.rs` : detection des aspects avec libelles objet/aspect.
- `rust_sqlx_connection_test/src/signals.rs` : construction et filtrage des signaux Basic.
- `rust_sqlx_connection_test/src/payload.rs` : assemblage du payload final.
- `rust_sqlx_connection_test/src/repositories.rs` : enrichissement SQL et persistance.
- `rust_sqlx_connection_test/src/runtime.rs` : orchestration et regeneration des anciens payloads.

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
  }
}
```

Les IDs restent presents pour l'audit et les relations DB. Les libelles viennent :

- des referentiels `astral_signs`, `astral_sign_profiles`,
  `astral_sign_keywords`, `astral_houses`, `astral_house_modalities`,
  `astral_chart_object_definitions`, `astral_object_nature_assignments` et
  `astral_object_motion_states` charges avant le calcul pour les nouveaux faits ;
- des joins equivalents quand un payload est reconstruit depuis la DB.

Le calcul geometrique conserve seulement les operations derivees de la
longitude : slot zodiacal et numero de maison. Les IDs, codes et noms de signes
ou de maisons sont resolus depuis les tables. Le runtime refuse de calculer si
les 12 signes ou les 12 maisons ne sont pas presents ou si les references sont
ambigues. Le contrat Basic v2 refuse aussi de reutiliser un payload existant si
les contextes de placement utiles sont absents ou incomplets.

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
      }
    }
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
  "interpretive_hint": "Sun and Mercury are connected by a conjunction, so their functions should be read together with attention to the separating phase.",
  "semantic_tags": [
    "aspect",
    "conjunction",
    "high_strength"
  ],
  "source_weight": 1.75,
  "aggregation_group": "aspect:conjunction",
  "writing_guidance": "Use the aspect as a relationship between two chart factors, not as a standalone verdict.",
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
    "orb_deg": 1.0084,
    "phase_state": "separating",
    "strength_score": 0.874
  }
}
```

### Champs semantiques 1B

Les champs ajoutes par l'etape 1B sont :

- `theme_code` : theme editorial principal du signal, derive de la maison pour
  les placements quand elle est connue, ou de la famille de signal pour les
  aspects.
- `interpretive_hint` : phrase courte orientee utilisateur, mais encore
  templatee.
- `semantic_tags` : tags stables utiles pour grouper, filtrer ou guider la
  redaction.
- `source_weight` : poids relatif de la source astrologique. Soleil et Lune
  valent plus que les planetes lentes.
- `aggregation_group` : cle de regroupement editoriale.
- `writing_guidance` : consigne courte pour la future couche de redaction.

Ces champs sont stockes dans `astral_interpretation_signals.payload_json`, puis
remontes dans le payload final par `payload.rs`.

### Champs contextuels 2A

Les champs ajoutes par l'etape 2A sont volontairement limites aux preuves utiles
pour le calcul et la redaction. Ils ne recopient pas les faits runtime bruts,
mais ils peuvent embarquer un referentiel semantique complet quand ce
referentiel est directement exploitable par le LLM.

- `sign_context` : element, modalite zodiacale, polarite et liste complete des
  mots-cles principaux du signe depuis `astral_sign_keywords.keywords_json`.
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
- les clusters semantiques sont ajoutes avant le tri final ;
- les sources secondaires d'un cluster retenu actif passent en `merged`, sauf
  Soleil, Lune, Ascendant et MC qui restent actifs comme marqueurs centraux ;
- quand des fusions liberent des places dans les 12 signaux Basic, le runtime
  remonte les prochains signaux eligibles sans reactiver les aspects faibles ;
- seuls les 12 premiers signaux actifs restent eligibles au payload ;
- `payload.rs` applique aussi `.take(12)` comme garde de lecture.

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
      ]
    },
    {
      "slot": "dominant_cluster",
      "title": "Dominant repeated theme",
      "source_signal_keys": [
        "cluster:capricorn:house_2",
        "object_position:sun"
      ]
    },
    {
      "slot": "main_tension_or_support",
      "title": "Main dynamic aspect",
      "source_signal_keys": [
        "aspect:sun:neptune:conjunction"
      ]
    }
  ]
}
```

Le plan est construit dans `payload.rs` a partir des signaux actifs, avec les
slots suivants quand les sources correspondantes existent :

- `core_identity` : Soleil, Lune, Ascendant, MC ;
- `dominant_cluster` : premier cluster actif et sources actives associees ;
- `main_tension_or_support` : jusqu'a trois aspects actifs prioritaires ;
- `expression_style` : Mercure, Venus, Mars ;
- `background_factors` : Jupiter, Saturne, Uranus, Neptune, Pluton si encore
  actifs.

Pour eviter une lecture trop lisse, `main_tension_or_support` force maintenant
l'inclusion d'au moins un aspect de tension fort quand un carre ou une opposition
atteint `strength_score >= 0.75`. Si les trois premiers aspects prioritaires ne
contiennent aucune tension forte, le troisieme est remplace par cette tension.

## Contrat canonique de handoff LLM

Le payload final contient aussi `llm_handoff_contract` et `drafting_plan`.
`llm_handoff_contract` pose les contraintes globales que le futur service LLM
doit respecter :

```json
{
  "llm_handoff_contract": {
    "contract_version": "basic_natal_structured_v2",
    "payload_language_code": "en",
    "target_language_policy": "provided_by_llm_service",
    "audience_level": "beginner",
    "tone": "clear, warm, non fatalistic",
    "must_use": [
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
        "object_position:sun"
      ],
      "writing_objective": "Explain in plain language that the chart emphasizes Capricorn, Resources, and structure, responsibility, security, grouping the related placements instead of enumerating them.",
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
  appuis et tensions ;
- `expression_style` : synthetiser pensee, communication, desir et action ;
- `background_factors` : garder les facteurs de fond proportionnes.

Cette couche est volontairement contractuelle : elle donne au futur LLM une
liste de sections a rediger, mais elle ne lui demande pas encore de produire un
theme complet libre. Le service LLM recevra ce payload canonique avec un champ
separe comme `target_language_code = "fr"` et produira la sortie localisee dans
un module distinct.

La validation de reutilisation des payloads existants force maintenant aussi :

- un `llm_handoff_contract` canonique exact pour Basic ;
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
  `basic_natal_structured_v2` ;
- positions avec `sign_code`, `sign_name`, `sign_context`, `house_modality`,
  `object_context` et `motion_context` ;
- signaux avec `evidence` ;
- signaux avec `theme_code`, `interpretive_hint`, `semantic_tags`,
  `aggregation_group` et `writing_guidance` non vides ;
- signaux de placement avec `evidence.placement_context` complet ;
- `reading_plan` present, non vide, compose de slots uniques et de sources qui
  existent dans les signaux du payload ;
- `drafting_plan` present, non vide, aligne sur les slots et sources du
  `reading_plan`, avec `section_title`, `writing_objective`, `max_words` et
  `avoid` renseignes ;
- slots du `reading_plan` connus et dans l'ordre Basic canonique ;
- champs redactionnels du `drafting_plan` en anglais canonique, sans lettres
  non ASCII ;
- absence d'anciens templates connus comme `by a opposition`.

Sinon, les signaux sont reconstruits depuis les positions et aspects persistants,
puis le payload est reecrit.

## Verification

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

Le run attendu doit afficher le payload canonique Basic. Il doit contenir :

- `product_code = "basic"` ;
- `llm_handoff_contract.payload_language_code = "en"` ;
- `llm_handoff_contract.target_language_policy = "provided_by_llm_service"` ;
- des positions avec `sign_code`, `sign_name`, `house_number`, `house_name`,
  `sign_context`, `house_modality`, `object_context` et `motion_context` ;
- au plus 12 signaux ;
- un `reading_plan` non vide ;
- un `drafting_plan` non vide et aligne sur le `reading_plan` ;
- des titres sans IDs techniques ;
- des champs semantiques 1B sur chaque signal ;
- un `evidence.placement_context` complet sur chaque signal de placement ;
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

- L'Ascendant et le MC ne sont pas encore exposes comme objets de position Basic.
- Les resumes restent des phrases templatees, pas une interpretation finale.
- Les `interpretive_hint` et `writing_guidance` restent aussi des templates.
- Les clusters Basic ne couvrent pour l'instant que les concentrations
  `sign_house`.
- Le programme consomme les libelles des referentiels tels quels. Il ne gere pas la traduction.
- La redaction LLM doit rester une etape ulterieure.
