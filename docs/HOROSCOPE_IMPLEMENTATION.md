# Cadrage implementation horoscope

Ce document fixe le cadre de developpement du module horoscope. Il doit etre lu
avant toute implementation liee aux services horoscope.

Convention de vocabulaire : dans le texte, utiliser "preuve astrologique" autant
que possible. Dans les contrats JSON, conserver le terme technique `evidence`.

## Decision V1

Services V1 retenus :

```text
horoscope_basic_daily_natal_3_slots
horoscope_free_daily
```

`horoscope_free_daily` est une projection Free synthetique du meme socle
horoscope quotidien natal. Malgre son nom court, il requiert un theme natal via
`chart_calculation_id`. Une future version non natale devra utiliser un
`service_code` distinct, par exemple `horoscope_free_daily_general`.

La V1 Basic est un horoscope quotidien personnalise sur theme natal, decoupe en
3 moments de journee : matin, apres-midi, soir.

Matrice produit :

| Service | Niveau | Natal requis | Slots internes | Slots publics | Sortie |
|---|---|---:|---:|---:|---|
| `horoscope_free_daily` | Free | Oui | 1 `day` | Non | `summary` + `advice` + `watch_point` |
| `horoscope_basic_daily_natal_3_slots` | Basic | Oui | 3 | Oui | `morning` / `afternoon` / `evening` |

Pour `horoscope_free_daily`, `day` est uniquement un slot technique de
projection. Il ne constitue pas une section publique et ne doit jamais apparaitre
dans la reponse utilisateur.

Status `horoscope_free_daily` V1 :

- Validation completed.
- Public response has no `slots`: PASS.
- No public leakage of `day` / `slot:day`: PASS.
- `advice` present: PASS.
- `evidence_keys` present and non-empty: PASS.
- `quality` present: PASS.
- Basic horoscope non-regression: PASS.
- Horoscope tests: PASS -- 45/45.
- French typography: PASS under current rule set.

