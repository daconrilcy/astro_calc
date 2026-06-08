# Fonctions de formatage et amelioration des textes LLM - services horoscope

Inventaire realise le 2026-06-08 sur les services horoscope et les utilitaires texte associes.

## Services horoscope couverts

- `horoscope_free_daily`
- `horoscope_basic_daily_natal_3_slots`
- `horoscope_premium_daily_local_2h_slots`
- `horoscope_free_next_7_days_natal`
- `horoscope_basic_next_7_days_natal`
- `horoscope_premium_next_7_days_natal`

## Horoscope periode - reparation et sanitation de reponse LLM

Fichier: `astral_llm/crates/astral_llm_application/src/horoscope/mod.rs`

| Fonction | Ligne | Role texte |
| --- | ---: | --- |
| `repair_period_response_shape` | 2306 | Reconstruit la forme publique periode, sanitise tous les champs textuels, complete les mots manquants, reduit les repetitions. |
| `repair_free_period_response_shape` | 2372 | Repare la variante Free 7 jours, limite les sections exposees et sanitise summary/advice/watch. |
| `sanitize_free_period_summary` | 2463 | Sanitise `summary.title` et `summary.text`, avec fallback public compact. |
| `sanitize_free_period_dominant_theme` | 2470 | Sanitise le theme dominant Free et injecte un texte fallback. |
| `sanitize_free_period_watch_summary` | 2481 | Sanitise le texte de vigilance Free et normalise les preuves exposees. |
| `sanitize_period_week_overview` | 2542 | Sanitise title/text/trajectory et ajoute une personnalisation si absente. |
| `sanitize_period_advice` | 2555 | Sanitise les conseils `main`, `best_use`, `avoid`. |
| `sanitize_period_watch_summary` | 2563 | Sanitise le resume de vigilance periodique. |
| `sanitize_period_markers` | 2583 | Sanitise les marqueurs de jours et leurs raisons publiques. |
| `sanitize_period_daily_timeline` | 2629 | Sanitise les textes journaliers et ajoute une phrase interpretative si necessaire. |
| `sanitize_period_domain_sections` | 2663 | Sanitise les sections domaine et enrichit la personnalisation. |
| `sanitize_period_windows` | 2698 | Filtre les fenetres conservees selon les fallbacks autorises. |
| `sanitize_period_window_from_fallback` | 2738 | Sanitise les libelles, titres, raisons et watch points de fenetres. |
| `sanitize_period_strategy` | 2769 | Sanitise la strategie Premium de semaine. |
| `sanitize_period_evidence_summary` | 3137 | Sanitise les libelles publics des preuves affichees. |

## Horoscope periode - generation de textes publics deterministes

Fichier: `astral_llm/crates/astral_llm_application/src/horoscope/mod.rs`

| Fonction | Ligne | Role texte |
| --- | ---: | --- |
| `ensure_period_personalization_text` | 2780 | Ajoute une phrase de personnalisation si le texte n'en contient pas. |
| `period_public_day_text` | 2789 | Genere un texte public journalier fallback selon le style. |
| `period_public_day_advice` | 2830 | Genere un conseil journalier fallback. |
| `period_daily_advice_expansion` | 2842 | Fournit des phrases d'expansion pour atteindre la longueur minimale. |
| `period_public_domain_text` | 2854 | Genere le texte public d'une section domaine. |
| `period_public_personalization_sentence` | 2866 | Produit une phrase de personnalisation pour un jour. |
| `period_public_interpretive_sentence` | 2870 | Produit une phrase interpretative de secours pour un jour. |
| `period_public_domain_personalization_sentence` | 2875 | Produit une phrase de personnalisation pour un domaine. |
| `period_public_domain_interpretive_sentence` | 2879 | Produit une phrase interpretative de secours pour un domaine. |
| `period_public_focus_text` | 2897 | Produit un focus public a partir des donnees internes. |
| `period_public_focus_from_hint` | 2913 | Transforme un hint interne en formulation publique. |
| `period_domain_focus` | 5477 | Genere un texte de focus domaine avec personnalisation. |
| `period_event_personalization_hint` | 5599 | Fournit un hint redactionnel par theme d'evenement. |
| `period_advice_hint` | 5613 | Compose un conseil public a partir du theme et du focus natal. |

## Horoscope periode - nettoyage typographique, repetition et longueur

Fichier: `astral_llm/crates/astral_llm_application/src/horoscope/mod.rs`

| Fonction | Ligne | Role texte |
| --- | ---: | --- |
| `sanitize_period_public_string` | 2941 | Nettoyage central des chaines publiques periode: fragments, bornes de phrase, ponctuation, substitutions. |
| `sanitize_period_french_fragments` | 2989 | Corrige des fragments francais recurrents. |
| `sanitize_period_broken_sentences` | 3000 | Corrige les phrases cassees ou terminaisons faibles. |
| `sanitize_period_sentence_boundaries` | 3037 | Normalise les frontieres de phrases. |
| `sanitize_period_french_colon_spacing` | 3060 | Corrige l'espacement autour des deux-points en francais. |
| `replace_ascii_case_insensitive` | 3088 | Remplacement insensible a la casse pour texte ASCII. |
| `replace_ascii_token_case_insensitive` | 3106 | Remplacement tokenise insensible a la casse. |
| `ensure_period_response_minimum_words` | 3169 | Complete ou reduit une reponse pour respecter les bornes de longueur. |
| `trim_period_response_to_hard_limit` | 3257 | Compacte la reponse si elle depasse la limite dure. |
| `trim_period_response_aggressively` | 3333 | Compactage fort des textes publics. |
| `fill_period_response_to_minimum` | 3416 | Ajoute des phrases pour atteindre le minimum de mots. |
| `normalize_period_week_overview_repetition` | 3458 | Supprime les repetitions specifiques dans la vue d'ensemble. |
| `normalize_period_repetitive_public_phrases` | 3503 | Lance la normalisation globale des phrases repetees. |
| `normalize_period_repetitive_value` | 3508 | Parcourt recursivement la reponse pour traiter les textes. |
| `normalize_period_repetitive_text` | 3552 | Remplace les occurrences repetitives dans une chaine. |
| `replace_period_phrase_after_allowed` | 3563 | Remplace une phrase apres un nombre d'occurrences autorise. |
| `period_repetitive_phrase_replacements` | 3594 | Catalogue local de substitutions anti-repetition. |
| `replace_period_phrase_all` | 3640 | Remplace toutes les occurrences d'une phrase. |
| `replace_period_phrase_after_first` | 3654 | Remplace les occurrences apres la premiere. |
| `compact_period_words` | 3675 | Tronque a un nombre de mots en preservant les phrases completes. |
| `period_complete_sentences` | 3702 | Extrait les phrases terminees. |
| `period_trim_incomplete_tail` | 3718 | Nettoie une fin de texte incomplete et ajoute une ponctuation finale. |
| `period_is_weak_sentence_ending` | 3739 | Detecte les mots faibles en fin de phrase tronquee. |
| `append_period_value_sentence` | 3774 | Ajoute une phrase a un champ JSON texte. |
| `append_period_sentence` | 3782 | Ajoute une phrase a une chaine sans doublonner. |

## Horoscope periode - humanisation de libelles publics

Fichier: `astral_llm/crates/astral_llm_application/src/horoscope/mod.rs`

| Fonction | Ligne | Role texte |
| --- | ---: | --- |
| `period_theme_public_label` | 5452 | Convertit un theme code en libelle public francais. |
| `period_domain_title` | 5465 | Convertit un theme code en titre de section. |
| `period_natal_focus` | 5510 | Fournit le libelle/hint de focus natal. |
| `period_natal_focus_labels` | 5521 | Charge les libelles de focus natal depuis `json_db/horoscope_natal_focus_labels.json`. |
| `period_style_variant_for_theme` | 5553 | Choisit la variante de style redactionnel d'un theme. |
| `period_style_variants` | 5572 | Charge les variantes de style depuis `json_db/horoscope_period_style_variants.json`. |
| `period_tone_public_label` | 5628 | Convertit un tone code en libelle public. |
| `period_tone_public_label_if_code` | 5635 | Convertit seulement si la valeur est un code connu. |
| `period_tone_labels` | 5647 | Charge les libelles de ton depuis `json_db/horoscope_tone_labels.json`. |
| `normalize_period_public_tones` | 5670 | Remplace les tons techniques du provider par les libelles publics attendus. |
| `period_object_public_label` | 5876 | Convertit les objets astrologiques en libelles publics. |
| `public_day_label` | 5889 | Formate une date en libelle jour francais. |

