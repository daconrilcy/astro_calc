# REV-022 — Review adversariale post-écarts (durcissement équinoxe)

Date : 2026-06-06  
Périmètre : clôture écarts plan + durcissement équinoxe (3 couches).

## Verdict

**Gate : OK** — écarts traités, findings corrigés, tests verts.

## Findings

| ID | Sévérité | Piège | Correction | Statut |
|----|----------|-------|------------|--------|
| R22-01 | P1 | Test intégration fallback absent (REV-019) | `ambiguous_fallback_chain_repairs_non_conformant_equinox_reading` + `ambiguous_fallback_does_not_trigger_on_mixed_violations` dans `astral_llm_simplified_reading_tests.rs` | **Fixed** |
| R22-02 | P1 | `apply_simplified_body_fallback` noop si `chapters` vide | Crée un chapitre minimal (`ambiguous_core_identity`, body déterministe, `confidence=low`) | **Fixed** |
| R22-03 | P2 | Fallback ne corrige pas `chapter.code` avant re-harden | `apply_simplified_body_fallback` force `chapter.code = chapter_code` | **Fixed** |
| R22-04 | P2 | `harden` ne ciblait que `chapters[0]` | Cible `ambiguous_core_identity` par code, sinon index 0 | **Fixed** |
| R22-05 | — | Livraison git incomplète (hors `cef6e65`) | Commit `1a5ca2c` durcissement + doc + REV-018…022 | **Fixed** |
| R22-06 | — | Smoke OpenAI revalidé post-merge local | Run utilisateur `2026-06-06T125816Z` — 7/7, `gate_passed: true`, `gpt-5.4-mini` | **Fixed** |

## Tests gate

```powershell
cargo test -p astral_llm_application simplified_reading_postprocess
cargo test -p astral_llm_application simplified_reading_guard
cargo test -p astral_llm_api --test astral_llm_simplified_reading_tests
```

Gate REV-022 : **OK**.
