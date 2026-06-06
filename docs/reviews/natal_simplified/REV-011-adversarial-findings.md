# Findings review natal simplifie (adversarial)

Date : 2026-06-05

## Corriges dans ce lot

| ID | Severite | Finding | Correctif |
|----|----------|---------|-----------|
| F-03 | P0 | Pas de whitelist `allowed_astro_basis_fact_ids` cote validateur | `simplified_reading_guard.rs` + branche dans `generate_reading_use_case` |
| F-02 | P0 | SafetyGuard inefficace sur affirmations FR de signes bloques | `blocked_sign_affirmation_violations` + `profile_excluded_affirmation_violations` |
| F-05 | P1 | Golden fixture obsolete / contradictoire | Split `stable_1990-06-15` + `equinox_1990-03-21` |
| F-06 | P1 | Route reading toujours HTTP 200 | `simplified_reading_http_status` dans `routes.rs` |
| F-09 | P1 | Pas de garde ASC sur `complete_birth_data` | Assertion PS1 `profile_excluded` + whitelist astro_basis |
| F-10 | P1 | FakeProvider metadata / basis fragiles | contract_version + filtre blocked IDs |
| F-11 | P1 | Payload LLM contient faits bloques | `scrub_simplified_payload_for_llm` |
| F-17 | P2 | Compteurs angular exposes au LLM | Suppression `position_count` / `house_cusp_count` / `aspect_count` du payload prompt |
| F-12 | P2 | Assertions PS1 incompletes | blocked cap allowed, astro_basis whitelist, intersection vide |

## Ouverts / differe

| ID | Severite | Finding | Raison |
|----|----------|---------|--------|
| F-04 | P1 | `forbidden_topics` non consomme | **Fermé** — renommé `forbidden_interpretation_topics` + miroir déprécié |
| F-07 | P1 | `PROFILE_INTERPRETATION_EXCLUDED` en dur | Migration DB produit a planifier |
| F-08 | P2 | Schema projection vs embed | Artefact standalone ; embed `astro_simplified_natal_response_v1` fait foi |
| F-13 | P2 | Cas negatifs reading incomplets | Partiellement couvert calculateur ; reading partage validateur entree |
| F-14 | P2 | Script guard monolingue | Scope V1 fr ; extension locale ulterieure |
| F-15 | P2 | E2E bootstrap / smoke etroit | Smoke date_only volontaire ; E2E complet 12+7 cas |