## Horoscope quotidien - fallbacks et rendus fake

Fichier: `astral_llm/crates/astral_llm_application/src/horoscope/mod.rs`

| Fonction | Ligne | Role texte |
| --- | ---: | --- |
| `fake_writer_response` | 6210 | Genere une reponse quotidienne Basic fake avec textes publics complets. |
| `fake_writer_premium_response` | 6313 | Genere une reponse Premium daily fake avec summary, timeline, conseils et sections. |
| `render_fake_premium_timeline_slot` | 6392 | Rend un slot timeline Premium fake. |
| `premium_timeline_title` | 6421 | Fournit les titres de slots Premium fake. |
| `premium_timeline_theme` | 6430 | Fournit les themes de slots Premium fake. |
| `premium_timeline_text` | 6439 | Fournit les textes de slots Premium fake. |
| `premium_timeline_advice` | 6448 | Fournit les conseils de slots Premium fake. |
| `fake_writer_free_response` | 6641 | Genere une reponse Free daily fake compacte. |
| `render_fake_slot` | 6859 | Rend les slots Basic fake avec theme, texte, conseil et watch point. |
| `slot_label` | 7930 | Fournit les libelles publics de slots daily. |

## Horoscope - validation qualite du texte public

Fichier: `astral_llm/crates/astral_llm_application/src/horoscope/mod.rs`

| Fonction | Ligne | Role texte |
| --- | ---: | --- |
| `validate_public_slot_text` | 7467 | Verifie les textes de slot: codes techniques, generique, typographie francaise. |
| `validate_public_text_no_technical_codes` | 7522 | Rejette les fuites de codes techniques dans le texte public. |
| `validate_free_text_quality` | 7556 | Verifie qualite Free daily: champs obligatoires, longueur, generique, typographie. |
| `validate_slot_diversity` | 7667 | Detecte les slots trop similaires. |
| `meaningful_words` | 7727 | Extrait les mots significatifs pour comparer les textes. |
| `first_words` | 7741 | Extrait les premiers mots normalises pour detection de similarite. |
| `normalized_text` | 7749 | Normalise un texte pour comparaison qualite. |
| `free_public_text` | 7851 | Agrege les champs publics Free daily pour validation. |

## Utilitaires texte partages utilises ou pertinents pour les rendus LLM

### Typographie francaise

Fichier: `astral_llm/crates/astral_llm_application/src/french_typography.rs`

| Fonction | Ligne | Role texte |
| --- | ---: | --- |
| `restore_french_elisions` | 17 | Restaure les apostrophes d'elision francaises apres generation LLM. |
| `french_elision_violations` | 34 | Detecte les elisions manquantes; utilise par les validations horoscope. |

### Garde script et sanitation alphabet

Fichier: `astral_llm/crates/astral_llm_application/src/reading_script_guard.rs`

| Fonction | Ligne | Role texte |
| --- | ---: | --- |
| `sanitize_text_for_french_script` | 86 | Supprime les caracteres hors alphabet francais autorise. |
| `collapse_whitespace` | 108 | Recompacte les espaces apres sanitation. |

### Repetition et amorces

Fichier: `astral_llm/crates/astral_llm_application/src/text_trigrams.rs`

| Fonction | Ligne | Role texte |
| --- | ---: | --- |
| `trigram_phrases` | 16 | Extrait des trigrammes pour detection de repetition. |
| `is_low_signal_trigram` | 34 | Ignore les trigrammes grammaticalement faibles. |
| `count_repeated_trigrams` | 47 | Compte les repetitions significatives. |
| `chapter_opening_phrase` | 132 | Normalise l'ouverture d'un chapitre. |
| `paragraph_opening_phrases` | 143 | Normalise les amorces de paragraphes. |
| `openings_to_avoid_from_prior` | 157 | Liste les amorces deja vues a eviter dans un prompt suivant. |
| `detect_duplicate_openings` | 178 | Detecte les ouvertures de chapitres/paragraphes dupliquees. |
| `phrases_to_avoid_from_prior` | 233 | Produit des phrases a eviter a partir des chapitres precedents. |
| `normalize_paragraph_for_placement_check` | 256 | Normalise un paragraphe pour detecter les amorces techniques. |
| `paragraph_starts_with_raw_placement` | 277 | Detecte une amorce brute du type planete en signe. |
| `soften_raw_placement_openings_in_body` | 315 | Ajoute un prefixe neutre pour eviter une ouverture trop technique. |
| `detect_raw_placement_paragraph_openings` | 332 | Liste les ouvertures brutes restantes. |

### Resume compact UX

Fichier: `astral_llm/crates/astral_llm_application/src/summary_ux_rules.rs`

| Fonction | Ligne | Role texte |
| --- | ---: | --- |
| `count_words` | 24 | Compte les mots pour les contraintes UX. |
| `count_sentences_fr` | 31 | Compte les phrases francaises. |
| `split_sentences_fr` | 70 | Decoupe un texte en phrases francaises. |
| `validate_summary_ux` | 115 | Valide longueur de titre, nombre de phrases et longueur de resume. |

### Post-traitement natal simplifie partage par le pipeline LLM

Ces fonctions ne sont pas specifiques aux services horoscope, mais elles visent explicitement a nettoyer, compacter ou remplacer des textes LLM.

Fichier: `astral_llm/crates/astral_llm_application/src/simplified_reading_postprocess.rs`

| Fonction | Ligne | Role texte |
| --- | ---: | --- |
| `post_process_single_pass_reading` | 52 | Applique sanitation script, typographie, resume compact et hardening. |
| `apply_simplified_body_fallback` | 88 | Remplace/cree un corps de chapitre deterministe. |
| `build_simplified_summary` | 109 | Reconstruit le resume a partir du premier chapitre. |
| `build_compact_summary_from_body` | 124 | Extrait un resume court en phrases completes. |
| `trim_to_complete_sentence` | 156 | Coupe un texte a un nombre de mots avec ponctuation finale. |
| `simplified_summary_title` | 172 | Titre fallback localise. |
| `simplified_summary_short_text` | 180 | Texte fallback localise. |
| `simplified_deterministic_body` | 189 | Corps fallback deterministe. |
| `harden_ambiguous_core_identity_chapter` | 206 | Corrige un chapitre ambigu: confiance, basis, prefixe d'incertitude. |
| `body_has_ambiguous_uncertainty_lexicon` | 252 | Detecte le lexique d'incertitude deja present. |
| `ambiguous_uncertainty_prefix_sentence` | 259 | Produit la phrase de prefixe d'incertitude. |
| `normalize_simplified_interpretive_roles` | 270 | Normalise les roles interpretatifs exposes. |
| `sanitize_reading_text_fields` | 285 | Sanitise summary, chapitres et astro_basis. |
| `restore_french_typography_fields` | 326 | Restaure les elisions dans summary, chapitres et astro_basis. |
| `sanitize_field` | 370 | Applique la sanitation script a un champ. |
| `typography_field` | 379 | Applique la correction typographique a un champ. |

## Matrice d'applicabilite par fonction

Legende services:

- `Horoscope daily`: services horoscope quotidiens Free, Basic, Premium.
- `Horoscope period`: services horoscope 7 jours Free, Basic, Premium.
- `Theme natal`: generation multi-chapitres `natal_prompter`.
- `Natal simplifie`: profil `natal_simplified`.
- `LLM shared`: utilitaire transversal, reusable par plusieurs produits.

### Horoscope period

