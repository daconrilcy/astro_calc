Statut: closed

Objet:
- follow-up adversarial de la tranche `included_days` apres verification de
  l'audit, du plan et de l'implementation courante.

Finding:
- F1: `astral_calculator/src/infra/db/horoscope_repository.rs` contenait encore
  un module `#[cfg(test)]` inline. Cela contredisait la regle workspace qui
  place les tests de comportement sous `tests/` et rendait le rapport d'audit
  "0 inline tests" faux pour l'etat courant.

Correction:
- retrait du module inline;
- ajout du garde `calculator_production_source_does_not_contain_inline_tests`
  dans `tests/refactor_governance_tests.rs`;
- ajout du garde
  `horoscope_repository_keeps_included_days_decode_contextualized_at_adapter_edge`
  pour prouver que le decode reste au bord repository avec une erreur
  contextualisee.

Verification:
- `rg -n "#\[cfg\(test\)\]|#\[test\]" astral_calculator/src`
- `cargo test -p astral_calculator --test horoscope_builders_tests`
- `cargo test -p astral_calculator --test refactor_governance_tests`

Findings restants: Aucun

Aucun finding ouvert.
