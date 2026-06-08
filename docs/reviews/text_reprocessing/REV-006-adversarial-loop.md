# REV-006 - Cycle adversarial boucle jusqu'a absence de findings

## Perimetre

Deuxieme passe complete apres correction de `REV-005`: mutation des champs JSON, audit fallback, extensibilite processors, validation qualite et typographie FR.

## Findings corriges

### P2 - Mutations appliquees aux champs techniques

Les processors de sanitation, typographie, longueur et repetition pouvaient modifier des champs comme `theme_code`, `fact_id`, `role`, `label` ou `interpretive_role`.

Correction:

- Ajout de `mutate_public_text_strings`.
- Protection centralisee via `is_technical_string_path`.
- Test ajoute: `text_reprocessing_public_text_processors_preserve_technical_fields`.

### P2 - Fallback structurant destructif sur payload non objet

`FallbackTextProcessor` et `PromptGuidanceProcessor` pouvaient transformer un texte brut en objet vide avant d'ajouter leurs champs.

Correction:

- Les processors retournent un warning et ne mutent pas le payload si le payload n'est pas un objet.
- Test ajoute: `text_reprocessing_fallback_does_not_drop_plain_text_payloads`.

### P2 - Audit fallback non specialise

Les fallbacks etaient audites comme `changed`, laissant `FallbackApplied` inutilise.

Correction:

- Ajout de `ProcessorOutcome::fallback_paths`.
- Audit `fallback_applied` avec action `FallbackApplied`.
- Test ajoute: `text_reprocessing_fallback_uses_dedicated_audit_action`.

### P2 - Processors structures trop fermes aux services ajoutes

`FallbackTextProcessor`, `PromptGuidanceProcessor`, `TraceFormattingProcessor` et `AstroBasisProcessor` etaient limites aux services/langues connus alors que leur logique peut etre pilotee par operation, registre ou forme du payload.

Correction:

- Supports ouverts (`&[]`) pour les processors generiques/structurels.
- Test ajoute: `text_reprocessing_registered_new_service_can_use_fallback`.

### P2 - Detection injection ignoree dans les champs techniques

Le filtre de texte public empechait aussi de detecter une injection cachee dans un champ technique.

Correction:

- `ScriptSanitizerProcessor` scanne toutes les chaines pour les violations.
- Il ne nettoie/recrit que les chaines publiques.
- Test ajoute: `text_reprocessing_sanitizer_detects_injection_in_technical_fields_without_rewriting_them`.

### P2 - Validation qualite comptait les codes comme texte public

Un payload compose uniquement de codes pouvait ne pas declencher `empty_public_text`.

Correction:

- `collect_public_text` ignore les chemins techniques.
- Test ajoute: `text_reprocessing_quality_ignores_technical_fields_for_public_text_presence`.

### P2 - Typographie FR cassait les URLs et heures

La normalisation `:` pouvait transformer une URL ou une heure (`12:30`).

Correction:

- Preservation des heures numeriques et tokens URL contenant `://`.
- Test ajoute: `text_reprocessing_typography_preserves_urls_and_times`.

## Findings restants

Aucun P0/P1/P2/P3 ouvert apres ce cycle.

## Verification

- `cargo test -p astral_llm_application text_reprocessing`
- `cargo test -p astral_llm_domain text_reprocessing`
