# REV-PORTS-BUILDERS-FAILFAST-2026-06-19

Statut: closed

## Scope

Correction ciblée des findings restants sur les ports applicatifs, les builders horoscope et le comportement fail-fast des tests PostgreSQL.

## Findings initiaux

- High: `engine` et `features/horoscope/builders.rs` dépendaient encore des repositories SQL concrets.
- High: `engine/calculation_refs.rs` gardait des caches globaux `OnceLock` sur des références canoniques DB.
- Medium: certains tests DB continuaient à sortir silencieusement quand PostgreSQL n'était pas prêt.

## Corrections

- Ajout des ports `ReferenceSystemCatalog` et `HoroscopeBuilderCatalog`, plus des DTOs applicatifs minimaux orientés usage.
- Implémentation de ces ports dans `ReferenceRepository` et `HoroscopeRepository`, sans déplacer le SQL hors `infra/db`.
- Généralisation de `engine::calculation_refs`, `engine::env`, `engine::projection::profiles` et des builders horoscope sur les ports applicatifs.
- Suppression des caches globaux dans `engine/calculation_refs.rs`.
- Remplacement du helper DB dans `tests/horoscope_builders_tests.rs` par un faux catalogue en mémoire.
- Passage des helpers DB de `tests/engine_contract_tests.rs` et `tests/astral_calculator_http_tests.rs` en fail-fast explicite.
- Ajout d'un garde-fou de gouvernance interdisant `infra::db` dans `engine/*` et `features/horoscope/builders.rs`.

## Findings adversariaux

- Medium: `tests/refactor_governance_tests.rs` autorisait par erreur tout import `crate::features::natal::*` dès lors que le fichier importait aussi `crate::features::natal::validate`.
- Medium: `features/horoscope/builders.rs` filtrait les profils de période `is_enabled = false` avant `PeriodWindowResolver`, ce qui supprimait la sémantique native de profil désactivé et la remplaçait par un comportement de type "profil absent".

## Corrections de follow-up

- Le garde-fou de gouvernance ne retire plus que la sous-chaîne autorisée `crate::features::natal::validate` avant de vérifier qu'aucun autre chemin `crate::features::natal::` n'est importé.
- Les builders horoscope transmettent désormais tous les profils au resolver et conservent `is_enabled` dans la définition mappée, pour préserver la validation native côté `astral_time_window`.

## Verification

- `cargo fmt`
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator --test horoscope_builders_tests`
- `cargo test -p astral_calculator`
- `cargo test -p astral_calculator_http --test astral_calculator_http_tests`

## Resultat

- Les suites ciblées de gouvernance et builders passent.
- Les suites complètes DB échouent désormais explicitement avec `PoolTimedOut` tant que PostgreSQL n'est pas prêt, conformément à la politique fail-fast.

## Re-review

Aucun finding ouvert.
