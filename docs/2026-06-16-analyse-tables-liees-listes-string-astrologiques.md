# Analyse approfondie des tables astrologiques liees a des listes de chaines

## Objet

Ce rapport repond au besoin suivant :

- identifier les tables astrologiques qui **contiennent des listes de chaines**
- lister les **tables liees** a ces tables
- qualifier la nature de ces liens : cle etrangere directe, table de traduction, table de definition, ou liaison faible par code

Analyse realisee sur le snapshot local SQLite [`horoscope.db`](/C:/dev/astral_calculation/horoscope.db), en lecture du schema et des donnees effectivement presentes.

## Methode

Le perimetre retenu couvre les tables astrologiques du snapshot local :

- prefixes `astral_`, `astro_`, `prediction_`, `ruleset_`

Une table a ete retenue comme "table contenant une liste de chaines" si au moins une colonne :

- est typée `JSON` ou nommee `*_json`
- contient reellement un tableau JSON
- et que ce tableau contient des valeurs textuelles

Important :

- `astral_chart_planet_dignity_results` contient bien des tableaux JSON, mais il s'agit surtout de **tableaux d'objets calcules**, pas d'un referentiel de mots-clefs. Cette table est donc signalee a part, mais exclue du coeur du rapport.

## Conclusion executive

Les principales tables astrologiques porteuses de listes de chaines sont :

1. `astral_aspect_interpretation_profiles`
2. `astral_house_interpretation_profiles`
3. `astral_planet_interpretation_profiles`
4. `astral_sign_profiles`
5. `astral_fixed_star_keywords`
6. `astral_fixed_star_keyword_translations`
7. `astral_point_interpretation_keywords`
8. `astral_point_interpretation_keyword_translations`
9. `astral_planet_natures`

Ces tables se repartissent en 4 familles de modelisation :

- **profils d'interpretation** : aspects, maisons, planetes
- **profils de qualification symbolique** : signes, etoiles fixes, points astrologiques
- **traductions** : mots-clefs derives de tables sources
- **liaison faible par codes JSON** : natures planetaires

## Inventaire detaille

### 1. `astral_aspect_interpretation_profiles`

Role :

- referentiel d'interpretation des aspects astrologiques

Volume :

- 40 lignes
- couverture observee : `20 aspects x 1 langue source x 1 systeme astrologique x 2 versions de reference`

Colonnes contenant des listes de chaines :

- `core_keywords_json`
- `shadow_keywords_json`
- `psychological_keywords_json`
- `relationship_keywords_json`
- `career_keywords_json`
- `spiritual_keywords_json`
- `energetic_dynamics_json`
- `growth_patterns_json`
- `conflict_patterns_json`
- `archetypes_json`
- `dos_json`
- `donts_json`
- `prompt_hints_json`

Exemples observes :

- `fusion`, `intensification`, `focus`
- `stress`, `blockage`, `reactivity`
- `the obstacle`, `the forge`, `the challenger`

Tables liees sortantes :

- `astral_aspects` via `aspect_id`
- `astral_systems` via `astral_system_id`
- `astral_reference_versions` via `reference_version_id`
- `languages` via `language_id`

Tables liees entrantes :

- `astral_aspect_interpretation_profile_translations` via `source_profile_id`

Lecture metier :

- c'est une table pivot forte entre un **aspect**, un **systeme astrologique**, une **version de referentiel** et une **langue source**
- les listes de chaines servent de base semantique pour l'interpretation LLM et editoriale

### 2. `astral_house_interpretation_profiles`

Role :

- referentiel d'interpretation des 12 maisons astrologiques

Volume :

- 24 lignes
- couverture observee : `12 maisons x 1 langue source x 1 systeme astrologique x 2 versions de reference`

Colonnes contenant des listes de chaines :

- `core_keywords_json`
- `shadow_keywords_json`
- `psychological_keywords_json`
- `material_keywords_json`
- `relationship_keywords_json`
- `career_keywords_json`
- `health_keywords_json`
- `spiritual_keywords_json`
- `body_parts_json`
- `archetypes_json`
- `dos_json`
- `donts_json`
- `prompt_hints_json`

