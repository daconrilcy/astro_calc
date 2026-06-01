# Remarques de developpement par table

Ce document centralise les remarques de developpement liees aux tables de reference du projet.

Chaque section doit rester centree sur une table precise et decrire :

- le role de la table ;
- les champs importants ;
- les choix de modelisation retenus ;
- l'utilisation attendue cote code ;
- les points de vigilance runtime.

Tables documentees pour l'instant :

- `astral_condition_operators`
- `astral_chart_objects`
- `astral_chart_object_definitions`
- `astral_heliacal_conditions`
- `astral_horizon_positions`
- `astral_house_modalities`
- `astral_houses`
- `astral_house_systems`
- `astral_interpretive_valence`
- `astral_object_motion_states`
- `astral_object_nature_types`
- `astral_object_nature_assignments`
- `astral_object_sign_dignities`
- `astral_object_interpretation_profiles`
- `interpretive_condition_signal_profiles`
- `prediction_object_category_weights`
- `astral_sign_genders`
- `astral_sign_keywords`
- `astral_speed_classes`
- `astral_speed_relations`
- `astral_prediction_calculation_profiles`

## astral_condition_operators

### Role de la table

`astral_condition_operators` est le catalogue des operateurs atomiques utilisables par le moteur de regles astrologiques.

Chaque ligne de la table decrit un test ou un calcul bas niveau, par exemple :

- tester si un objet est dans un signe ;
- tester si un objet est proche d'un angle ;
- calculer une distance angulaire ;
- combiner plusieurs conditions logiques ;
- comparer une valeur numerique calculee.

La table ne doit pas porter l'interpretation astrologique finale. Elle sert a separer trois niveaux :

- **calcul** : produire une valeur factuelle, par exemple une distance angulaire ;
- **condition** : transformer une valeur ou une configuration en resultat `true` / `false` ;
- **interpretation** : produire une lecture humaine a partir des conditions validees.

### Champs principaux

| Champ | Role |
| --- | --- |
| `id` | Identifiant stable de l'operateur. Il peut etre reference par les regles, comme `astral_accidental_dignity_rules.condition_json.condition_operator_id`. |
| `code` | Cle technique stable utilisee par le runtime pour retrouver la fonction d'evaluation associee. |
| `label` | Libelle lisible de l'operateur. |
| `operator_family` | Famille fonctionnelle : longitude zodiacale, aspect, signe, maison, mouvement, secte, composition logique, etc. |
| `description` | Ce que l'operateur teste ou calcule. |
| `utility` | Pourquoi l'operateur existe et dans quels types de regles il est utile. |
| `input_contract` | Contrat d'entree lisible par un humain. Il reste documentaire. |
| `input_schema` | Contrat d'entree structure, exploitable par le runtime pour valider les payloads avant evaluation. |
| `result_type` | Type de resultat attendu : `boolean` pour une condition, `number` pour un calcul numerique. |
| `implementation_notes` | Points de vigilance pour l'execution, comme la longitude circulaire ou le passage par 0 degre Belier. |
| `sort_order` | Ordre d'affichage ou de lecture. |

### Option A retenue

L'option A separe les operateurs de calcul des operateurs de condition.

Les operateurs suivants retournent maintenant une valeur numerique :

- `angular_distance_between_objects` -> `number`
- `shortest_angular_distance_between_objects` -> `number`

Ils ne doivent plus contenir eux-memes la comparaison avec un seuil. Pour transformer leur resultat en condition booleenne, le moteur doit utiliser des operateurs numeriques separes :

- `compare_number`
- `value_between`

Cette separation permet de tracer plus clairement le calcul effectue, puis le test applique sur ce calcul.

### Utilisation prevue dans le code

Le backend doit charger les operateurs depuis PostgreSQL ou depuis `json_db/astral_condition_operators.json`, puis construire un registre d'execution qui associe chaque `code` a une fonction pure d'evaluation.

Le role du registre est de :

- verifier qu'un operateur reference par une regle existe ;
- valider le payload de condition avec `input_schema` ;
- router la condition vers la bonne fonction d'evaluation ;
- retourner un resultat explicable ;
- supporter les operateurs logiques recursifs comme `all_conditions_true`, `any_condition_true` et `not_condition` ;
- eviter de melanger calcul, condition et interpretation.

Chaque fonction d'evaluation devrait recevoir les memes familles d'informations :

- `chart_context` : donnees calculees du theme ;
- `condition_payload` : parametres concrets de la regle ;
- `operator_definition` : ligne de definition issue de cette table.

Le resultat retourne doit etre tracable. Il doit indiquer :

- si la condition a matche ;
- quel `operator_code` a ete utilise ;
- quels inputs ont ete evalues ;
- quelles valeurs ont ete calculees ;
- eventuellement pourquoi la condition a echoue.

Exemple de logique attendue pour l'option A :

1. `shortest_angular_distance_between_objects` calcule une distance, par exemple `7.2`.
2. `value_between` teste si cette distance est comprise entre `0` et `8`.
3. La condition composee retourne `true` ou `false`.
4. L'interpretation humaine est produite ensuite par une autre couche.

### Points de vigilance runtime

Les operateurs de longitude circulaire doivent toujours gerer le passage par `0` degre Belier.

Pour les operateurs suivants, le champ `arc_mode` est obligatoire dans `input_schema` afin d'eviter deux implementations incompatibles :

- `target_between_two_objects_by_longitude`
- `target_outside_two_objects_by_longitude`
- `object_before_object_by_longitude`
- `object_after_object_by_longitude`

`arc_mode` doit preciser si l'arc teste est :

- l'arc direct dans l'ordre zodiacal ;
- l'arc le plus court ;
- un arc explicitement choisi par la regle metier.

### Relation avec les regles existantes

`between_two_planets_by_longitude` est conserve avec `id = 1` pour compatibilite avec les regles deja presentes dans `astral_accidental_dignity_rules`.

Les nouvelles regles devraient preferer `target_between_two_objects_by_longitude`, qui est plus generique et peut fonctionner avec des planetes, angles, noeuds, parts, cuspides, etoiles fixes ou points calcules.

## astral_heliacal_conditions

### Role de la table

