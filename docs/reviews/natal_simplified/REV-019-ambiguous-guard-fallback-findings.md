# REV-019 — Review adversariale post-couche 2 (guard + fallback ambigu)

Date : 2026-06-06  
Périmètre : `ambiguous_core_identity_violations`, branche `ambiguous_core_body_fallback` dans `single_pass_hardening.rs`.

## Verdict

**Gate : OK** — tests guard + ordre fallback documenté ; E2E fake 24/24.

## Findings

| ID | Piège | Vérification | Statut |
|----|-------|--------------|--------|
| R19-01 | Fallback masque une vraie hallucination ASC | Fallback **uniquement** si `violations_are_ambiguous_core_only` (préfixe `ambiguous_core_identity`) | **OK** |
| R19-02 | Double fallback script + ambigu | Ordre : retry script → `script_body_fallback` → `ambiguous_core_body_fallback` → rejet 422 | **OK** |
| R19-03 | `fallback_used` exposé en qualité | `reading.quality.fallback_used = true` sur succès fallback ambigu | **OK** |
| R19-04 | Rejet 422 si fallback échoue encore | Pas de succès silencieux — `break` puis `safety_validation_error` | **OK** |

## Ordre des branches `single_pass_hardening.rs`

1. Retry OpenAI si violations script-only et attempts restants
2. `script_body_fallback` si script-only et attempts épuisés
3. `ambiguous_core_body_fallback` si violations ambiguous-only
4. Rejet `PostSafetyValidationFailed` (422 orchestré)

## Tests gate

```powershell
cargo test -p astral_llm_application simplified_reading_guard
cargo test -p astral_llm_api --test astral_llm_simplified_reading_tests
.\scripts\test_natal_simplified_e2e.ps1
```

Test intégration : `ambiguous_fallback_chain_repairs_non_conformant_equinox_reading` (REV-022).

Gate REV-019 : **OK**.
