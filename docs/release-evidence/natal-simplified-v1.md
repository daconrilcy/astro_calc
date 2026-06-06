# Natal simplifié — preuve de clôture (release evidence v1)

**Statut : CLOSED WITH MONITORING**  
**Produit :** natal simplifié moteur **v2.4** (`interpretation_profile_code` : `natal_simplified`)  
**Date de clôture :** 2026-06-06  
**Baseline git :** `381c8a1fcf5fe55f6e38464ea1702a6288951c2e` — `docs(natal_simplified): aligner doc sur certification E2E v2.4`

Ce document constitue le livrable de preuve pour la clôture produit **natal simplifié V1** (parcours one-shot « données partielles → lecture indicative »). Il complète les reviews REV-001…012 et la doc métier ; il ne remplace pas les contrats OpenAPI ni les schémas JSON.

---

## Verdict

| Gate | Résultat |
|------|----------|
| Reviews adversariales REV-001…011 | **OK** (findings fermés) |
| Audit documentation REV-012 | **OK** |
| Recette E2E intégrée `test_natal_simplified_e2e.ps1` | **24/24 OK** (double run verte) |
| Contrats HTTP 422/400 documentés + testés | **OK** |
| Clé canonique `llm_payload.forbidden_interpretation_topics` | **OK** (miroir déprécié `forbidden_topics` émis) |
| OpenAPI calculateur + LLM | **Alignées** |

**Périmètre gelé** pour maintenance corrective : moteur `astral_calculator/src/simplified/*`, endpoints orchestrés, profil `natal_simplified`, pipeline `single_pass_hardening`, validateurs simplified, scripts E2E et assertions PS1 associées. Toute évolution fonctionnelle = nouveau cycle (CS / REV).

---

## Périmètre livré

### Endpoints

| Rôle | Méthode | Contrat requête | Contrat réponse |
|------|---------|-----------------|-----------------|
| Calculateur seul | `POST /v1/calculations/natal/simplified` | `astro_simplified_natal_request_v1` | `astro_simplified_natal_response_v1` |
| Orchestration one-shot | `POST /v1/readings/natal/simplified` | idem + `user_language`, `audience_level` | enveloppe `{ calculation, reading, reading_completeness, run_id }` |

### Matrice HTTP (entrée invalide)

| Endpoint | HTTP | Code métier | Enveloppe orchestrée |
|----------|------|-------------|----------------------|
| Calculateur seul | **422** | `VALIDATION_FAILED` | Non |
| Orchestration | **400** | `INVALID_INPUT` | Non |
| Orchestration (calcul OK, garde-fou lecture) | **422** | `safety_rejected` | Oui (enveloppe complète) |

Référence : [`docs/natal_simplified_reading_contract.md`](../natal_simplified_reading_contract.md), [`contracts/calculator/openapi.yaml`](../../contracts/calculator/openapi.yaml), [`contracts/llm/openapi.yaml`](../../contracts/llm/openapi.yaml).

### Profil LLM

- Fichier : `config/natal_interpretation_profiles/natal_simplified.json`
- Mode génération : `single_pass` avec durcissement (`single_pass_hardening.rs`)
- Post-traitement serveur : disclaimer légal, typographie FR, summary compact, normalisation rôles interprétatifs, sanitisation script

---

## Certification E2E

### Commande de recette (référence)

Prérequis Docker : stack locale up, bootstrap référentiel, **rebuild des deux services** après changement calculateur :

```powershell
docker compose up -d --build astral_calculator_api astral_llm_api
.\scripts\docker_bootstrap.ps1
.\scripts\test_natal_simplified_e2e.ps1
```

Par défaut la suite E2E active **`-ForceFake`** (deterministe, sans OpenAI). Recette OpenAI optionnelle : `-UseReal -SubmitProfile -TimeoutSec 900`.

### Résultats certifiés

| Phase | Cas | Résultat | Notes |
|-------|-----|----------|-------|
| 1 — Calculateur | 7 positifs + 5 négatifs | **12/12 OK** | Négatifs → **422** `VALIDATION_FAILED` |
| 2 — Lecture orchestrée | 7 positifs | **7/7 OK** | Provider fake, profil resoumis |
| 2b — Négatifs orchestration | 5 cas | **5/5 OK** | **400** `INVALID_INPUT`, sans enveloppe |

**Run de clôture utilisateur** (terminal local, suite complète) :

```text
Resultat : 12 OK, 0 FAIL sur 12 cas   (calculateur)
Resultat :  7 OK, 0 FAIL sur  7 cas   (lectures)
Resultat :  5 OK, 0 FAIL sur  5 cas   (négatifs orchestration)
Suite E2E natal simplifie OK.
```

