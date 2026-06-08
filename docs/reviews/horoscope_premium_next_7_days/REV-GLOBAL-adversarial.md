# REV-GLOBAL - Review adversariale globale

## Contexte

Review finale implementation hors documentation.

## Checks

- Pas de nouveau worker, endpoint, table jobs ou idempotence.
- Service route via `POST /v1/jobs`.
- `chart_calculation_id`, `anchor_date`, `timezone`, `target_language` obligatoires.
- `birth_data` inline refuse.
- Tests Rust critiques passent.

## Findings

- P1 : les goldens Premium existaient mais n'etaient pas valides par un test dedie.
- P1 : le prompt writer period restait trop generique pour garantir une vraie forme Premium.
- P1 : le repair pouvait masquer une reponse Premium incomplete en reinjectant des fenetres attendues.
- P1 : le fake writer Premium echouait sur un calcul period reel sans tension, car seules 3 entrees quotidiennes portaient une personnalisation natale reconnue par le guard.
- P1 : le schema envoye au provider reel etait incompatible Structured Outputs car les proprietes Premium racine n'etaient pas toutes dans `required`.
- P1 : le provider reel pouvait produire des fragments tronques ou une ponctuation francaise `:` non espacee que le repair ne corrigeait pas avant le guard final.
- P1 : le provider reel pouvait repeter des amorces editoriales dans plusieurs entrees quotidiennes et declencher `HOROSCOPE_PERIOD_REPETITIVE_DAILY_TEXT`.
- P1 : le dernier E2E reel exposait des champs `*_utc` en offset local `+02:00` dans la trace period au lieu de l'UTC canonique `+00:00`.
- P1 : `best_days` pouvait contenir deux entrees pour la meme date.
- P2 : la timeline Premium pouvait etre trop dominee par un seul theme public.
- V1.1 P1 : Premium retournait `watch_summary.status = none` dans des cas sans tension forte mais avec signaux exploitables.
- V1.1 P1 : les `best_windows` pouvaient rester trop generiques (`Fenetre favorable`, `best_for` peu differencies).
- V1.1 follow-up P1 : `best_for` variait par theme mais pas toujours par snapshot ; trois fenetres d'un meme theme pouvaient donc partager le meme usage conseille.
- V1.1 P1 : le texte public pouvait conserver des fragments francais casses comme `s'dynamique`.
- V1.1 real-run P2 : un premier E2E reel a produit l'adjectif maladroit `rédynamique` dans un domaine public.
- V1.1 real-run P2 : l'advice quotidien pouvait repeter `Hiérarchisez une priorité...` sur plusieurs jours d'organisation.
- V1.1 real-run P1 : un run reel a echoue sur `HOROSCOPE_PERIOD_BROKEN_SENTENCE` apres une phrase tronquee ou un debut de phrase minuscule non repare.
- V1.1 real-run P1 : la validation du payload provider s'executait avant le repair complet ; une phrase tronquee reparable pouvait donc echouer avant correction.
- V1.1 P2 : `premium_scores.domain_score` pouvait ressembler a un placeholder sature.
- V1.1 product note : un E2E reel valide peut retourner seulement deux `best_days`
  quand seules deux dates ressortent nettement ; `max_best_days = 3` est un
  plafond, pas un minimum a forcer.
- V1.1 product note : `watch_days = []` avec `watch_windows` non vide est valide
  en statut `low`, car `watch_days` designe seulement les journees de vigilance
  forte.
- Aucun P0/P1 restant apres corrections.

## Corrections

- Test golden Premium ajoute pour calcul, interpretation, response schema et evidence.
- Goldens Premium corriges pour daily plans complets, watch days coherents, evidence de domaines distinctes et texte sans repetition excessive.
- Branche prompt Premium dediee ajoutee : fenetres sourcees, strategie, 3 a 5 domaines, limites 2200/3200 mots et interdiction d'inventer evidence/snapshots.
- Repair durci : il filtre les fenetres invalides mais ne cree plus de fenetre absente.
- Fake writer Premium durci : chaque texte quotidien peut recevoir un rappel explicite au theme natal, et le cas V1.1 `watch_summary.status = low` / `watch_windows` evidencées est couvert quand aucun watch fort ne ressort.
- Schema provider adapte par service : Basic retire les champs Premium du schema OpenAI, Premium rend `best_windows`, `watch_windows` et `strategy` requis pour Structured Outputs.
- Repair texte durci : suppression/remplacement des fragments de phrase faibles et correction des espaces francais autour des deux-points.
- Prompt Premium et repair durcis contre la repetition : respect explicite des `avoid_terms`, variantes d'advice par jour et normalisation des phrases publiques repetees avant validation finale.
- UTC period canonise de bout en bout : le calculateur renvoie des champs `*_utc` en `+00:00`, et le scan plan LLM rejette les offsets non canoniques.
- Markers jour durcis : `key_days`, `best_days` et `watch_days` rejettent les dates dupliquees, et le builder `best_days` ne selectionne plus deux evenements de la meme date.
- Diversite Premium durcie : les daily plans penalisent un theme deja dominant quand des signaux alternatifs existent, et le theme public quotidien est aligne sur le daily plan.
- V1.1 vigilance douce : `watch_summary.status = low` et 1 a 3 `watch_windows` evidencées sont construits quand aucun watch fort n'existe mais que des signaux non-best exploitables restent disponibles.
- V1.1 fenetres : titres et `best_for` derives du theme/snapshot, guard `HOROSCOPE_PERIOD_PREMIUM_WINDOWS_TOO_GENERIC`, repair limite a la reformulation sans invention de sources ni evidence.
- V1.1 follow-up fenetres : `best_for` est maintenant differencie par theme et par heure locale, avec un test qui rejette trois ensembles `best_for` identiques meme si les titres divergent.
- V1.1 prose : repair des fragments francais casses, guard `HOROSCOPE_PERIOD_BROKEN_FRENCH_FRAGMENT`, limitation des amorces natales repetees.
- V1.1 real-run prose : `rédynamique` est réécrit en `dynamisante`, et l'amorce d'advice `Hiérarchisez une priorité` est intégrée au normaliseur anti-répétition.
- V1.1 real-run broken sentence : le repair capitalise maintenant les debuts de phrase apres `.`, `!`, `?` et retire les queues tronquees avec apostrophes typographiques.
- V1.1 real-run ordering : le repair de forme/texte s'applique maintenant avant la validation provider-public, puis la validation evidence/post-safety reste stricte apres normalisation.
- V1.1 scores : `premium_scores.domain_score` devient un score variable de couverture themes/evidence.
- V1.1 best days policy : la documentation precise que Premium retourne jusqu'a
  trois `best_days`, sans inventer ni forcer une troisieme date faible.
- V1.1 watch semantics : la documentation distingue `watch_days` pour les dates
  de vigilance forte et `watch_windows` pour les fenetres locales de vigilance
  douce en statut `low`.
- Tests Premium et non-regression ajoutes.
- Scripts period et Docker mis a jour.

## Statut

Closed - aucun P0/P1 ouvert.