| Fonction | Services applicables | Langues applicables | Doublon / recouvrement |
| --- | --- | --- | --- |
| `repair_period_response_shape` | Horoscope period | fr | Non. Orchestrateur central periode. |
| `repair_free_period_response_shape` | Horoscope period Free | fr | Recouvre `repair_period_response_shape` pour la branche Free. |
| `sanitize_free_period_summary` | Horoscope period Free | fr | Recouvre `sanitize_period_week_overview` sur le role "resume". |
| `sanitize_free_period_dominant_theme` | Horoscope period Free | fr | Recouvre les fonctions de libelles theme (`period_theme_public_label`). |
| `sanitize_free_period_watch_summary` | Horoscope period Free | fr | Recouvre `sanitize_period_watch_summary`, variante Free. |
| `sanitize_period_week_overview` | Horoscope period Basic, Premium | fr | Recouvre les fonctions de fallback/longueur sur overview. |
| `sanitize_period_advice` | Horoscope period Basic, Premium | fr | Recouvre les fonctions de generation de conseils fallback. |
| `sanitize_period_watch_summary` | Horoscope period Basic, Premium | fr | Recouvre `sanitize_free_period_watch_summary`. |
| `sanitize_period_markers` | Horoscope period | fr | Non, special markers jours. |
| `sanitize_period_daily_timeline` | Horoscope period Basic, Premium | fr | Recouvre `period_public_day_text` et `period_public_day_advice` comme fallbacks. |
| `sanitize_period_domain_sections` | Horoscope period Basic, Premium | fr | Recouvre `period_public_domain_text` comme fallback. |
| `sanitize_period_windows` | Horoscope period Premium | fr | Non, filtre structurel de fenetres. |
| `sanitize_period_window_from_fallback` | Horoscope period Premium | fr | Recouvre les fonctions de sanitation champ par champ. |
| `sanitize_period_strategy` | Horoscope period Premium | fr | Recouvre `period_advice_hint` sur le role conseil/strategie. |
| `ensure_period_personalization_text` | Horoscope period Basic, Premium | fr | Recouvre les phrases `period_public_*personalization_sentence`. |
| `period_public_day_text` | Horoscope period Basic, Premium | fr | Recouvre les textes fake/fallback de slots, mais pour periode. |
| `period_public_day_advice` | Horoscope period Basic, Premium | fr | Recouvre `period_daily_advice_expansion` et `period_advice_hint`. |
| `period_daily_advice_expansion` | Horoscope period Basic, Premium | fr | Recouvre `append_period_sentence` pour combler la longueur. |
| `period_public_domain_text` | Horoscope period Basic, Premium | fr | Recouvre `period_domain_focus`. |
| `period_public_personalization_sentence` | Horoscope period Basic, Premium | fr | Recouvre `period_public_interpretive_sentence`. |
| `period_public_interpretive_sentence` | Horoscope period Basic, Premium | fr | Recouvre `period_public_personalization_sentence`. |
| `period_public_domain_personalization_sentence` | Horoscope period Basic, Premium | fr | Recouvre `period_public_domain_interpretive_sentence`. |
| `period_public_domain_interpretive_sentence` | Horoscope period Basic, Premium | fr | Recouvre `period_public_domain_personalization_sentence`. |
| `period_public_focus_text` | Horoscope period | fr | Recouvre `period_public_focus_from_hint`. |
| `period_public_focus_from_hint` | Horoscope period | fr | Recouvre `period_public_focus_text`, sous-cas texte brut. |
| `sanitize_period_public_string` | Horoscope period | fr | Recouvre `sanitize_text_for_french_script` et `restore_french_elisions`, mais avec corrections periode specifiques. |
| `sanitize_period_french_fragments` | Horoscope period | fr | Recouvre partiellement `sanitize_period_broken_sentences`. |
| `sanitize_period_broken_sentences` | Horoscope period | fr | Recouvre `period_trim_incomplete_tail`. |
| `sanitize_period_sentence_boundaries` | Horoscope period | fr | Recouvre `split_sentences_fr` / `period_complete_sentences`. |
| `sanitize_period_french_colon_spacing` | Horoscope period | fr | Recouvre la typographie francaise mais uniquement deux-points. |
| `replace_ascii_case_insensitive` | Horoscope period | fr,en,es,de | Doublon technique avec les autres remplacements ASCII. |
| `replace_ascii_token_case_insensitive` | Horoscope period | fr,en,es,de | Doublon technique tokenise de `replace_ascii_case_insensitive`. |
| `sanitize_period_evidence_summary` | Horoscope period | fr | Recouvre `period_object_public_label` / humanisation labels. |
| `ensure_period_response_minimum_words` | Horoscope period Basic, Premium | fr | Recouvre `fill_period_response_to_minimum` et trim functions. |
| `trim_period_response_to_hard_limit` | Horoscope period Basic, Premium | fr | Recouvre `trim_period_response_aggressively`, version moderee. |
| `trim_period_response_aggressively` | Horoscope period Basic, Premium | fr | Recouvre `trim_period_response_to_hard_limit`, version forte. |
| `fill_period_response_to_minimum` | Horoscope period Basic, Premium | fr | Recouvre `ensure_period_response_minimum_words`, sous-etape. |
| `normalize_period_week_overview_repetition` | Horoscope period Basic, Premium | fr | Recouvre `normalize_period_repetitive_public_phrases`, cas specifique overview. |
| `normalize_period_repetitive_public_phrases` | Horoscope period | fr | Recouvre `text_trigrams` sur l'objectif anti-repetition, mais en correction directe. |
| `normalize_period_repetitive_value` | Horoscope period | fr | Recouvre `normalize_period_repetitive_text`, parcours recursif. |
| `normalize_period_repetitive_text` | Horoscope period | fr | Recouvre `replace_period_phrase_after_allowed`. |
| `replace_period_phrase_after_allowed` | Horoscope period | fr | Recouvre `replace_period_phrase_after_first`, avec compteur global. |
| `period_repetitive_phrase_replacements` | Horoscope period | fr | Catalogue de substitutions; recouvre le role des listes `STOCK_OPENINGS_FR`. |
| `replace_period_phrase_all` | Horoscope period | fr,en,es,de | Doublon technique de remplacement. |
| `replace_period_phrase_after_first` | Horoscope period | fr,en,es,de | Doublon technique de remplacement apres seuil. |
| `compact_period_words` | Horoscope period | fr,en,es,de | Recouvre `truncate_words` et `trim_to_complete_sentence`. |
| `period_complete_sentences` | Horoscope period | fr,en,es,de | Recouvre `split_sentences_fr`. |
| `period_trim_incomplete_tail` | Horoscope period | fr,en,es,de | Recouvre `trim_to_complete_sentence`. |
| `period_is_weak_sentence_ending` | Horoscope period | fr | Non, liste grammaticale francaise. |
| `append_period_value_sentence` | Horoscope period | fr,en,es,de | Recouvre `append_period_sentence`, wrapper JSON. |
| `append_period_sentence` | Horoscope period | fr,en,es,de | Non, operation texte primitive. |
| `period_theme_public_label` | Horoscope period | fr | Recouvre `period_domain_title`, niveau libelle. |
| `period_domain_title` | Horoscope period | fr | Recouvre `period_theme_public_label`, niveau titre. |
| `period_domain_focus` | Horoscope period | fr | Recouvre `period_public_domain_text`. |
| `period_natal_focus` | Horoscope period | fr | Recouvre `period_natal_focus_labels`, resolution avec fallback. |
| `period_natal_focus_labels` | Horoscope period | fr | Source DB/JSON des libelles de focus. |
| `period_style_variant_for_theme` | Horoscope period | fr | Recouvre `period_style_variants`, resolution avec fallback. |
| `period_style_variants` | Horoscope period | fr | Source DB/JSON des variantes de style. |
| `period_event_personalization_hint` | Horoscope period | fr | Recouvre les phrases de personnalisation publiques. |
| `period_advice_hint` | Horoscope period | fr | Recouvre `period_public_day_advice`. |
| `period_tone_public_label` | Horoscope period | fr | Recouvre `period_tone_public_label_if_code`. |
| `period_tone_public_label_if_code` | Horoscope period | fr | Recouvre `period_tone_public_label`, garde la valeur inconnue. |
| `period_tone_labels` | Horoscope period | fr | Source DB/JSON des tons publics. |
| `normalize_period_public_tones` | Horoscope period Basic, Premium | fr | Recouvre les fonctions de libelle tone, application structurelle. |
| `period_object_public_label` | Horoscope period | fr | Recouvre `AstroLabelHumanizer::object_label`, mais hardcode horoscope. |
| `public_day_label` | Horoscope period | fr | Non, formatage date/jour francais. |

