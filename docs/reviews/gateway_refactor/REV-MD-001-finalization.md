# REV-MD-001 - Finalisation catalogue et surfaces publiees

Date: 2026-06-13

Scope:

- `astral_llm_api::integration_routes`
- `contracts/llm/integration_service_v1.schema.json`
- `contracts/llm/integration_service_contract_v1.schema.json`
- `docs/integration_api_contract.md`
- `docs/integration_api_guide.md`

## Findings

### P2 - Le catalogue ne distinguait pas explicitement surface publique V2 et surfaces legacy

Risque:

- un integrateur pouvait continuer a consommer une route sync legacy ou deduire lui-meme la route publique cible
- la frontiere gateway/async jobs restait implicite dans la documentation et dans les reponses catalogue

Correction:

- ajout de `api_surface` dans `GET /v1/services` et `GET /v1/services/{service_code}/contract`
- publication de `async_job_v1_status`, `sync_legacy_status`, `public_gateway_v2_status`, `recommended_entrypoint`
- documentation alignee pour poser `astral_gateway` comme facade publique et `astral_llm_api` comme surface technique jobs

Statut: corrige

### P2 - `quality_tier` etait encore derive par heuristique stringly typed

Risque:

- toute evolution de `service_code` pouvait casser silencieusement la projection catalogue
- la projection etait en contradiction avec l'objectif de suppression des inférences implicites

Correction:

- remplacement par un mapping explicite par `service_code`
- ajout d'un test de schema pour verrouiller la publication de `api_surface`

Statut: corrige

## Resultat de revue

- aucun finding P1/P2 ouvert apres corrections
- la separation gateway publique / async jobs / legacy sync est maintenant explicite au niveau contrat publie
- le reliquat legacy restant est documentaire et de compatibilite, plus structurel dans le dispatch central

## Validation

- `cargo test -p astral_llm_api --test integration_services_tests`
- `cargo test -p astral_llm_api --test contracts_publish_tests`
- `cargo test -p astral_llm_worker`
