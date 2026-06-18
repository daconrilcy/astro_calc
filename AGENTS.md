# Workspace Rules

- **Git** : ne pas creer de branches. Tous les commits doivent rester directement sur `main`.
- **Rust** : workspace Cargo a la racine (`astral_calculator`, `astral_calculator_http`, `astral_llm`). Commandes :
  `cargo test -p astral_calculator`, `cargo run -p astral_calculator`,
  `cargo run -p astral_calculator_http`, `cargo test -p astral_calculator_http --test astral_calculator_http_tests`,
  `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`,
  `cargo run -p astral_llm_api`, `cargo run -p astral_llm_worker`, `cargo test -p astral_llm_api --test astral_llm_tests`,
 `cargo test -p astral_llm_api --test integration_services_tests`, `cargo test -p astral_llm_api --test integration_jobs_tests`,
 `cargo test -p astral_llm_api --test astral_llm_editorial_fixtures`,
 `cargo test -p astral_llm_api --test astral_llm_load_tests`,
 `cargo test -p astral_llm_api --test astral_llm_interpretation_profile_tests`,
 `cargo test -p astral_llm_api --test astral_llm_evidence_planner_tests`,
 `cargo test -p astral_llm_api --test astral_llm_simplified_reading_tests`,
 `cargo test -p astral_llm_application simplified_reading_guard`,
 `cargo test -p astral_llm_application simplified_reading_postprocess`,
 `cargo test -p astral_llm_providers --test provider_real_smoke -- --ignored`.
