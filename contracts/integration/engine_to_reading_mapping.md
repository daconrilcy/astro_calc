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

## Natal simplifie (v2.4)

Source : reponse `astro_simplified_natal_response_v1` :

```json
{
  "response_contract_version": "astro_simplified_natal_response_v1",
  "simplified_payload": {
    "payload_contract": "natal_simplified_structured_v1",
    "payload": { }
  },
  "llm_payload": {
    "profile_code": "natal_simplified",
    "allowed_fact_codes": ["mercury.sign"],
    "allowed_astro_basis_fact_ids": ["placement:mercury"],
    "blocked_interpretation_fact_codes": ["sun.sign", "moon.sign"],
    "excluded_feature_codes": ["ascendant", "houses", "sect", "house_placements"],
    "profile_excluded_feature_codes": ["ascendant", "houses", "sect", "house_placements"],
    "allowed_limitation_mentions": ["sun.sign", "moon.sign", "ascendant", "houses", "birth_time_missing"]
  },
  "reading_hint": {
    "recommended_profile_code": "natal_simplified",
    "reading_completeness": "partial"
  }
}
```

Mapping manuel vers `POST /v1/readings/generate` :

- `interpretation_profile_code`: **`natal_simplified`** (obligatoire)
- `astro_result.contract_version`: **`natal_simplified_structured_v1`**
- `astro_result.data`: `simplified_payload.payload` enrichi de `llm_controls` (= `llm_payload`)
- `response_contract.generation_mode`: **`single_pass`** (derive du profil)

Orchestration integree : `POST /v1/readings/natal/simplified` avec le corps `astro_simplified_natal_request_v1` + `user_language`.

Exemples : [examples/natal_simplified_examples.json](examples/natal_simplified_examples.json)