`astral_heliacal_conditions` normalise les types de conditions heliacales simples utilises par les calculs astronomiques et astrologiques.

La table ne contient pas les evenements heliacaux eux-memes. Elle contient les types de conditions qui permettent ensuite de qualifier un resultat calcule, par exemple dans une table runtime ou d'audit comme `astral_heliacal_events`.

Elle distingue deux cas fondamentaux :

- `rising_before_sun` : l'astre se leve avant le Soleil ;
- `setting_after_sun` : l'astre se couche apres le Soleil.

### Champs principaux

| Champ | Role |
| --- | --- |
| `id` | Identifiant stable de la condition heliacale. |
| `code` | Cle technique stable utilisee dans les payloads de regles et les calculs runtime. |
| `label` | Libelle lisible de la condition. |
| `description` | Definition astronomique/astrologique de la condition. |
| `runtime_usage` | Usage attendu dans le moteur de calcul et les qualifications de visibilite solaire. |
| `sort_order` | Ordre d'affichage ou de lecture. |

### Definitions

`rising_before_sun` signifie que l'astre se leve avant le Soleil.

Dans ce cas, il peut etre visible a l'est avant l'aube, juste avant que la lumiere solaire ne le rende invisible. C'est la condition de base associee a un lever heliacale : un astre redevient visible dans le ciel du matin avant le lever du Soleil. Le lever heliacale est classiquement compris comme une premiere visibilite dans la lumiere de l'aube.

`setting_after_sun` signifie que l'astre se couche apres le Soleil.

Dans ce cas, il peut rester visible a l'ouest apres le coucher du Soleil. C'est la condition de base associee a une visibilite du soir, notamment au coucher heliacale du soir : dernier moment ou l'astre reste visible apres le coucher du Soleil avant de devenir trop proche du Soleil pour etre observe.

### Utilisation prevue dans le code

Le moteur doit utiliser cette table comme reference de qualification, pas comme source d'evenements.

Une table de resultats ou d'evenements peut ensuite referencer cette condition, par exemple :

| Champ | Role |
| --- | --- |
| `chart_object_id` | Objet astrologique observe. |
| `date` | Date du calcul. |
| `condition_id` | Reference vers `astral_heliacal_conditions.id`. |
| `sun_rise_time` | Heure de lever du Soleil. |
| `body_rise_time` | Heure de lever de l'astre. |
| `sun_set_time` | Heure de coucher du Soleil. |
| `body_set_time` | Heure de coucher de l'astre. |

Exemples de logique runtime :

1. Venus se leve a `05:12`.
2. Le Soleil se leve a `06:43`.
3. Le moteur qualifie la condition comme `rising_before_sun`.

Autre cas :

1. Le Soleil se couche a `18:20`.
2. Mars se couche a `20:04`.
3. Le moteur qualifie la condition comme `setting_after_sun`.

### Points de vigilance runtime

Cette table ne suffit pas a elle seule a conclure qu'un evenement heliacale complet a eu lieu. Elle indique seulement la relation temporelle de base entre l'astre et le Soleil.

Pour qualifier un phenomene comme `morning star`, `evening star`, lever heliacale, coucher heliacale, visibilite solaire ou invisibilite solaire, le moteur devra aussi tenir compte des seuils de visibilite, de la separation angulaire, de la magnitude, des conditions d'observation et de la politique de calcul retenue.

### Relation avec les regles existantes

La table est actuellement referencee dans `astral_accidental_dignity_rules.condition_json` via `heliacal_condition_id`.

Les deux cas existants correspondent aux regles suivantes :

- `heliacal_condition_id = 1` : `rising_before_sun` ;
- `heliacal_condition_id = 2` : `setting_after_sun`.

## astral_horizon_positions

### Role de la table

`astral_horizon_positions` normalise les conditions de position locale d'un astre par rapport a l'horizon pour un lieu et une heure donnes.

Cette table est dans le meme esprit que `astral_heliacal_conditions`, car elle decrit aussi des conditions astronomico-astrologiques de bas niveau. Elle ne represente toutefois pas la meme famille :

- `astral_heliacal_conditions` decrit une relation temporelle avec le Soleil ;
- `astral_horizon_positions` decrit une position spatiale locale par rapport a l'horizon.

Elle sert notamment a distinguer :

- `above_horizon` : l'astre est au-dessus de l'horizon local ;
- `below_horizon` : l'astre est sous l'horizon local ;
- `on_horizon` : l'astre est sur ou tres proche de l'horizon selon une tolerance technique.

### Champs principaux

| Champ | Role |
| --- | --- |
| `id` | Identifiant stable de la position horizon. |
| `code` | Cle technique stable utilisee dans les payloads de regles et les calculs runtime. |
| `label` | Libelle lisible de la position. |
| `family` | Famille de condition, ici `horizon_position`. |
| `description` | Definition astronomique/astrologique de la position. |
| `runtime_usage` | Usage attendu dans le moteur de calcul. |
| `result_type` | Type de resultat, ici `boolean_condition`. |
| `calculation_notes` | Notes de calcul et points de vigilance pour le runtime. |
| `sort_order` | Ordre d'affichage ou de lecture. |

### Definitions

`above_horizon` signifie que l'astre est geometriquement au-dessus de l'horizon local.

Exemple :

1. Mars a une altitude de `+23°`.
2. Le moteur qualifie la position comme `above_horizon`.

`below_horizon` signifie que l'astre est sous l'horizon local et n'est donc pas directement visible geometriquement.

Exemple :

1. Venus a une altitude de `-8°`.
2. Le moteur qualifie la position comme `below_horizon`.

`on_horizon` couvre le cas limite ou l'astre est exactement sur l'horizon ou dans une tolerance proche de l'Ascendant/Descendant.

Ce cas evite de classer arbitrairement un astre en `above_horizon` ou `below_horizon` quand son altitude ou son ecart a l'horizon est proche de zero.

### Utilisation prevue dans le code

Le moteur doit utiliser cette table comme reference de qualification locale.

Elle est utile pour :

- determiner l'hemisphere visible ou invisible d'un theme ;
- etablir la secte : theme diurne si le Soleil est au-dessus de l'horizon, theme nocturne s'il est sous l'horizon ;
- qualifier les conditions de visibilite ;
- composer les conditions de hayz ;
- alimenter certaines dignites accidentelles.