Note : la normalisation de l'apostrophe typographique (`'` -> `’`) ne fait pas
partie des regles bloquantes de typographie francaise en V1. Les controles
actuels ciblent les elisions cassees comme `l impression` et les ponctuations
invalides comme `Conseil:`. Ce point reste une amelioration editoriale future,
pas un bloqueur V1.

Roadmap produit indicative :

- Free : daily natal synthetique, sans slots publics.
- Basic : daily natal 3 slots, V1 retenue.
- Premium : `horoscope_premium_daily_local_2h_slots`, horoscope quotidien
  local en 12 creneaux publics de 2 heures.

## Decision Premium V1

Service Premium V1 retenu :

```text
horoscope_premium_daily_local_2h_slots
```

Premium V1 reste une extension du workflow horoscope existant :

- pas de nouvel endpoint ;
- pas de nouveau worker ;
- pas de nouvelle table de jobs ;
- `POST /v1/jobs` et l'idempotence existante restent le chemin public ;
- `chart_calculation_id`, `timezone`, `location.latitude` et
  `location.longitude` sont obligatoires ;
- `birth_data` inline reste refuse.

Matrice produit mise a jour :

| Service | Niveau | Natal requis | Location requise | Slots publics | Sortie |
|---|---|---:|---:|---:|---|
| `horoscope_free_daily` | Free | Oui | Non | Non | `summary` + `advice` + `watch_point` |
| `horoscope_basic_daily_natal_3_slots` | Basic | Oui | Non | 3 | `morning` / `afternoon` / `evening` |
| `horoscope_premium_daily_local_2h_slots` | Premium | Oui | Oui | 12 | `best_slots` + `watch_slots` + `timeline[12]` + domaines |

Premium V1 utilise `detail_level = premium_rich` : maximum de detail utile,
pas payload illimite. Le profil porte `max_words_target = 2500` et
`max_words_hard_limit = 3200`.

## Decision Period V1

Service Period V1 retenu :

```text
horoscope_basic_next_7_days_natal
```

Ce service est un horoscope de periode, pas une concatenation de sept
horoscopes quotidiens. Le terme contractuel est `period`.

Configuration service :

```text
period_profile_code = next_7_days
detail_profile_code = basic_standard
scan_profile_code = daily_noon_7_days
```

`astral_time_window` est la source canonique de resolution temporelle. Le module
horoscope ne recode pas `next_7_days`, `end_exclusive`, la timezone, l'anchor
policy ou les jours inclus. L'orchestrateur LLM consomme une fenetre resolue,
construit un `scan_plan`, appelle le calculateur, puis construit les
`period_events`, le scoring, la projection et la reponse publique.

Payload public V1 :

- `anchor_date` obligatoire, interpretee comme date civile locale dans
  `timezone` ;
- `timezone`, `target_language`, `chart_calculation_id` obligatoires ;
- pas de `birth_data` inline ;
- le payload public ne peut pas surcharger `period_profile_code`,
  `detail_profile_code` ou `scan_profile_code`.

Contrats dedies :

- `horoscope_period_natal_request_v1` ;
- `horoscope_period_calculation_request_v1` ;
- `horoscope_period_calculation_response_v1` ;
- `horoscope_period_interpretation_request_v1` ;
- `horoscope_period_response_v1`.

La reponse publique contient `week_overview`, `key_days`, `best_days`,
`watch_days`, `watch_summary`, `daily_timeline[7]`, `domain_sections`, `advice`,
`evidence_summary` et `quality`. Un meme jour peut etre `key_day` et aussi
`watch_day`, mais `best_days` est construit hors `watch_days` et hors
`key_days`.

Durcissement real E2E period :

- les champs `*_utc` period sont normalises en UTC reel (`+00:00` ou `Z`) ;
- l'endpoint calculateur period recupere l'input natal complet et les positions
  natales persistees du `chart_calculation_id`, puis recalcule les positions de
  transit de chaque snapshot via l'`EphemerisEngine` existant ;
- le writer period passe par le provider LLM configure lorsque le provider par
  defaut n'est pas `fake`; le fake writer reste reserve aux smokes rapides ;
- les textes publics utilisent des libelles francais (`theme_label`) et ne
  doivent pas exposer `theme_code`, `evidence_key`, `period:`, `natal_`,
  `transit_exact`, `transit_active`, `moon_house_by_day` ou les codes tone
  internes (`focused`, `supportive`, `careful`, etc.) ;
- `daily_timeline[].tone` reste present dans le contrat public, mais porte
  exclusivement un libelle utilisateur francais actif depuis
  `horoscope_tone_labels`; les tons generes par le provider sont remplaces par
  le label attendu du `daily_plan` correspondant ;
- le service period Basic reel respecte les bornes du profil
  `basic_standard` dans `horoscope_detail_profiles` : cible 800-1200 mots
  publics, avec post-traitement deterministe pour completer ou condenser la
  reponse avant rejet sous `target_words_min` ou au-dessus de
  `hard_limit_words` ;
- les aspects period nommes sont limites a la bande maximale du referentiel
  `horoscope_orb_weight_bands` ; au-dela, le calculateur produit un fait de
  contexte non aspecte (`transit_context`) plutot qu'un aspect large ;
- les `period_events` portent un score deterministe discriminant, trie par score
  decroissant puis date croissante, avec bonus limite de repetition de theme ;
- `key_days` est limite a deux entrees maximum et ne remonte que les pics nets
  (`score >= 0.60` et proche du meilleur score), avec rarete de theme comme
  departage ;
- `best_days` est limite a deux entrees, hors `key_days`/`watch_days`, avec des
  themes distincts et des titres qualitatifs (`Jour de clarte`, `Jour le plus
  structurant`, etc.) ;
- `watch_days` est construit uniquement depuis les evenements de vigilance
  credibles (`careful`, `square`, `opposition`) et peut etre vide ; dans ce cas,
  `watch_summary.status = "none"` annonce qu'aucun point de vigilance
  particulier ne ressort ;
- les marqueurs `key_days`, `best_days` et `watch_days` utilisent
  `fallback_reason: null` hors fallback explicite, jamais une chaine vide ;
- les faits de contexte (`transit_context`, `moon_house_by_day`, aspect
  `context`) n'exposent pas d'`orb_deg`; seuls les aspects nommes valides
  conservent une orbe, bornee a 6 degres ;
- les preuves period portent des hints de personnalisation natale
  (`natal_focus_label`, `natal_focus_hint`, `personalization_hint`) issus des
  referentiels `horoscope_natal_focus_labels`; la lecture publique doit utiliser
  une nuance natale dans la vue d'ensemble, chaque domaine et au moins quatre
  jours ;
- ces hints restent strictement internes : le texte public ne doit jamais
  recopier des consignes comme `Personnaliser ce signal`, `Relier ce signal`,
  `plutot que rester sur un conseil generique`, `donne le relief principal`,
  `summary_hint`, `advice_hint`, `personalization_hint` ou
  `natal_focus_hint` ; les guards dedies
  `HOROSCOPE_PERIOD_INTERNAL_GUIDANCE_LEAK` et
  `HOROSCOPE_PERIOD_BROKEN_SENTENCE` bloquent aussi les phrases tronquees par
  post-traitement ; le prompt reel doit parler d'indications internes de
  personnalisation sans demander au provider de recopier les noms de champs ;
- la prose publique ne doit pas decrire le processus de personnalisation :
  formulations comme `plus personnel que generique`, `conseil generique`,
  `cette nuance reste liee`, `avec un echo personnel autour de`,
  `secteur personnel active`, `la lecture relie`, `zones personnelles`,
  `zones natales activees`, `theme natal comme fil directeur` ou
  `le point d'appui concerne` sont bloquees par
  `HOROSCOPE_PERIOD_META_PERSONALIZATION_LEAK` ; les deux-points
  publics doivent respecter l'espacement francais via
  `HOROSCOPE_PERIOD_FRENCH_TYPOGRAPHY_FAILED` ;
