# Plan de refactorisation — Horoscope V1 slot-based

## Summary

Refactoriser l'horoscope daily V1 sans etendre le perimetre fonctionnel horoscope.
Le chantier porte uniquement sur la projection slot-based, le fake writer, les
guards qualite, les schemas et les goldens.

Objectif : passer d'une lecture globale decoupee artificiellement en trois textes
a trois moments reellement planifies, evidences, differencies et valides.

## Non-objectifs V1

Sont explicitement hors scope :

- Premium 2h slots ;
- ciel local du moment ;
- Ascendant / MC du moment ;
- maisons locales ;
- aspects mineurs ;
- nouveaux points astrologiques avances ;
- nouvelle infrastructure async ;
- nouvelle table de jobs ;
- nouveau worker ;
- correction par prompt d'un defaut de payload.

Le chantier doit corriger la projection, la differenciation par slot, le fake
writer, les guards, les schemas, les goldens et la documentation.

## Baseline

Avant toute modification fonctionnelle, capturer :

- le texte final produit ;
- le `horoscope_interpretation_request` envoye au fake writer ou au LLM ;
- le `horoscope_response` final.

But : distinguer clairement les defauts de matiere, de projection et de
redaction.

Commandes de baseline :

```powershell
cargo test -p astral_llm_api --test horoscope_v1_tests
.\scripts\test_horoscope_basic_daily_fake.ps1
```

Si le script est conserve :

```powershell
.\scripts\show_real_horoscope_text.ps1
```

Ne pas ecraser les changements existants :

- `docs/BASIC_PAYLOAD_IMPLEMENTATION.md` deja modifie ;
- `scripts/show_real_horoscope_text.ps1` non suivi.

## Implementation Changes

- Ajouter des tests rouges avant correction ; ils doivent demontrer les defauts
  actuels et peuvent etre commites ou isoles dans une premiere passe.
- Introduire `SlotInterpretationPlan` comme unite centrale de projection par slot.
- Dans `horoscope_interpretation_request`, `slots[]` devient la source
  obligatoire de redaction pour `horoscope_basic_daily_natal_3_slots`.
- `main_signals` et `dominant_themes` peuvent rester pour compatibilite interne,
  mais ne doivent plus piloter directement la redaction des trois moments.
- `day_overview` sert uniquement a introduire la tonalite generale de la journee
  et ne doit pas etre recopie dans chaque slot.
- Verifier les referentiels avant code : `horoscope_theme_advice_axes`,
  `horoscope_signal_theme_mappings`, `horoscope_shortlist_profiles`,
  `horoscope_time_slot_profiles`.
- Le fake writer doit produire trois slots distincts, stables, accentues et
  evidences.
- Ajouter des guards qualite : repetition, copie de `day_overview` dans les
  slots, evidence manquante, langage generique, reference astrologique absente,
  conseil duplique, `best_for` duplique, typographie FR, fuite de code technique.
- Mettre a jour les prompts seulement apres la shortlist par slot et le fake
  writer.

## Slot and Evidence Rules

Valeurs strictes de `specificity` :

- `specific` : le slot possede au moins une evidence propre.
- `shared` : le slot utilise un signal partage avec un autre slot, mais avec un
  role ou une intensite distincte.
- `fallback` : aucun signal propre exploitable ; le slot doit porter
  `fallback_reason`.

Regles bloquantes :

- La valeur par defaut attendue est `specific` si une evidence propre existe,
  `fallback` si aucune evidence propre n'existe.
- `specificity = shared` ne doit etre utilise que lorsqu'un meme signal couvre
  reellement plusieurs slots.
- Si `specificity = shared`, le slot doit avoir au moins un differenciant
  mesurable : `tone`, `intensity`, `advice_axis`, `watch_point`, `best_for` ou
  `slot_specific_wording_hint`.
- `specificity = fallback` implique `required_evidence_keys` vide et
  `fallback_reason` non vide. Toute exception doit etre refusee en V1.
- Chaque `required_evidence_key` presente dans un slot doit exister dans
  `evidence[]`.
- Chaque evidence utilisee par le texte final doit etre referencee par le slot
  correspondant.
- Une evidence d'un autre slot ne peut pas etre utilisee sauf si elle est
  explicitement partagee avec `specificity = shared`.
- Les trois slots ne doivent pas partager des `best_for` identiques si les
  signaux permettent de les differencier.
- Les labels publics doivent venir de `slot_label` ou d'un referentiel localise.
- Le renderer public ne doit jamais concatener `title + slot_code`.
- Le post-process typographique ne doit pas faire de correction lexicale naive
  susceptible d'alterer le sens.