### Horoscope daily

| Fonction | Services applicables | Langues applicables | Doublon / recouvrement |
| --- | --- | --- | --- |
| `fake_writer_response` | Horoscope daily Basic | fr | Recouvre `render_fake_slot`, orchestrateur fake. |
| `fake_writer_premium_response` | Horoscope daily Premium | fr | Recouvre `render_fake_premium_timeline_slot`, orchestrateur fake. |
| `render_fake_premium_timeline_slot` | Horoscope daily Premium | fr | Recouvre `premium_timeline_*`. |
| `premium_timeline_title` | Horoscope daily Premium | fr | Recouvre hardcoded text fallback. |
| `premium_timeline_theme` | Horoscope daily Premium | fr | Recouvre hardcoded text fallback. |
| `premium_timeline_text` | Horoscope daily Premium | fr | Recouvre hardcoded text fallback. |
| `premium_timeline_advice` | Horoscope daily Premium | fr | Recouvre hardcoded text fallback. |
| `fake_writer_free_response` | Horoscope daily Free | fr | Recouvre les fallbacks Free period en intention, pas en schema. |
| `render_fake_slot` | Horoscope daily Basic | fr | Recouvre `period_public_day_text` pour le daily. |
| `validate_public_slot_text` | Horoscope daily Basic, Premium | fr | Recouvre `validate_free_text_quality` sur certains checks. |
| `validate_public_text_no_technical_codes` | Horoscope daily | fr,en,es,de | Recouvre plusieurs validations anti-fuite technique. |
| `validate_free_text_quality` | Horoscope daily Free | fr | Recouvre `validate_public_slot_text` pour Free. |
| `validate_slot_diversity` | Horoscope daily Basic, Premium | fr,en,es,de | Recouvre `detect_duplicate_openings` / trigrammes en objectif qualite. |
| `meaningful_words` | Horoscope daily | fr,en,es,de | Recouvre `normalized_text`, extraction pour comparaison. |
| `first_words` | Horoscope daily | fr,en,es,de | Recouvre `chapter_opening_phrase`, version courte. |
| `normalized_text` | Horoscope daily | fr,en,es,de | Recouvre plusieurs normalisations locales. |
| `free_public_text` | Horoscope daily Free | fr,en,es,de | Non, aggregation de champs pour validation. |
| `slot_label` | Horoscope daily | fr | Recouvre `ReferenceData::slot_label` et libelles DB. |

### Theme natal et astro basis

Fichier: `astral_llm/crates/astral_llm_application/src/astro_label_humanizer.rs`

| Fonction | Ligne | Services applicables | Langues applicables | Doublon / recouvrement |
| --- | ---: | --- | --- | --- |
| `AstroLabelHumanizer::new` | 15 | Theme natal, Natal simplifie, LLM shared | fr,en,es,de | Non, constructeur. |
| `AstroLabelHumanizer::locale_key` | 19 | Theme natal, Natal simplifie, LLM shared | fr,en,es,de | Recouvre plusieurs resolutions locale locales. |
| `AstroLabelHumanizer::object_label` | 29 | Theme natal, Natal simplifie, LLM shared | fr,en,es,de | Recouvre `period_object_public_label`, mais catalogue canonique. |
| `AstroLabelHumanizer::sign_label` | 36 | Theme natal, Natal simplifie, LLM shared | fr,en,es,de | Recouvre `title_case_token` fallback. |
| `AstroLabelHumanizer::placement_label` | 43 | Theme natal, Natal simplifie | fr,en,es,de | Recouvre plusieurs humanisations placement. |
| `AstroLabelHumanizer::humanize_fact_label` | 64 | Theme natal, Natal simplifie | fr,en,es,de | Orchestrateur; recouvre helpers `humanize_*` et `label_from_fact_id`. |
| `AstroLabelHumanizer::label_for_fact_id` | 107 | Theme natal, Natal simplifie | fr,en,es,de | Recouvre `label_from_fact_id`, wrapper public. |
| `AstroLabelHumanizer::natal_planet_display_names` | 116 | Theme natal, Natal simplifie | fr,en,es,de | Recouvre `object_label` sur planetes natales. |
| `AstroLabelHumanizer::interpretive_hint_for_fact_id` | 127 | Theme natal, Natal simplifie | fr,en,es,de | Recouvre `interpretive_hint_from_fact_id`, wrapper public. |
| `AstroLabelHumanizer::enrich_chapter_astro_basis` | 136 | Theme natal, Natal simplifie | fr,en,es,de | Recouvre `humanize_fact_label`, application aux sorties LLM. |
| `humanize_signal_fact_impl` | 161 | Theme natal, Natal simplifie | fr,en,es,de | Orchestrateur signal; recouvre helpers signal. |
| `AstroLabelHumanizer::humanize_signal_fact` | 186 | Theme natal, Natal simplifie | fr,en,es,de | Wrapper de `humanize_signal_fact_impl`. |
| `normalize_sign_code` | 196 | Theme natal, Natal simplifie | fr,en,es,de | Non, normalisation technique. |
| `resolve_object_placement` | 200 | Theme natal, Natal simplifie | fr,en,es,de | Recouvre `placement_from_pool`. |
| `placement_from_pool` | 237 | Theme natal, Natal simplifie | fr,en,es,de | Recouvre une branche de `resolve_object_placement`. |
| `humanize_object_position_signal` | 255 | Theme natal, Natal simplifie | fr,en,es,de | Recouvre `placement_label` pour signaux objet. |
| `parse_placement_title` | 271 | Theme natal, Natal simplifie | en | Parser hardcode anglais: "in", "house". |
| `humanize_aspect_signal` | 281 | Theme natal, Natal simplifie | fr,en,es,de | Recouvre humanisation d'aspects. |
| `humanize_dignity_signal` | 306 | Theme natal, Natal simplifie | fr,en,es,de | Wrapper de `humanize_dignity_from_fact_id`. |
| `humanize_dignity_from_fact_id` | 314 | Theme natal, Natal simplifie | fr,en,es,de | Recouvre libelles dignite. |
| `label_from_fact_id` | 339 | Theme natal, Natal simplifie | fr,en,es,de | Orchestrateur interne; recouvre plusieurs helpers parse/humanize. |
| `interpretive_hint_from_fact_id` | 422 | Theme natal, Natal simplifie | fr,en,es,de | Recouvre `label_from_fact_id`, version hint interpretatif. |
| `humanized_axis_code_fallback` | 469 | Theme natal, Natal simplifie | fr,en,es,de | Fallback de libelle axe. |
| `humanize_cluster_signal` | 479 | Theme natal, Natal simplifie | fr,en,es,de | Recouvre concentration signe/maison. |
| `parse_ruler_fact_id` | 504 | Theme natal, Natal simplifie | fr,en,es,de | Parser technique. |
| `humanize_ruler_fact_id` | 517 | Theme natal, Natal simplifie | fr,en,es,de | Wrapper de `humanize_ruler_label`. |
| `humanize_ruler_label` | 532 | Theme natal, Natal simplifie | fr,en,es,de | Humanise les maitres d'angles/maisons. |
| `parse_signal_angle_sign_fact_id` | 592 | Theme natal, Natal simplifie | fr,en,es,de | Parser technique. |
| `humanize_angle_sign_label` | 601 | Theme natal, Natal simplifie | fr,en,es,de | Humanise angle en signe. |
| `label_from_fact_value` | 616 | Theme natal, Natal simplifie | fr,en,es,de | Recouvre `placement_label` depuis `value`. |
| `parse_placement_fact_id` | 666 | Theme natal, Natal simplifie | fr,en,es,de | Parser technique. |
| `parse_angle_fact_id` | 678 | Theme natal, Natal simplifie | fr,en,es,de | Parser technique. |
| `title_case_token` | 686 | Theme natal, Natal simplifie, LLM shared | fr,en,es,de | Recouvre `normalize_sign_code`/fallback label simple. |

