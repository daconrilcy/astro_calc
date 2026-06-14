# REV-MC-002 Horoscope Cleanup

Statut : closed after corrections

Findings :

- P1 la gateway `horoscope` dependait encore des builders calculator exposes par `astral_llm_application`
- P1 `UnifiedReadingOrchestrator` gardait un dispatch central stringly typed pour chaque code horoscope
- P2 le transfert des builders laissait du code mort et des warnings qui brouillaient la frontiere calculator/llm
- P2 `integration_services_tests` n'etait pas aligne sur l'enrichissement typed de `IntegrationService`
- P2 `raw_provider_trace` etait flaky en suite complete car le test dependait de variables d'environnement globales

Corrections appliquees :

- bascule des use cases gateway `horoscope` vers `astral_calculator::horoscope::{build_horoscope_daily_calculation_request_from_public, build_horoscope_period_calculation_request_from_public}`
- delegation des builders legacy `astral_llm_application` vers `astral_calculator` pour conserver la compatibilite sans garder la logique calculatoire dans LLM
- reduction de `UnifiedReadingOrchestrator` a un routage par descripteur partage `horoscope_service_descriptor`, avec branchement par famille de contrat `daily` vs `period`
- remplacement du point d'entree worker par `IntegrationJobExecutor`, route sur `IntegrationService` au lieu d'un dispatch central historique
- alignement de l'API d'integration et du worker sur `supports_integration_service` comme source unique de support d'execution
- suppression du code mort et des champs inutilises restants apres transfert
- realignement de `tests/integration_services_tests.rs` sur le contrat enrichi
- realignement de `tests/integration_jobs_tests.rs` sur le contrat enrichi
- stabilisation de `raw_provider_trace` via un helper d'ecriture pur teste sans dependance au gating par environnement
- factorisation des notes de mapping horoscope du catalogue API sur le registre partage au lieu d'une enumeration manuelle des codes

Validation :

- `cargo test -p astral_gateway`
- `cargo test -p astral_llm_application`
- `cargo test -p astral_llm_api --test contracts_publish_tests`
- `cargo test -p astral_llm_api --test integration_jobs_tests`
- `cargo test -p astral_llm_api --test integration_services_tests`
- `cargo test -p astral_llm_worker`

Risques residuels :

- aucun finding actif sur le dispatch central restant dans ce perimetre
- le catalogue legacy conserve encore des contrats publics historiques tant que l'extinction complete n'est pas engagee, mais ce n'est plus un risque d'orchestration centralisee
