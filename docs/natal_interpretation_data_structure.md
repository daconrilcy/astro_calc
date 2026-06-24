# Structure des donnees - interpretation du theme natal

Ce document decrit les objets JSON produits quand on demande une interpretation de theme natal via les surfaces courantes.

Sources de verite:
- surface publique recommandee: `astral_gateway`, routes `/v2/natal/*`;
- calcul complet: `contracts/calculator/natal_structured_v14.schema.json`;
- projection LLM theme complet: `contracts/calculator/llm_projection_natal_v1.schema.json`;
- calcul sans heure ou partiel: `contracts/calculator/astro_simplified_natal_response_v1.schema.json` et `contracts/calculator/natal_simplified_structured_v1.schema.json`;
- lecture LLM: `contracts/llm/generate_reading_response_v1.schema.json` et `contracts/llm/natal_reading_v1.schema.json`;
- explications neutres pre-generation: `docs/natal_explanations_contract.md`;
- profils editoriaux: `config/natal_interpretation_profiles/natal_light.json`, `natal_basic.json`, `natal_premium.json`.

## 1. Taxonomie produit

La passerelle publique expose deux variantes et trois niveaux.

| Route publique | Variant | Tier | `metadata.product_code` | Profil LLM | Projection calculateur |
|---|---|---|---|---|---|
| `POST /v2/natal/full/free` | `full` | `free` | `natal_full_free` | `natal_light` | `compact` |
| `POST /v2/natal/full/basic` | `full` | `basic` | `natal_full_basic` | `natal_basic` | `standard` |
| `POST /v2/natal/full/premium` | `full` | `premium` | `natal_full_premium` | `natal_premium` | `rich` |
| `POST /v2/natal/simplified/free` | `simplified` | `free` | `natal_simplified_free` | `natal_simplified` | n/a |
| `POST /v2/natal/simplified/basic` | `simplified` | `basic` | `natal_simplified_basic` | `natal_simplified` | n/a |
| `POST /v2/natal/simplified/premium` | `simplified` | `premium` | `natal_simplified_premium` | `natal_simplified` | n/a |

Important: il n'existe pas, dans la passerelle V2 courante, de `full` degrade accepte sans heure de naissance. Les routes `/v2/natal/full/*` exigent `birth.time`, `birth.timezone` et `birth.location`. Quand l'heure manque, le parcours honnete est `simplified`; la reponse doit etre nommee lecture partielle, simplifiee ou indicative, jamais lecture full degradee.

## 2. Entree publique commune

Toutes les routes V2 recoivent:

```json
{
  "context": {
    "request_id": "client-request-id",
    "idempotency_key": "optional-client-key",
    "target_language_code": "fr",
    "audience_level": "general|beginner|intermediate|expert"
  },
  "birth": {
    "date": "1990-06-15",
    "time": "14:30:00",
    "timezone": "Europe/Paris",
    "location": {
      "label": "Paris",
      "latitude": 48.8566,
      "longitude": 2.3522
    }
  }
}
```

Regles par variant:
- `full`: `date`, `time`, `timezone`, `location.latitude`, `location.longitude` obligatoires.
- `simplified`: `date` obligatoire; `time`, `timezone`, `location` optionnels, avec `time` interdit sans `timezone`.
- `audience_level = general` est remplace par le defaut du tier: `free -> beginner`, `basic -> intermediate`, `premium -> expert`.

## 3. Enveloppe de reponse publique V2

Une interpretation natal V2 retourne toujours une enveloppe gateway:

```json
{
  "metadata": {
    "product_code": "natal_full_basic",
    "tier": "basic",
    "variant": "full",
    "contract_version": "natal_reading_response_v2"
  },
  "quality": {
    "calculator_contract_version": "natal_structured_v14",
    "llm_contract_version": "generate_reading_response_v1",
    "reading_completeness": "completed"
  },
  "calculation": {},
  "reading": {},
  "debug": {
    "run_id": "...",
    "llm_request": {}
  }
}
```