- `domain_sections` contient 2 a 4 domaines distincts selectionnes par score de
  theme, et les `daily_plans` portent un `style_variant_code` avec termes a
  eviter depuis `horoscope_period_style_variants` ;
- `scripts/test_horoscope_basic_next_7_days_real_e2e.ps1` echoue si le
  calculateur ou le writer reste fake, si la timeline est repetitive ou si les
  sections de domaine reutilisent toutes la meme preuve, si un tone public
  n'est pas reference en DB, si la personnalisation natale est absente ou si la
  longueur publique sort des bornes Basic.

Les creneaux Premium sont construits en heure locale depuis `timezone`, puis
chaque `reference_local_time` est converti en `reference_datetime_utc`. Certains
creneaux locaux peuvent donc correspondre a la veille ou au lendemain en UTC.

Le `house_system_code` vient du referentiel de service
`horoscope_services.json`. La valeur configuree pour Premium V1 est `placidus`.
Elle ne doit pas etre une constante cachee dans le code metier.

Guards Premium bloquants :

- `timeline` publique exactement 12 entrees, ordonnees selon le profil horaire ;
- labels publics horaires attendus ;
- `local_chart` obligatoire par slot avec Ascendant, MC et maisons locales ;
- `best_slots` et `watch_slots` non vides, evidences et sans chevauchement ;
- aucun `slot_code` technique dans le texte public ;
- si `location.label` est absent, ne pas inventer de ville.

## Perimetre V1

Inclus V1 :

- horoscope quotidien uniquement ;
- theme natal requis ;
- `chart_calculation_id` obligatoire pour recuperer le theme natal ;
- Free : tendance generale courte, sans slots publics ;
- Basic : 3 moments, matin, apres-midi, soir ;
- date interpretee dans la timezone utilisateur ;
- influence lunaire prioritaire ;
- transits majeurs uniquement ;
- aspects majeurs uniquement ;
- scoring deterministe ;
- payload LLM filtre ;
- orchestration fake testee.

Hors perimetre V1 :

- Free daily general non natal ;
- Premium 2h slots ;
- ciel local du moment ;
- Ascendant du moment ;
- MC du moment ;
- maisons locales du moment ;
- semaine / mois ;
- aspects mineurs ;
- Chiron, Lilith, Part de Fortune, Vertex ;
- progressions, directions, revolutions solaires ;
- prediction evenementielle ;
- birth data inline, sauf contrat explicite ulterieur.

V1 Basic ne requiert pas le lieu actuel de l'utilisateur. La timezone est
requise. Le lieu de naissance est deja porte par le theme natal existant.

## Decision d'architecture

Le module horoscope ne cree pas une nouvelle infrastructure transverse en V1.
Il ajoute un workflow metier specialise sur l'infrastructure async existante.

La separation cible est la suivante :

| Zone | Responsabilite |
|------|----------------|
| `astral_calculator` | Calculs deterministes : ciel du moment, positions, transits, aspects, contexte lunaire. |
| `astral_llm` | Redaction et interpretation a partir d'un payload horoscope deja structure. |
| `astral_llm_api` / `astral_llm_worker` | Jobs async, idempotence, polling, persistance, erreurs publiques, catalogue de services. |
| Module applicatif horoscope | Validation metier, orchestration calculateur -> scoring -> normalisation -> LLM -> validation de coherence. |

En V1, il ne faut donc pas creer :

- un `astral_horoscope_api` autonome ;
- un worker dedie horoscope ;
- une table de jobs dediee horoscope ;
- un mecanisme d'idempotence dedie ;
- des endpoints paralleles de type `/v1/horoscope/jobs`.

Une table metier dediee est autorisee uniquement pour l'audit des calculs,
scorings ou projections horoscope. Elle ne doit pas remplacer `llm_jobs`.

Un service API separe ne pourra etre envisage que plus tard, si le cycle de vie
horoscope devient incompatible avec les endpoints d'integration existants.

## Exposition API V1

Le service horoscope doit utiliser l'API d'integration existante :

```http
GET  /v1/services
GET  /v1/services/{code}/contract
POST /v1/jobs
GET  /v1/jobs/{run_id}
```

Le routage public se fait par `service_code` dans le catalogue
`llm_integration_services`.

Exemple de payload public V1 :

```json
{
  "service_code": "horoscope_basic_daily_natal_3_slots",
  "payload": {
    "date": "2026-06-06",
    "timezone": "Europe/Paris",
    "target_language": "fr",
    "chart_calculation_id": "123",
    "audience_level": "general"
  }
}
```

