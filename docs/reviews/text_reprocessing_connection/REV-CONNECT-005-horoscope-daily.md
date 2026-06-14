# REV-CONNECT-005 - Horoscope daily

## Review

Aucun P0/P1/P2 ouvert.

Les sorties fake daily Free/Basic/Premium sont passees dans `reprocess_horoscope_daily` apres construction structurelle. Les fonctions de rendu fake restent conservees pour la forme produit et les textes deterministes.

## Verification

- Fixture: `tests/fixtures/text_reprocessing_migration/horoscope_daily/baseline.json`
- `cargo test -p astral_llm_api --test horoscope_tests`
