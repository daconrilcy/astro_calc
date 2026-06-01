# Tables manquantes ajoutées

Ces fichiers JSON ont été générés depuis `horoscope.db` pour compléter les références absentes dans `json_db`.

| Table JSON ajoutée | Source dans horoscope.db | Lignes | Remarque |
|---|---:|---:|---|
| `astral_polarities` | `astral_polarities` | 2 | Nom identique dans SQLite. |
| `astral_reference_versions` | `astral_reference_versions` | 2 | Nom identique dans SQLite. |
| `chart_results` | `chart_results` | 2 | Nom identique dans SQLite. |
| `astral_speed_classes` | `astral_speed` | 3 | Table de compatibilité générée depuis `astral_speed` car `astral_speed_classes` n'existe pas comme table SQLite exacte. |
| `reference_versions` | `astral_reference_versions` | 2 | Table de compatibilité générée depuis `astral_reference_versions` car `reference_versions` n'existe pas comme table SQLite exacte. |

Références qui ont motivé ces ajouts :

- `astral_planet_definitions.speed_class_id` -> `astral_speed_classes.id`
- `astral_planet_definitions.typical_polarity_id` -> `astral_polarities.id`
- `astral_prediction_daily_planet_profiles.reference_version_id` -> `reference_versions.id`
- `astral_planet_interpretation_profiles.reference_version_id` -> `reference_versions.id`
- Plusieurs tables de règles/profils -> `astral_reference_versions.id`
- `astral_chart_planet_dignity_results.chart_result_id` -> `chart_results.id`

Note : `astral_speed_classes` et `reference_versions` ne sont pas présentes sous ces noms exacts dans `horoscope.db`. Elles ont été générées depuis les tables équivalentes les plus directes, respectivement `astral_speed` et `astral_reference_versions`.
