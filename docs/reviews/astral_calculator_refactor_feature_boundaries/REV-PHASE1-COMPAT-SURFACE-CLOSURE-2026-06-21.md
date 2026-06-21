Statut: closed

Objet:
- review adversariale orientee frontieres pour la fermeture de la surface de
  compatibilite calculator en fin de Phase 1.

Frontieres revues:
- `astral_calculator/src/domain/mod.rs` reste une facade de domaine stable, mais
  n'utilise plus de wildcard re-exports opaques;
- `astral_calculator/src/lib.rs` garde les aliases racine deprécies comme shims
  publics explicites, sans nouvel usage interne workspace;
- `astral_calculator_http/src/*` consomme le calculateur par les chemins
  canoniques `bootstrap::{db,env}` et `astrology::ephemeris`;
- `tests/refactor_governance_tests.rs` et les railguards verrouillent ces
  frontieres pour eviter une reintroduction progressive de la surface legacy.

Cycle 1 - Finding:
- F1: la borne "shims externes uniquement" etait implicite dans l'audit mais non
  enforcee, car `astral_calculator_http` importait encore les aliases racine
  deprécies.

Correction:
- migration des imports HTTP vers les modules canoniques;
- ajout d'un controle de gouvernance dedie aux imports canoniques de
  `astral_calculator_http`;
- mise a jour des railguards et de `docs/BASIC_PAYLOAD_IMPLEMENTATION.md` pour
  rendre la fermeture de cette borne explicite et tracable.

Verification:
- `rg -n "astral_calculator::(config|db|ephemeris)" astral_calculator_http`
- `rg -n "^pub use .*\\*;" astral_calculator/src/domain/mod.rs`
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator_http --test astral_calculator_http_tests`
- `cargo test -p astral_calculator`

Findings restants: Aucun

Aucun finding ouvert.