Champs:
- `metadata`: identifie le produit public, le tier, la variante et le contrat de l'enveloppe gateway.
- `quality.calculator_contract_version`: version du contrat calculateur detectee dans `calculation`.
- `quality.llm_contract_version`: `generate_reading_response_v1` en generation, `generate_reading_request_v1` sur les routes `/inspect`.
- `quality.reading_completeness`: pour `full`, reprend typiquement le statut calculateur `completed`; pour `simplified`, reprend `reading_hint.reading_completeness`, actuellement `partial`.
- `calculation`: reponse brute du calculateur.
- `reading`: reponse brute du LLM, taguee par `status`.
- `debug.llm_request`: requete LLM interne assemblee par la passerelle.

Les routes `/inspect` ne generent pas de lecture. Elles retournent:

```json
{
  "metadata": {},
  "quality": {},
  "calculation": {},
  "llm_request": {}
}
```

## 4. Lecture LLM commune (`reading`)

`reading` est un `GenerateReadingResponse` tague.

Succes:

```json
{
  "status": "success",
  "run_id": "...",
  "reading": {
    "schema_version": "natal_reading_v1",
    "language": "fr",
    "reading_type": "natal",
    "summary": {
      "title": "...",
      "short_text": "..."
    },
    "chapters": [],
    "legal": {
      "disclaimer": "..."
    },
    "quality": {
      "used_provider": "openai",
      "used_model": "gpt-5-mini",
      "generation_mode": "single_pass|chapter_orchestrated",
      "prompt_family": "natal_prompter",
      "prompt_version": "v1",
      "astro_contract_version": "natal_structured_v14",
      "fallback_used": false
    }
  },
  "token_usage": {}
}
```

Rejet securite:

```json
{
  "status": "safety_rejected",
  "run_id": "...",
  "error": {
    "code": "SAFETY_POLICY_VIOLATION",
    "category": "...",
    "message": "...",
    "rule_id": "..."
  },
  "violations": [],
  "token_usage": {}
}
```

Echec generation:

```json
{
  "status": "failed",
  "run_id": "...",
  "error": {
    "code": "INVALID_JSON_OUTPUT|SCHEMA_VALIDATION_FAILED|READING_QUALITY_FAILED|...",
    "message": "...",
    "details": {}
  },
  "token_usage": {}
}
```

Chaque chapitre de `reading.reading.chapters[]` a la structure:

```json
{
  "code": "identity",
  "title": "Identite",
  "body": "...",
  "astro_basis": [
    {
      "fact_id": "signal:...",
      "label": "Soleil en Gemeaux",
      "factor": "Soleil en Gemeaux",
      "interpretive_role": "core|supporting|nuance"
    }
  ],
  "confidence": "low|medium|high",
  "safety_flags": []
}
```

`astro_basis` peut etre vide en Free/Basic selon le profil. En Premium, il est attendu et valide contre les preuves disponibles.

## 5. Niveau Free: theme natal complet

Route: `POST /v2/natal/full/free`

Mapping:
- profil LLM: `natal_light`;
- mode profil: `single_pass`;
- domaines / chapitres max: 1;
- chapitre attendu: `identity`;
- cible mots chapitre: min 60, cible 120, max 200;
- preuves structurees: `evidence.enabled = false`;
- `astro_basis` non obligatoire.

Donnees produites:
- `metadata.product_code = natal_full_free`;
- `calculation`: calcul complet `astro_engine_response_v1`, contenant le payload audit `natal_structured_v14` et une projection `compact`;
- `debug.llm_request.product_context.interpretation_profile_code = natal_light`;
- `debug.llm_request.response_contract.generation_mode = chapter_orchestrated` cote gateway, puis le resolveur de profil applique le mode effectif du profil `natal_light`;
- `reading.reading.quality.generation_mode` doit refleter le mode effectif utilise.

Structure fonctionnelle de la lecture Free:
- `summary`: titre et synthese tres courte;
- `chapters[0]`: lecture d'identite generale;
- `legal`: disclaimer;
- `quality`: provider, modele, prompt et contrat astro utilises.

## 6. Niveau Basic: theme natal complet

