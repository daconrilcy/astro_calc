# REV-APPLICATION-PORTS follow-up 1

Statut: closed

## Scope

Re-review du finding restant: services applicatifs encore câblés directement sur `infra/db`.

## Findings initiaux

- High: `NatalCalculationService`, `HoroscopeService`, `SimplifiedNatalService` et `EngineFacadeService` importaient des repositories SQL concrets.

## Corrections

- Ajout de ports applicatifs fins dans `application::ports`.
- Les services applicatifs dépendent maintenant des traits `ReferenceCatalog`, `ProjectionCatalog`, `HoroscopeCatalog`, `PayloadCatalogStore`, `SimplifiedCatalogStore` et `NatalCalculationStore`.
- Le wiring concret PostgreSQL reste dans `runtime::build_runtime_service`.
- Ajout d'un test de gouvernance interdisant les imports `infra::db` dans les services applicatifs.

## Re-review

Aucun finding ouvert.
