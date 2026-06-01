# Tables manquantes ajoutées

Ces fichiers JSON ont été générés depuis `horoscope.db` pour compléter les références absentes dans `json_db`.

| Table JSON ajoutée | Source dans horoscope.db | Lignes | Remarque |
|---|---:|---:|---|
| `astral_polarities` | `astral_polarities` | 2 | Nom identique dans SQLite. |
| `astral_reference_versions` | `astral_reference_versions` | 0 | Table conservée, mais aucune version de référentiel n'est validée pour l'instant. |
| `astral_speed_classes` | `astral_speed` | 3 | Table de compatibilité générée depuis `astral_speed` car `astral_speed_classes` n'existe pas comme table SQLite exacte. |

Références qui ont motivé ces ajouts :

- `astral_chart_object_definitions.speed_class_id` -> `astral_speed_classes.id`
- `astral_chart_object_definitions.typical_polarity_id` avait d'abord été repéré avec une référence vers `astral_polarities.id`; la contrainte PostgreSQL utilise finalement `astral_typical_polarities.id`, qui contient bien les valeurs `positive`, `negative` et `neutral`.
- `astral_prediction_daily_object_profiles.reference_version_id` -> `astral_reference_versions.id`
- `astral_object_interpretation_profiles.reference_version_id` -> `astral_reference_versions.id`
- Plusieurs tables de règles/profils -> `astral_reference_versions.id`

Note : `astral_speed_classes` n'est pas présente sous ce nom exact dans `horoscope.db`. Elle a été générée depuis la table équivalente la plus directe, `astral_speed`.

## Complément depuis horoscope.db

Ajouts générés après comparaison de toutes les tables `astral_%` de `horoscope.db` avec `json_db`.

| Table JSON ajoutée | Source dans horoscope.db | Lignes |
|---|---:|---:|
| `astral_default_valence` | `astral_default_valence` | 4 |
| `astral_dignity_type` | `astral_dignity_type` | 4 |
| `astral_house_category_weights` | `astral_house_category_weights` | 24 |
| `astral_houses` | `astral_houses` | 12 |
| `astral_interpretive_valence` | `astral_interpretive_valence` | 5 |
| `astral_modalities` | `astral_modalities` | 3 |
| `prediction_object_category_weights` | `astral_planet_category_weights` | 85 |
| `astral_prediction_calculation_profiles` | `prediction_rulesets` | 2 |
| `astral_prediction_daily_house_profiles` | `astral_prediction_daily_house_profiles` | 12 |
| `astral_sign_fertility_classes` | `astral_sign_fertility_classes` | 3 |
| `astral_sign_form_classes` | `astral_sign_form_classes` | 4 |
| `astral_sign_profiles` | `astral_sign_profiles` | 12 |
| `astral_sign_seasonal_quadrants` | `astral_sign_seasonal_quadrants` | 4 |
| `astral_sign_voice_classes` | `astral_sign_voice_classes` | 3 |
| `languages` | `languages` | 5 |
| `prediction_categories` | `prediction_categories` | 12 |

Liaisons explicites ajoutées ou corrigées :

- `astral_house_axis_members.house_id` -> `astral_houses.id`
- `astral_house_axis_members.opposite_house_id` -> `astral_houses.id`
- `astral_house_axis_members.axis_id` -> `astral_house_axis_definitions.id`
- `astral_house_axis_definitions.astral_system_id` -> `astral_systems.id`
- Le doublon `astral_house` a ete supprime : `astral_houses` est la table canonique.
- `prediction_rulesets` a ete reprise sous le nom canonique `astral_prediction_calculation_profiles`.
- Les foreign keys déclarées dans `horoscope.db` ont été reprises dans les structures JSON quand les colonnes existent dans le JSON source.
- Les tables contenant `translation` ou `translations` ont ensuite été retirées du catalogue JSON et de PostgreSQL à la demande.
- `reference_versions` a ensuite été retirée : les tables utilisent `astral_reference_versions`.
- `chart_results` et la table runtime dépendante `astral_chart_planet_dignity_results` ont ensuite été retirées du catalogue JSON et de PostgreSQL pour l'instant.
- `astral_structural_reference_catalog` a ensuite été retirée : cette table isolée dupliquait les référentiels normalisés sous forme de tableaux JSON.
- `astral_aspect_orb_rule_inheritance` a été ajoutée pour matérialiser les 2 héritages de règles d'orbe auparavant stockés dans des blocs JSON.

Fichiers existants dont les métadonnées de liaison ont été mises à jour :

- `astral_aspect_definitions.json`
- `astral_aspect_interpretation_profiles.json`
- `astral_aspect_orb_rules.json`
- `astral_aspect_profiles.json`
- `astral_aspects.json`
- `astral_constellations.json`
- `astral_fixed_star_definitions.json`
- `astral_house_axis_definitions.json`
- `astral_house_axis_members.json`
- `astral_house_interpretation_profiles.json`
- `astral_chart_object_definitions.json`
- `astral_object_interpretation_profiles.json`
- `astral_object_sign_dignities.json`
- `astral_point_aliases.json`
- `astral_point_interpretation_profiles.json`
- `astral_points.json`
- `astral_prediction_daily_object_profiles.json`
- `astral_zodiacal_reference_systems.json`
