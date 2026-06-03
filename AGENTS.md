# Workspace Rules

- **Rust** : workspace Cargo a la racine (`astral_calculator`, `astral_llm`). Commandes :
  `cargo test -p astral_calculator`, `cargo run -p astral_calculator`,
  `cargo run -p astral_llm_api`, `cargo test -p astral_llm_api --test astral_llm_tests`,
 `cargo test -p astral_llm_api --test astral_llm_editorial_fixtures`,
 `cargo test -p astral_llm_api --test astral_llm_load_tests`,
 `cargo test -p astral_llm_providers --test provider_real_smoke -- --ignored`.
- **Donnees canoniques** : tout element referentiel (codes, libelles, seuils, mappings, definitions) provient de la base de donnees. Aucune constante en dur dans le code si la valeur peut etre en base.
- **Processus base avant code** : (1) verifier que la table existe et contient les lignes necessaires ; (2) inserer les valeurs absentes ; (3) sinon creer la table avec les jointures correctes vers les tables de reference ; (4) puis consommer ces donnees depuis la base dans le code (repository / runtime).
- Tous les tests doivent etre enregistres dans le repertoire `tests` a la racine du projet.
- Chaque nouvelle implementation doit etre decrite dans `BASIC_PAYLOAD_IMPLEMENTATION.md`.
- Toute implementation doit suivre scrupuleusement les principes YAGNI, KISS, DRY et SOLID.
