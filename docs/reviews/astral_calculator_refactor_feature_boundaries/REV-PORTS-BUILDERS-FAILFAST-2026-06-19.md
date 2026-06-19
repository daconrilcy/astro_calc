# REV-PORTS-BUILDERS-FAILFAST-2026-06-19

Statut: closed

## Scope

Validation adversariale de la frontière `features/application/engine/infra` après suppression des derniers imports `infra::db` depuis les helpers métier et les builders horoscope.

## Boundary checks

- `engine/*` ne référence plus `ReferenceRepository` ni `ProjectionRepository` directement.
- `features/horoscope/builders.rs` dépend d'un port de catalogue orienté usage au lieu de `HoroscopeRepository`.
- `features/simplified/service.rs` importe la validation canonique `features::natal::validate` et non la façade runtime.
- Les repositories SQL restent les seuls adaptateurs concrets des lectures DB concernées.

## Findings adversariaux

- Medium: la garde de frontière `simplified_and_horoscope_do_not_import_natal_internals` était devenue trop permissive et ne protégeait plus réellement la frontière si un fichier importait `validate` plus un autre module natal interne.
- Medium: le pré-filtrage des profils `is_enabled = false` dans `features/horoscope/builders.rs` déplaçait une règle de validation hors de la couche dédiée `astral_time_window`.

## Corrections de follow-up

- La règle de gouvernance retire uniquement le chemin autorisé `crate::features::natal::validate` avant d'évaluer les imports résiduels.
- La validation d'activation des profils de période reste centralisée dans `astral_time_window::PeriodWindowResolver`.

## Verification

- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator --test horoscope_builders_tests`

## Re-review

Aucun finding ouvert.