Exemples observes :

- maison 1 : `self`, `identity`, `appearance`
- maison 2 : `money`, `income`, `resources`
- maison 3 : `communication`, `learning`, `speech`

Tables liees sortantes :

- `astral_houses` via `house_id`
- `astral_systems` via `astral_system_id`
- `astral_reference_versions` via `reference_version_id`
- `languages` via `language_id`

Tables liees entrantes :

- `astral_house_interpretation_profile_translations` via `source_profile_id`

Lecture metier :

- meme modelisation que pour les aspects
- les listes de chaines couvrent plusieurs domaines d'interpretation : psychologique, materiel, relationnel, sante, spirituel

### 3. `astral_planet_interpretation_profiles`

Role :

- referentiel d'interpretation des planetes

Volume :

- 20 lignes
- couverture observee : `10 planetes x 1 langue source x 1 systeme astrologique x 2 versions de reference`

Colonnes contenant des listes de chaines :

- `core_keywords_json`
- `shadow_keywords_json`
- `psychological_expression_json`
- `relational_expression_json`
- `vocational_expression_json`
- `spiritual_expression_json`
- `energetic_dynamics_json`
- `growth_patterns_json`
- `conflict_patterns_json`
- `archetypes_json`
- `dos_json`
- `donts_json`
- `prompt_hints_json`

Exemples observes :

- Soleil : `identity`, `vitality`, `will`
- Lune : `emotion`, `instinct`, `needs`
- Mercure : `thought`, `communication`, `analysis`

Tables liees sortantes :

- `astral_planets` via `planet_id`
- `astral_systems` via `astral_system_id`
- `astral_reference_versions` via `reference_version_id`
- `languages` via `language_id`

Tables liees entrantes :

- `astral_planet_interpretation_profile_translations` via `source_profile_id`

Lecture metier :

- table structurante pour la semantique planetaire
- meme logique de versioning et de traduction que les aspects et les maisons

### 4. `astral_sign_profiles`

Role :

- profil symbolique des signes zodiacaux

Volume :

- 12 lignes

Colonnes contenant des listes de chaines :

- `keywords_json`
- `shadow_keywords_json`

Exemples observes :

- `aries` : `initiative`, `action`, `courage`, `drive`
- `taurus` : `stability`, `security`, `patience`
- `gemini` : `communication`, `curiosity`, `adaptability`

Tables liees sortantes :

- `astral_signs` via `astral_sign_id`
- `astral_elements` via `astral_element_id`
- `astral_modalities` via `astral_modality_id`
- `astral_polarities` via `astral_polarity_id`
- `astral_sign_seasonal_quadrants` via `seasonal_quadrant_id`
- `astral_sign_fertility_classes` via `fertility_class_id`
- `astral_sign_voice_classes` via `voice_class_id`
- `astral_sign_form_classes` via `form_class_id`

Tables liees entrantes :

- aucune cle etrangere entrante detectee vers cette table dans le snapshot

Lecture metier :

- table de synthese riche
- elle relie le signe a plusieurs taxonomies astrologiques : element, modalite, polarite, saison, fertilite, voix, forme

### 5. `astral_fixed_star_keywords`

Role :

- referentiel central de mots-clefs pour les etoiles fixes

Volume :

- 10 lignes

Colonnes contenant des listes de chaines :

- `keywords_json`

Exemples observes :

- `royalty`, `leadership`, `honor`, `success`, `power`
- `intensity`, `crisis`, `obsession`, `extremes`, `danger`

Tables liees entrantes :

- `astral_fixed_star_keyword_translations` via `astral_fixed_star_keywords_id`
- `astral_fixed_star_definitions` via `astral_fixed_star_keywords_id`

Tables liees sortantes :

- aucune cle etrangere directe dans cette table

Tables astrologiques reliees indirectement :

- `astral_fixed_stars` via `astral_fixed_star_definitions.fixed_star_id`
- `astral_constellations` via `astral_fixed_star_definitions.constellation_id`
- `astral_signs` via `astral_fixed_star_definitions.zodiac_sign_id`
- `astral_reference_epochs`
- `astral_zodiacal_reference_systems`

