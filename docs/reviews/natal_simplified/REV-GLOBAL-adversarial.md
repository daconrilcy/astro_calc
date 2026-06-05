# REV-GLOBAL — Review adversariale globale (natal simplifié v2.4)

Date : 2026-06-05  
Périmètre : CS-001 → CS-011, alignement plan v2.4 figé vs code + contrats + docs + tests.

## Verdict

**Gate : OK après corrections** — écarts blocker/major identifiés lors de la première passe ont été corrigés dans cette review.

## Findings et statut

| ID | Sévérité | Finding | Statut |
|----|----------|---------|--------|
| G-001 | Blocker | `llm_controls` présents dans `astro_result.data` mais absents du prompt LLM → risque hallucination ASC/Lune | **Fixed** — `prompt_compiler.rs` injecte `prompt_constraints_block` dans `task_instructions` + `llm_controls` / `excluded_features` dans `data_payload` |
| G-002 | Blocker | `GUIDE_DEBUTANT_DOCKER.md` sans section natal simplifié | **Fixed** — section smoke `docker_simplified_natal_smoke.ps1` |
| G-003 | Blocker | `Astral_llm_implementation.md` sans doc `natal_simplified` | **Fixed** — endpoint, profil, contrôles anti-hallucination |
| G-004 | Major | Route reading sans validation explicite requête calculateur | **Fixed** — `validate_simplified_calculation_request` avant appel calculator |
| G-005 | Major | `contracts/llm/openapi.yaml` sans `/v1/readings/natal/simplified` | **Fixed** |
| G-006 | Major | Pas de golden fixture simplified | **Fixed** — `tests/golden/simplified_natal_calculation_stable_1990-06-15.json`, `simplified_natal_calculation_equinox_1990-03-21.json` |
| G-007 | Major | Tests anti-hallucination CS-009 insuffisants | **Fixed** — `tests/astral_llm_simplified_reading_tests.rs` (prompt + forbidden_wording + golden) |
| G-008 | Minor | Schéma `natal_simplified_structured_v1` avec `$ref` externe fragile | **Fixed** — `$defs` inline (source + `contracts/calculator/`) |
| G-009 | Minor | `reference_instant_utc` / `reference_based` non exposés en fenêtre | **Accepted** — code mort documenté ; pas requis par le plan v2.4 pour l'exposition API |
| G-010 | Doc | REV-005 : Lune 3+ signes physiquement rare sur ~50h | **Accepted** — test assoupli à 2+ signes (documenté REV-005) |
| G-011 | Minor | `.env.example` sans commentaire orchestration LLM → calculateur | **Fixed** |

## Alignement plan v2.4

| Objectif plan | Implémentation | OK |
|---------------|----------------|-----|
| Matrice input_precision / computed_scope | `astral_calculator/src/simplified/resolve.rs` | ✓ |
| Fenêtre incertitude ~50h / 24h locale | `uncertainty_window.rs` + policy DB | ✓ |
| Fiabilité faits stable / ambiguous | `facts.rs`, `payload.rs` | ✓ |
| Contrat `natal_simplified_structured_v1` | schémas + API | ✓ |
| Projection LLM allowed/blocked | `llm_payload` + prompt | ✓ |
| Endpoint calculateur | `POST /v1/calculations/natal/simplified` | ✓ |
| Profil `natal_simplified` | `config/natal_interpretation_profiles/natal_simplified.json` | ✓ |
| Endpoint reading orchestré | `POST /v1/readings/natal/simplified` | ✓ |
| Smoke E2E | `scripts/docker_simplified_natal_smoke.ps1` | ✓ |
| Données canoniques en base | 5 tables + seeds JSON | ✓ |
| Tests régression | `simplified_natal_tests`, `astral_calculator_api_tests`, `astral_llm_simplified_reading_tests` | ✓ |

## Tests exécutés (gate)

```powershell
cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests
cargo test -p astral_calculator_api --test astral_calculator_api_tests
cargo test -p astral_llm_api --test contracts_publish_tests
cargo test -p astral_llm_api --test astral_llm_simplified_reading_tests
```

## Risques résiduels (non bloquants)

1. **Qualité rédactionnelle OpenAI** — les gates `natal_simplified` sont non bloquantes ; un provider réel peut encore formuler malgré les contraintes (mitigation : `forbidden_wording` + task fragment profil).
2. **REV-001…012 antérieurs** — marqués OK avec 0 finding ; cette review globale remplace leur optimisme pour les écarts G-001…G-011.

## Recommandation

Périmètre **gelable** pour livraison v2.4 : moteur simplified, contrats, endpoints, profil LLM, smoke script, tests ci-dessus. Toute évolution (ex. exposition `reference_based` en API) = nouveau CS.
