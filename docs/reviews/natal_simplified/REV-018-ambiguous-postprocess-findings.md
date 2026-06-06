# REV-018 — Review adversariale post-couche 1 (harden ambiguous_core)

Date : 2026-06-06  
Périmètre : `harden_ambiguous_core_identity_chapter` dans `simplified_reading_postprocess.rs`.

## Verdict

**Gate : OK** — tests unitaires postprocess + `astral_llm_simplified_reading_tests` ; E2E fake 24/24 après rebuild LLM.

## Findings

| ID | Piège | Vérification | Statut |
|----|-------|--------------|--------|
| R18-01 | Hardening appliqué sur cas stable | `harden_skips_stable_identity_case` — aucune mutation si `sun_sign_blocked=false` | **OK** |
| R18-02 | Préfixe duplique l'incertitude | `harden_prefix_is_idempotent_when_lexicon_present` — pas de second préfixe si lexique PS1 déjà présent | **OK** |
| R18-03 | Summary regénéré après hardening | `build_simplified_summary` appelé **après** `harden_ambiguous_core_identity_chapter` dans `post_process_single_pass_reading` | **OK** |
| R18-04 | Clamp `low` trop agressif sur chapitre non ambigu | Garde `sun_sign_blocked` stricte — hardening no-op si false | **OK** |
| R18-05 | Pas de constante métier calculateur | Lexique FR aligné PS1 ; préfixe = 1re phrase de `simplified_deterministic_body(SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE)` | **OK** |

## Tests gate

```powershell
cargo test -p astral_llm_application simplified_reading_postprocess
cargo test -p astral_llm_api --test astral_llm_simplified_reading_tests
docker compose up -d --build astral_llm_api
.\scripts\test_natal_simplified_e2e.ps1
```

Gate REV-018 : **OK**.
