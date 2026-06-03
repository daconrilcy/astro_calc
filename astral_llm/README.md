# astral_llm — Gateway LLM astrologique

Service HTTP independant du moteur `astral_calculator`. Transforme un resultat
astrologique calcule en lecture interpretative structuree.

## Demarrage

```powershell
# Copier et renseigner .env a la racine du depot
cargo run -p astral_llm_api
```

## Crates

| Crate | Role |
|---|---|
| `astral_llm_domain` | Contrats request/response |
| `astral_llm_application` | Use cases, safety, prompts, router |
| `astral_llm_providers` | Adapters OpenAI, Anthropic, Mistral, Fake |
| `astral_llm_infra` | Config `.env`, persistence, referentiel canonique |
| `astral_llm_api` | Serveur Axum |

## Securite

- Auth optionnelle via `ASTRAL_LLM_API_KEY` (header `Authorization: Bearer` ou `X-API-Key`)
- Bind local par defaut (`127.0.0.1:8081`)
- Limites body / domaines / timeout configurables
- Anti-injection sur payload astro et `custom_instructions`
- Pas de FakeProvider implicite en prod (`ASTRAL_LLM_ENABLE_FAKE=false`)

## Tests

```powershell
cargo test -p astral_llm_api --test astral_llm_tests
cargo test -p astral_llm_application
```

Documentation detaillee : `Astral_llm_implementation.md`
