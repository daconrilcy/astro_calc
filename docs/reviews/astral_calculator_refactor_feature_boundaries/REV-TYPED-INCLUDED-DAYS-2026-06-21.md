Statut: closed

Objet:
- review adversariale orientee frontieres pour la tranche 2026-06-21 qui tape
  `included_days` a la frontiere application/adaptateur du builder horoscope.

Frontieres revues:
- `astral_calculator/src/application/ports.rs` expose des jours types
  `Option<Vec<String>>` au lieu d'un blob JSON;
- `astral_calculator/src/infra/db/horoscope_repository.rs` reste l'unique
  endroit autorise pour decoder `included_days` depuis SQLx/JSON;
- `astral_calculator/src/features/horoscope/builders.rs` ne decode plus le
  JSON et consomme seulement des donnees deja typees;
- aucun import `infra/db` n'entre dans `application`, et aucun contrat JSON
  public n'est modifie par cette tranche.

Cycle 1 - Finding:
- F1: le deplacement du decode vers `infra/db` fermait bien la frontiere
  applicative, mais il manquait une defense repository-wide contre le retour du
  decode JSON dans le builder horoscope.

Correction:
- ajout d'un garde-fou de gouvernance interdisant
  `serde_json::from_value::<Vec<String>>` dans
  `src/features/horoscope/builders.rs`;
- mise a jour de `docs/BASIC_PAYLOAD_IMPLEMENTATION.md` pour tracer cette
  sous-vague.

Verification:
- `cargo test -p astral_calculator --test horoscope_builders_tests`
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `rg -n "included_days|serde_json::from_value::<Vec<String>>" astral_calculator/src/application/ports.rs astral_calculator/src/infra/db/horoscope_repository.rs astral_calculator/src/features/horoscope/builders.rs`

Findings restants: Aucun

Aucun finding ouvert.
