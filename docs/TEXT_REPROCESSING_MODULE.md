# Module `text_reprocessing`

## Objectif v1

Le module `text_reprocessing` centralise les fonctionnalites de retraitement des textes LLM sans etre branche aux flux applicatifs existants.

La v1 sert a valider les contrats, registres, processors, fixtures par service et audits. Les services `horoscope`, `natal`, `calculator_projection`, `prompt_trace` et les pipelines existants restent inchanges.

## Architecture

- Contrats: `astral_llm_domain::text_reprocessing`.
- Implementation: `astral_llm_application::text_reprocessing`.
- Classe mere Rust: trait `TextRetreatmentProcessor`.
- Classes specialisees: structs de processors (`ScriptSanitizerProcessor`, `TypographyProcessor`, etc.).
- Orchestrateur: `TextRetreatmentPipeline`.
- Extensibilite:
  - `LanguageRegistry` associe un code langue a un `LanguageRuleSet`.
  - `ServiceRegistry` associe un code service a un `ServiceRuleSet`.
  - `ProcessorRegistry` ordonne les processors.

Les codes langue/service sont des `String` validees par registres, pas des enums fermes. Les constantes fournissent les codes connus pour limiter les fautes de frappe.

## Contrats

Entree minimale:

```json
{
  "language": { "code": "fr" },
  "service": { "code": "horoscope_period" },
  "target": "horoscope_period_response",
  "operations": ["typography", "reduce_repetition", "normalize_length"],
  "payload": {
    "summary": {
      "title": "Vos 7 prochains jours",
      "text": "l impression demande de garder une marge. gardez une marge."
    }
  },
  "context": {
    "word_limits": { "min_words": null, "max_words": 60, "hard_limit_words": 90 }
  }
}
```

Sortie:

```json
{
  "payload": {
    "summary": {
      "title": "Vos 7 prochains jours",
      "text": "l'impression demande de garder une marge. préservez un espace de recul."
    }
  },
  "audit": [
    {
      "processor_id": "typography",
      "operation": "typography",
      "field_path": "$.summary.text",
      "action": "changed",
      "reason_code": "updated"
    }
  ],
  "warnings": [],
  "violations": [],
  "changed": true
}
```

## Processors v1

| Processor | Fonctionnalite | Services principaux |
| --- | --- | --- |
| `ScriptSanitizerProcessor` | sanitation alphabet FR et detection injection | shared, natal, horoscope |
| `TypographyProcessor` | elisions francaises et ponctuation FR protegee URL/heures | tout service FR via operation |
| `SentenceAndLengthProcessor` | trim, limites de mots, completion minimale | horoscope, natal |
| `RepetitionProcessor` | substitutions anti-repetition | tous services via `LanguageRuleSet` |
| `AstroLabelHumanizerProcessor` | humanisation de codes simples | tout service via operation et `LanguageRuleSet` |
| `AstroBasisProcessor` | normalisation `astro_basis.interpretive_role` | tout service contenant `astro_basis` |
| `QualityValidationProcessor` | checks texte public | horoscope, natal |
| `FallbackTextProcessor` | summary/advice fallback non destructif | tout service objet via `ServiceRuleSet` |
| `PromptGuidanceProcessor` | bloc guidance langue/repetition non destructif | tout service objet via operation |
| `TraceFormattingProcessor` | format trace `<<< role >>>` | tout service avec `messages` |

Les processors de sanitation, typographie, longueur et repetition ne modifient que les champs de texte public. Les chemins techniques (`code`, `*_code`, `id`, `*_id`, `role`, `label`, `factor`, `interpretive_role`) sont proteges contre les recritures. Le sanitizer scanne toutefois toutes les chaines pour detecter des injections. Le controle de longueur est plus restrictif que le texte public general: il ne cible pas les titres et ne s'applique qu'aux textes racine ou champs de corps (`text`, `body`, `content`, `advice`, `watch_point`, `main`).

## Ajouter une langue

1. Creer un `LanguageRuleSet` avec `code`, `sentence_prefix`, `default_summary_title`, `fallback_sentence`, `fallback_summary_text`, `fallback_advice`, `repetitive_replacements`, `humanized_labels`.
2. L'enregistrer via `LanguageRegistry::insert`.
3. Ajouter un test qui utilise cette langue avec au moins un processor generique.
4. Les processors non applicables doivent retourner un audit `skipped`.

## Ajouter un service

1. Creer un `ServiceRuleSet` avec `code`, `default_operations`, `word_limits`, `fallback_summary_title`.
2. L'enregistrer via `ServiceRegistry::insert`.
3. Ajouter une fixture de service dans `tests/text_reprocessing_application_tests.rs`.
4. Ne pas modifier les processors generiques si le service ne demande pas de logique specifique.

## Reason codes, warnings et audit

Actions:

- `changed`: le payload a ete modifie.
- `validated`: un controle est passe.
- `skipped`: processor non applicable ou aucune modification.
- `fallback_applied`: reserve pour les fallbacks explicites futurs.

Audit reason codes v1:

- `updated`
- `fallback_applied`
- `validated`
- `no_applicable_change`
- `unsupported_language_or_service`

Warnings v1:

- `unregistered_language:<code>`
- `unregistered_service:<code>`
- `fallback_requires_object_payload`
- `prompt_guidance_requires_object_payload`

Violation codes v1:

- `prompt_injection_like_text`
- `empty_public_text`
- `forbidden_wording`

## Strategie future de branchement

La v1 est isolee. Le branchement applicatif devra etre progressif:

1. transformer les anciennes fonctions en wrappers optionnels vers le pipeline;
2. verifier la parite via tests existants;
3. brancher d'abord `natal_simplified`, puis `horoscope_period`, puis `natal_theme`;
4. conserver un audit exploitable pour chaque modification de champ;
5. migrer ensuite les textes hardcodes vers la base/cataloque canonique.

## Commandes de verification

Les tests dedies sont declares comme targets Cargo et stockes dans `tests/` a la racine du projet:

- `tests/text_reprocessing_domain_tests.rs`
- `tests/text_reprocessing_application_tests.rs`

```powershell
cargo test -p astral_llm_domain text_reprocessing
cargo test -p astral_llm_application text_reprocessing
```
