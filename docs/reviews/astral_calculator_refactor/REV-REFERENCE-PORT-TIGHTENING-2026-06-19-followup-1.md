Statut: closed

Objet:
- re-review adversariale apres ajout des artefacts et de la garde de gouvernance pour la finalisation des petits ports de references.

Resultat:
- les services applicatifs concernes n'utilisent plus `ReferenceCatalog`;
- la revalidation `simplified_natal_tests` a ete executee sur la vague qui touche `simplified`;
- les artefacts de review sont presentes et fermes.

Verification:
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator`

Findings restants: Aucun

Aucun finding ouvert.
