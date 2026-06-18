# Review adversariale frontieres features — decouplage gateway

Date : 2026-06-17

## Perimetre

- Suppression des dependances Cargo directes de `astral_gateway` vers `astral_calculator`, `astral_llm_application`, `astral_llm_domain` et `astral_llm_infra`.
- Maintien de l'orchestration publique dans `astral_gateway`.
- Conservation des builders et validations editoriales LLM dans les couches LLM.

## Cycle 1 — Findings

- P1 — La gateway utilisait les DTO LLM Rust pour natal, ce qui couplait la facade publique au domaine LLM.
- P1 — La gateway reutilisait le client calculateur de `astral_llm_infra`.
- P2 — Les parcours horoscope utilisaient directement les builders/validators applicatifs LLM.

## Corrections Cycle 1

- Remplacement de la frontiere gateway/LLM par des payloads JSON `serde_json::Value`.
- Ajout d'un client HTTP calculateur local a `astral_gateway` utilisant uniquement `/v1/internal/calculations/*`.
- Ajout d'endpoints internes LLM `render-gateway` pour reconstruire et valider les requetes writer cote LLM apres calcul.
- Construction locale, cote gateway, des requetes calculateur horoscope depuis les donnees canoniques `json_db`.
- Ajout d'un test de gouvernance interdisant les dependances Cargo internes dans `astral_gateway`.

## Cycle 2 — Re-review

- Verification de `astral_gateway/Cargo.toml` : aucune dependance aux crates internes interdites.
- Verification des imports `astral_gateway/src` : aucune utilisation de types LLM application/domain/infra.
- Verification que les appels calculateur restent sur les routes internes canoniques.

### Finding Cycle 2

- P1 — La premiere correction faisait porter a `astral_gateway` des references compile-time a `json_db` pour reconstruire des requetes calculateur horoscope. Cela reintroduisait du referentiel canonique dans la facade publique.

### Correction Cycle 2

- Suppression des lectures `json_db` et des builders de catalogue dans `astral_gateway`.
- Ajout d'endpoints internes LLM de construction de requete calculateur horoscope:
  `/v1/internal/horoscope/daily/calculation-request` et
  `/v1/internal/horoscope/period/calculation-request`.
- La gateway orchestre le parcours public via des ports JSON : elle obtient d'abord la requete calculateur depuis l'API interne LLM, appelle `astral_calculator_http`, puis renvoie le calcul a l'API interne LLM pour rendu.

## Cycle 3 — Re-review

- Verification que `astral_gateway` ne depend ni des crates internes ni des fichiers `json_db`.
- Verification que les builders canoniques restent cote LLM application/API.
- Verification que les tests gateway couvrent le flux build-calculation-request -> calculator -> render.
- Ajout d'un garde de gouvernance interdisant `json_db` et `include_str!` dans `astral_gateway`.

## Statut

Statut: closed

Aucun finding ouvert.
