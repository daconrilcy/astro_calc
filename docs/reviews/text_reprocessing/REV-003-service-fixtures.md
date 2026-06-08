# REV-003 - Service fixtures

## Perimetre

Fixtures isolees par service, sans appel aux services existants.

## Findings adversariales

- Aucun P0/P1/P2/P3 ouvert.
- Note: les fixtures couvrent chaque service cible de la v1 isolee. Les variantes de schemas publics seront ajoutees au moment du branchement progressif.

## Corrections appliquees

- Tests ajoutes pour `horoscope_daily`, `horoscope_period`, `natal_simplified`, `natal_theme`, `calculator_projection`, `prompt_trace`, `shared`.
- Les tests valident les outputs du module, pas une comparaison dynamique avec les anciens services.