### Theme natal - synthese, resume et reparations

| Fonction | Emplacement | Services applicables | Langues applicables | Doublon / recouvrement |
| --- | --- | --- | --- | --- |
| `SummarySynthesizer::new` | `summary_synthesizer.rs:73` | Theme natal | fr,en,es,de | Non, constructeur. |
| `validate_summary_content` | `summary_synthesizer.rs:230` | Theme natal | fr,en,es,de avec regles surtout fr | Recouvre `validate_summary_ux`. |
| `is_summary_banned_pattern_error` | `summary_synthesizer.rs:315` | Theme natal | fr | Recouvre validations safety de wording. |
| `deterministic_safe_summary_fallback` | `summary_synthesizer.rs:327` | Theme natal | fr | Recouvre `simplified_summary_short_text`, mais pour natal complet. |
| `build_summary_messages` | `summary_synthesizer.rs:338` | Theme natal | fr,en,es,de | Prompt de reformulation/resume LLM. |
| `truncate_words` | `summary_synthesizer.rs:405` | Theme natal | fr,en,es,de | Doublon de `compact_period_words` et `trim_to_complete_sentence`. |
| `FinalSynthesisSynthesizer::new` | `final_synthesis_synthesizer.rs:52` | Theme natal Premium Plus | fr,en,es,de | Non, constructeur. |
| `build_synthesis_messages` | `final_synthesis_synthesizer.rs:265` | Theme natal Premium Plus | fr,en,es,de | Prompt de synthese finale LLM. |
| `synthesis_repair_directive` | `final_synthesis_synthesizer.rs:356` | Theme natal Premium Plus | fr,en,es,de | Recouvre `append_repair_instructions`. |
| `truncate_words` | `final_synthesis_synthesizer.rs:395` | Theme natal Premium Plus | fr,en,es,de | Doublon exact de nom et role avec `summary_synthesizer.rs:405`. |
| `safety_repair_from_error` | `chapter_quality_repair.rs:43` | Theme natal | fr,en,es,de | Recouvre `violations_are_script_only` pour classification de repair. |
| `length_repair_from_error` | `chapter_quality_repair.rs:76` | Theme natal | fr,en,es,de | Recouvre les controles de longueur. |
| `is_min_words_violation` | `chapter_quality_repair.rs:163` | Theme natal | fr,en,es,de | Recouvre `length_repair_from_error`, detection specifique. |
| `append_repair_instructions` | `chapter_quality_repair.rs:306` | Theme natal | fr,en,es,de | Recouvre `synthesis_repair_directive`, mais pour chapitres. |

### Natal simplifie et utilitaires partages

| Fonction | Services applicables | Langues applicables | Doublon / recouvrement |
| --- | --- | --- | --- |
| `restore_french_elisions` | Natal simplifie, Theme natal, LLM shared | fr | Recouvre une partie de `sanitize_period_public_string`. |
| `french_elision_violations` | Horoscope daily, Horoscope period, LLM shared | fr | Pair validation de `restore_french_elisions`. |
| `sanitize_text_for_french_script` | Natal simplifie, LLM shared | fr | Recouvre la sanitation script interne de `sanitize_period_public_string`. |
| `collapse_whitespace` | Natal simplifie, LLM shared | fr,en,es,de | Doublon avec plusieurs normalisations whitespace locales. |
| `post_process_single_pass_reading` | Natal simplifie | fr principalement; disclaimer fr,en,es,de | Orchestrateur; recouvre sanitation, typo et summary. |
| `apply_simplified_body_fallback` | Natal simplifie | fr | Recouvre `simplified_deterministic_body`, application structurelle. |
| `build_simplified_summary` | Natal simplifie | fr,en | Recouvre `build_compact_summary_from_body`. |
| `build_compact_summary_from_body` | Natal simplifie | fr,en | Recouvre `split_sentences_fr`, `count_words`, `trim_to_complete_sentence`. |
| `trim_to_complete_sentence` | Natal simplifie | fr,en,es,de | Doublon avec `period_trim_incomplete_tail` et `compact_period_words`. |
| `simplified_summary_title` | Natal simplifie | fr,en | Fallback titre. |
| `simplified_summary_short_text` | Natal simplifie | fr,en | Fallback resume. |
| `simplified_deterministic_body` | Natal simplifie | fr | Fallback corps. |
| `harden_ambiguous_core_identity_chapter` | Natal simplifie | fr | Recouvre `apply_simplified_body_fallback` sur le cas soleil ambigu. |
| `body_has_ambiguous_uncertainty_lexicon` | Natal simplifie | fr | Detection specifique. |
| `ambiguous_uncertainty_prefix_sentence` | Natal simplifie | fr | Recouvre `simplified_deterministic_body`, extrait la premiere phrase. |
| `normalize_simplified_interpretive_roles` | Natal simplifie | fr,en,es,de | Recouvre `AstroBasisRoleNormalizer::normalize_chapter` sur l'objectif role canonique. |
| `sanitize_reading_text_fields` | Natal simplifie | fr | Recouvre `sanitize_field`. |
| `restore_french_typography_fields` | Natal simplifie | fr | Recouvre `typography_field`. |
| `sanitize_field` | Natal simplifie | fr | Wrapper de `sanitize_text_for_french_script`. |
| `typography_field` | Natal simplifie | fr | Wrapper de `restore_french_elisions`. |
| `trigram_phrases` | Theme natal, LLM shared | fr,en,es,de | Base de detection repetition. |
| `is_low_signal_trigram` | Theme natal, LLM shared | fr,en,es; de fallback anglais | Recouvre stopwords locaux. |
| `count_repeated_trigrams` | Theme natal, LLM shared | fr,en,es; de fallback anglais | Recouvre validation anti-repetition. |
| `chapter_opening_phrase` | Theme natal | fr,en,es,de | Recouvre `first_words`. |
| `paragraph_opening_phrases` | Theme natal | fr,en,es,de | Recouvre detection amorces. |
| `openings_to_avoid_from_prior` | Theme natal | fr,en,es,de | Recouvre `phrases_to_avoid_from_prior`, mais sur amorces. |
| `detect_duplicate_openings` | Theme natal | fr principalement | Recouvre `validate_slot_diversity`, version chapitres. |
| `phrases_to_avoid_from_prior` | Theme natal | fr,en,es; de fallback anglais | Recouvre `openings_to_avoid_from_prior`, mais trigrammes. |
| `normalize_paragraph_for_placement_check` | Theme natal | fr,en,es,de | Normalisation technique. |
| `paragraph_starts_with_raw_placement` | Theme natal | fr,en,es,de | Detection d'ouverture trop technique. |
| `soften_raw_placement_openings_in_body` | Theme natal | fr,en,es,de | Correction directe d'ouverture brute. |
| `detect_raw_placement_paragraph_openings` | Theme natal | fr,en,es,de | Pair validation de `soften_raw_placement_openings_in_body`. |
| `count_words` | Theme natal, Natal simplifie, LLM shared | fr,en,es,de | Doublon avec `TokenBudget::word_count` et plusieurs `split_whitespace().count()`. |
| `count_sentences_fr` | Theme natal, Natal simplifie | fr | Recouvre `split_sentences_fr` et `period_complete_sentences`. |
| `split_sentences_fr` | Theme natal, Natal simplifie | fr | Recouvre `period_complete_sentences`. |
| `validate_summary_ux` | Theme natal, Natal simplifie | fr principalement | Recouvre `validate_summary_content` sur les contraintes UX. |

## Autres services - fonctions de formatage texte ajoutees

Cette section etend l'inventaire aux autres services et couches qui produisent, nettoient, humanisent ou controlent du texte expose au LLM, au client ou aux traces.

### Projection LLM calculateur - nettoyage et humanisation de libelles

Fichiers:

- `astral_calculator/src/llm_projection/clean_text.rs`
- `astral_calculator/src/llm_projection/axis_labels.rs`