Regles V1 :

- `chart_calculation_id` est obligatoire.
- `birth_data` inline est hors perimetre V1.
- Le service horoscope ne modifie jamais le payload natal existant.
- Si un theme natal plus recent est necessaire, le service doit echouer avec
  une erreur typee ou declencher un recalcul seulement selon un contrat explicite.

## Theme natal source

La V1 Basic necessite un theme natal deja calcule. L'orchestrateur recoit un
`chart_calculation_id` permettant de recuperer un payload natal courant et
compatible.

L'orchestrateur ne doit pas recalculer le natal silencieusement. Si le theme
natal existant est absent, obsolete ou incompatible, le job doit echouer avec
une erreur typee :

- `HOROSCOPE_NATAL_CHART_REQUIRED`
- `HOROSCOPE_NATAL_CHART_NOT_FOUND`
- `HOROSCOPE_NATAL_CHART_OBSOLETE`

Le service horoscope peut utiliser le theme natal comme reference, mais il ne
consomme pas directement `llm_projection_natal_v1` comme payload principal. Il
doit construire un contrat dedie `horoscope_interpretation_request_v1`.

Les champs `summary`, `interpretive_hint` ou `reading_plan` du natal ne doivent
pas etre recopies tels quels dans la reponse horoscope.

## Timezone et slots

La date d'horoscope est interpretee dans la timezone utilisateur.

Pour `horoscope_basic_daily_natal_3_slots`, les slots sont construits en heure
locale :

| Slot | Plage locale | Timestamp de reference |
|------|--------------|------------------------|
| `morning` | 06:00-12:00 | 09:00 local |
| `afternoon` | 12:00-18:00 | 15:00 local |
| `evening` | 18:00-00:00 | 21:00 local |

Les timestamps fixes sont la strategie V1. Le scan continu d'une plage est hors
perimetre V1.

Ces horaires doivent venir du referentiel `horoscope_time_slot_profiles`, pas
d'une constante metier codee en dur.

## Workflow metier

Le workflow horoscope suit cette sequence obligatoire :

1. Valider l'enveloppe job existante.
2. Resoudre `horoscope_basic_daily_natal_3_slots` dans le catalogue existant.
3. Valider le payload metier horoscope avec son schema JSON dedie.
4. Recuperer le theme natal via `chart_calculation_id`.
5. Construire la requete calculateur.
6. Appeler le service dedie dans `astral_calculator`.
7. Recevoir des faits astrologiques deterministes.
8. Scorer les signaux horoscope dans le module applicatif horoscope.
9. Agreger les signaux en themes produit lisibles.
10. Construire une requete LLM structuree et courte.
11. Appeler le service/prompt dedie dans `astral_llm`.
12. Valider que la reponse LLM reste reliee aux preuves astrologiques fournies.
13. Renvoyer le resultat au mecanisme async existant.

Le LLM intervient uniquement apres calcul, tri, scoring et normalisation.

## Branchement orchestration

Le routage doit s'appuyer sur le catalogue
`llm_integration_services.payload_contract`.

Le validateur d'integration valide :

1. l'enveloppe `integration_job_request_v1` ;
2. le payload `horoscope_basic_daily_natal_request_v1`.

Le routage d'execution doit ajouter un cas explicite dans
`unified_reading_orchestrator` ou dans le routeur de services d'integration
existant :

- si `service_code = horoscope_basic_daily_natal_3_slots`, appeler
  `HoroscopeBasicDailyNatalOrchestrator` ;
- sinon conserver les flux existants.

Un service `active` ou `beta` sans orchestrateur doit retourner
`501 SERVICE_NOT_IMPLEMENTED` avant persistance.

## Emplacement recommande du code

Emplacement conceptuel recommande en V1 :

```text
astral_calculator/
  src/horoscope/
    sky_snapshot.rs
    transits.rs
    daily_slots.rs

astral_llm/
  prompts/horoscope_basic_daily_natal/
    system.md
    task.md
    format.md
    safety.md

astral_llm/crates/astral_llm_application/src/horoscope/
  request_validator.rs
  natal_chart_loader.rs
  calculation_request_builder.rs
  signal_scorer.rs
  theme_aggregator.rs
  llm_request_builder.rs
  response_validator.rs
  orchestrator.rs
```

En V1, le module d'orchestration horoscope peut etre heberge dans
`astral_llm_application` pour reutiliser l'infrastructure jobs existante, mais il
doit rester isole dans un namespace `horoscope`.

Ce module n'est pas "LLM" au sens strict : il orchestre le workflow complet
calculateur -> scoring -> LLM -> validation. Il ne doit pas dependre des modules
internes de prompt natal ni des contrats `natal_reading_v1`.