Route: `POST /v2/natal/full/basic`

Mapping:
- profil LLM: `natal_basic`;
- mode: `chapter_orchestrated`;
- domaines / chapitres max: 6;
- chapitres: `identity`, `emotional_life`, `relationships`, `career`, `growth_path`, `talents`;
- cible mots par chapitre: min 70, cible 130, max 250;
- preuves structurees: `evidence.enabled = false`;
- gates qualite non bloquantes (`blocking_gate = false`).

Donnees produites:
- `metadata.product_code = natal_full_basic`;
- `calculation`: calcul complet `astro_engine_response_v1`, audit `natal_structured_v14`, projection `standard`;
- `debug.llm_request.product_context.interpretation_profile_code = natal_basic`;
- `reading.reading.chapters[]`: jusqu'a 6 chapitres correspondant aux domaines ci-dessus;
- `astro_basis`: optionnel ou non exige par le profil.

Structure fonctionnelle de la lecture Basic:
- une synthese globale;
- plusieurs chapitres courts et distincts;
- une qualite de generation indiquant `chapter_orchestrated`;
- disclaimer obligatoire.

## 7. Niveau Premium: theme natal complet

Route: `POST /v2/natal/full/premium`

Mapping:
- profil LLM: `natal_premium`;
- mode: `chapter_orchestrated`;
- domaines max: 12;
- chapitres max: 12;
- chapitres configures: `identity`, `emotional_life`, `relationships`, `career`, `communication_mind`, `family_roots`, `money`, `family`, `inner_conflicts`, `talents`, `growth_path`;
- structure de corps: 4 paragraphes, 60 a 110 mots par paragraphe;
- cible mots par chapitre: min 260, cible 360, max 480;
- preuves structurees: `evidence.enabled = true`;
- evidence policy: au moins 4 preuves candidates par chapitre, au moins 2 familles de preuves distinctes, au moins 1 preuve non-placement si disponible;
- gates qualite bloquantes (`blocking_gate = true`).

Donnees produites:
- `metadata.product_code = natal_full_premium`;
- `calculation`: calcul complet `astro_engine_response_v1`, audit `natal_structured_v14`, projection `rich`;
- `debug.llm_request.product_context.interpretation_profile_code = natal_premium`;
- `reading.reading.chapters[]`: lecture longue par domaines;
- `chapters[].astro_basis[]`: preuves astrologiques structurees, avec `fact_id` valide dans le pack d'evidence, role normalise (`core`, `supporting`, `nuance`) et coherence controlee.

Structure fonctionnelle de la lecture Premium:
- synthese globale;
- chapitres longs;
- preuves astrologiques citees par chapitre;
- controle anti-repetition et qualite editoriale;
- disclaimer obligatoire.

## 8. Calcul complet (`calculation` pour variant `full`)

La passerelle appelle le calculateur avec `astro_engine_request_v1`, puis verifie que la reponse contient:

```json
{
  "response_contract_version": "astro_engine_response_v1",
  "calculation_result": {
    "status": "completed"
  },
  "audit_payload": {
    "contract_version": "natal_structured_v14",
    "payload": {}
  }
}
```

Le payload audit `natal_structured_v14` est la structure complete de reference:

```json
{
  "product_code": "basic",
  "chart_calculation_id": 123,
  "reference_version_id": 1,
  "subject_label": null,
  "birth_datetime_utc": "1990-06-15T12:30:00Z",
  "chart_context": {},
  "positions": [],
  "angles": [],
  "dignities": [],
  "chart_emphasis": {},
  "rulership_context": {},
  "house_axis_emphasis": [],
  "lunar_phase_context": {},
  "accidental_dignities": [],
  "signals": [],
  "reading_plan": []
}
```

