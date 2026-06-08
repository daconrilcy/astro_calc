# REV-CONNECT-009-editorial-quality - correction du residuel fixtures premium

## Scope

Correction du residuel `cargo test -p astral_llm_api --test astral_llm_editorial_fixtures`.

## Findings

### P2 - Densite `astro_basis` premium non retraitable

Les fixtures `natal_premium_psychological_fr` et `natal_premium_traditional_en` avaient 2 entrees `astro_basis` par chapitre alors que la policy evidence du profil `natal_premium` impose 4 references minimum par chapitre.

Correction: ajout de `AstroBasisDensityProcessor` dans `text_reprocessing`. Le processor utilise `TextRetreatmentRequestContext.min_astro_basis_per_chapter` et complete les chapitres sous-denses uniquement a partir de preuves autorisees: `allowed_evidence_by_chapter` pour les lectures multi-chapitres, `allowed_evidence_keys` seulement pour un payload mono-chapitre.

Correction de branchement: `EditorialValidator` valide une copie de la lecture apres passage par `reprocess_natal_theme_with_context`. Pour les fixtures statiques sans evidence pack, le seuil utilise est le seuil qualite de base du profil, pas le minimum de la policy evidence complete.

### P1 - Risque de fabrication d'evidence par densite

Review adversariale: une premiere version du processor ajoutait des `fact_id` synthetiques `text_reprocessing:density:*`, ce qui pouvait satisfaire le compteur sans preuve canonique.

Correction: le processor ne fabrique plus de `fact_id`. Sans `allowed_evidence_keys`, il ne modifie pas le payload et emet `astro_basis_density_insufficient_allowed_evidence:<chapter_code>`.

### P2 - Facteur public mal derive depuis certains `fact_id`

Review adversariale: `factor_from_fact_id` utilisait le dernier segment du `fact_id`. Pour `placement:moon:pisces:house:4`, cela produisait `factor = "4"` au lieu de `moon`.

Correction: parsing par famille de `fact_id` (`placement`, `aspect`, `angle`, `ruler`, `domain_score`, balances, etc.) pour produire un facteur public stable.

### P1 - Evidence globale reutilisable sur le mauvais chapitre

Review adversariale: `allowed_evidence_keys` etait une liste globale. Sur une lecture multi-chapitres, le processor pouvait completer `identity` avec une preuve destinee a `relationships`.

Correction: ajout de `TextChapterEvidenceKeys` et de `TextRetreatmentRequestContext.allowed_evidence_by_chapter`. Sur une lecture multi-chapitres, le processor exige des preuves scopees par `chapter_code`; la liste globale reste acceptee uniquement pour un payload mono-chapitre.

Tests:

- `text_reprocessing_natal_theme_completes_astro_basis_density`
- `text_reprocessing_natal_theme_density_does_not_fabricate_evidence`
- `text_reprocessing_natal_theme_density_requires_chapter_scoped_evidence_for_multi_chapter`
- `text_reprocessing_natal_theme_density_uses_chapter_scoped_evidence`
- `cargo test -p astral_llm_application text_reprocessing`
- `cargo test -p astral_llm_api --test astral_llm_editorial_fixtures`

## Residual findings

Aucun P0/P1/P2 ouvert sur le branchement `text_reprocessing`.