Etoiles fixes observees :

- `regulus`
- `algol`
- `spica`
- `antares`
- `aldebaran`
- `sirius`
- `fomalhaut`
- `betelgeuse`
- `achernar`
- `vega`

Lecture metier :

- les mots-clefs ne pointent pas eux-memes vers l'etoile fixe
- la liaison passe par `astral_fixed_star_definitions`, qui joue le role de **pont de contextualisation astronomique et zodiacale**

### 6. `astral_fixed_star_keyword_translations`

Role :

- traductions des mots-clefs des etoiles fixes

Volume :

- 40 lignes
- couverture observee : `10 jeux de mots-clefs x 4 langues`

Colonnes contenant des listes de chaines :

- `keywords_json`

Tables liees sortantes :

- `astral_fixed_star_keywords` via `astral_fixed_star_keywords_id`
- `languages` via `language_id`

Lecture metier :

- table strictement dependante du referentiel source `astral_fixed_star_keywords`
- sert a localiser les listes de mots-clefs sans dupliquer les definitions astronomiques

### 7. `astral_point_interpretation_keywords`

Role :

- referentiel de mots-clefs pour certains points astrologiques calcules

Volume :

- 5 lignes

Colonnes contenant des listes de chaines :

- `core_keywords_json`
- `shadow_keywords_json`
- `psychological_keywords_json`
- `spiritual_keywords_json`
- `relationship_keywords_json`
- `career_keywords_json`

Exemples observes :

- `north_node` : `evolution`, `growth`, `direction`
- `south_node` : `heritage`, `habit`, `memory`
- `lunar_apogee` : `distance`, `remoteness`, `abstraction`

Tables liees entrantes :

- `astral_point_interpretation_profiles` via `keyword_set_id`
- `astral_point_interpretation_keyword_translations` via `keyword_set_id`

Tables liees sortantes :

- aucune cle etrangere directe dans cette table

Lecture metier :

- comme pour les etoiles fixes, la table de mots-clefs est separee de la table qui porte le contexte editorial complet

### 8. `astral_point_interpretation_keyword_translations`

Role :

- traductions des mots-clefs des points astrologiques

Volume :

- 5 lignes
- couverture observee : `5 jeux de mots-clefs x 1 langue de traduction`

Colonnes contenant des listes de chaines :

- `core_keywords_json`
- `shadow_keywords_json`
- `psychological_keywords_json`
- `spiritual_keywords_json`
- `relationship_keywords_json`
- `career_keywords_json`

Tables liees sortantes :

- `astral_point_interpretation_keywords` via `keyword_set_id`
- `languages` via `language_id`

Points astrologiques relies indirectement :

- `north_node`
- `south_node`
- `lunar_apogee`
- `lunar_perigee`
- `black_moon_lilith`

Lecture metier :

- la liaison au point astrologique n'est pas dans cette table
- elle passe par `astral_point_interpretation_profiles`, qui relie `keyword_set_id` a `astral_point_code`

### 9. `astral_planet_natures`

Role :

- classification des natures planetaires

Volume :

- 2 lignes

Colonnes contenant des listes de chaines :

- `planet_codes_json`

Valeurs observees :

- `benefic` -> `["venus", "jupiter"]`
- `malefic` -> `["mars", "saturn"]`

Tables liees :

- aucune cle etrangere detectee

Observation importante :

- la liaison vers `astral_planets` est **faible** et repose sur des codes textes inclus dans le JSON
- il n'existe pas ici de table de jointure normalisee du type `astral_planet_nature_members`

Lecture metier :

- cette table est semantiquement reliee aux planetes
- techniquement, ce lien n'est pas garanti par le schema relationnel

## Tables liees complementaires importantes

Certaines tables ne contiennent pas elles-memes les listes de chaines, mais sont essentielles pour relier ces donnees astrologiques au reste du modele :

### Tables de traduction de profils

