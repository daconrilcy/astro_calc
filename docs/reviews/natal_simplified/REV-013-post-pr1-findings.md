# REV-013 — Review post-PR1 (F-07 + reading_completeness)

Date : 2026-06-06  
Périmètre : migration `profile_excluded_feature_codes` → DB, constante `READING_COMPLETENESS_V1`, assertions E2E strictes.

## Verdict

**Gate : OK** (implémentation + tests unitaires ; E2E fake 24/24 requis après rebuild calculateur).

## Findings

| ID | Sévérité | Finding | Statut |
|----|----------|---------|--------|
| R13-01 | — | Exclusions liées à `profile_code`, pas `calculation_policies` | **Fixed** — table `astral_simplified_profile_feature_exclusions` |
| R13-02 | — | Table vide → erreur explicite, pas fallback constante | **Fixed** — `service.rs` `InvalidRuntimeTable` |
| R13-03 | — | Ordre goldens préservé via `sort_order` seed 10/20/30/40 | **Fixed** |
| R13-04 | — | `computed_scope_code` nullable = exclusion globale | **Fixed** — seed `null` V1 |
| R13-05 | — | Runtime + PS1 = `partial` uniquement ; schema conserve `simplified` reserved | **Fixed** |
| R13-06 | — | Doc PR1 BASIC_PAYLOAD + contrat lecture | **Fixed** |
| R13-07 | — | `forbidden_interpretation_topics` inchangé fonctionnellement | **OK** |

## Tests gate PR1

```powershell
cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests
docker compose up -d --build astral_calculator_api
.\scripts\test_natal_simplified_e2e.ps1
```

## Doc PR1 (REV-013-doc)

| Document | Statut |
|----------|--------|
| `docs/BASIC_PAYLOAD_IMPLEMENTATION.md` | **Aligné** — table exclusions, F-07 closed |
| `docs/natal_simplified_reading_contract.md` | **Aligné** — source DB, `partial` strict |
| Schémas `astro_simplified_natal_response_v1` | **Aligné** — description `simplified` reserved |
| `docs/GUIDE_DEBUTANT_DOCKER.md` | À compléter REV-016 (import table) |
| `docs/release-evidence/natal-simplified-v1.md` | v1.1 prévu PR2/D |

Gate REV-013 : **OK** après E2E fake 24/24 post-rebuild.