Si le module grossit, extraction possible vers `astral_horoscope_application`,
sans API, worker ni tables jobs dediees.

## Contrats a prevoir

Les contrats doivent preceder les endpoints et les branchements worker.

| Contrat | Role |
|---------|------|
| `horoscope_basic_daily_natal_request_v1` | Payload metier public du service V1. |
| `horoscope_calculation_request_v1` | Requete envoyee par l'orchestrateur au calculateur. |
| `horoscope_calculation_response_v1` | Vue audit complete des faits deterministes produits par le calculateur. |
| `horoscope_interpretation_request_v1` | Vue courte, filtree, scoree, destinee au LLM. |
| `horoscope_response_v1` | Reponse finale publique du service horoscope. |

Ces schemas doivent etre places dans `contracts/` selon la responsabilite :

- `contracts/calculator/` pour les contrats calculateur ;
- `contracts/llm/` pour les contrats LLM ;
- `contracts/integration/` pour les exemples et notes de mapping.

Tout changement cassant sur le payload public ou la reponse publique impose un
bump de version. Les changements internes de scoring sans modification de
structure ne necessitent pas forcement un bump, mais doivent etre traces dans le
referentiel et les tests golden.

## Contrat calculateur horoscope

`horoscope_calculation_response_v1` doit retourner au minimum :

- `period` ;
- `slots[]` ;
- `sky_snapshot` par slot ;
- `moon_context` par slot ;
- `transits_to_natal[]` ;
- `current_sky_aspects[]` ;
- `natal_house_activations[]` si le theme natal les rend disponibles ;
- `calculation_warnings[]` ;
- `evidence_keys[]`.

Le calculateur ne retourne pas :

- `dominant_themes` ;
- `opportunities` ;
- `watch_points` ;
- `advice` ;
- texte interpretatif final.

Le niveau produit ne doit pas modifier les faits astronomiques calcules, seulement
la projection, le scoring et la shortlist utilisee pour la redaction.

## Payload LLM horoscope

`horoscope_interpretation_request_v1` est une projection courte et filtree.

Contraintes anti raw dump :

- `main_signals.length <= N` selon le profil de shortlist ;
- `dominant_themes.length <= M` selon le profil de shortlist ;
- `evidence.length <= K` selon le profil de shortlist ;
- aucun tableau `raw_transits` ;
- aucun champ `all_transits` ;
- aucun champ `debug_aspects` hors mode debug.

Les faits bruts complets peuvent etre persistes en audit, mais ne doivent pas
etre transmis au LLM hors mode debug explicitement interdit en production.

Le payload public doit porter `target_language`. Le calculateur reste
independant de la langue. Le module horoscope construit un payload LLM avec des
codes stables et, si necessaire, des labels humanises. La localisation finale
des textes appartient a `astral_llm`.

Les `evidence_key` restent stables et non localisees.

## Preuves astrologiques

Chaque preuve astrologique horoscope doit avoir au minimum :

- `evidence_key` ;
- `fact_type` ;
- `slot_id` ;
- `source` ;
- `transiting_object` ;
- `natal_target` optionnel ;
- `aspect` optionnel ;
- `orb_deg` optionnel ;
- `natal_house` optionnel ;
- `theme_code` ;
- `score_contribution` ;
- `human_label`.

Exemples de cles :

```text
slot:morning:moon:natal_house:6
slot:afternoon:mars:square:natal_moon
slot:evening:venus:trine:natal_mercury
```

La reponse LLM ne peut citer que des `evidence_key` fournies dans
`horoscope_interpretation_request_v1`.

## Scoring

Le scoring produit au minimum :

- `priority_score` ;
- `intensity` ;
- `tone` ;
- `theme_code` ;
- `duration_class` ;
- `confidence_score` ;
- `score_breakdown`.

Le `score_breakdown` doit permettre d'auditer :

- poids planete transitante ;
- poids cible natale ;
- poids aspect ;
- poids orbe ;
- poids maison ;
- bonus repetition thematique ;
- bonus exactitude ;
- penalite signal trop court ou trop faible.

Le scoring doit etre deterministe, testable et reproductible. Aucun poids, seuil
ou mapping metier ne doit etre hardcode si la valeur peut etre portee par le
referentiel.

Si aucun signal ne depasse les seuils de shortlist :

- ne pas envoyer un payload vide au LLM ;
- produire un fallback deterministe contractuel base sur la Lune et le ciel
  general, ou echouer avec `HOROSCOPE_NO_SIGNIFICANT_SIGNAL`.

Pour V1 Basic, la recommandation est un fallback sobre si ce comportement est
inscrit dans `horoscope_response_v1`.

## Reponse publique

Structure cible minimale de `horoscope_response_v1` :

