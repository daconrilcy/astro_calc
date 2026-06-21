Statut: closed

Objet:
- review adversariale du split infra des requetes de systemes de reference.

Finding:
- Aucun finding bloquant apres correction. Le risque principal etait de creer
  une nouvelle surface publique ou de deplacer du SQL hors de `infra/db`.

Preuves:
- `astral_calculator/src/infra/db/runtime_queries/reference/systems.rs`
  contient les methodes `RuntimeQueries` liees aux systemes de reference;
- `astral_calculator/src/infra/db/runtime_queries/reference.rs` declare le
  sous-module et conserve les autres familles existantes;
- le fichier hub passe de 698 a 558 lignes, sans changement de noms de
  methodes ni de schema.

Verification:
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator`

Findings restants: Aucun

Aucun finding ouvert.
