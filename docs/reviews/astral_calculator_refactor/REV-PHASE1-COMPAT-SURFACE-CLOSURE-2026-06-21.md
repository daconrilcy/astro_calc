Statut: closed

Objet:
- review adversariale de la sous-vague Phase 1 qui ferme `P1-T3` en gardant les
  shims de compatibilite explicites, en supprimant les derniers usages internes
  workspace des aliases racine deprécies, et en remplaçant les wildcard
  re-exports de `astral_calculator/src/domain/mod.rs`.

Perimetre:
- `astral_calculator/src/domain/mod.rs`
- `astral_calculator/src/lib.rs`
- `astral_calculator_http/src/config.rs`
- `astral_calculator_http/src/reference_status.rs`
- `astral_calculator_http/src/routes.rs`
- `astral_calculator_http/src/state.rs`
- `tests/refactor_governance_tests.rs`
- `docs/BASIC_PAYLOAD_IMPLEMENTATION.md`
- `RAILGUARD.md`
- `astral_calculator/RAILGUARD.md`

Cycle 1 - Finding:
- F1: les aliases racine `astral_calculator::{config,db,ephemeris}` restaient
  utilises par `astral_calculator_http`, ce qui contredisait la fermeture
  explicite de `P1-T3` et maintenait une dependance interne sur la surface
  legacy.

Correction:
- migration de `astral_calculator_http/src` vers
  `astral_calculator::bootstrap::{db,env}` et
  `astral_calculator::astrology::ephemeris`;
- remplacement des wildcard `pub use ...::*;` de `src/domain/mod.rs` par une
  facade explicite;
- ajout d'un garde-fou de gouvernance qui interdit a `astral_calculator_http`
  d'utiliser les aliases racine deprécies;
- mise a jour de la documentation et des railguards pour figer le statut
  "shims externes uniquement".

Verification:
- `cargo fmt --check`
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator --test deprecated_root_alias_compat_tests`
- `cargo test -p astral_calculator --test natal_reuse_policy_tests`
- `cargo test -p astral_calculator --test runtime_identity_bootstrap_tests`
- `cargo test -p astral_calculator_http --test astral_calculator_http_tests`
- `cargo test -p astral_calculator`

Findings restants: Aucun

Aucun finding ouvert.
