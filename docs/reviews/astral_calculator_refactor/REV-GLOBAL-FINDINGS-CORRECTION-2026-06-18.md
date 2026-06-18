# REV-GLOBAL-FINDINGS-CORRECTION-2026-06-18

Statut: closed

## Scope

Review globale des corrections appliquées aux findings d'audit post-refactor.

## Findings initiaux

- High: ports applicatifs incomplets.
- High: repository runtime encore monolithique.
- Medium: `shared::astro_math` contenait un mapping canonique d'IDs.
- Medium: horoscope pouvait encore générer des faits synthétiques sans transits.

## Corrections

- Ports applicatifs ajoutés et services découplés des imports `infra/db`.
- `runtime_repository.rs` réduit à un helper résiduel.
- Résolution de mouvement déplacée sous `astrology`.
- Fallbacks horoscope `derived_*` supprimés du runtime source.
- Garde-fous de gouvernance ajoutés pour empêcher les régressions.

## Re-review

Aucun finding ouvert.
