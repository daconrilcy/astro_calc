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
- Aucun P0/P1 restant apres corrections.

## Corrections

- Test golden Premium ajoute pour calcul, interpretation, response schema et evidence.
- Goldens Premium corriges pour daily plans complets, watch days coherents, evidence de domaines distinctes et texte sans repetition excessive.
- Branche prompt Premium dediee ajoutee : fenetres sourcees, strategie, 3 a 5 domaines, limites 2200/3200 mots et interdiction d'inventer evidence/snapshots.
- Repair durci : il filtre les fenetres invalides mais ne cree plus de fenetre absente.
- Fake writer Premium durci : chaque texte quotidien peut recevoir un rappel explicite au theme natal, et le cas `watch_summary.status = none` / `watch_windows = []` est couvert.
- Schema provider adapte par service : Basic retire les champs Premium du schema OpenAI, Premium rend `best_windows`, `watch_windows` et `strategy` requis pour Structured Outputs.
- Repair texte durci : suppression/remplacement des fragments de phrase faibles et correction des espaces francais autour des deux-points.
- Prompt Premium et repair durcis contre la repetition : respect explicite des `avoid_terms`, variantes d'advice par jour et normalisation des phrases publiques repetees avant validation finale.
- UTC period canonise de bout en bout : le calculateur renvoie des champs `*_utc` en `+00:00`, et le scan plan LLM rejette les offsets non canoniques.
- Markers jour durcis : `key_days`, `best_days` et `watch_days` rejettent les dates dupliquees, et le builder `best_days` ne selectionne plus deux evenements de la meme date.
- Diversite Premium durcie : les daily plans penalisent un theme deja dominant quand des signaux alternatifs existent, et le theme public quotidien est aligne sur le daily plan.
- Tests Premium et non-regression ajoutes.
- Scripts period et Docker mis a jour.

## Statut

Closed - aucun P0/P1 ouvert.