```json
{
  "contract_version": "horoscope_response_v1",
  "service_code": "horoscope_basic_daily_natal_3_slots",
  "period": {
    "date": "2026-06-06",
    "timezone": "Europe/Paris"
  },
  "summary": {
    "title": "string",
    "text": "string"
  },
  "slots": [
    {
      "slot_code": "morning",
      "title": "string",
      "text": "string",
      "advice": "string",
      "evidence_keys": []
    }
  ],
  "watch_points": [],
  "opportunities": [],
  "evidence_summary": [],
  "quality": {}
}
```

Chaque slot doit avoir au moins une preuve astrologique ou une justification de
fallback.

Longueurs cibles pour `horoscope_basic_daily_natal_3_slots` :

- summary : 60-100 mots ;
- chaque slot : 80-140 mots ;
- conseil final : 30-60 mots ;
- total cible : 350-600 mots.

## Referentiel base de donnees

Toute valeur canonique doit venir de la base si elle peut etre referencee,
modifiee ou auditee.

Avant de coder une constante, suivre le processus base avant code :

1. verifier que la table existe et contient les lignes necessaires ;
2. inserer les valeurs absentes ;
3. sinon creer la table avec les jointures correctes vers les tables de reference ;
4. consommer ces donnees depuis le code via repository/runtime.

Referentiels cibles :

- `json_db/horoscope_services.json` ;
- `json_db/horoscope_product_levels.json` ;
- `json_db/horoscope_time_slot_profiles.json` ;
- `json_db/horoscope_supported_objects.json` ;
- `json_db/horoscope_supported_aspects.json` ;
- `json_db/horoscope_transiting_object_weights.json` ;
- `json_db/horoscope_natal_target_weights.json` ;
- `json_db/horoscope_aspect_weights.json` ;
- `json_db/horoscope_orb_weight_bands.json` ;
- `json_db/horoscope_duration_classes.json` ;
- `json_db/horoscope_signal_theme_mappings.json` ;
- `json_db/horoscope_theme_advice_axes.json` ;
- `json_db/horoscope_shortlist_profiles.json` ;
- `json_db/horoscope_intensity_bands.json`.

Seed V1 attendu pour `horoscope_time_slot_profiles` :

| service_code | slot_code | start_local_time | end_local_time | reference_local_time |
|--------------|-----------|------------------|----------------|----------------------|
| `horoscope_basic_daily_natal_3_slots` | `morning` | `06:00` | `12:00` | `09:00` |
| `horoscope_basic_daily_natal_3_slots` | `afternoon` | `12:00` | `18:00` | `15:00` |
| `horoscope_basic_daily_natal_3_slots` | `evening` | `18:00` | `00:00` | `21:00` |

Les tables doivent etre importees via `scripts/import_json_db_to_postgres.py` ou
un patch idempotent dedie si l'import complet n'est pas souhaite.

Les tests de scoring doivent charger les seeds JSON via `include_str!` ou
fixtures partagees afin d'eviter une divergence entre referentiel et tests.

## Persistance et statuts

`llm_jobs` reste la table de cycle de vie async. Les payloads et resultats
horoscope peuvent etre stockes :

- soit dans les payloads existants du job si le format le permet ;
- soit dans une table specialisee de payload metier pour l'audit.

Une table metier dediee est autorisee uniquement pour l'audit des calculs ou
scorings horoscope, pas pour remplacer `llm_jobs`.

Transitions publiques attendues :

```text
queued -> running -> completed | failed | safety_rejected
```

Mapping d'erreurs :

- calculateur indisponible -> `failed` ;
- LLM indisponible -> `failed` ;
- preuve astrologique inventee ou incoherente -> `failed` ou `safety_rejected`
  selon la nature ;
- payload invalide -> `422` avant job si validation pre-persistance ;
- service non implemente -> `501` avant persistance.

Disponibilite catalogue :

- `planned` : non executable, refuse proprement a l'execution ;
- `beta` : executable en fake/staging ou tenants autorises ;
- `active` : executable publiquement.

## Codes d'erreur horoscope

Codes a utiliser ou mapper explicitement :

- `HOROSCOPE_PAYLOAD_INVALID`
- `HOROSCOPE_SERVICE_NOT_IMPLEMENTED`
- `HOROSCOPE_NATAL_CHART_REQUIRED`
- `HOROSCOPE_NATAL_CHART_NOT_FOUND`
- `HOROSCOPE_NATAL_CHART_OBSOLETE`
- `HOROSCOPE_CALCULATOR_UNAVAILABLE`
- `HOROSCOPE_CALCULATION_FAILED`
- `HOROSCOPE_SCORING_FAILED`
- `HOROSCOPE_NO_SIGNIFICANT_SIGNAL`
- `HOROSCOPE_LLM_UNAVAILABLE`
- `HOROSCOPE_LLM_FAILED`
- `HOROSCOPE_EVIDENCE_MISMATCH`
- `HOROSCOPE_RESPONSE_INVALID`
- `HOROSCOPE_IDEMPOTENCY_CONFLICT`

