# REV-016 — Audit documentation post-risques résiduels (PR1)

Date : 2026-06-06  
Successeur : REV-012 (v2.4 initiale)

## Matrice doc ↔ code (PR1)

| Document | Statut | Notes |
|----------|--------|-------|
| `docs/BASIC_PAYLOAD_IMPLEMENTATION.md` § Natal simplifié | **Aligné** | Table `astral_simplified_profile_feature_exclusions`, F-07 closed |
| `docs/natal_simplified_reading_contract.md` | **Aligné** | Source DB exclusions ; `partial` strict ; `simplified` reserved |
| `docs/natal_simplified_forbidden_topics.md` | **Partiel** | Recette OpenAI P0/P1 à compléter post-smoke |
| `docs/GUIDE_DEBUTANT_DOCKER.md` §9 | **Partiel** | Mention import table exclusions à ajouter |
| `docs/Astral_llm_implementation.md` § simplified | **Partiel** | `-StrictOpenAiQuality` documenté via scripts ; monitoring privacy post-D |
| `docs/release-evidence/natal-simplified-v1.md` | **Partiel** | v1.1 après smoke OpenAI |
| Schémas `astro_simplified_natal_response_v1` | **Aligné** | Description `simplified` reserved |
| `contracts/README.md` | **Partiel** | Recette OpenAI optionnelle |
| `AGENTS.md` | **OK** | Commandes E2E inchangées |

## Écarts fermés (PR1)

| Sujet | Avant | Après |
|-------|-------|-------|
| F-07 | Constante Rust | Table `astral_simplified_profile_feature_exclusions` |
| `reading_completeness` PS1 | `partial \| simplified` | **`partial` strict** |
| REV-012 écarts connus F-07 | Ouvert | **Closed** REV-013 |

## Gate REV-016

**Partiel OK pour PR1** — audit OpenAI / monitoring / release evidence v1.1 à compléter après smoke `-UseReal` (REV-014).

Commande smoke :

```powershell
.\scripts\test_natal_simplified_e2e.ps1 -UseReal -SubmitProfile -TimeoutSec 900
```

Artefacts : `output/natal_simplified_openai/{timestamp}/` + `quality_summary.json`.