Les maisons peuvent donner une approximation traditionnelle :

- maisons 7 a 12 : generalement au-dessus de l'horizon ;
- maisons 1 a 6 : generalement sous l'horizon.

Pour un moteur robuste, il faut toutefois privilegier le calcul direct de l'altitude locale ou de la position par rapport a l'axe Ascendant/Descendant plutot qu'un raccourci fonde uniquement sur les maisons, surtout avec certains systemes de domification.

### Points de vigilance runtime

Une planete peut satisfaire une condition heliacale comme `rising_before_sun` tout en n'etant pas reellement visible si elle est encore trop basse, trop proche du Soleil, ou sous l'horizon au moment d'observation retenu.

Le moteur devrait donc traiter les positions horizon comme des briques combinees avec d'autres donnees :

- separation angulaire au Soleil ;
- heure de lever/coucher de l'astre ;
- heure de lever/coucher du Soleil ;
- altitude locale ;
- magnitude et seuils de visibilite ;
- tolerance `horizon_delta_degrees` pour le cas `on_horizon`.

### Relation avec les regles existantes

La table est actuellement referencee dans `astral_accidental_dignity_rules.condition_json` via `horizon_position_id`.

Les IDs existants restent stables :

- `horizon_position_id = 1` : `above_horizon` ;
- `horizon_position_id = 2` : `below_horizon`.

Le nouvel ID `3` correspond a `on_horizon` et peut etre utilise plus tard par les regles qui ont besoin d'un traitement explicite des objets situes sur l'axe Ascendant/Descendant.

## astral_house_modalities

### Role de la table

`astral_house_modalities` normalise la puissance d'expression d'une maison astrologique selon sa position dans le theme.

Cette notion ne correspond pas aux modalites des signes (`cardinal`, `fixed`, `mutable`). Elle qualifie la dynamique des maisons :

- `angular` : action forte, visibilite, manifestation directe ;
- `succedent` : consolidation, continuite, stabilite ;
- `cadent` : transition, arriere-plan, mentalisation, moindre visibilite externe.

La table sert de typologie stable pour les calculs de force contextuelle, notamment quand une planete est placee dans une maison.

### Champs principaux

| Champ | Role |
| --- | --- |
| `id` | Identifiant stable de la modalite de maison. |
| `name` | Cle technique stable : `angular`, `succedent`, `cadent`. |
| `label` | Libelle lisible de la modalite. |
| `description` | Definition astrologique de la modalite. |
| `house_numbers_json` | Liste documentaire des numeros de maisons concernes. |
| `accidental_strength` | Niveau de force accidentelle associe. |
| `score_modifier` | Symbole de ponderation technique (`+`, `0_or_light_plus`, `-`). |
| `interpretation_weight` | Poids indicatif pour prioriser l'interpretation. |
| `runtime_usage` | Usage attendu dans le moteur de calcul. |
| `sort_order` | Ordre d'affichage ou de lecture. |

### Definitions

`angular` concerne les maisons `1`, `4`, `7` et `10`.

Ces maisons sont liees aux quatre grands angles du theme :

- maison 1 : Ascendant ;
- maison 4 : Fond du Ciel / IC ;
- maison 7 : Descendant ;
- maison 10 : Milieu du Ciel / MC.

Elles sont considerees comme les maisons les plus fortes, les plus visibles et les plus actives. Dans le moteur, une planete en maison angulaire peut recevoir une ponderation forte pour sa capacite de manifestation.

`succedent` concerne les maisons `2`, `5`, `8` et `11`.

Ces maisons viennent apres les maisons angulaires. Elles stabilisent, maintiennent, developpent ou consolident ce qui a ete initie par les maisons angulaires. Elles portent une force moyenne : moins visibles que les angulaires, mais plus stables que les cadentes.

`cadent` concerne les maisons `3`, `6`, `9` et `12`.

Ces maisons precedent les maisons angulaires suivantes. Elles decrivent des zones preparatoires, mentales, adaptatives, de transition ou moins visibles. Elles peuvent reduire la manifestation externe immediate sans rendre le facteur astrologique mauvais.

### Utilisation prevue dans le code

Le moteur peut utiliser cette table comme facteur de ponderation pour :

- `planet_in_house` ;
- `accidental_dignity` ;
- `planetary_strength` ;
- `interpretation_priority` ;
- `visibility_score` ;
- `manifestation_power`.

La modalite de maison ne doit pas etre utilisee seule comme verdict final. Elle doit etre combinee avec d'autres facteurs, comme la proximite reelle aux angles, la secte, la vitesse, la combustion, les aspects, les dignites essentielles et les seuils retenus par le moteur.

## astral_houses

### Role de la table

`astral_houses` est la table canonique des douze maisons astrologiques.

Le doublon `astral_house` a ete supprime. Toutes les references doivent pointer vers `astral_houses.id`.

La table relie maintenant chaque maison a sa modalite via `house_modality_id`.

### Champs principaux

| Champ | Role |
| --- | --- |
| `id` | Identifiant stable de la maison. |
| `number` | Numero astrologique de la maison, de `1` a `12`. |
| `name` | Nom fonctionnel court de la maison. |
| `description` | Definition interpretative courte de la maison, utilisable par l'UI, les aides de lecture et les traces runtime. |
| `house_modality_id` | Reference vers `astral_house_modalities.id`. |

### Descriptions des maisons

Les descriptions de `astral_houses.description` donnent le champ interpretatif de base de chaque maison :

| Maison | Description courte |
| ---: | --- |
| 1 | Identite, presence physique, temperament, expression de soi, maniere d'entrer dans la vie. |
| 2 | Ressources personnelles, argent, possessions, valeurs, securite materielle. |
| 3 | Communication, apprentissage, fratrie, environnement proche, courts deplacements. |
| 4 | Racines, foyer, famille, vie privee, fondations, ascendance. |
| 5 | Creativite, plaisir, romance, enfants, jeu, joie personnelle. |
| 6 | Routines de travail, service, sante, maintenance, devoirs, competences pratiques. |
| 7 | Partenariat, mariage, contrats, adversaires declares, relation directe a l'autre. |
| 8 | Ressources partagees, dettes, heritage, crise, intimite, transformation. |
| 9 | Croyances, etudes superieures, philosophie, religion, droit, grands voyages. |
| 10 | Carriere, vocation, role public, reputation, autorite, ambition. |
| 11 | Amis, groupes, reseaux, alliances, projets collectifs, aspirations. |
| 12 | Retrait, choses cachees, solitude, inconscient, enfermement, liberation spirituelle. |