| Fonction | Emplacement | Services applicables | Langues applicables | Doublon / recouvrement |
| --- | --- | --- | --- | --- |
| `title_case_sign` | `clean_text.rs:3` | Theme natal, payload LLM calculateur, LLM shared | en; fallback technique pour fr,es,de | Doublon avec `title_case_token`. |
| `importance_label` | `clean_text.rs:11` | Theme natal, payload LLM calculateur | en | Recouvre labels d'intensite horoscope (`period_tone_public_label` en intention). |
| `accidental_overall_label` | `clean_text.rs:23` | Theme natal, payload LLM calculateur | en | Humanisation hardcodee; recouvre catalogue de dignites/valences. |
| `humanize_reason` | `clean_text.rs:38` | Theme natal, payload LLM calculateur | en | Recouvre plusieurs humanisations `AstroLabelHumanizer`, mais en anglais et hardcode. |
| `humanize_condition` | `clean_text.rs:110` | Theme natal, payload LLM calculateur | en | Recouvre `humanize_dignity_from_fact_id` / conditions accidentelles. |
| `humanize_dynamic_quality` | `clean_text.rs:137` | Theme natal, payload LLM calculateur | en | Recouvre libelles de dynamique/tonalite. |
| `humanize_valence` | `clean_text.rs:150` | Theme natal, payload LLM calculateur | en | Recouvre libelles de valence. |
| `humanize_phase` | `clean_text.rs:167` | Theme natal, payload LLM calculateur | en | Recouvre libelles de phase aspect. |
| `dignity_meaning` | `clean_text.rs:176` | Theme natal, payload LLM calculateur | en | Recouvre libelles interpretatifs de dignite. |
| `chart_sect_label` | `clean_text.rs:186` | Theme natal, payload LLM calculateur | en | Recouvre labels de secte du catalogue. |
| `hemisphere_dominant_area` | `clean_text.rs:194` | Theme natal, payload LLM calculateur | en | Recouvre labels hemisphere/context. |
| `reading_slot_section` | `clean_text.rs:204` | Theme natal, payload LLM calculateur | en | Recouvre titres de sections chapitre/slot. |
| `axis_balance_label` | `clean_text.rs:215` | Theme natal, payload LLM calculateur | en | Recouvre `expected_interpretive_hint` et humanisation axes. |
| `axis_importance` | `clean_text.rs:228` | Theme natal, payload LLM calculateur | en | Wrapper de `importance_label`. |
| `limit_keywords` | `clean_text.rs:232` | Theme natal, payload LLM calculateur, LLM shared | fr,en,es,de | Recouvre `push_unique` et dedupe de tags. |
| `clean_semantic_tags` | `clean_text.rs:253` | Theme natal, payload LLM calculateur | en; neutralise des tags toute langue | Recouvre sanitation de mots-clefs techniques. |
| `is_technical_keyword` | `clean_text.rs:262` | Theme natal, payload LLM calculateur | en; codes techniques | Recouvre `validate_public_text_no_technical_codes` en detection de leaks. |
| `push_unique` | `clean_text.rs:285` | Theme natal, payload LLM calculateur, LLM shared | fr,en,es,de | Doublon avec `house_axes::push_unique`. |
| `humanize_theme_code` | `clean_text.rs:294` | Theme natal, payload LLM calculateur | en | Doublon avec `period_theme_public_label` / `period_domain_title`, mais anglais. |
| `humanize_axis_summary` | `clean_text.rs:318` | Theme natal, payload LLM calculateur | en | Recouvre `humanize_residual_snake_case`. |
| `humanize_residual_snake_case` | `clean_text.rs:327` | Theme natal, payload LLM calculateur | en | Doublon avec plusieurs `replace('_', " ")`. |
| `is_unremarkable_motion_condition` | `clean_text.rs:355` | Theme natal, payload LLM calculateur | en | Filtre textuel, recouvre validation de conditions non informatives. |
| `humanize_motion_label` | `clean_text.rs:368` | Theme natal, payload LLM calculateur | en | Recouvre `humanize_condition` pour mouvement. |
| `seed_axis_labels` | `axis_labels.rs:9` | Theme natal, payload LLM calculateur | en; donnees JSON | Source de libelles axes. |
| `house_axis_label` | `axis_labels.rs:28` | Theme natal, payload LLM calculateur | en; fallback code | Doublon avec `AstroLabelHumanizer::label_for_fact_id` pour `house_axis:*`. |

### Projection LLM calculateur - hints et controle texte runtime

Fichiers:

- `astral_calculator/src/runtime/payload_freshness/house_axes.rs`
- `astral_calculator/src/runtime/payload_freshness/text.rs`

| Fonction | Emplacement | Services applicables | Langues applicables | Doublon / recouvrement |
| --- | --- | --- | --- | --- |
| `expected_interpretive_hint` | `house_axes.rs:247` | Theme natal, payload freshness calculateur | en | Doublon avec `humanize_axis_summary` / hints axes. |
| `axis_label` | `house_axes.rs:277` | Theme natal, payload freshness calculateur | en | Doublon avec `house_axis_label`. |
| `has_text` | `text.rs:1` | LLM shared, payload freshness calculateur | fr,en,es,de | Doublon avec `has_text_value`, `json_value_has_text`, plusieurs checks `trim().is_empty()`. |
| `has_current_aspect_hint` | `text.rs:5` | Theme natal, payload freshness calculateur | en | Validation de hint textuel anglais mal forme. |

### Langue, style et compilation de prompt

Fichiers:

- `astral_llm/crates/astral_llm_application/src/writing_language.rs`
- `astral_llm/crates/astral_llm_application/src/prompt_compiler.rs`
- `astral_llm/crates/astral_llm_application/src/prompt_trace.rs`
- `astral_llm/crates/astral_llm_application/src/payload_sanitizer.rs`
- `astral_llm/crates/astral_llm_domain/src/legal_copy.rs`

| Fonction | Emplacement | Services applicables | Langues applicables | Doublon / recouvrement |
| --- | --- | --- | --- | --- |
| `WritingLanguage::prompt_block` | `writing_language.rs:6` | Theme natal, Natal simplifie, LLM shared | fr,en,es,de | Recouvre `AstroLabelHumanizer::locale_key` et instructions langue dans prompts. |
| `PromptCompiler::to_provider_messages` | `prompt_compiler.rs:150` | Theme natal, Natal simplifie, LLM shared | fr,en,es,de | Formate les messages provider; recouvre `format_compiled_messages` pour traces. |
| `build_profile_block` | `prompt_compiler.rs:201` | Theme natal, Natal simplifie | fr,en,es,de | Formate bloc style/tone/jargon; recouvre `sanitize_custom_instructions`. |
| `safety_policy_text` | `prompt_compiler.rs:237` | Theme natal, Natal simplifie, LLM shared | fr,en,es,de | Formate la policy en JSON lisible. |
| `prompt_log_chapter_segment` | `prompt_trace.rs:137` | Theme natal, Natal simplifie, LLM shared | fr,en,es,de | Recouvre `sanitize_filename_segment`, appliqué aux traces. |
| `sanitize_filename_segment` | `prompt_trace.rs:145` | LLM shared | fr,en,es,de | Doublon technique de sanitation de segment. |
| `format_compiled_messages` | `prompt_trace.rs:162` | LLM shared | fr,en,es,de | Doublon avec `PromptCompiler::to_provider_messages`, format trace. |
| `contains_prompt_injection` | `payload_sanitizer.rs:20` | Theme natal, Natal simplifie, LLM shared | fr,en; patterns surtout fr/en | Detection textuelle de prompt injection. |
| `scan_json_for_injection` | `payload_sanitizer.rs:27` | Theme natal, Natal simplifie, LLM shared | fr,en; patterns surtout fr/en | Recouvre `contains_prompt_injection`, parcours JSON. |
| `wrap_astro_payload` | `payload_sanitizer.rs:42` | Theme natal, Natal simplifie | fr,en,es,de | Formate payload comme bloc data-only. |
| `sanitize_custom_instructions` | `payload_sanitizer.rs:56` | Theme natal, Natal simplifie | fr,en; patterns surtout fr/en | Recouvre `contains_prompt_injection` et trim. |
| `default_legal_disclaimer` | `legal_copy.rs:3` | Theme natal, Natal simplifie, LLM shared | fr,en; es,de recoivent anglais | Doublon d'intention avec disclaimers de safety/prompt. |