- **Docker Compose local** : voir le guide complet [docs/GUIDE_DEBUTANT_DOCKER.md](docs/GUIDE_DEBUTANT_DOCKER.md). Demarrage rapide : `docker compose up -d --build` (reseau `astral_net`, calculateur `:8080`, LLM `:8081`, worker jobs, Mercure `:3000`, PostgreSQL interne). Import referentiel initial (1re fois) : `python scripts/import_json_db_to_postgres.py` (postgres up). Catalogue integration : `.\scripts\manage_integration_services.ps1 -Submit`. Ephemerides : `./ephe/se-2026a/*.se1`. **`DATABASE_URL` obligatoire** dans `.env` pour bootstrap. Override port DB hote : `docker compose -f docker-compose.yml -f docker-compose.dev-db-port.yml up -d`. Bootstrap : `.\scripts\docker_bootstrap.ps1`. Smoke HTTP fake : `.\scripts\docker_compose_smoke.ps1`. Smoke integration jobs : `.\scripts\test_integration_jobs_e2e.ps1` (natal_simplified async, worker requis). E2E full natal jobs : `.\scripts\test_natal_from_birth_e2e.ps1`. Contrats publics : `contracts/`. Doc integration : `docs/integration_api_contract.md`, `docs/integration_api_guide.md`. Test coherence schemas : `cargo test -p astral_llm_api --test contracts_publish_tests`.
- **Archives sync supprimees** : les anciens parcours sync retires du runtime ne doivent plus etre utilises comme outillage courant.
- **E2E lecture longue** : `.\scripts\test_natal_premium_profile.ps1`, `.\scripts\generate_premium_reading_e2e.ps1`, `.\scripts\test_natal_premium_plus_profile.ps1` et `.\scripts\generate_premium_plus_reading_e2e.ps1` testent directement le rendu lecture via l'endpoint interne LLM `POST /v1/internal/readings/render`. Seuils premium plus : **520/720/850** mots, 6 basis domaine, synthesis 520 mots / 4 basis depuis `natal_premium_plus.json` ; `chapter_length_expansion_codes` pour chapitres souvent courts ; `-SkipGenerate` pour revalider une sortie existante. Comparaison baseline : `.\scripts\compare_premium_plus_versions.ps1`. Certification v2 : run `e76a8156`, sortie ref. `output\premium_plus_reading_e2e_v2d.json` (5 517 mots). Final polish (P3e) : run `31d81052` (5 582 mots). **Premium Plus final certification — CLOSED** : run `673f2950`, sortie `output\premium_plus_reading_e2e.json` (5 537 mots, summary UX compact 64 mots / 2 phrases, gates polish + summary hardening + audit SQL, 0 `repair_too_short`). Perimetre gele : `ChapterWritingGuidance`, `ChapterEvidencePlanner`, `FinalSynthesisSynthesizer`, `SummarySynthesizer`, `ReadingOpeningDiversityValidator`, `ReadingQualityValidator`, `AstroLabelHumanizer`, `test_natal_premium_plus_profile.ps1`. Fixture `request-premium-plus-rich.json`, `domain_count: 8`. Requis `.env` : `ASTRAL_LLM_REQUEST_TIMEOUT_MS=900000`, `OPENAI_API_KEY`.
- **Donnees canoniques** : tout element referentiel (codes, libelles, seuils, mappings, definitions) provient de la base de donnees. Aucune constante en dur dans le code si la valeur peut etre en base. Natal simplifie : exclusions profil (`profile_excluded_feature_codes`) via table `astral_simplified_profile_feature_exclusions` (seed `json_db/`, import `python scripts/import_json_db_to_postgres.py`).
- **Refacto astral_calculator** :
  DB uniquement sous `infra/db` ;
  aucune dependance `domain -> infra` ;
  aucun `connect_from_env`, `PgPool`, `block_on` ou `run_blocking` dans les couches metier (`domain`, `engine`, `horoscope`, `simplified`, regles pures) ;
  aucune constante canonique codee en dur si la valeur peut venir de la DB ;
  les features produit (`natal`, `simplified`, `horoscope`) sont des orchestrateurs de contrats: elles valident l'entree, chargent les donnees via repositories, appellent les calculs communs, puis assemblent leur sortie ;
  les calculs astrologiques intrinseques reutilisables doivent vivre sous `astrology/` (`aspects`, `ephemeris`, positions, maisons, transits si necessaire), pas sous une feature produit ;
  une feature ne doit pas importer les details internes d'une autre feature. Exemple interdit: `simplified` ou `horoscope` important `crate::natal::aspects` ou `crate::natal::ephemeris` ;
  `features/shared` est interdit pour les calculs metier astrologiques; utiliser un module au nom metier explicite sous `astrology/` ;
  les anciens chemins publics peuvent rester en wrappers/re-exports temporaires, mais tout nouveau code doit utiliser les chemins canoniques ;
  les changements de structure doivent conserver les contrats JSON publics, sauf decision documentee explicitement ;
  chaque vague de refacto documentee dans `docs/BASIC_PAYLOAD_IMPLEMENTATION.md` avec un resume court, les invariants de couche, les commandes de verification et les liens vers les reviews ;
  chaque vague doit avoir une review adversariale sous `docs/reviews/astral_calculator_refactor_feature_boundaries/`, suivie de corrections puis re-review jusqu'a `Aucun finding ouvert` ;
  reviews adversariales obligatoires par vague sous `docs/reviews/astral_calculator_refactor/`.
- **Moteur natal** : produit unique `natal_prompter` + profil `interpretation_profile_code` (`natal_light`, `natal_basic`, `natal_premium`, `natal_premium_plus`). Profils JSON : `config/natal_interpretation_profiles/`, commande `.\scripts\manage_natal_interpretation_profiles.ps1` (-Submit, -List, -Get, -Delete), puis redemarrer `astral_llm_api`.
- **Modeles LLM par produit** : editer `config/llm_product_models.conf`, puis `.\scripts\set_product_llm_models.ps1`, redemarrer `astral_llm_api`.
- **Processus base avant code** : (1) verifier que la table existe et contient les lignes necessaires ; (2) inserer les valeurs absentes ; (3) sinon creer la table avec les jointures correctes vers les tables de reference ; (4) puis consommer ces donnees depuis la base dans le code (repository / runtime).
- Tous les tests doivent etre enregistres dans le repertoire `tests` a la racine du projet.
- Chaque nouvelle implementation doit etre decrite dans `BASIC_PAYLOAD_IMPLEMENTATION.md`.
- Toute implementation doit suivre scrupuleusement les principes YAGNI, KISS, DRY et SOLID.