- `astral_aspect_interpretation_profile_translations`
- `astral_house_interpretation_profile_translations`
- `astral_planet_interpretation_profile_translations`

Role :

- traduction des titres, resumes et micro-notes
- elles pointent toutes vers la table source de profils via `source_profile_id`

### Table de profil des points astrologiques

- `astral_point_interpretation_profiles`

Role :

- relie `keyword_set_id` a un `astral_point_code`
- lie aussi la langue et, le cas echeant, la variante de calcul du point

Liens sortants :

- `astral_point_interpretation_keywords`
- `astral_points`
- `astral_point_calculation_variants`
- `languages`

### Table de definition des etoiles fixes

- `astral_fixed_star_definitions`

Role :

- relie un jeu de mots-clefs a une etoile fixe, une constellation, un signe zodiacal et un systeme de reference

Liens sortants :

- `astral_fixed_star_keywords`
- `astral_fixed_stars`
- `astral_constellations`
- `astral_signs`
- `astral_reference_epochs`
- `astral_zodiacal_reference_systems`
- `astral_reference_sources`

## Anomalies et points d'attention de modelisation

### 1. Tables a listes de chaines bien normalisees

Les familles suivantes sont bien reliees par cles etrangeres :

- aspects
- maisons
- planetes
- etoiles fixes via table pont
- points astrologiques via table pont

### 2. Liaison faible dans `astral_planet_natures`

`astral_planet_natures.planet_codes_json` stocke des codes de planetes dans un tableau JSON.

Impact :

- aucune integrite referentielle native
- risque de divergence entre les codes JSON et `astral_planets.code`
- requetes plus fragiles qu'avec une vraie table de jointure

### 3. Faux positif technique exclu du coeur du rapport

`astral_chart_planet_dignity_results` contient :

- `essential_breakdown_json`
- `accidental_breakdown_json`

Mais ces colonnes stockent surtout :

- des tableaux d'objets de calcul
- parfois des tableaux vides

Ce n'est donc pas un referentiel de listes de mots ou de libelles astrologiques.

## Synthese finale

Le schema astrologique du snapshot local contient un noyau coherent de tables portant des listes de chaines, principalement pour :

- l'interpretation des aspects
- l'interpretation des maisons
- l'interpretation des planetes
- les mots-clefs des signes
- les mots-clefs des etoiles fixes
- les mots-clefs des points astrologiques
- la categorisation benefique/malefique des planetes

Le schema distingue correctement :

- les **donnees semantiques source**
- les **traductions**
- les **tables ponts de contextualisation astrologique**

Le point le moins robuste du modele est `astral_planet_natures`, car la relation aux planetes est embarquee dans un JSON de codes plutot que dans une relation normalisee.

## Liste compacte des tables astrologiques liees a des listes de chaines

| Table | Type de contenu | Tables liees principales |
|---|---|---|
| `astral_aspect_interpretation_profiles` | profils d'interpretation | `astral_aspects`, `astral_systems`, `astral_reference_versions`, `languages`, translations |
| `astral_house_interpretation_profiles` | profils d'interpretation | `astral_houses`, `astral_systems`, `astral_reference_versions`, `languages`, translations |
| `astral_planet_interpretation_profiles` | profils d'interpretation | `astral_planets`, `astral_systems`, `astral_reference_versions`, `languages`, translations |
| `astral_sign_profiles` | mots-clefs de signes | `astral_signs`, `astral_elements`, `astral_modalities`, `astral_polarities`, classes annexes |
| `astral_fixed_star_keywords` | mots-clefs d'etoiles fixes | `astral_fixed_star_definitions`, translations |
| `astral_fixed_star_keyword_translations` | traductions de mots-clefs | `astral_fixed_star_keywords`, `languages` |
| `astral_point_interpretation_keywords` | mots-clefs de points astrologiques | `astral_point_interpretation_profiles`, translations |
| `astral_point_interpretation_keyword_translations` | traductions de mots-clefs | `astral_point_interpretation_keywords`, `languages` |
| `astral_planet_natures` | groupes de planetes par nature | liaison faible vers `astral_planets` par codes JSON |