### Guidance, repair et normalisation `astro_basis`

Fichiers:

- `astral_llm/crates/astral_llm_application/src/chapter_writing_guidance.rs`
- `astral_llm/crates/astral_llm_application/src/reading_opening_diversity_validator.rs`
- `astral_llm/crates/astral_llm_application/src/astro_basis_role_normalizer.rs`
- `astral_llm/crates/astral_llm_application/src/chapter_evidence_basis_enricher.rs`

| Fonction | Emplacement | Services applicables | Langues applicables | Doublon / recouvrement |
| --- | --- | --- | --- | --- |
| `ChapterWritingGuidance::append_upstream_directives` | `chapter_writing_guidance.rs:20` | Theme natal | fr,en,es,de; texte de consigne surtout en anglais avec exemples fr | Recouvre anti-repetition de `text_trigrams` et repair openings. |
| `length_expansion_focus_block` | `chapter_writing_guidance.rs:194` | Theme natal Premium Plus | fr,en,es,de | Recouvre `length_repair_from_error`, mais en amont prompt. |
| `ReadingOpeningDiversityValidator::detect` | `reading_opening_diversity_validator.rs:30` | Theme natal | fr,en,es,de | Wrapper bloquant de `detect_duplicate_openings`. |
| `ReadingOpeningDiversityValidator::detect_all` | `reading_opening_diversity_validator.rs:37` | Theme natal | fr,en,es,de | Doublon structurel de `detect_duplicate_openings`. |
| `ReadingOpeningDiversityValidator::detect_warnings` | `reading_opening_diversity_validator.rs:52` | Theme natal | fr,en,es,de | Recouvre `detect_all`, non bloquant. |
| `ReadingOpeningDiversityValidator::validate` | `reading_opening_diversity_validator.rs:59` | Theme natal | fr,en,es,de | Validation bloquante des ouvertures. |
| `ReadingOpeningDiversityValidator::append_opening_repair_directives` | `reading_opening_diversity_validator.rs:84` | Theme natal | fr,en,es,de; exemples fr | Recouvre `ChapterWritingGuidance::append_upstream_directives`, mais en repair. |
| `ReadingOpeningDiversityValidator::opening_phrase_for_chapter` | `reading_opening_diversity_validator.rs:173` | Theme natal | fr,en,es,de | Wrapper de `chapter_opening_phrase`. |
| `ReadingOpeningDiversityValidator::detect_raw_placement_warnings` | `reading_opening_diversity_validator.rs:177` | Theme natal | fr,en,es,de | Wrapper de `detect_raw_placement_paragraph_openings`. |
| `AstroBasisRoleNormalizer::normalize_chapter` | `astro_basis_role_normalizer.rs:13` | Theme natal, Natal simplifie | fr,en,es,de | Recouvre `normalize_simplified_interpretive_roles`, mais plus riche. |
| `AstroBasisRoleNormalizer::coerce_role_string` | `astro_basis_role_normalizer.rs:74` | Theme natal, Natal simplifie | fr,en | Doublon de mapping role libre vers code canonique. |
| `ChapterEvidenceBasisEnricher::enrich_missing_pack_slots` | `chapter_evidence_basis_enricher.rs:13` | Theme natal | fr,en,es,de | Modifie `astro_basis` expose; recouvre humanisation/normalisation evidence. |

### Validation redactionnelle, safety et qualite hors horoscope

Fichiers:

- `astral_llm/crates/astral_llm_application/src/reading_quality_validator.rs`
- `astral_llm/crates/astral_llm_application/src/editorial_validation.rs`
- `astral_llm/crates/astral_llm_application/src/summary_forbidden_patterns.rs`
- `astral_llm/crates/astral_llm_application/src/safety_guard.rs`

| Fonction | Emplacement | Services applicables | Langues applicables | Doublon / recouvrement |
| --- | --- | --- | --- | --- |
| `ReadingQualityValidator::assess` | `reading_quality_validator.rs:58` | Theme natal | fr,en,es,de | Orchestrateur qualite; recouvre plusieurs detecteurs locaux. |
| `ReadingQualityValidator::assess_with_thresholds` | `reading_quality_validator.rs:81` | Theme natal | fr,en,es,de | Coeur d'analyse qualite texte. |
| `ReadingQualityValidator::chapter_repetition_score` | `reading_quality_validator.rs:186` | Theme natal | fr,en,es; de fallback anglais | Wrapper de `count_repeated_trigrams`. |
| `ReadingQualityValidator::chapter_exceeds_repetition` | `reading_quality_validator.rs:190` | Theme natal | fr,en,es; de fallback anglais | Wrapper de seuil repetition. |
| `ReadingQualityValidator::validate_for_product` | `reading_quality_validator.rs:199` | Theme natal | fr,en,es,de | Validation qualite produit. |
| `ReadingQualityValidator::assess_or_warn` | `reading_quality_validator.rs:230` | Theme natal | fr,en,es,de | Deprecated; doublon de `validate_for_product`. |
| `thresholds_for_request` | `reading_quality_validator.rs:239` | Theme natal | fr,en,es,de | Resolution de seuils, non linguistique. |
| `word_count` | `reading_quality_validator.rs:265` | Theme natal | fr,en,es,de | Doublon avec `count_words`, `TokenBudget::word_count`. |
| `has_interpretive_framing` | `reading_quality_validator.rs:269` | Theme natal | fr,en; es,de peu couverts | Recouvre `SafetyGuard::has_symbolic_framing`. |
| `has_deterministic_wording` | `reading_quality_validator.rs:312` | Theme natal | fr,en | Recouvre safety fataliste. |
| `has_beginner_jargon` | `reading_quality_validator.rs:325` | Theme natal | fr,en | Recouvre `editorial_validation::has_excessive_jargon`. |
| `chapter_has_symbolic_disclaimer_boilerplate` | `reading_quality_validator.rs:338` | Theme natal | fr | Recouvre controle disclaimer repetitif. |
| `count_symbolic_disclaimer_boilerplate_chapters` | `reading_quality_validator.rs:345` | Theme natal | fr | Agregation du controle precedent. |
| `EditorialValidator::validate_fixture` | `editorial_validation.rs:37` | Theme natal fixtures | fr,en,es,de | Orchestrateur fixture. |
| `EditorialValidator::validate_reading` | `editorial_validation.rs:51` | Theme natal fixtures | fr,en,es,de | Recouvre `ReadingQualityValidator::validate_for_product` et safety wording. |
| `corpus` | `editorial_validation.rs:122` | Theme natal fixtures | fr,en,es,de | Doublon avec `SafetyGuard::collect_text`. |
| `has_fatalistic_wording` | `editorial_validation.rs:131` | Theme natal fixtures | fr,en | Recouvre `has_deterministic_wording`. |
| `has_forbidden_advice` | `editorial_validation.rs:144` | Theme natal fixtures | fr,en | Recouvre `SafetyGuard::matches_patterns`. |
| `has_excessive_jargon` | `editorial_validation.rs:157` | Theme natal fixtures | fr,en | Doublon de `has_beginner_jargon`. |
| `summary_forbidden_regex` | `summary_forbidden_patterns.rs:8` | Theme natal summary | fr | Source regex interdits resume. |
| `find_forbidden_summary_patterns` | `summary_forbidden_patterns.rs:17` | Theme natal summary | fr | Recouvre `validate_summary_content`, extraction des motifs. |
| `SafetyGuard::validate_response` | `safety_guard.rs:42` | Theme natal, Natal simplifie | fr,en,es,de; patterns surtout fr/en | Orchestrateur safety texte. |
| `SafetyGuard::validate_chapter_text` | `safety_guard.rs:113` | Theme natal, Natal simplifie | fr,en,es,de; patterns surtout fr/en | Controle safety par chapitre. |
| `collect_text` | `safety_guard.rs:173` | Theme natal, Natal simplifie | fr,en,es,de | Doublon avec `editorial_validation::corpus`. |
| `contains_unsafe_override` | `safety_guard.rs:186` | Theme natal, Natal simplifie | fr,en | Recouvre `contains_prompt_injection`. |
| `matches_patterns` | `safety_guard.rs:191` | Theme natal, Natal simplifie | fr,en | Detection pattern generique. |
| `has_symbolic_framing` | `safety_guard.rs:199` | Theme natal, Natal simplifie | fr,en | Recouvre `has_interpretive_framing`. |
| `has_builtin_interpretive_framing` | `safety_guard.rs:206` | Theme natal, Natal simplifie | fr,en | Variante hardcodee de framing. |