## Railguards obligatoires

Ces regles sont bloquantes en revue.

### Architecture

- Ne pas creer de nouvelle infrastructure async pour l'horoscope en V1.
- Reutiliser `POST /v1/jobs`, `GET /v1/jobs/{run_id}`, le worker existant,
  `llm_jobs`, l'idempotence et les statuts publics existants.
- Ajouter l'horoscope comme nouveau `service_code` catalogue.
- Ne pas melanger calcul, scoring et redaction dans un meme module.
- Ne pas mettre le scoring horoscope dans `astral_calculator`.

### Calculateur

- `astral_calculator` produit uniquement des faits deterministes.
- Aucun texte final utilisateur ne doit etre produit par le calculateur.
- Aucun appel LLM ne doit exister dans le calculateur.
- Les positions, aspects, orbes et activations doivent rester auditables.
- Les valeurs canoniques doivent venir du referentiel DB quand elles existent.

### Orchestration horoscope

- L'orchestrateur doit appeler le calculateur avant le LLM.
- L'orchestrateur doit reduire les signaux avant l'appel LLM.
- Le LLM ne doit jamais recevoir une liste brute non filtree de transits.
- Le scoring doit etre deterministe, testable et reproductible.
- Les themes produit doivent etre derives des signaux scores, pas inventes par le LLM.
- Les erreurs doivent distinguer validation payload, service non implemente,
  calculateur indisponible, LLM indisponible et echec de coherence.
- Le service horoscope ne doit jamais muter le payload natal existant.

### LLM

- Le LLM recoit un payload structure, pas des faits astrologiques bruts en vrac.
- Le LLM ne doit pas inventer de nouveaux transits, aspects ou activations.
- Chaque section importante de la reponse doit pouvoir etre reliee a une preuve
  astrologique fournie.
- Une garde doit verifier la coherence preuve / reponse.
- Les prompts horoscope doivent etre dedies.
- Ne pas reutiliser directement les prompts natal si le contrat de sortie est
  different.

### Contrats et compatibilite

- Tout nouveau service doit avoir un contrat JSON public et des exemples.
- Les contrats doivent etre valides par tests avant branchement worker.
- Le statut initial public reste `queued`.
- Les replays idempotents doivent garder la semantique existante.
- Un service `active` ou `beta` sans orchestrateur doit rester en
  `501 SERVICE_NOT_IMPLEMENTED` avant persistance.

### Tests

- Tous les nouveaux tests Rust doivent etre places dans `tests/` a la racine.
- Couvrir au minimum :
  - validation schema du payload horoscope ;
  - scoring deterministe ;
  - aggregation de themes ;
  - orchestration fake calculateur + fake LLM ;
  - erreurs negatives payload ;
  - idempotence/replay via `POST /v1/jobs` ;
  - coherence preuve / reponse.
- Aucun test ne doit dependre d'OpenAI pour la V1 fake.
- Les tests de scoring ne doivent pas recopier des constantes metier divergentes
  du referentiel.

Fake calculator V1 :

- retourne 3 slots deterministes ;
- contient au moins 1 signal lunaire ;
- contient au moins 1 aspect tendu ;
- contient au moins 1 aspect fluide ;
- contient au moins 2 themes distincts.

Fake LLM V1 :

- retourne une reponse conforme `horoscope_response_v1` ;
- cite uniquement les `evidence_key` fournies ;
- permet de tester les refus de preuve astrologique inventee.

Goldens a prevoir :

- `horoscope_calculation_response_v1_basic_daily_paris_1990.json` ;
- `horoscope_interpretation_request_v1_basic_daily_paris_1990.json` ;
- `horoscope_response_v1_basic_daily_fake.json`.

Le golden LLM fake doit etre stable. Le golden OpenAI reel ne doit pas etre
bloquant.

### Documentation

- Toute implementation horoscope doit mettre a jour ce document.
- `docs/BASIC_PAYLOAD_IMPLEMENTATION.md` ne doit contenir qu'un renvoi court
  vers ce document pour l'articulation avec le payload natal/basic.
- Le service horoscope doit ajouter une section dediee dans
  `docs/integration_api_contract.md` :
  - `service_code` ;
  - `availability` ;
  - `payload_contract` ;
  - exemple `POST /v1/jobs` ;
  - exemple `GET /v1/jobs/{run_id}` ;
  - erreurs possibles.

## Plan de mise en oeuvre V1

1. Figer `service_code = horoscope_basic_daily_natal_3_slots`.
2. Ajouter le service dans le catalogue existant avec `availability = planned`.
3. Creer les schemas JSON et exemples de payload.
4. Ajouter les tables ou lignes de referentiel necessaires aux slots, scoring,
   shortlist et mappings de themes.
