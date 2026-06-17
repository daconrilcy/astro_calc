# REV-PHYSICAL-FEATURES-adversarial

- Status: `closed`
- Statut: closed
- Decision: `fix now`

## Scope

- Deplacement physique des orchestrateurs produit sous `src/features`.
- Conservation des wrappers legacy `src/natal`, `src/simplified`, `src/horoscope`.
- Gouvernance des frontieres entre `domain`, `infra/db`, `astrology` et features produit.

## Findings corriges

- F-001: la premiere implementation ne verrouillait pas le caractere strictement minimal des wrappers legacy. Correction: ajout du test `legacy_root_feature_modules_are_compatibility_wrappers_only`.
- F-002: la vague etait documentee dans les reviews feature-boundaries, mais pas dans le repertoire general obligatoire `docs/reviews/astral_calculator_refactor/`. Correction: ajout de cette review generale et d'un controle de gouvernance associe.
- F-003: la compatibilite des anciens chemins publics etait conservee par les wrappers, mais pas verrouillee explicitement par compilation. Correction: ajout du test `legacy_public_feature_paths_still_compile`.

## Resultat

Aucun finding ouvert.