Exemples `run_id` (phase 2, fake) :

| Cas | `run_id` | Mots (approx.) |
|-----|----------|----------------|
| `date_only` | `5fdfb84d-5026-4653-8d7d-13a1607390f3` | 386 |
| `complete_birth_data` | `2968046d-cbef-42b3-a19f-da83a5369877` | 333 |
| `date_only_equinox_window` | `a00b502f-4c7c-4924-904f-f2fd5c71f562` | 285 |

### Artefacts JSON (preuve locale)

Générés par défaut sous `output/natal_simplified/` (non versionnés git) :

| Répertoire | Contenu |
|------------|---------|
| `calculator/` | Réponses `POST /v1/calculations/natal/simplified` (12 fichiers) |
| `reading/` | Réponses `POST /v1/readings/natal/simplified` (7 succès + 5 `.error.json`) |
| `e2e_summary.json` | Index des fichiers + horodatage UTC |

Dernière génération indexée : `e2e_summary.json` → `generated_at_utc` **2026-06-06T08:53:59Z** (run agent + run utilisateur post-rebuild calculateur).

### Spot-check obligatoire — `llm_payload`

Sur **chaque** artefact positif (`calculator/*.json` et `reading/*/calculation.llm_payload`), les deux clés suivantes doivent être présentes avec **les mêmes valeurs** :

```json
"forbidden_interpretation_topics": ["ascendant", "house_placements", "houses", "sect", …],
"forbidden_topics": ["ascendant", "house_placements", "houses", "sect", …]
```

Exemple extrait (`output/natal_simplified/calculator/complete_birth_data.json`, scope `angular_chart`) :

```json
"forbidden_interpretation_topics": ["ascendant", "house_placements", "houses", "sect"],
"forbidden_topics": ["ascendant", "house_placements", "houses", "sect"]
```

Gate E2E automatisé : `Assert-SimplifiedCalculatorResponse` dans `scripts/lib/simplified_natal_assertions.ps1` (vérifie présence + cohérence miroir).

> **Piège opérationnel documenté :** après le rename (`adc050a`), un rebuild **`astral_calculator_api` seul** est requis. Un conteneur calculateur stale n’émet que `forbidden_topics` ; la recette E2E échoue alors avec `forbidden_interpretation_topics absent (rebuild astral_calculator_api)`.

---

## Tests unitaires / intégration (gate CI locale)

```powershell
cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests
cargo test -p astral_calculator_api --test astral_calculator_api_tests
cargo test -p astral_llm_api --test contracts_publish_tests
cargo test -p astral_llm_api --test astral_llm_simplified_reading_tests
```

Golden calculateur (incluent `forbidden_interpretation_topics` + miroir) :

- `tests/golden/simplified_natal_calculation_stable_1990-06-15.json`
- `tests/golden/simplified_natal_calculation_equinox_1990-03-21.json`

Test Rust dédié au rename : `llm_controls_block_ambiguous_and_allow_stable`, `llm_controls_deserializes_legacy_forbidden_topics_alias` (`tests/simplified_natal_tests.rs`).

---

## Commits structurants (post-moteur v2.4)

| SHA | Message |
|-----|---------|
| `d424701` | feat(natal_simplified): moteur v2.4, endpoints orchestrés et doc débutant |
| `f27708c` | feat(natal_simplified): gardes anti-hallucination, recette E2E et doc alignée |
| `7276e54` | fix(astral_llm): injecter le disclaimer legal en single_pass |
| `20ecaa8` | feat(natal_simplified): durcir single_pass contre contamination script |
| `e7b40a8` | feat(natal_simplified): finition typographie FR et summary compact |
| `671c43f` | docs(natal_simplified): aligner OpenAPI et E2E sur 422/400 |
| `adc050a` | refactor(natal_simplified): renommer forbidden_interpretation_topics |
| `381c8a1` | docs(natal_simplified): aligner doc sur certification E2E v2.4 |
| `c30648d` | docs(natal_simplified): clôture V1 avec preuve E2E et gate llm_payload |
| `ba65f94` | feat(natal_simplified): fermer risques résiduels F-07 et reading_completeness |

---

## Reviews et documentation

