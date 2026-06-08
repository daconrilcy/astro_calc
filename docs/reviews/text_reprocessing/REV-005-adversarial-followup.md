# REV-005 - Review adversariale complete de l'implementation

## Perimetre

Contrats domain, pipeline application, processors, audit, extensibilite langue/service, tests et documentation.

## Findings corriges

### P2 - Typographie trop fermee aux nouveaux services

Le `TypographyProcessor` declarait `ALL_KNOWN_SERVICES`, ce qui contredisait l'objectif d'ajouter un service via `ServiceRegistry` sans modifier les processors.

Correction:

- `TypographyProcessor::supported_services()` est ouvert (`&[]`).
- Test ajoute: `text_reprocessing_typography_supports_registered_new_services`.

### P2 - Normalisation des deux-points FR inactive

La ligne de normalisation faisait un remplacement identique (`" :" -> " :"`), donc `Conseil:` n'etait pas corrige.

Correction:

- Ajout de `normalize_french_colon_spacing`.
- Test couvert via `text_reprocessing_typography_supports_registered_new_services`.

### P2 - Langue/service non enregistres invisibles

Une requete avec langue/service inconnus pouvait produire un resultat sans signal explicite, surtout avec processors generiques ouverts.

Correction:

- Le pipeline ajoute `unregistered_language:<code>` et `unregistered_service:<code>` dans `warnings`.
- Test ajoute: `text_reprocessing_unregistered_codes_are_visible_in_warnings`.

### P2 - Violations sans modification non auditees

Un processor pouvait detecter une violation sans `changed` ni `validated`, laissant l'audit incomplet.

Correction:

- Le pipeline ajoute un audit `validated` avec `reason_code = "violations_detected"` si un processor ne fait que produire des violations.
- Test ajoute: `text_reprocessing_violations_are_audited_even_without_changes`.

## Findings P3 corriges

### P3 - Humanisation minimale

`AstroLabelHumanizerProcessor` portait une petite table interne fermee.

Correction:

- Les libelles sont maintenant fournis par `LanguageRuleSet::humanized_labels`.
- Le processor est ouvert aux langues/services ajoutes par registre lorsque l'operation `humanize_labels` est demandee.
- Test couvert via `text_reprocessing_extensible_language_and_service_are_registry_driven`.

### P3 - Textes fallback hardcodes

Les textes fallback etaient disperses entre le service et une fonction de match par langue.

Correction:

- Les textes fallback publics sont maintenant portes par `LanguageRuleSet`.
- `ServiceRuleSet` garde uniquement le titre de fallback lie au service.

## Findings restants

Aucun P0/P1/P2/P3 ouvert apres corrections.

## Verification

- `cargo test -p astral_llm_domain text_reprocessing`
- `cargo test -p astral_llm_application text_reprocessing`

Les deux commandes passent apres corrections.