## Payload Limits

`horoscope_interpretation_request` doit imposer des limites mesurables :

- `slots.length = 3` pour `horoscope_basic_daily_natal_3_slots` ;
- `main_signals` par slot limite par le referentiel ;
- `required_evidence_keys` par slot limite par le referentiel ;
- `evidence` total limite par le referentiel ;
- aucun champ `raw_transits`, `all_transits`, `debug_aspects` hors mode debug
  explicitement interdit en production.

## Compatibility and Migration

Le refactor ne doit pas casser l'infrastructure async existante :

- meme endpoint `POST /v1/jobs` ;
- meme logique de polling ;
- meme semantique d'idempotence ;
- meme `service_code` horoscope ;
- pas de nouvelle table de jobs ;
- pas de nouveau worker.

Pendant le refactor, le service doit rester en `planned` ou `beta` controle. Il
ne doit pas passer en `active` tant que les tests fake, les goldens, les guards
qualite et les reviews adversariales ne sont pas clos.

Les changements de `horoscope_interpretation_request` sont internes au
workflow horoscope.

Les changements de `horoscope_response` doivent rester compatibles si le
contrat est deja consomme. Toute suppression, renommage ou modification de type
dans la reponse publique impose un bump de version ou une note de compatibilite
explicite.

## Target Output Level

Exemple minimal de niveau attendu, a adapter aux vraies evidences :

```markdown
## Matin

La Lune met l'accent sur l'organisation et les gestes utiles. C'est un bon
moment pour clarifier une priorite concrete plutot que d'ouvrir trop de sujets.

Conseil : choisissez une action verifiable et terminez-la avant de passer a la
suivante.
```

Le rendu cible doit viser la qualite produit, pas seulement la conformite JSON.

## Test Plan

Tests de non-regression a couvrir dans `tests/` :

- `horoscope_interpretation_request_contains_slot_shortlists`
- `horoscope_each_slot_has_required_evidence`
- `horoscope_rejects_repeated_slot_bodies`
- `horoscope_rejects_day_overview_copied_into_slots`
- `horoscope_rejects_generic_signal_wording`
- `horoscope_rejects_public_slot_codes_in_markdown`
- `horoscope_applies_french_typography`
- `horoscope_requires_distinct_advice_axes`
- `horoscope_fake_writer_uses_slot_specific_evidence`
- `horoscope_response_quality_flags_are_set`
- `horoscope_slot_without_evidence_requires_fallback_reason`
- `horoscope_interpretation_request_does_not_contain_raw_transit_dump`

Validation :

```powershell
cargo test -p astral_llm_api --test horoscope_v1_tests
.\scripts\test_horoscope_basic_daily_fake.ps1
```

## Execution Order

1. Tests rouges commitables separement ou isoles en premiere passe.
2. Refactor structurel sans changement comportemental.
3. `SlotInterpretationPlan` et shortlist par slot.
4. Referentiels, schemas et goldens.
5. Fake writer differencie.
6. Guards qualite.
7. Prompts.
8. Smoke fake.
9. Reviews finales.

Ne pas essayer de resoudre par le prompt ce qui doit etre resolu par le payload.

## Acceptance Criteria

- Les trois slots ont des themes, tons, conseils, `best_for` et preuves
  distincts quand les signaux le permettent.
- `horoscope_interpretation_request.slots[]` pilote la redaction des trois
  moments.
- `day_overview` n'est pas recopie comme texte de slot.
- Une reponse repetitive echoue.
- Une reponse sans reference astrologique vulgarisee par slot echoue, sauf slot
  fallback explicite.
- Une reponse contenant `[morning]`, `[afternoon]`, `[evening]` ou tout autre
  code technique dans le rendu public echoue.
- Le rendu markdown public est genere depuis les labels publics, jamais depuis
  les codes techniques.
- Un slot sans evidence porte `specificity = fallback`, `required_evidence_keys`
  vide et un `fallback_reason`.
- Un slot `shared` possede au moins un differenciant mesurable.
- Chaque `required_evidence_key` de slot existe dans `evidence[]`.
- Une evidence d'un autre slot n'est jamais utilisee sans `specificity = shared`.
- Le fake writer produit un francais accentue, lisible, stable et differencie.
- Le fake writer utilise les evidence specifiques a chaque slot.
- Le payload LLM ne contient pas de dump brut non filtre et respecte les limites
  de taille.
- Les contrats JSON et goldens sont a jour.
- Les reviews adversariales sont documentees.
- Le service ne passe pas en `active` avant cloture des tests fake, goldens,
  guards qualite et reviews adversariales.
- Les tests horoscope et le smoke fake passent.