5. Ajouter le calculateur horoscope minimal.
6. Ajouter le module applicatif horoscope : validation, chargement natal,
   scoring, aggregation, construction requete LLM, validation reponse.
7. Ajouter le prompt et le validateur LLM horoscope.
8. Brancher le service dans l'orchestrateur de jobs existant.
9. Passer le service en `beta` seulement lorsque l'orchestration fake est
   couverte par tests.
10. Ajouter `scripts/test_horoscope_basic_daily_fake.ps1`.
11. Ajouter ulterieurement `scripts/test_horoscope_basic_daily_openai.ps1`
    avec option `-UseReal`, non bloquant pour la V1 fake.

## Definition de pret pour V1

Une V1 est prete a implementer lorsque les points suivants sont figes :

- schema du payload public ;
- contrat de recuperation du theme natal via `chart_calculation_id` ;
- date/periode cible ;
- timezone cible ;
- liste des corps astrologiques inclus ;
- aspects inclus ;
- regles de shortlist ;
- structure de sortie utilisateur ;
- comportement en cas de calculateur indisponible ;
- comportement en cas de LLM indisponible ;
- seuils minimaux de validation preuve / reponse ;
- comportement `HOROSCOPE_NO_SIGNIFICANT_SIGNAL`.

Tant que ces elements ne sont pas figes, les agents doivent limiter leur travail
aux contrats, au referentiel, aux tests de validation et aux scaffolds sans
brancher un service public `active`.

## Criteres d'acceptation V1

1. Service present dans `llm_integration_services` en `planned`, puis `beta`.
2. Contrats JSON publies et testes.
3. Validation payload en deux temps operationnelle.
4. Calculateur retourne 3 slots deterministes.
5. Scoring produit une shortlist stable.
6. Payload LLM ne contient aucun dump brut.
7. Fake LLM produit une reponse conforme.
8. Evidence guard detecte une preuve astrologique inventee.
9. `POST /v1/jobs` cree un job `queued`.
10. Worker complete le job en fake.
11. Replay idempotent conserve la semantique existante.
12. Service sans orchestrateur retourne `501` avant persistance.
13. Smoke fake Docker passe.

## Etat implementation V1 fake

Implementation initiale :

- service `horoscope_basic_daily_natal_3_slots` ajoute au catalogue en `beta` ;
- contrats JSON ajoutes pour payload public, calculateur, payload LLM filtre et
  reponse publique ;
- referentiels JSON ajoutes pour slots, poids de scoring, mappings themes,
  shortlist et bandes d'intensite ;
- endpoint calculateur fake `/v1/calculations/horoscope/daily-natal` ;
- module applicatif `astral_llm_application::horoscope` avec validation,
  construction de requete calculateur, scoring deterministe, aggregation,
  construction du payload LLM filtre et garde de preuves astrologiques ;
- verification calculateur que `chart_calculation_id` pointe vers un calcul
  natal termine dans `astral_chart_calculations` ;
- validation stricte de timezone IANA et garde de preuves astrologiques
  bloquant les cles inventees, non textuelles ou les slots sans preuve ;
- orchestration async branchee via `POST /v1/jobs` et worker existant ;
- prompts dedies ajoutes sous `astral_llm/prompts/horoscope_basic_daily_natal/v1/` ;
- script smoke fake `scripts/test_horoscope_basic_daily_fake.ps1`.

Limite assumee de cette etape : le redacteur V1 est le fake stable requis pour
les tests, sans appel fournisseur OpenAI.

## Etat refactor slot-based V1

Refactor V1 applique :

- `horoscope_interpretation_request_v1` porte maintenant `day_overview` et
  `slots[]` ; pour `horoscope_basic_daily_natal_3_slots`, `slots[]` pilote la
  redaction des trois moments.
- Chaque slot porte `specificity`, `theme_code`, `tone`, `intensity`,
  `main_signal_keys`, `required_evidence_keys`, `advice_axis`, `avoid_axis`,
  `watch_point`, `best_for` et `fallback_reason`.
- Le fake writer produit trois textes differencies, relies aux evidence du slot
  et typographiquement francais.
- Le validateur rejette les repetitions, la copie de `day_overview`, les
  formulations generiques, les references astrologiques absentes, les conseils
  ou `best_for` dupliques, les fuites de codes techniques et les incoherences
  evidence / slot.
- `horoscope_response_v1` est enrichi de champs compatibles par slot :
  `theme`, `tone`, `best_for`, `watch_point`, et de flags qualite.
- Le service reste en `beta` tant que les validations fake, goldens, guards et
  reviews adversariales cadrent le perimetre.

Plan et reviews :
[`docs/reviews/horoscope_v1/INDEX.md`](reviews/horoscope_v1/INDEX.md).
