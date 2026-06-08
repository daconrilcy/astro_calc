# REV-001 - Contracts, catalogue et scan profile

## Contexte

Service cible : `horoscope_premium_next_7_days_natal`.

## Checks

- Le service est present dans `horoscope_services` et `llm_integration_services`.
- `service_has_v1_orchestrator` reconnait le service avant passage en `beta`.
- `premium_rich` est charge depuis `horoscope_detail_profiles`.
- `six_hour_7_days` est charge depuis `horoscope_scan_profiles`.
- Les schemas period acceptent Basic et Premium sans changer de version.

## Findings

- P1 : les schemas period etaient verrouilles sur `horoscope_basic_next_7_days_natal`.
- P1 : le scan etait code en dur sur `daily_noon_7_days`.

## Corrections

- Schemas calculator/LLM ouverts aux deux services.
- Builder period pilote par service/catalogue.
- Profil `six_hour_7_days` ajoute avec 4 snapshots par jour.

## Statut

Closed - aucun P0/P1 ouvert.