Ces descriptions restent des definitions de reference. L'interpretation finale doit venir des profils, des ponderations et du contexte calcule.

### Mapping des modalites

| Maison | Modalite |
| ---: | --- |
| 1 | `angular` |
| 2 | `succedent` |
| 3 | `cadent` |
| 4 | `angular` |
| 5 | `succedent` |
| 6 | `cadent` |
| 7 | `angular` |
| 8 | `succedent` |
| 9 | `cadent` |
| 10 | `angular` |
| 11 | `succedent` |
| 12 | `cadent` |

### Utilisation prevue dans le code

Les tables qui decrivent des interpretations ou des ponderations par maison doivent referencer `astral_houses.id`.

Pour evaluer la puissance d'expression d'une planete en maison, le moteur doit joindre :

1. la maison calculee pour la planete ;
2. `astral_houses.house_modality_id` ;
3. `astral_house_modalities` pour recuperer la force accidentelle, le symbole de ponderation et le poids d'interpretation.

Les axes de maisons utilisent aussi cette table :

- `astral_house_axis_members.house_id` -> `astral_houses.id` ;
- `astral_house_axis_members.opposite_house_id` -> `astral_houses.id`.

## astral_house_systems

### Role de la table

`astral_house_systems` definit les methodes de domification disponibles dans le moteur.

Cette table ne contient pas les maisons elles-memes. Elle decrit comment le moteur doit decouper un theme astral en douze maisons au moment du calcul.

Le systeme de maisons a un impact direct sur l'interpretation : une planete peut changer de maison selon que le theme est calcule en `placidus`, `whole_sign`, `equal` ou `porphyry`.

### Champs principaux

| Champ | Role |
| --- | --- |
| `id` | Identifiant technique stable. |
| `code` | Cle metier stable a utiliser cote code : `placidus`, `whole_sign`, `equal`, `porphyry`. |
| `name` | Nom lisible affiche dans l'interface. |
| `description` | Definition courte de la methode de domification. |
| `astronomical_family` | Famille astronomique : `quadrant`, `sign_based`, `ascendant_based`. |
| `supports_polar_regions` | Indique si la methode est utilisable dans les zones proches des poles. |
| `is_quadrant_based` | Indique si la methode s'appuie sur les quatre angles du theme. |
| `requires_exact_birth_time` | Indique si le calcul depend fortement de l'heure exacte de naissance. |
| `birth_time_precision_level` | Niveau de precision requis pour eviter l'ambiguite de l'ancien champ `requires_precise_birth_time`. |
| `is_default` | Systeme propose par defaut dans le produit. |
| `is_active` | Systeme disponible dans l'interface et le moteur. |
| `fallback_house_system_code` | Systeme de repli, par exemple pour les hautes latitudes. |
| `calculation_engine_code` | Cle de routage vers l'implementation de calcul. |
| `interpretation_note` | Note d'impact interpretatif. |
| `runtime_usage` | Usage attendu dans le moteur. |
| `sort_order` | Ordre d'affichage dans l'interface. |

### Definitions

`placidus` est le systeme par defaut pour un theme natal moderne occidental. Il est quadrant-based, depend fortement de l'heure et du lieu de naissance, et doit prevoir un fallback pour les hautes latitudes.

`whole_sign` associe chaque maison a un signe zodiacal entier a partir du signe ascendant. Il est plus robuste et moins sensible a la minute exacte, mais il exige quand meme une heure de naissance assez fiable pour determiner le signe ascendant.

`equal` produit douze maisons de `30` degres a partir du degre exact de l'Ascendant. Il conserve l'importance du degre ascendant sans produire les maisons inegales des systemes par quadrants.

`porphyry` divise chaque quadrant entre Ascendant, Milieu du Ciel, Descendant et Fond du Ciel en trois parties egales. C'est une alternative quadrant-based plus simple que Placidus.

### Utilisation prevue dans le code

Le moteur ne doit pas generer un theme natal sans conserver le `house_system_code` utilise.

Flux attendu :

1. recuperer les donnees de naissance ;
2. choisir le `house_system_code` ;
3. calculer les cuspides de maisons ;
4. assigner les planetes aux maisons ;
5. generer l'interpretation ;
6. stocker le theme avec le `house_system_code`.

Changer de systeme de maisons n'est pas une simple preference d'affichage. Ce choix peut changer les maisons des planetes et donc modifier les interpretations de type `planet_in_house`.

### Points de vigilance runtime

`whole_sign` ne doit pas etre traite comme calculable sans heure de naissance. Le champ `requires_exact_birth_time = false` signifie seulement qu'il est moins sensible a la minute exacte que les systemes par quadrants. Le moteur doit quand meme verifier que l'heure permet de determiner le signe ascendant.

Pour les systemes quadrant-based, le moteur doit gerer les cas de hautes latitudes et utiliser `fallback_house_system_code` quand la methode choisie ne peut pas produire un resultat fiable.

## astral_interpretive_valence

### Role de la table

`astral_interpretive_valence` classe l'effet interpretatif produit par une configuration astrologique.

Cette table n'est pas une donnee astronomique et ne decrit pas un calcul astrologique brut. Elle appartient a la couche semantique et redactionnelle du moteur : elle indique quelle tonalite ou quel effet de formulation utiliser quand une regle astrologique est detectee.

Le champ `name` sert de cle technique existante. Il est notamment reference par `astral_aspect_profiles.interpretive_valence`.

`name` est immuable une fois publie ou reference par une autre table. Le `label`, la `description` et `writing_guidance` peuvent evoluer, mais `name` ne doit pas etre renomme sans migration explicite des donnees dependantes.

