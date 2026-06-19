Statut: closed

Perimetre:
- verification des nouvelles frontieres `application::ports`;
- verification de la sortie effective de `shared::astro_math` des modules `astrology/` et `features/`;
- verification que `runtime` reste une facade de wiring et que les helpers residuels vivent sous `runtime::compat`;
- verification que le contexte typé lit `facts_json` sans schema DB nouveau ni changement de contrat runtime.

Finding:
- la premiere extraction du workflow natal relachait la transaction verrouillee avant `insert_running_calculation`, ce qui cassait une invariance de serialisation presente avant le refacto.

Correction:
- `NatalReusePolicy` propage desormais le `tx` verrouille quand il faut poursuivre le calcul, et `NatalCalculationWorkflow` continue dans la meme transaction.

Findings restants: Aucun

Commandes executees:
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator --test position_fact_context_tests`
- `cargo test -p astral_calculator`
- `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`
