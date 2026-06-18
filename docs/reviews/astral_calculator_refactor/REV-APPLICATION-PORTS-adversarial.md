# REV-APPLICATION-PORTS

Statut: closed

## Scope

Audit adversarial du découplage applicatif dans la vague courante.

## Findings initiaux

- Medium: les services applicatifs restent câblés par des repositories concrets pour les chemins transactionnels.
- Low: introduire des ports complets sur `NatalCalculationService` aurait augmenté le risque de régression transactionnelle dans cette vague.

## Corrections

- La vague limite le changement applicatif au chemin horoscope daily, où l'orchestration est isolée et testable sans modifier les transactions natales.
- Les responsabilités sont clarifiées: `runtime::build_runtime_service` reste le point de wiring concret; `astrology::transits` devient le port métier stable pour la logique de transit.
- La prochaine extraction de ports doit cibler d'abord les lectures sans transaction (`ReferenceCatalog`, `HoroscopeCatalog`, `ProjectionCatalog`) avant le store natal transactionnel.

## Re-review

Aucun finding ouvert.