### Champs principaux

| Champ | Role |
| --- | --- |
| `id` | Identifiant stable de l'effet interpretatif. |
| `name` | Cle technique stable, par exemple `supportive`, `dynamic_challenging` ou `amplifying`. |
| `label` | Libelle lisible de l'effet. |
| `description` | Definition semantique de l'effet interpretatif. |
| `interpretive_family` | Famille de l'effet : `tonal`, `intensity`, `adaptive`, `creative`, `symbolic`, `spiritual`. |
| `is_tonal_valence` | Indique si la valeur decrit une tonalite qualitative. |
| `is_intensity_modifier` | Indique si la valeur modifie l'intensite sans porter de jugement favorable/defavorable. |
| `writing_guidance` | Consigne de redaction pour le moteur d'interpretation. |
| `default_valence_id` | Reference vers `astral_default_valence.id` pour rattacher l'effet a une valence generale. |
| `sort_order` | Ordre d'affichage ou de lecture. |
| `is_active` | Indique si l'effet est utilisable par le moteur. |

### Contraintes de donnees

La table declare des contraintes `CHECK` materialisees dans PostgreSQL par le script d'import :

- `name` doit rester en `snake_case` ;
- `interpretive_family` doit appartenir a la liste controlee : `tonal`, `intensity`, `adaptive`, `creative`, `symbolic`, `spiritual` ;
- une ligne ne peut pas avoir simultanement `is_tonal_valence = true` et `is_intensity_modifier = true`.

La contrainte sur les deux booleens reste volontairement souple : elle interdit le double marquage tonal + intensite, mais elle n'impose pas qu'une future categorie contextuelle soit obligatoirement l'un ou l'autre.

### Modelisation retenue

La table garde le nom `astral_interpretive_valence`, mais elle ne doit pas etre lue comme une simple opposition positif/negatif.

Elle represente plutot un type d'effet interpretatif produit par une regle astrologique.

Deux familles doivent etre distinguees :

- les tonalites qualitatives, comme `supportive`, `harmonious`, `dynamic_challenging`, `polarizing`, `adjustment` ou `creative` ;
- les modificateurs d'intensite, comme `amplifying` et `obsessive_focus`.

`amplifying` n'est pas une valence favorable ou defavorable. Il indique que la configuration augmente la visibilite, la force ou l'impact d'un facteur. Le moteur doit donc le combiner avec une autre tonalite quand c'est possible.

Exemples :

1. Jupiter conjoint Venus peut etre interprete comme `amplifying` + `supportive` ou `harmonious`.
2. Mars conjoint Saturne peut etre interprete comme `amplifying` + `dynamic_challenging`.

### Utilisation prevue dans le code

Le moteur doit utiliser cette table pour guider la formulation, le scoring interpretatif et les regroupements de tonalite.

Pour une regle astrologique detectee, le runtime peut :

1. recuperer l'effet interpretatif via `name` ;
2. lire `interpretive_family` ;
3. verifier si l'effet est une tonalite ou un modificateur d'intensite ;
4. appliquer `writing_guidance` pour eviter des formulations trop simplistes ;
5. utiliser `default_valence_id` seulement comme rattachement general, pas comme interpretation finale.

### Tables consommatrices

Consommateur confirme :

- `astral_aspect_profiles.interpretive_valence` -> `astral_interpretive_valence.name`.

Cette relation est justifiee parce que les profils d'aspects ne decrivent pas seulement une geometrie angulaire brute. Ils portent aussi une orientation de lecture utile au moteur de redaction.

Consommateurs candidats dans le schema actuel si ces tables sont enrichies avec un champ structure :

- `astral_interpretation_signal_types`, pour qualifier la tonalite produite par un signal transmis au moteur de texte ;
- `interpretive_condition_signal_profiles`, pour qualifier la tonalite d'un signal issu d'un axe de condition planetaire ;
- `astral_interpretation_adapter_rules`, seulement si la regle porte elle-meme une orientation redactionnelle et pas uniquement un routage source -> signal.

Consommateurs candidats a plus long terme si ces tables sont creees :

- `astral_accidental_dignity_rules`, pour decrire l'effet interpretatif d'une condition de dignite accidentelle ;
- `astral_essential_dignity_rules`, pour decrire l'effet interpretatif d'une dignite essentielle ;
- `astral_advanced_condition_profiles`, pour rattacher un profil de condition avancee a une orientation redactionnelle ;
- `astral_interpretation_rules`, pour associer une regle de generation a une tonalite ou a un effet interpretatif ;
- `astral_theme_activation_rules`, pour definir la tonalite produite par l'activation d'un theme.

Ces liens ne doivent etre ajoutes que si la table porte une orientation interpretative explicite. Les champs narratifs libres, les notes, les resumes ou les descriptions ne doivent pas etre transformes automatiquement en FK vers `astral_interpretive_valence`.

Tables a ne pas connecter directement :

- les tables d'objets astrologiques bruts, comme les signes, planetes, maisons, points et systemes de maisons ;
- les tables de typologie technique, comme `astral_condition_operators`, `astral_heliacal_conditions` et `astral_horizon_positions` ;
- `astral_default_valence`, qui reste la couche de valence generale et ne remplace pas la couche redactionnelle d'`astral_interpretive_valence`.

Regle de modelisation : si une table ne possede qu'un seul champ `interpretive_valence`, ce champ doit normalement pointer vers une ligne ou `is_tonal_valence = true`. Les lignes ou `is_intensity_modifier = true`, comme `amplifying` ou `obsessive_focus`, doivent plutot etre portees par un champ dedie ou une table d'effets secondaires.

Etat actuel : `astral_aspect_profiles` utilise encore `amplifying` et `obsessive_focus` dans son champ unique `interpretive_valence`. Ce choix reste compatible avec les donnees existantes, mais une evolution plus propre serait de creer une table de liaison, par exemple `astral_aspect_profile_interpretive_effects`, avec un `role` permettant de distinguer `primary_valence`, `intensity_modifier` ou `secondary_effect`.

### Points de vigilance runtime

Ne pas traiter `amplifying` comme positif par defaut. Une configuration amplifiante peut amplifier un facteur favorable, difficile ou ambivalent.

