# Mapping engine → reading (v1)

Transformation entre la reponse calculateur et la requete LLM.

## Source

Reponse `astro_engine_response_v1` du calculateur :

```json
{
  "response_contract_version": "astro_engine_response_v1",
  "audit_payload": {
    "contract_version": "natal_structured_v13",
    "payload": { }
  }
}
```

## Cible

Corps `POST /v1/readings/generate` :

```json
{
  "product_context": {
    "product_code": "natal_prompter",
    "interpretation_profile_code": "natal_basic",
    "user_language": "fr",
    "audience_level": "beginner"
  },
  "astro_result": {
    "contract_version": "<audit_payload.contract_version>",
    "chart_type": "natal",
    "data": "<audit_payload.payload>"
  },
  "astrologer_profile": {
    "tone": "warm",
    "jargon_level": "beginner",
    "wording_style": "clear"
  },
  "engine": {
    "provider": "openai",
    "model": "gpt-5.4-mini",
    "allow_fallback": true
  },
  "response_contract": {
    "output_schema_version": "natal_reading_v1",
    "generation_mode": "chapter_orchestrated",
    "format": "structured_json",
    "include_astro_sources": true,
    "include_legal_disclaimer": true
  }
}
```

## Regles

- `calculation_result.status` doit etre `completed` avant le mapping.
- `astro_result.data` recoit **uniquement** `audit_payload.payload`, pas l enveloppe complete.
- `interpretation_profile_code` est obligatoire pour `product_code: natal_prompter`.
- `response_contract.generation_mode` peut etre omis ou incorrect : l API l aligne sur le profil via `InterpretationProfileResolver`.

## Exemples

- [examples/natal_calculation_request_v1.paris_1990.json](examples/natal_calculation_request_v1.paris_1990.json)
- [examples/natal_calculation_response_v1.paris_1990.json](examples/natal_calculation_response_v1.paris_1990.json)
- [examples/generate_reading_request_v1.from_engine_paris_1990.json](examples/generate_reading_request_v1.from_engine_paris_1990.json)
