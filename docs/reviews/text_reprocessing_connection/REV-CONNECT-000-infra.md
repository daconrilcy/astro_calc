# REV-CONNECT-000 - Infrastructure

## Review

Finding P2 corrige: les services auraient chacun construit des requetes `TextRetreatmentRequest` manuellement.

Correction: ajout de `text_reprocessing_service_adapter` avec helpers par service, conversion typed JSON pour natal, normalizer de parite et fonctions dediees.

## Verification

- `cargo test -p astral_llm_application text_reprocessing`