Ne pas reduire `dynamic_challenging` a une lecture negative. Cette valeur designe une tension motrice qui peut pousser a l'action ou a la croissance.

Ne pas employer `symbolic_fated` avec une formulation fataliste. Cette valeur doit rester une indication de charge symbolique, de recurrence ou de signification particuliere.

Pour les valeurs ou `is_intensity_modifier = true`, le moteur devrait chercher une tonalite complementaire avant de produire une phrase finale.

## Domaine des objets astrologiques

### Refonte du socle

Le socle historique `astral_planets` a ete renomme `astral_chart_objects`.

Le nouveau nom est volontairement plus large : le moteur doit pouvoir manipuler des planetes, des luminaires et, a terme, des angles, points mathematiques, noeuds, lots ou autres objets calculables sans les qualifier artificiellement de planetes.

La separation retenue est :

| Couche | Tables principales | Role |
| --- | --- | --- |
| Identite et calcul | `astral_chart_objects`, `astral_chart_object_definitions` | Identite stable, mode de calcul et proprietes astrologiques de l'objet. |
| Doctrine | `astral_object_nature_types`, `astral_object_nature_assignments`, `astral_object_sign_dignities` | Classifications dependantes d'une tradition, d'un systeme ou d'une version de referentiel. |
| Runtime | `astral_object_motion_states` | Etats calcules au moment d'un theme ou d'une prediction. |
| Interpretation | `astral_object_interpretation_profiles`, `interpretive_condition_signal_profiles` | Contrats redactionnels et traduction des scores en signaux lisibles. |
| Produit et prediction | `prediction_object_category_weights`, `astral_prediction_daily_object_profiles` | Ponderation des objets pour la priorisation des contenus et predictions. |

### Renommages appliques

| Ancienne table | Nouvelle table |
| --- | --- |
| `astral_planets` | `astral_chart_objects` |
| `astral_planet_definitions` | `astral_chart_object_definitions` |
| `astral_planet_motion_states` | `astral_object_motion_states` |
| `astral_planet_sign_dignities` | `astral_object_sign_dignities` |
| `astral_planet_interpretation_profiles` | `astral_object_interpretation_profiles` |
| `astral_planet_condition_signal_profiles` | `interpretive_condition_signal_profiles` |
| `astral_planet_category_weights` | `prediction_object_category_weights` |
| `astral_prediction_daily_planet_profiles` | `astral_prediction_daily_object_profiles` |

Les anciennes colonnes `planet_id`, `astral_planet_id`, `source_planet_id`, `target_planet_id` et `relative_planet_id` sont remplacees par des variantes basees sur `chart_object_id`.

### astral_chart_objects

`astral_chart_objects` porte l'identite des objets consommables par un theme astral.

Le socle contient notamment `object_type_id`, `calculation_type_id`, `swe_id`, `is_physical_body`, `is_calculable`, `is_mobile`, `is_active` et `sort_order`.

`swe_id` est nullable pour permettre l'ajout futur d'objets calcules sans identifiant Swiss Ephemeris direct.

### astral_chart_object_definitions

`astral_chart_object_definitions` reste une table complementaire en relation 1-1 avec `astral_chart_objects`.

Elle porte les proprietes astrologiques comme `astrological_role_id`, `speed_rank`, `speed_class_id`, `typical_polarity_id`, `is_luminary`, `is_planet_symbolic` et `is_visible_to_naked_eye`.

Le champ `is_planet_symbolic` evite de confondre la classification astrologique et la classification astronomique.

### Natures et dignites

L'ancienne table `astral_planet_natures`, qui stockait des listes d'objets dans une colonne JSON, est remplacee par :

- `astral_object_nature_types`, catalogue des natures ;
- `astral_object_nature_assignments`, table de liaison versionnee et rattachee a un systeme astrologique.

Les natures `benefic`, `malefic`, `variable`, `luminary`, `neutral` et `transpersonal` sont separees par `nature_family` afin de ne pas melanger nature morale classique, role symbolique et famille astrologique moderne.

`astral_object_sign_dignities` remplace `astral_planet_sign_dignities`. La table est versionnee et impose une unicite sur :

- `reference_version_id` ;
- `astral_system_id` ;
- `astral_sign_id` ;
- `chart_object_id` ;
- `astral_dignity_type_id`.

### Signes et vitesses

`astral_sign_genders` est reliee a `astral_sign_profiles.sign_gender_id`. Elle porte la qualification traditionnelle `masculine` ou `feminine`.

Cette qualification reste distincte de `astral_polarities`, qui porte les codes `yang` et `yin`. Une relation explicite `astral_sign_genders.astral_polarity_id` permet de conserver les deux vocabulaires sans dupliquer leur logique d'alternance.

`astral_sign_keywords` devient la source unique des mots-cles et mots-cles d'ombre des signes. La table est reliee a `astral_signs.id` par `astral_sign_id`. Les colonnes JSON dupliquees ont ete retirees de `astral_sign_profiles`.

`astral_speed_classes` remplace l'ancienne table `astral_speed`. Elle porte les classes de vitesse typiques `fast`, `medium` et `slow`. `astral_chart_object_definitions.speed_class_id` reference maintenant `astral_speed_classes.id`.

`astral_speed_relations` reste distincte des classes de vitesse. Elle qualifie un resultat calcule par rapport a la vitesse moyenne de l'objet : `greater_than_mean` ou `less_than_mean`.

Les regles de dignite accidentelle exposent maintenant deux FK explicites en plus de leur `condition_json` :

- `astral_accidental_dignity_rules.sign_gender_id` -> `astral_sign_genders.id` pour les regles de hayz ;
- `astral_accidental_dignity_rules.speed_relation_id` -> `astral_speed_relations.id` pour les regles de vitesse relative.

Ces colonnes explicites permettent a PostgreSQL et aux outils comme DBeaver d'afficher les relations, tout en conservant le contrat JSON utilise par le moteur de regles.

### Regle d'architecture

Le calcul brut ne doit pas consommer directement les tables redactionnelles.

Le moteur d'interpretation ne doit pas recevoir une liste non filtree de signaux techniques. Une couche applicative intermediaire doit produire un contrat resume contenant les objets actifs, leur position, leur etat runtime, leurs dignites, les axes de condition pertinents et les consignes de redaction utiles.

