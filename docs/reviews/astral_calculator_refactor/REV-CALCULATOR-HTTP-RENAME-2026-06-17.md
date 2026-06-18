# Review adversariale — renommage calculator HTTP

Date : 2026-06-17

## Perimetre

- Renommage sans alias transitoire de `astral_calculator_api` vers `astral_calculator_http`.
- Mise a jour Cargo, Docker, scripts, contrats, tests et documentation active.
- Conservation des routes HTTP existantes, dont `/v1/internal/calculations/*` et les aliases legacy `/v1/calculations/*`.

## Cycle 1 — Findings

- P1 — Les surfaces actives pouvaient encore reintroduire l'ancien nom sans garde de gouvernance dedie.
- P2 — Le renommage Docker devait verifier a la fois le nom du service Compose, le container et le chemin Dockerfile.

## Corrections Cycle 1

- Ajout d'un test de gouvernance bloquant toute reference active a `astral_calculator_api` hors historique de review.
- Bascule Compose/Dockerfile/container vers `astral_calculator_http`.
- Mise a jour des noms de crate, binaire, tests et commandes courantes.

## Cycle 2 — Re-review

- Recherche recursive des references actives a `astral_calculator_api`.
- Verification des manifests Cargo et du service Compose.
- Verification que les routes HTTP contractuelles ne sont pas supprimees par le renommage.

## Statut

Statut: closed

Aucun finding ouvert.

