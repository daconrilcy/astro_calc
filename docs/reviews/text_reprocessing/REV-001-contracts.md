# REV-001 - Contracts

## Perimetre

Contrats `astral_llm_domain::text_reprocessing`.

## Findings adversariales

- Aucun P0/P1/P2/P3 ouvert.
- Note: les codes langue/service sont ouverts par `String` par conception. La validation non bloquante est portee par les registres et les warnings du pipeline.

## Corrections appliquees

- `TextLanguage` et `TextService` restent extensibles.
- `TextRetreatmentOperation` derive `Hash` pour permettre la selection par `HashSet`.
- Test domain dedie: `text_reprocessing_contracts_accept_open_language_and_service_codes`.