Les profils d'interpretation actuellement dupliques entre versions sont conserves pour compatibilite. Une evolution ulterieure pourra eviter la duplication en utilisant un profil de base ou un fallback vers la version precedente quand le contenu n'a pas change.

## astral_object_motion_states

### Role de la table

`astral_object_motion_states` qualifie le mouvement apparent d'une planete ou d'un objet astrologique mobile au moment du calcul du theme.

Elle repond a une question precise : l'objet avance-t-il normalement dans le zodiaque, recule-t-il, ou est-il quasiment immobile autour d'un changement de direction ?

Cette table ne decrit pas une propriete statique de la planete. Une planete n'est pas retrograde par nature : elle l'est pour une date, une heure et un contexte de calcul donnes.

### Champs principaux

| Champ | Role |
| --- | --- |
| `id` | Identifiant stable de l'etat de mouvement. |
| `code` | Cle technique en `snake_case`, par exemple `direct`, `retrograde` ou `stationary`. |
| `label` | Libelle lisible. |
| `description` | Definition de l'etat de mouvement apparent. |
| `motion_family` | Famille technique : `forward`, `backward` ou `station`. |
| `requires_speed_threshold` | Indique si l'etat depend d'un seuil de vitesse proche de zero. |
| `runtime_usage` | Usage attendu dans le moteur. |
| `calculation_notes` | Notes de calcul pour determiner l'etat. |
| `sort_order` | Ordre d'affichage ou de lecture. |
| `is_active` | Indique si l'etat est utilisable par le moteur. |

### Modelisation retenue

La table conserve les trois valeurs existantes pour compatibilite avec les regles deja presentes :

- `direct` : la longitude zodiacale apparente augmente ;
- `retrograde` : la longitude zodiacale apparente diminue ;
- `stationary` : la vitesse apparente est proche de zero.

La valeur `stationary` reste volontairement generique. Pour un moteur plus expert, l'evolution propre serait de distinguer :

- `stationary_direct`, quand l'objet est stationnaire avant de reprendre un mouvement direct ;
- `stationary_retrograde`, quand l'objet est stationnaire avant d'entrer en retrogradation.

Cette evolution ne doit pas etre faite en supprimant brutalement `stationary`, car `astral_accidental_dignity_rules` reference deja `motion_state_id = 3`.

### Utilisation prevue dans le code

Cette table doit etre consommee par les positions calculees des planetes ou objets mobiles, par exemple une future table de type `calculated_planet_positions`, `chart_object_positions`, `natal_chart_planet_positions` ou `prediction_runtime_facts`.

Le champ attendu dans ces resultats serait :

- `motion_state_id` -> `astral_object_motion_states.id`.

Elle est deja pertinente pour :

- les operateurs de conditions, notamment `object_motion_state_is` ;
- les regles de dignite accidentelle fondees sur la retrogradation ou la station ;
- les regles d'interpretation et de prediction ;
- l'affichage expert, quand l'information apporte une vraie valeur.

Le runtime peut determiner l'etat en lisant la vitesse apparente fournie par l'ephemeride ou en comparant la longitude autour de l'instant calcule.

### Points de vigilance runtime

Ne pas stocker `motion_state_id` dans une table statique de planetes. Le mouvement apparent appartient aux donnees calculees.

Ne pas confondre l'etat de mouvement avec la vitesse. Une planete peut etre directe mais lente, retrograde mais rapide, ou stationnaire parce que sa vitesse est proche de zero. Les resultats calcules devraient donc separer :

- `motion_state_id` ;
- `apparent_speed_deg_per_day` ;
- eventuellement `speed_condition_id`.

Pour le Soleil et la Lune en theme geocentrique classique, l'etat attendu est normalement `direct`.

Pour les noeuds lunaires, le comportement depend du mode de calcul, notamment noeud moyen ou noeud vrai.

Pour les angles, cuspides, maisons, parts ou lots, `motion_state_id` doit rester nullable si un modele generique couvre tous les objets du theme.

## astral_prediction_calculation_profiles

### Role de la table

`astral_prediction_calculation_profiles` definit les profils de calcul utilises par le moteur de predictions astrologiques.

Cette table reprend le role de l'ancienne table SQLite `prediction_rulesets`, mais sous un nom plus explicite et prefixe `astral_`.

La table ne contient pas les resultats de prediction. Elle fige le contrat technique utilise pour les produire : type de zodiaque, mode de coordonnees, systeme de maisons et granularite temporelle.

Elle repond a trois besoins principaux :

- reproductibilite des predictions deja generees ;
- auditabilite et comparaison entre profils du moteur ;
- pilotage de la precision de calcul selon les offres produit ou les profils d'execution.

### Champs principaux

| Champ | Role |
| --- | --- |
| `id` | Identifiant stable du profil de calcul. |
| `zodiac_type` | Type de zodiaque utilise, par exemple `tropical` ou `sidereal`. |
| `coordinate_mode` | Mode de calcul des coordonnees, par exemple `geocentric`. |
| `house_system_id` | Reference vers `astral_house_systems.id`. |
| `time_step_minutes` | Granularite de balayage temporel du moteur. |
| `description` | Description humaine du profil et de son usage. |
| `is_locked` | Indique si le profil est verrouille et ne doit plus etre modifie. |
| `created_at` | Date de creation du profil. |

### Relations

La relation principale est :

- `astral_prediction_calculation_profiles.house_system_id` -> `astral_house_systems.id`.

Ce lien est important car les predictions peuvent dependre des maisons :

- transit d'une planete dans une maison ;
- activation d'un axe de maisons ;
- interpretation liee aux angles ;
- changement de maison selon le systeme de domification.

### Utilisation prevue dans le code

Le moteur doit selectionner un profil avant de calculer une prediction.

Flux attendu :

1. selectionner le profil de calcul ;
2. lire `zodiac_type`, `coordinate_mode`, `house_system_id` et `time_step_minutes` ;
3. calculer les positions, transits, entrees en signes, entrees en maisons et evenements selon ces parametres ;
4. stocker les resultats avec l'identifiant du profil utilise.