Detail des niveaux:
- `chart_context`: type de carte, systemes astrologiques, version referentiel, scope `full_natal`, fiabilite, secte, hemispheres, snapshots de scoring.
- `positions[]`: placements des objets; chaque item contient objet, longitude, signe, maison, mouvement, contexte signe/maison/objet, dignites, visibilite, dignite accidentelle.
- `angles[]`: exactement 4 angles (`ascendant`, `descendant`, `mc`, `ic`) avec signe, maison et longitude.
- `dignities[]`: dignites essentielles detectees, polarite, score et signal lie.
- `chart_emphasis`: signes, maisons et objets dominants.
- `rulership_context`: maitres, dispositeurs, chaines, receptions mutuelles.
- `house_axis_emphasis[]`: axes de maisons dominants et scores.
- `lunar_phase_context`: phase lunaire, angle Soleil-Lune, progression, tags et signaux lies.
- `accidental_dignities[]`: evaluations de dignite accidentelle par objet et conditions.
- `signals[]`: signaux interpretatifs prioritaires. Types de cle: `object_position:*`, `angle:*`, `dignity:*`, `cluster:*`, `aspect:*`. Chaque signal porte theme, titre, resume, scores, tags, contexte d'aspect si applicable et preuves.
- `reading_plan[]`: plan de lecture calcule a partir des signaux, par slots (`core_identity`, `dominant_cluster`, `main_tension_or_support`, `expression_style`, `background_factors`).

La projection LLM `llm_projection_natal_v1`, quand elle est produite, est une vue reduite et lisible du theme:

```json
{
  "contract_version": "llm_projection_natal_v1",
  "projection_level": "compact|standard|rich|expert",
  "projection_limits": {},
  "chart": {},
  "reading_order": [],
  "core_identity": {},
  "dominant_themes": {},
  "placements": {
    "primary": [],
    "supporting": [],
    "background": []
  },
  "angles": {},
  "strengths": {},
  "relationship_network": {},
  "dynamics": {},
  "house_axes": [],
  "keywords": {}
}
```

`projection_level` depend du tier public: Free `compact`, Basic `standard`, Premium `rich`.

## 9. Donnees sans heure: parcours simplifie, pas full degrade

Quand l'heure de naissance n'est pas fournie, la structure produite est celle du calcul simplifie:

```json
{
  "response_contract_version": "astro_simplified_natal_response_v1",
  "input_precision": {
    "level": "date_only",
    "date_provided": true,
    "time_provided": false,
    "timezone_provided": false,
    "location_provided": false
  },
  "computed_scope": "stable_birth_date_profile",
  "limitations": [],
  "facts": [],
  "ambiguous_facts": [],
  "excluded_features": [],
  "cusp_warnings": [],
  "simplified_payload": {
    "payload_contract": "natal_simplified_structured_v1",
    "payload": {}
  },
  "llm_payload": {},
  "reading_hint": {
    "recommended_profile_code": "natal_simplified",
    "reading_completeness": "partial"
  }
}
```

Matrice de precision:

| Donnees naissance | `input_precision.level` | `computed_scope` |
|---|---|---|
| date seule | `date_only` | `stable_birth_date_profile` |
| date + lieu sans timezone | `date_with_location_without_timezone` | `stable_birth_date_profile` |
| date + timezone sans heure | `date_with_timezone_without_time` | `stable_birth_date_profile` |
| date + lieu + timezone sans heure | `date_with_location_and_timezone_without_time` | `stable_birth_date_profile` |
| date + heure + timezone sans lieu | `datetime_without_location` | `planetary_positions` |
| date + heure + timezone + lieu | `complete_birth_data` | `angular_chart` |

`natal_simplified_structured_v1` contient:

```json
{
  "payload_contract": "natal_simplified_structured_v1",
  "computed_scope": "stable_birth_date_profile|planetary_positions|angular_chart",
  "input_precision_level": "date_only",
  "facts": [],
  "ambiguous_facts": [],
  "excluded_features": [],
  "planets": {}
}
```

Detail:
- `facts[]`: faits astrologiques stables, principalement signes d'objets, avec `object_code`, `fact_type = sign`, `sign_code`, `reliability`, `longitude_deg`.
- `ambiguous_facts[]`: faits qui changent dans la fenetre d'incertitude; contient `possible_sign_codes` et `reliability = ambiguous_across_uncertainty_window`.
- `excluded_features[]`: capacites non calculees a cause de l'entree (`ascendant`, `houses`, `sect`, `house_placements`, etc.).
- `planets{}`: miroir legacy pour objets fiables seulement; les objets ambigus sont absents ou `null`.

