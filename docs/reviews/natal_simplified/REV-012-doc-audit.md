# REV-012 — Audit documentation natal simplifié

Date initiale : 2026-06-05 (alignement doc ↔ code runtime v2.4)

Mise à jour : 2026-06-05 (post-certification E2E + finitions qualité + contrat HTTP + rename champs)

| Document | Statut | Notes |
|----------|--------|-------|
| `docs/natal_simplified_reading_contract.md` | **Aligné** | Matrice HTTP 422/400, `llm_payload`, post-traitement serveur, OpenAPI |
| `docs/natal_simplified_forbidden_topics.md` | **Aligné** | `forbidden_interpretation_topics`, pipeline `single_pass_hardening`, E2E 12+7+5 |
| `docs/BASIC_PAYLOAD_IMPLEMENTATION.md` § Natal simplifié | **Aligné** | Typographie FR, summary compact, E2E phase 2b |
| `docs/Astral_llm_implementation.md` § simplified | **Aligné** | Ordre validateurs + post-traitement, recette E2E |
| `docs/GUIDE_DEBUTANT_DOCKER.md` §9 | **Aligné** | 24 assertions E2E, rebuild calculateur + LLM, champs llm_payload |
| `contracts/calculator/openapi.yaml` | **Aligné** | 422 `VALIDATION_FAILED` documenté |
| `contracts/llm/openapi.yaml` | **Aligné** | 400 entrée / 422 `safety_rejected` orchestration |
| `contracts/README.md` | **Aligné** | Suite E2E 12 + 7 + 5 |

## Écarts doc → code corrigés (v2.4 initiale)

| Sujet | Doc avant | Code runtime |
|-------|-----------|--------------|
| Ordre garde-fous | `SafetyGuard` + `reading_script_guard` séparés | `reading_script_guard` **dans** `SafetyGuard::validate_response` |
| HTTP erreurs entrée orchestration | 422 généralisé | **400** `INVALID_INPUT`, sans enveloppe `{ calculation, reading }` |
| HTTP erreurs calculateur seul | (implicite) | **422** `VALIDATION_FAILED` |
| `reading_completeness` | `partial \| simplified` | Toujours **`partial`** |
| Miroir `planets{}` | Smoke natal_light uniquement | Aussi **`simplified_payload.payload`** |
| Normalisation `mercury.sign` | « avant validation » | `normalize_chapter_astro_basis_fact_ids` post-parse LLM |

## Écarts doc → code corrigés (finitions post-recette)

| Sujet | Correction doc |
|-------|----------------|
| Apostrophes FR (`l impression`) | Post-traitement `french_typography.rs` documenté |
| Summary tronqué `…` | Summary compact 1–2 phrases (`build_compact_summary_from_body`) |
| `interpretive_role: domain_score` | Normalisation serveur → `core/supporting/nuance` |
| OpenAPI LLM annonçait 422 pour entrée invalide | **400** documenté |
| E2E négatifs orchestration | Phase **2b** + `test_natal_simplified_reading.ps1 -NegativeOnly` |
| `forbidden_topics` ambigu | Renommé **`forbidden_interpretation_topics`** + miroir déprécié |

## Écarts connus (hors doc produit)

| ID | Sujet | Statut |
|----|-------|--------|
| F-07 | `PROFILE_INTERPRETATION_EXCLUDED` en constante Rust | **Fermé** — table `astral_simplified_profile_feature_exclusions` (REV-013) |
| F-04 | `forbidden_topics` ambigu pour consommateurs API | **Fermé** — rename + alias compat |
| — | Client calculateur LLM mappe rejet calculateur en `InvalidInput` (400) | Comportement implémenté, doc orchestration |
| — | Tolérance PS1 `reading_completeness ∈ { partial, simplified }` | **Fermé** — PS1 strict `partial` uniquement (REV-013) |

> Post-risques résiduels : voir [`REV-016-doc-audit.md`](REV-016-doc-audit.md) et [`REV-015-final-closure.md`](REV-015-final-closure.md).

Gate REV-012 : **OK** — documentation produit alignée sur le code et la recette E2E certifiée (double run verte 2026-06-05).
