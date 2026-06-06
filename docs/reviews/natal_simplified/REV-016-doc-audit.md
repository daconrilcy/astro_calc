# REV-016 — Audit documentation post-risques résiduels

Date : 2026-06-06  
Successeur : REV-012 (v2.4 initiale)  
Statut : **CLOSED**

## Matrice doc ↔ code (finale)

| Document | Statut | Notes |
|----------|--------|-------|
| `docs/BASIC_PAYLOAD_IMPLEMENTATION.md` § Natal simplifié | **Aligné** | Table `astral_simplified_profile_feature_exclusions`, F-07 closed |
| `docs/natal_simplified_reading_contract.md` | **Aligné** | Source DB exclusions ; `partial` strict ; `simplified` reserved |
| `docs/natal_simplified_forbidden_topics.md` | **Aligné** | Source DB + gate OpenAI P0/P1 (`Assert-SimplifiedStrictOpenAiQuality`) |
| `docs/GUIDE_DEBUTANT_DOCKER.md` §9 | **Aligné** | Import table exclusions ; recette `-UseReal` |
| `docs/Astral_llm_implementation.md` § simplified | **Aligné** | Monitoring OpenAI + note privacy (volet D deferred) |
| `docs/release-evidence/natal-simplified-v1.md` | **Aligné** | v1.1 post-smoke REV-014 |
| Schémas `astro_simplified_natal_response_v1` | **Aligné** | Description `simplified` reserved |
| `contracts/README.md` | **Aligné** | Recette OpenAI optionnelle |
| `AGENTS.md` | **OK** | Commandes E2E inchangées |

## Écarts fermés (PR1 + PR2)

| Sujet | Avant | Après |
|-------|-------|-------|
| F-07 | Constante Rust | Table `astral_simplified_profile_feature_exclusions` |
| `reading_completeness` PS1 | `partial \| simplified` | **`partial` strict** |
| REV-012 écarts connus F-07 | Ouvert | **Closed** REV-013 |
| Recette OpenAI P0/P1 | Non documentée | **Closed** REV-014 + doc forbidden_topics |
| Import table exclusions §9 | Implicite | **Explicit** GUIDE_DEBUTANT_DOCKER |
| Logs Rust privacy simplified | Plan volet D | **Deferred** — artefacts E2E documentés |

## Gate REV-016

Smoke OpenAI exécuté (REV-014) :

```powershell
.\scripts\test_natal_simplified_e2e.ps1 -UseReal -SubmitProfile -TimeoutSec 900
```

Résultat : **7/7**, P0=0, P1=0 — `output/natal_simplified_openai/2026-06-06T100348Z/`.

Gate REV-016 : **CLOSED** (doc alignée ; volet D logs Rust hors périmètre V1).