`llm_payload` controle strictement ce que le LLM peut affirmer:

```json
{
  "profile_code": "natal_simplified",
  "allowed_fact_codes": ["mercury.sign"],
  "allowed_astro_basis_fact_ids": ["placement:mercury"],
  "blocked_interpretation_fact_codes": ["sun.sign"],
  "excluded_feature_codes": ["ascendant", "houses"],
  "profile_excluded_feature_codes": ["ascendant", "houses", "sect", "house_placements"],
  "allowed_limitation_mentions": ["birth_time_missing"],
  "forbidden_interpretation_topics": [],
  "forbidden_topics": []
}
```

Regles:
- `allowed_fact_codes`: affirmations textuelles autorisees.
- `allowed_astro_basis_fact_ids`: seuls IDs autorises dans `chapters[].astro_basis[].fact_id`.
- `blocked_interpretation_fact_codes`: faits calculatoirement ambigus; ne pas les affirmer.
- `excluded_feature_codes`: non calcules par manque d'information.
- `profile_excluded_feature_codes`: calcules eventuellement, mais exclus par le produit `natal_simplified`.
- `reading_hint.reading_completeness`: V1 emet `partial`.

Si `sun.sign` est bloque, la lecture utilise le chapitre `ambiguous_core_identity`, force `confidence = low`, et retire `placement:sun` / `placement:moon` du basis.

## 10. Requete LLM interne

La passerelle transforme le calcul en `generate_reading_request_v1`.

Pour `full`:

```json
{
  "request_id": "...",
  "idempotency_key": null,
  "product_context": {
    "product_code": "natal_prompter",
    "interpretation_profile_code": "natal_basic",
    "user_language": "fr",
    "audience_level": "intermediate"
  },
  "astro_result": {
    "contract_version": "natal_structured_v14",
    "chart_type": "natal",
    "data": {}
  },
  "astrologer_profile": {
    "tone": "warm",
    "jargon_level": "beginner",
    "wording_style": "clear",
    "preferred_domains": [
      "identity",
      "emotional_life",
      "relationships",
      "career",
      "growth_path"
    ],
    "forbidden_wording": [],
    "custom_instructions": null
  },
  "engine": {
    "allow_fallback": true
  },
  "response_contract": {
    "output_schema_version": "natal_reading_v1",
    "generation_mode": "chapter_orchestrated",
    "format": "structured_json",
    "chapters": [],
    "global_max_tokens": null,
    "include_astro_sources": true,
    "include_legal_disclaimer": true
  },
  "safety_policy": null
}
```

Pour `simplified`, `astro_result.contract_version = natal_simplified_structured_v1`, `generation_mode = single_pass`, `include_astro_sources = false`, et `chapters` contient `identity` ou `ambiguous_core_identity`.

## 11. Comparaison rapide des niveaux

| Dimension | Free full | Basic full | Premium full | Sans heure / simplified |
|---|---|---|---|---|
| Route | `/v2/natal/full/free` | `/v2/natal/full/basic` | `/v2/natal/full/premium` | `/v2/natal/simplified/*` |
| Heure requise | oui | oui | oui | non |
| Profil LLM | `natal_light` | `natal_basic` | `natal_premium` | `natal_simplified` |
| Contrat calculateur principal | `natal_structured_v14` | `natal_structured_v14` | `natal_structured_v14` | `natal_simplified_structured_v1` |
| Completeness | `completed` | `completed` | `completed` | `partial` |
| Projection | `compact` | `standard` | `rich` | n/a |
| Mode attendu | single pass effectif | chapitre orchestre | chapitre orchestre | single pass |
| Chapitres | 1 | jusqu'a 6 | jusqu'a 11/12 selon profil | 1 |
| `astro_basis` | non requis | non requis | requis et controle | whitelist stricte |
| Angles / maisons / secte | calcules | calcules | calcules | exclus si non fiables ou hors profil |
| Nom UX recommande | lecture courte | lecture structuree | lecture approfondie | lecture partielle / indicative |