Exemple :

1. l'utilisateur demande une prediction premium ;
2. le moteur selectionne un profil avec un pas de calcul fin ;
3. le profil indique `tropical`, `geocentric`, `Placidus`, `30` minutes ;
4. les resultats sont stockes avec ce profil pour rester explicables plus tard.

### Points de vigilance runtime

Changer de profil de calcul peut changer les predictions, notamment quand le systeme de maisons change.

Un profil deja utilise en production ne doit pas etre modifie. Quand `is_locked = true`, le moteur ou l'administration doivent creer une nouvelle ligne au lieu d'editer la ligne existante.

Le champ `time_step_minutes` doit etre traite comme un compromis precision/cout :

- un pas large reduit le cout mais peut manquer des evenements fins ;
- un pas court augmente la precision mais demande plus de calcul ;
- les offres produit peuvent choisir des profils differents selon le niveau de service.

## Relations materialisees des dignites accidentelles

`astral_accidental_dignity_rules.condition_json` reste le payload polymorphe consomme par le moteur de regles. Les identifiants singuliers qu'il contient sont egalement exposes comme colonnes nullable avec FK PostgreSQL afin de rendre le schema navigable, verifiable et exploitable dans les outils comme DBeaver.

Relations materialisees :

- `house_modality_id` -> `astral_house_modalities.id` ;
- `motion_state_id` -> `astral_object_motion_states.id` ;
- `relative_chart_object_id` -> `astral_chart_objects.id` ;
- `heliacal_condition_id` -> `astral_heliacal_conditions.id` ;
- `horizon_position_id` -> `astral_horizon_positions.id` ;
- `chart_sect_id` -> `astral_sect.id` ;
- `house_id` -> `astral_houses.id` ;
- `condition_operator_id` -> `astral_condition_operators.id` ;
- `aspect_from_object_nature_type_id` -> `astral_object_nature_types.id` ;
- `bounding_object_nature_type_id` -> `astral_object_nature_types.id`.

Les listes `house_ids` et `aspect_ids` restent dans `condition_json` : une FK relationnelle classique ne peut pas garantir l'integrite des elements d'un tableau JSON. Si ces listes doivent devenir requetables ou administrables individuellement, creer des tables de liaison dediees.

## Normalisation des aspects

Les tables d'aspects doivent utiliser les identifiants relationnels et non dupliquer les codes texte des referentiels :

- `astral_aspect_definitions.aspect_id` -> `astral_aspects.id` ;
- `astral_aspect_definitions.astral_system_id` -> `astral_systems.id` ;
- `astral_aspect_profiles.aspect_id` -> `astral_aspects.id` ;
- `astral_aspect_orb_rules.aspect_id` -> `astral_aspects.id` ;
- `astral_aspect_orb_rules.astral_system_id` -> `astral_systems.id` ;
- `astral_aspect_orb_rules.source_astral_point_id` et `target_astral_point_id` -> `astral_points.id` ;
- `astral_aspect_orb_rules.source_angle_point_id` et `target_angle_point_id` -> `astral_angle_points.id`.

`astral_aspect_definitions` contient une ligne explicite par aspect et par systeme. Une definition desactivee reste donc visible avec `is_enabled = false`, au lieu d'etre cachee dans une liste de codes.

`astral_aspect_orb_rule_inheritance` conserve le fallback des regles d'orbe entre systemes. Le moteur doit chercher une regle locale avant de consulter `inherited_from_astral_system_id`.

Les anciennes colonnes polymorphes `source_point_code` et `target_point_code` ont ete retirees. Une FK de point renseignee cible un point exact. Une FK nulle avec `source_body_type` ou `target_body_type` egal a `point` ou `angle` reste une regle generique de categorie. Les contraintes PostgreSQL interdisent qu'une meme extremite melange un objet celeste, un point calcule et un angle.

## Catalogue structurel retire

`astral_structural_reference_catalog` a ete retire du schema PostgreSQL. Cette table isolee dupliquait sous forme de tableaux JSON les planetes, signes, classes de signes, dignites et maisons deja exposes par les tables normalisees. Elle ne doit pas devenir une seconde source de verite.

## Relations versionnees de la couche interpretative

Les codes de la couche interpretative sont stables dans une version de referentiel, mais peuvent exister dans plusieurs versions. Les relations utilisent donc une cle composite :

- `astral_interpretation_signal_types.(reference_version_id, theme_code)` -> `astral_interpretation_themes.(reference_version_id, code)` ;
- `astral_interpretation_adapter_rules.(reference_version_id, signal_code)` -> `astral_interpretation_signal_types.(reference_version_id, code)`.

Les tables versionnees concernees imposent une unicite sur `(reference_version_id, code)` plutot qu'une unicite globale sur `code`.

## Durcissements issus de la review

Les contraintes d'unicite generees par l'importeur utilisent `UNIQUE NULLS NOT DISTINCT`. Une cle composite contenant une valeur nullable reste donc reellement unique, notamment pour les regles d'orbe generiques et les profils d'interpretation de points sans variante explicite.

Les champs redondants `reference_version_code` ont ete retires de `astral_advanced_condition_score_profiles`, `astral_dominance_score_profiles` et `astral_interpretation_adapter_rules`. La source de verite est `reference_version_id` -> `astral_reference_versions.id`.

L'importeur refuse maintenant les noms de fichiers differents du nom de table, les noms de tables dupliques et tout import qui ignorerait une FK declaree.

Les marqueurs declaratifs `unique`, `snake_case` et `enum:` presents dans les descriptions de colonnes sont egalement materialises en contraintes PostgreSQL.

## Referentiels sans version validee

`astral_reference_versions` est conservee comme table de rattachement, mais elle reste vide tant qu'aucun referentiel n'a ete valide. Les colonnes `reference_version_id` sont donc conservees et nullable. Elles devront pointer vers une ligne de `astral_reference_versions` uniquement lors de la publication explicite d'une version de referentiel.

Les copies strictement identiques provenant des anciennes versions `1.0.0` et `2.0.0` ont ete fusionnees. Les FK internes ont ete remappees vers les lignes conservees avant la suppression de ces deux versions.
