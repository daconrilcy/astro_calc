# REV-015 — Clôture finale natal simplifié V1 (v2.4)

Date : 2026-06-06  
Statut : **CLOSED**

## Verdict global

| Gate | Résultat |
|------|----------|
| Reviews adversariales REV-001…011 | **CLOSED** |
| Audit doc REV-012 (v2.4 initiale) | **CLOSED** |
| Post-PR1 risques résiduels REV-013 | **CLOSED** — F-07 DB, `reading_completeness` strict |
| Post-PR2 OpenAI REV-014 | **CLOSED** — smoke 7/7, P0=0, P1=0 |
| Audit doc post-risques REV-016 | **CLOSED** |
| Recette E2E fake (gate CI) | **24/24** |
| Release evidence | [`docs/release-evidence/natal-simplified-v1.md`](../../release-evidence/natal-simplified-v1.md) |

**Produit natal simplifié V1 — CLOSED WITH MONITORING** (recette OpenAI périodique `-UseReal`, hors gate CI fake).

## Baseline git

| SHA | Rôle |
|-----|------|
| `ba65f94` | F-07 DB + `reading_completeness` + assertions StrictOpenAiQuality |
| `931e810` | Clôture REV-014 smoke OpenAI |
| *(HEAD post REV-015/016)* | Doc finale + registre reviews |

## Périmètre gelé

Moteur `astral_calculator/src/simplified/*`, endpoints orchestrés, profil `natal_simplified`, pipeline `single_pass_hardening`, validateurs simplified, scripts E2E / assertions PS1, table `astral_simplified_profile_feature_exclusions`.

Toute évolution fonctionnelle = nouveau cycle (CS / REV / release evidence).

## Risques résiduels — statut final

| ID | Sujet | Statut |
|----|-------|--------|
| F-07 | Exclusions profil | **CLOSED** — table DB |
| `reading_completeness` | Tolérance `simplified` | **CLOSED** — runtime + PS1 = `partial` ; schema reserved |
| Qualité OpenAI | Variabilité provider | **CLOSED WITH MONITORING** — REV-014 7/7 ; re-smoke manuel |
| Volet D | Logs Rust privacy run | **DEFERRED** — audit via artefacts E2E `-UseReal` documenté |

## Recettes de revalidation

```powershell
# Gate CI / développement (fake)
.\scripts\test_natal_simplified_e2e.ps1

# Monitoring OpenAI (facturé, ~90 s)
.\scripts\test_natal_simplified_e2e.ps1 -UseReal -SubmitProfile -TimeoutSec 900

# Unités
cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests
cargo test -p astral_llm_api --test astral_llm_simplified_reading_tests
```

## Signature

| Champ | Valeur |
|-------|--------|
| Produit | Natal simplifié V1 (moteur v2.4) |
| Statut | **CLOSED WITH MONITORING** |
| Date | 2026-06-06 |
| Dernier smoke OpenAI | `output/natal_simplified_openai/2026-06-06T100348Z/` — 7/7 |

Gate REV-015 : **CLOSED**.