| Référence | Statut |
|-----------|--------|
| [`docs/reviews/natal_simplified/INDEX.md`](../reviews/natal_simplified/INDEX.md) | REV-001…016 **closed** |
| [`docs/reviews/natal_simplified/REV-015-final-closure.md`](../reviews/natal_simplified/REV-015-final-closure.md) | Clôture finale V1 |
| [`docs/reviews/natal_simplified/REV-GLOBAL-adversarial.md`](../reviews/natal_simplified/REV-GLOBAL-adversarial.md) | Gate OK après G-001…G-011 |
| [`docs/reviews/natal_simplified/REV-012-doc-audit.md`](../reviews/natal_simplified/REV-012-doc-audit.md) | Doc produit alignée runtime |
| [`docs/natal_simplified_reading_contract.md`](../natal_simplified_reading_contract.md) | Contrat produit |
| [`docs/natal_simplified_forbidden_topics.md`](../natal_simplified_forbidden_topics.md) | Sémantique garde-fous LLM |
| [`docs/GUIDE_DEBUTANT_DOCKER.md`](../GUIDE_DEBUTANT_DOCKER.md) §9 | Tutoriel + recette E2E |
| [`docs/BASIC_PAYLOAD_IMPLEMENTATION.md`](../BASIC_PAYLOAD_IMPLEMENTATION.md) | Impl. calculateur |
| [`docs/Astral_llm_implementation.md`](../Astral_llm_implementation.md) | Impl. LLM simplified |

---

## Risques résiduels — statut post-PR1 (2026-06-06)

| ID | Sujet | Statut | Preuve |
|----|-------|--------|--------|
| F-07 | Exclusions profil en constante Rust | **CLOSED** | Table `astral_simplified_profile_feature_exclusions`, REV-013, E2E 24/24 |
| `reading_completeness` | Tolérance PS1 `simplified` | **CLOSED** | Runtime + PS1 = `partial` strict, REV-013 |
| Qualité OpenAI | Variabilité provider | **CLOSED WITH MONITORING** | `-StrictOpenAiQuality` + REV-014 ; smoke **7/7** (2026-06-06) |

Recette OpenAI (monitoring périodique) :

```powershell
.\scripts\test_natal_simplified_e2e.ps1 -UseReal -SubmitProfile -TimeoutSec 900
```

Dernier smoke certifié : `output/natal_simplified_openai/2026-06-06T100348Z/` — **7/7**, P0=0, P1=0, ~87 s wall time.

| Volet D | Logs Rust privacy run (contenu prompt/réponse) | **DEFERRED** | Audit manuel via artefacts E2E `-UseReal` ; doc Astral_llm § simplified |

Exemples `run_id` (phase 2, OpenAI) :

| Cas | `run_id` | Mots |
|-----|----------|------|
| `date_only` | `1d1082a1-7d4e-4b96-94ea-989f7ad4ae15` | 263 |
| `complete_birth_data` | `1006bb48-8baa-4e0b-b362-753555469437` | 319 |
| `date_only_equinox_window` | `6bbc98b7-9f38-4c3e-bb79-f2e03d45447f` | 250 |

### Règles de réouverture du risque OpenAI

Réévaluer le statut **CLOSED WITH MONITORING** (nouveau cycle REV / release evidence) si l’un des seuils suivants est atteint sur des runs **OpenAI réels** (`-UseReal`, hors fake CI) :

| Signal | Seuil |
|--------|--------|
| `safety_rejected` | **2** occurrences sur **20** runs positifs consécutifs |
| Hallucination P0 confirmée | **1** (ASC/maison affirmatif, `astro_basis` hors whitelist, signe bloqué affirmé) |
| Contamination langue / script | **1** confirmée |
| Retours utilisateurs qualifiés négatifs | **3** sur le même type de défaut |

Artefacts de preuve : `output/natal_simplified_openai/{timestamp}/` + `quality_summary.json` (compteurs P0/P1, warnings P2, `gate_passed`, `model` depuis `GET /v1/providers`).

---

## Risques résiduels (historique v1 initiale)

## Revalidation rapide

```powershell
# Spot-check clés llm_payload sur un artefact
Select-String -Path output\natal_simplified\calculator\complete_birth_data.json -Pattern "forbidden_interpretation_topics|forbidden_topics"

# Recette complète
.\scripts\test_natal_simplified_e2e.ps1

# Smoke Docker minimal
.\scripts\docker_simplified_natal_smoke.ps1
```

---

## Signature de clôture

| Champ | Valeur |
|-------|--------|
| Produit | Natal simplifié V1 (moteur v2.4) |
| Statut | **CLOSED WITH MONITORING** |
| Recette E2E | **24/24** (fake provider) ; OpenAI **7/7** (REV-014) |
| Review finale | **REV-015** |
| Baseline git | `931e810` (+ doc REV-015/016) |
| Date | 2026-06-06 |

Toute réouverture fonctionnelle requiert un nouveau numéro de release evidence et une mise à jour explicite de ce fichier.
