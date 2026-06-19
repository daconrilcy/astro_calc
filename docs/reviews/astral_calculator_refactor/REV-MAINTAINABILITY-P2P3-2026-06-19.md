# Review maintenabilite P2/P3 - 2026-06-19

Perimetre audite:

- lifecycle typé dans `application::ports` et mapping DB runtime
- split de `features/natal/application/natal_calculation_service.rs`
- typing de `visibility_context`
- deprecation des aliases historiques racine

Constat:

- `CalculationStatus` et `CalculationProgressState` gardent les valeurs DB
  canoniques via `as_str()` / `from_db_str()`.
- Le workflow natal n’utilise plus les chaînes lifecycle directes.
- Le service natal est découpé en sous-modules privés sans nouvelle surface
  publique.
- `visibility_context` est accessible via un accessor typé en conservant
  `facts_json`.

Conclusion:

- Aucun finding ouvert.
