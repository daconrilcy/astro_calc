# REV-020 — Review adversariale post-couche 3 (prompt + smoke OpenAI)

Date : 2026-06-06  
Périmètre : `task_fragment` profil `natal_simplified.json`, smoke `-UseReal`.

## Verdict

**Gate : OK** — smoke OpenAI **7/7**, `gate_passed: true`, P0=0, P1=0.

## Findings

| ID | Piège | Vérification | Statut |
|----|-------|--------------|--------|
| R20-01 | Prompt contredit le post-traitement | Doc : prompt = prévention, serveur = vérité (`Astral_llm_implementation.md`, `forbidden_topics.md`) | **OK** |
| R20-02 | Profil resoumis en Docker | `-SubmitProfile` dans E2E + `manage_natal_interpretation_profiles.ps1 -Submit` | **OK** |
| R20-03 | `chapter_word_targets` incohérents | min 60 vs smoke body 120 — doc only ; gate OpenAI reste 120+ mots | **OK** (doc) |

## Smoke certifiant

```powershell
docker compose up -d --build astral_llm_api
.\scripts\manage_natal_interpretation_profiles.ps1 -Submit -Path config\natal_interpretation_profiles\natal_simplified.json
.\scripts\test_natal_simplified_e2e.ps1 -UseReal -SubmitProfile -TimeoutSec 900
```

| Métrique | Valeur |
|----------|--------|
| Artefacts | `output/natal_simplified_openai/2026-06-06T113454Z/` |
| Cas | **7/7** |
| `gate_passed` | **true** |
| Modèle | `gpt-4.1-mini` |
| Équinoxe `date_only_equinox_window` | `ambiguous_core_identity`, `confidence=low`, run `9443f1c3-6069-4c5c-8752-2731ec358343` |

Gate REV-020 : **OK**.
