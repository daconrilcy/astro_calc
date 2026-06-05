# REV-012 — Audit documentation natal simplifié

Date : 2026-06-05 (alignement doc ↔ code runtime)

| Document | Statut | Corrections appliquées |
|----------|--------|------------------------|
| `docs/natal_simplified_reading_contract.md` | **Aligné** | Enveloppe succès (`GenerateReadingResponse`), HTTP 400 vs 422 selon endpoint, `reading_completeness: partial` seul, `planets{}` dans simplified payload, normalisation fact_id |
| `docs/natal_simplified_forbidden_topics.md` | **Aligné** | Ordre validateurs (script guard dans SafetyGuard, ReadingQualityValidator non bloquant) |
| `docs/BASIC_PAYLOAD_IMPLEMENTATION.md` § Natal simplifié | **Aligné** | Chaîne garde-fous, HTTP 400/422, note constante `PROFILE_INTERPRETATION_EXCLUDED` |
| `docs/Astral_llm_implementation.md` § simplified | **Aligné** | Ordre validateurs, enveloppes HTTP, script guard intégré |
| `docs/GUIDE_DEBUTANT_DOCKER.md` §9 | **Aligné** | `reading_completeness: partial`, distinction 400 orchestration / 422 calculateur |
| `contracts/integration/engine_to_reading_mapping.md` | **OK** | Déjà cohérent (`reading_completeness: partial`, llm_payload complet) |
| `docs/reviews/natal_simplified/REV-011-adversarial-findings.md` | **OK** | F-07 (profile_excluded en dur) toujours ouvert |

## Écarts doc → code corrigés

| Sujet | Doc avant | Code runtime |
|-------|-----------|--------------|
| Ordre garde-fous | `SafetyGuard` + `reading_script_guard` séparés | `reading_script_guard` **dans** `SafetyGuard::validate_response` |
| HTTP erreurs entrée orchestration | 422 généralisé | **400** `INVALID_INPUT`, sans enveloppe `{ calculation, reading }` |
| HTTP erreurs calculateur seul | (implicite) | **422** `VALIDATION_FAILED` |
| `reading_completeness` | `partial \| simplified` | Toujours **`partial`** (`payload.rs` / `reading_hint`) |
| Miroir `planets{}` | Smoke natal_light uniquement | Aussi dans **`simplified_payload.payload`** |
| Normalisation `mercury.sign` | « avant validation » | `normalize_chapter_astro_basis_fact_ids` post-parse LLM |

## Écarts connus (hors doc produit)

| ID | Sujet | Statut |
|----|-------|--------|
| F-07 | `PROFILE_INTERPRETATION_EXCLUDED` en constante Rust | Migration DB à planifier — documenté |
| — | Client calculateur LLM mappe rejet calculateur en `InvalidInput` (400) | Comportement implémenté, doc orchestration mise à jour |
| — | Assertions PS1 acceptent `reading_completeness ∈ { partial, simplified }` | Tolérance forward-compat ; runtime n'émet que `partial` |

Gate REV-012 : **OK** — documentation produit alignée sur le code post-recette v2.4.