## Fonctions en double ou a mutualiser

| Groupe | Fonctions concernees | Diagnostic |
| --- | --- | --- |
| Decoupe/compaction de phrases | `compact_period_words`, `period_complete_sentences`, `period_trim_incomplete_tail`, `trim_to_complete_sentence`, `summary_synthesizer::truncate_words`, `final_synthesis_synthesizer::truncate_words`, `split_sentences_fr` | Fort recouvrement. Mutualisation possible dans un module `text_length` avec support langue. |
| Comptage de mots | `count_words`, `period_public_word_count`, `TokenBudget::word_count`, usages directs de `split_whitespace().count()` | Doublon transversal. Standardiser une fonction unique par politique de comptage. |
| Normalisation texte pour comparaison | `normalized_text`, `meaningful_words`, `first_words`, `chapter_opening_phrase`, `paragraph_opening_phrases`, `normalize_paragraph_for_placement_check` | Recouvrements partiels entre horoscope daily et theme natal. |
| Anti-repetition | `normalize_period_repetitive_public_phrases`, `normalize_period_repetitive_text`, `replace_period_phrase_after_allowed`, `detect_duplicate_openings`, `count_repeated_trigrams`, `phrases_to_avoid_from_prior` | Deux approches: correction deterministic period vs detection/prompting natal. A garder separees ou extraire une couche commune de detection. |
| Typographie francaise | `sanitize_period_french_colon_spacing`, `restore_french_elisions`, `french_elision_violations`, `sanitize_period_public_string` | Recouvrement partiel. Les corrections francaises devraient etre centralisees dans `french_typography`. |
| Sanitation script | `sanitize_text_for_french_script`, `sanitize_reading_text_fields`, `sanitize_field`, `sanitize_period_public_string` | Doublon partiel: horoscope period n'utilise pas le meme chemin que natal simplifie. |
| Libelles objets astrologiques | `period_object_public_label`, `AstroLabelHumanizer::object_label`, `title_case_token` | `period_object_public_label` est hardcode et devrait idealement deleguer au catalogue/humanizer. |
| Libelles theme/tone/focus | `period_theme_public_label`, `period_domain_title`, `period_tone_public_label`, `period_tone_public_label_if_code`, `period_natal_focus`, `period_*_labels` | Recouvrement attendu entre resolution label et source labels. Pas un doublon critique, mais separation resolution/source a clarifier. |
| Conseils et personnalisation periode | `ensure_period_personalization_text`, `period_public_personalization_sentence`, `period_public_interpretive_sentence`, `period_public_domain_personalization_sentence`, `period_public_domain_interpretive_sentence`, `period_advice_hint` | Plusieurs fonctions produisent des phrases proches. Mutualisation possible par templates DB ou catalogue. |
| Fallbacks fake/deterministes | `fake_writer_*`, `render_fake_*`, `premium_timeline_*`, `simplified_deterministic_body`, `deterministic_safe_summary_fallback` | Doublons d'intention, mais schemas differents. Les textes hardcodes devraient etre signales comme dette vis-a-vis de la regle "donnees canoniques". |
| Directives de repair | `append_repair_instructions`, `synthesis_repair_directive`, `SCRIPT_REPAIR_INSTRUCTION`, `length_repair_from_error`, `safety_repair_from_error` | Recouvrement entre chapitre, synthese finale et single-pass. Mutualisation possible par type de violation. |
| Humanisation anglaise calculateur vs humanizer catalogue | `humanize_reason`, `humanize_condition`, `humanize_theme_code`, `humanize_axis_summary`, `house_axis_label`, `AstroLabelHumanizer::*`, `period_*_public_label` | Plusieurs couches traduisent des codes en libelles. Le calculateur est surtout anglais/hardcode, l'application utilise le catalogue. Risque de divergence. |
| Role `astro_basis` | `AstroBasisRoleNormalizer::normalize_chapter`, `coerce_role_string`, `normalize_simplified_interpretive_roles` | Deux normalisations de roles coexistent; la version simplifiee est moins riche. |
| Guidance anti-repetition amont/retry | `ChapterWritingGuidance::append_upstream_directives`, `ReadingOpeningDiversityValidator::append_opening_repair_directives`, `openings_to_avoid_from_prior`, `phrases_to_avoid_from_prior` | Meme objectif avec textes de consignes dupliques; difference entre generation initiale et repair. |
| Prompt formatting / trace formatting | `PromptCompiler::to_provider_messages`, `format_compiled_messages`, `build_profile_block`, `WritingLanguage::prompt_block` | Plusieurs formatages de messages provider/prompt. Garder separe trace/provider mais mutualiser role labels et serialization. |
| Prompt injection / unsafe override | `contains_prompt_injection`, `scan_json_for_injection`, `sanitize_custom_instructions`, `contains_unsafe_override`, `matches_patterns` | Deux systemes de detection pattern coexistent: payload sanitizer et safety guard. |
| Corpus aggregation | `free_public_text`, `collect_period_public_text`, `editorial_validation::corpus`, `SafetyGuard::collect_text`, `collect_reading_corpus` | Meme operation d'aggregation de champs texte selon schemas differents. Possible helper par type de reponse. |
| Disclaimers/framing symbolique | `default_legal_disclaimer`, `has_symbolic_framing`, `has_builtin_interpretive_framing`, `has_interpretive_framing`, `chapter_has_symbolic_disclaimer_boilerplate` | Generation, detection et anti-repetition du framing sont separees; coherent mais fragile si wording change. |
| Jargon/debutant | `has_beginner_jargon`, `has_excessive_jargon`, `validate_free_text_quality` checks generiques | Listes de jargon et generiques dispersees. |

## Mapping vers `text_reprocessing`

Le module `astral_llm_application::text_reprocessing` v1 reprend ces fonctionnalites sous forme de processors isoles. Il n'est pas encore branche aux services applicatifs; les anciennes fonctions restent la source de verite runtime.

| Fonctionnalite cible | Processor v1 | Groupes remplaces a terme |
| --- | --- | --- |
| Sanitation alphabet / injection | `ScriptSanitizerProcessor` | `sanitize_text_for_french_script`, `sanitize_field`, `contains_prompt_injection`, `scan_json_for_injection`, `sanitize_custom_instructions` |
| Typographie francaise | `TypographyProcessor` | `restore_french_elisions`, `typography_field`, fragments typographiques periode |
| Longueur et phrase complete | `SentenceAndLengthProcessor` | `compact_period_words`, `period_trim_incomplete_tail`, `trim_to_complete_sentence`, `truncate_words`, word-count helpers |
| Anti-repetition | `RepetitionProcessor` | substitutions periode, trigrammes, ouvertures a eviter |
| Humanisation libelles | `AstroLabelHumanizerProcessor` | `period_*_public_label`, `humanize_*`, `house_axis_label`, label fallbacks |
| `astro_basis` | `AstroBasisProcessor` | `AstroBasisRoleNormalizer`, `normalize_simplified_interpretive_roles`, enrichissement basis |
| Qualite / safety texte | `QualityValidationProcessor` | quality validators, forbidden wording, framing, jargon |
| Fallbacks publics | `FallbackTextProcessor` | fake/fallback summary/advice/body par service |
| Guidance de prompt | `PromptGuidanceProcessor` | `ChapterWritingGuidance`, opening repair directives, language prompt block |
| Trace provider | `TraceFormattingProcessor` | `format_compiled_messages`, prompt trace formatting |

Voir `docs/TEXT_REPROCESSING_MODULE.md` pour les contrats, registres, exemples JSON et strategie de branchement futur.
