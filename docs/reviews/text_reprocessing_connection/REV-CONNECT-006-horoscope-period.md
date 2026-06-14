# REV-CONNECT-006 - Horoscope period

## Review

Aucun P0/P1/P2 ouvert.

Le flux provider period passe dans `reprocess_horoscope_period` apres `repair_period_response_shape` et `normalize_period_public_tones`, avant validation publique.

Correction supplementaire: les sous-fonctions locales de sanitation de chaine publique periode (`sanitize_period_french_fragments`, `sanitize_period_broken_sentences`, `sanitize_period_sentence_boundaries`, `sanitize_period_french_colon_spacing`, remplacements ASCII) ont ete migrees dans `ScriptSanitizerProcessor` pour `SERVICE_HOROSCOPE_PERIOD`.

Note: les fonctions periode restantes sont conservees pour la structure de reparation, car elles normalisent aussi la shape, les preuves, les fenetres et les fallbacks de contrat. Le chemin actif de retraitement texte passe par `reprocess_horoscope_period`.

## Verification

- Fixture: `tests/fixtures/text_reprocessing_migration/horoscope_period/baseline.json`
- Test central: `text_reprocessing_horoscope_period_sanitizes_public_technical_leaks`
- `cargo test -p astral_llm_api --test horoscope_tests`
