# Review adversariale maintenabilite P2/P3 - boucle 1 - 2026-06-19

Perimetre audite:

- typing lifecycle applicatif
- split `natal_calculation_service`
- typing `visibility_context`
- validation runtime des `house_axis_emphasis`

Findings:

1. `astral_calculator/src/features/natal/payload/validate/house_axes.rs`
   continuait a valider les axes via `canonical_axis()` / `axis_label()` codes
   en dur, alors que le builder et les references runtime reposent deja sur
   `HouseAxisReference`. Ce doublon laissait diverger la validation de payload
   du catalogue canonique charge depuis la DB.

2. `tests/natal_reuse_policy_tests.rs` utilisait encore des fixtures
   `HouseAxisReference` et `HouseReference` non canoniques, ce qui masquait la
   divergence precedente et ne couvrait plus le vrai contrat de reuse.

Conclusion:

- Findings ouverts, corrections requises avant cloture.
