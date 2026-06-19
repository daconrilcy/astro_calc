Statut: closed

Objet:
- review adversariale orientee frontieres pour la finalisation de la vague de petits ports de references.

Frontieres revues:
- `engine/application` depend uniquement des ports de references necessaires;
- `features/horoscope/application` depend des ports explicites `ReferenceSystemResolver + NatalReferenceStore`;
- `features/simplified` ne revient pas au trait composite `ReferenceCatalog`;
- la verification `simplified_natal_tests` couvre la zone fonctionnelle touchee.

Cycle 1 - Finding:
- F1: absence d'artefacts de review fermes specifiques a cette sous-vague finale, alors meme qu'elle change les bornes de dependance des services applicatifs.

Correction:
- ajout des artefacts de review dans les deux repertoires de gouvernance;
- verrouillage de leur presence par `refactor_governance_tests`.

Verification:
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator --features "swisseph-engine,test-utils" --test simplified_natal_tests`

Findings restants: Aucun

Aucun finding ouvert.
