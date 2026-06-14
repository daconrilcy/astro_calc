# Horoscope V2 Premium Next 7 Days - Validation du resultat LLM

Date : 2026-06-12

Service : `horoscope_premium_next_7_days_natal`

Perimetre : validation post-LLM du parcours `semantic_brief_v2`.

## Principe

Le chemin V2 valide uniquement le contrat public, la coherence avec le
`writer_request`, les dates, les cles de preuve, les snapshots, les sections
Premium obligatoires et le volume de texte public.

Les controles editoriaux par mots, fragments ou phrases ne sont pas bloquants.
Ils ne doivent pas declencher de retry, ni retourner `GenerationError`, ni
produire `HOROSCOPE_PERIOD_V2_QUALITY_FAILED`.

## Point d'entree

1. `period_writer_response()` recupere la sortie provider.
2. `repair_period_response_shape_v2()` restaure les champs techniques et la
   forme attendue.
3. `postprocess_period_provider_response_v2()` applique uniquement des
   normalisations deterministes limitees.
4. `period_writer_response_with_quality_loop()` appelle
   `validate_period_response_quality_gates_v2()`.
5. `validate_period_response_quality_gates_v2()` delegue a
   `validate_period_response_contract_gates_v2()`.
6. L'orchestrateur relance le meme gate contractuel final pour V2 et ajoute
   `debug.period_v2_editorial_audit`.

## Validations bloquantes

- `validate_period_response_schema()` : JSON Schema
  `horoscope_period_response`.
- `contract_version` et `service_code` attendus.
- `period_resolution.included_dates` contient exactement 7 dates.
- `daily_timeline` contient exactement 7 dates uniques, toutes dans la
  periode.
- `key_days`, `best_days`, `watch_days`, `best_windows`, `watch_windows` et
  `evidence_summary` ne referencent que des dates incluses.
- Toutes les `evidence_keys` publiques existent dans `request.evidence`.
- Toutes les `source_snapshot_keys` de fenetres existent dans
  `request.scan_plan.snapshots`.
- `watch_summary` est coherent avec `watch_days` et `watch_windows`.
- `domain_sections`, `strategy`, `best_windows`, `watch_windows` et
  `evidence_summary` respectent la forme Premium.
- `best_windows` et `watch_windows` ne partagent pas la meme identite
  `date + source_snapshot_keys`.
- `quality.provider = "fake"` ignore le controle de word count ; les providers
  reels doivent respecter les bornes du profil.

## Hors bloquant

Ces signaux sont uniquement editoriaux et non bloquants en V2 :

- termes ou fragments anciennement interdits (`semantic_brief`, `evidence_key`,
  `public_role`, `fonction narrative`, etc.) ;
- absence de mots comme `natal`, `lune`, `maison`, `soleil`, `mars`,
  `mercure`, etc. ;
- raisons jugees generiques dans `best_windows` ;
- libelles meta dans `watch_windows` ;
- repetition de phrases mecaniques ;
- presence ou absence de marqueurs lexicaux de personnalisation.

## Audit editorial

Fonction : `period_v2_editorial_audit(request, response)`.

L'audit retourne un objet JSON avec `mode = "non_blocking"`. Il peut exposer
des diagnostics comme :

- `public_word_count`
- `section_word_counts`
- `signal_excessif_de_repetition`
- `titres_dupliques`
- `mismatch_titre_horaire`

L'audit est ajoute dans `result.debug.period_v2_editorial_audit`. Il ne doit
pas etre injecte dans `result.reading.quality`.

## Normalisations V2

`postprocess_period_provider_response_v2()` peut :

- tailler les espaces ;
- appliquer les remplacements typographiques objectifs
  (`demie-journée` -> `demi-journée`, `reorganiser` -> `réorganiser`) ;
- normaliser certains labels publics issus de codes ;
- supprimer les `watch_windows` qui chevauchent une `best_window` ;
- rendre `watch_summary.status` coherent avec l'absence de vigilance.

Il ne doit pas completer la prose publique pour atteindre le minimum de mots.
Une sortie trop courte avec provider reel doit echouer au gate contractuel et
laisser le retry editor produire une nouvelle version.

## Verification

Commandes cible :

```powershell
cargo fmt
cargo test -p astral_llm_api --test horoscope_tests
cargo test -p astral_llm_api --test contracts_publish_tests
```

Smokes locaux utiles :

```powershell
.\scripts\test_horoscope_premium_next_7_days_fake.ps1
.\scripts\test_horoscope_premium_next_7_days_v2_openai.ps1
.\scripts\test_horoscope_premium_next_7_days_all.ps1
```
