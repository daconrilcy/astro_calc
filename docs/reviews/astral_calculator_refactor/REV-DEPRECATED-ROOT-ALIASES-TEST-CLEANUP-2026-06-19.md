# Review adversariale - nettoyage repo-wide des aliases racine deprécies - 2026-06-19

Portee:

- passe de nettoyage des tests internes pour remplacer les usages de
  `astral_calculator::{catalog,db,config,cli,dignities,ephemeris,facts,idempotency,aspects}`
  par leurs chemins canoniques;
- extension du garde-fou `internal_sources_do_not_use_historical_root_aliases`
  a `tests/`.

Cycle 1 - findings:

- P1: le garde-fou etendu interdisait les aliases dans tout `tests/`, mais la
  base n'avait plus aucun test dedie garantissant que les aliases publics
  deprécies compilaient encore pour la compatibilite externe progressive.

Corrections:

- ajout de `tests/deprecated_root_alias_compat_tests.rs` avec `#![allow(deprecated)]`
  pour verifier la compilation des aliases publics historiques;
- exemption explicite et minimale de ce fichier dans
  `tests/refactor_governance_tests.rs`.

Verification:

```powershell
cargo test -p astral_calculator --test deprecated_root_alias_compat_tests
cargo test -p astral_calculator --test refactor_governance_tests
cargo test -p astral_calculator
cargo test -p astral_calculator_http --test astral_calculator_http_tests
```

Conclusion:

- Aucun finding ouvert.