## 12. Services async V1 lies

L'API `astral_llm_api` conserve aussi des jobs async:
- `natal_simplified`: calcul simplifie + lecture;
- `natal_light`, `natal_basic`, `natal_premium`, `natal_premium_plus`: calcul complet selon catalogue si actifs;
- `*_from_payload`: services historiques/deprecies pour soumettre directement un payload deja calcule.

L'enveloppe async est differente de V2:

```json
{
  "run_id": "...",
  "service_code": "natal_basic",
  "status": "queued|running|completed|failed|safety_rejected",
  "result": {
    "calculation": {},
    "interpretation_request": {},
    "reading": {}
  }
}
```

Pour le produit public courant, la surface recommandee reste `astral_gateway` `/v2/natal/*`.

## 13. Explications neutres pre-generation

La gateway V2 natal expose un sibling public `explanations` a cote de `reading`.
Ce bloc ne remplace pas la lecture finale. Il sert de glossaire factuel et neutre
pour guider le prompt principal, puis peut aussi etre inspecte dans l'UI de test.

Flux:
1. la passerelle envoie un sous-payload interne a `/v1/internal/natal/explanations/prepare`;
2. le runtime selectionne deterministement un petit ensemble d'elements majeurs;
3. le cache PostgreSQL renvoie les explications deja connues quand la combinaison existe;
4. sur miss, `gpt-5-mini` genere des phrases courtes, neutres et explicatives;
5. le resultat est injecte dans `astro_result.data.neutral_explanations` pour la lecture LLM;
6. la reponse gateway publie le sibling `explanations` sans modifier `reading`.

Structure publique de `explanations`:

```json
{
  "status": "complete|partial|unavailable",
  "items": [
    {
      "fact_id": "placement:sun_taurus_house_x",
      "kind_code": "placement|angle|house|axis|aspect",
      "title": "Soleil en Taureau",
      "explanation": "Une identite stable, concrete et patiente.",
      "expression_primary": "Maison X - Carriere",
      "source": "cache|generated"
    }
  ],
  "missing_fact_ids": [],
  "errors": []
}
```

Champs:
- `status`: etat de la preparation. `complete` signifie que tous les items retenus sont disponibles; `partial` signifie qu'une partie seulement a pu etre produite ou relue; `unavailable` signifie que la preparation a echoue sans bloquer la lecture.
- `items[]`: liste ordonnee des explications neutres retenues pour la lecture.
- `fact_id`: cle metier stable pour la combinaison expliquee.
- `kind_code`: type logique de l'element source.
- `title`: libelle court lisible dans l'UI.
- `explanation`: une phrase courte, descriptive et non prescriptive.
- `expression_primary`: expression principale retenue pour guider la lecture.
- `source`: `cache` quand la combinaison existe deja en base, `generated` quand elle vient du moteur LLM.
- `missing_fact_ids[]`: elements attendus mais absents du cache ou de la generation.
- `errors[]`: erreurs non bloquantes de preparation.

Contraintes editoriales:
- pas d'interpretation psychologique au sens fort, pas de prediction, pas de conseil, pas de diagnostic;
- phrase courte, neutre, pedagogique, au present;
- vocabulaire simple, sans jargon technique expose au public;
- si l'item est un axe, une maison dominante ou un aspect, l'explication doit decrire la relation ou la zone activee, pas produire une conclusion globale.

Exemple d'usage dans la lecture:

```json
{
  "explanations": {
    "status": "complete",
    "items": [
      {
        "fact_id": "placement:sun_taurus_house_x",
        "kind_code": "placement",
        "title": "Soleil en Taureau",
        "explanation": "Une identite stable, concrete et patiente, qui cherche a se construire dans la carriere et la place sociale.",
        "expression_primary": "Maison X - Carriere",
        "source": "cache"
      }
    ]
  }
}
```
